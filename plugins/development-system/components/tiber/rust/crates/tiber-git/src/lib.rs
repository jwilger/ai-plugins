use crate::git_event_store::{GitEventStore, SynchronizeOutcome};
use eventcore::model::{ModelCommandLogic, Modeled, ModeledEvents};
use eventcore::{
    mapping, ModelCommand, ModelInput, ModelOutput, ModelState, RetryPolicy, StreamIdentity,
};
use eventcore_types::{BatchSize, EventFilter, EventPage, EventReader, EventStoreError, StreamId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(test)]
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::fs::{OpenOptions, TryLockError};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tiber_core::task::{
    ChecklistItem, Claim, FinalReviewCompletionSnapshot, FinalReviewOutcome, FinalReviewRecord,
    Note, Subtask, Task, ValidationRepair,
};
use tiber_core::{events::*, BoardSnapshot, OrderReconciliation, TaskSnapshot, TaskTitle};

pub mod git_event_store;

const STATUS_DIRS: &[&str] = &["backlog", "in-progress", "done", "abandoned"];
const OPEN_STATUS_DIRS: &[&str] = &["backlog", "in-progress"];
const TASK_ID_ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789";
const DEFAULT_LOCK_RETRY_TIMEOUT: Duration = Duration::from_secs(3);
const DEFAULT_LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(50);
const CONFIG_FILE: &str = ".tiber.toml";
const MAX_SYNC_ATTEMPTS: usize = 8;
const CI_RECOVERY_LEASE_SECONDS: u64 = 60 * 60;
const CI_RECOVERY_TEXT_MAX_BYTES: usize = 16 * 1024;
const FINAL_REVIEW_GIT_OUTPUT_MAX_BYTES: usize = 16 * 1024 * 1024;
const FINAL_REVIEW_SCOPE_MAX_PATHS: usize = 100_000;
const FINAL_REVIEW_SCOPE_MAX_CONTENT_BYTES: u64 = 64 * 1024 * 1024;
const FINAL_REVIEW_FINGERPRINT_CACHE_MAX_ENTRIES: usize = 256;
#[cfg(test)]
thread_local! {
    static FINAL_REVIEW_TREE_GIT_PROCESS_COUNT: Cell<u64> = const { Cell::new(0) };
}

#[cfg(test)]
fn count_final_review_tree_git_process() {
    FINAL_REVIEW_TREE_GIT_PROCESS_COUNT
        .set(FINAL_REVIEW_TREE_GIT_PROCESS_COUNT.get().saturating_add(1));
}

#[cfg(test)]
fn final_review_tree_git_process_count() -> u64 {
    FINAL_REVIEW_TREE_GIT_PROCESS_COUNT.get()
}
const REPOSITORY_STREAM: &str = "tiber:repository";
const BOARD_STREAM: &str = "tiber:board";
const CI_RECOVERY_STREAM: &str = "tiber:ci-recovery";

#[derive(ModelInput)]
struct BacklogCapacityInput {
    #[model(origin)]
    queued: usize,
    #[model(origin)]
    max_queued: usize,
}

#[derive(ModelOutput)]
struct BacklogAdmissionAllowed {
    value: bool,
}

fn has_backlog_capacity(queued: &usize, max_queued: &usize) -> bool {
    queued < max_queued
}

mapping! { BacklogCapacityToAdmission: (BacklogCapacityInput.queued, BacklogCapacityInput.max_queued) => BacklogAdmissionAllowed.value using has_backlog_capacity; }

fn backlog_admission_allowed(queued: usize, max_queued: usize) -> bool {
    let input = BacklogCapacityInput::model_builder()
        .queued(queued)
        .max_queued(max_queued)
        .build();
    BacklogAdmissionAllowed::model_builder()
        .value(BacklogCapacityToAdmission::apply((
            input.as_ref(),
            input.as_ref(),
        )))
        .build()
        .into_inner()
        .value
}

#[cfg(test)]
fn check_tiber_model() -> Result<eventcore::model::CheckReport, eventcore::model::CheckError> {
    eventcore::model::check()
}

#[derive(Clone, Debug, Eq, PartialEq, StreamIdentity)]
struct CiRecoveryStream(StreamId);

// Compatibility events are origins from historical streams, not facts emitted
// by fresh domain commands.
#[derive(ModelInput)]
struct LegacyTaskStatePublishedHistory {
    #[model(origin)]
    payload: TaskStatePublishedEvent,
}

mapping! {
    LegacyTaskStatePublishedHistoryToEvent:
        LegacyTaskStatePublishedHistory.payload => TiberEvent.LegacyTaskStatePublished
        using clone;
}

#[derive(ModelInput)]
struct LegacyRecoveryStatePublishedHistory {
    #[model(origin)]
    payload: CiRecoveryEvent,
}

#[derive(ModelInput)]
struct LegacyTaskClaimChangedHistory {
    #[model(origin)]
    payload: TaskClaimChangedEvent,
}

mapping! {
    LegacyTaskClaimChangedHistoryToEvent:
        LegacyTaskClaimChangedHistory.payload => TiberEvent.LegacyTaskClaimChanged
        using clone;
}

#[derive(ModelInput)]
struct LegacyTaskRemovedHistory {
    #[model(origin)]
    payload: TaskStemEvent,
}

#[derive(ModelInput)]
struct LegacyTaskClosedFromTrailerHistory {
    #[model(origin)]
    payload: TaskStemEvent,
}

mapping! {
    LegacyTaskClosedFromTrailerHistoryToEvent:
        LegacyTaskClosedFromTrailerHistory.payload => TiberEvent.LegacyTaskClosedFromTrailer
        using clone;
}

mapping! {
    LegacyTaskRemovedHistoryToEvent:
        LegacyTaskRemovedHistory.payload => TiberEvent.LegacyTaskRemoved
        using clone;
}

mapping! {
    LegacyRecoveryStatePublishedHistoryToEvent:
        LegacyRecoveryStatePublishedHistory.payload => TiberEvent.LegacyRecoveryStatePublished
        using clone;
}

fn validate_historical_task_state_publication(payload: &TaskStatePublishedEvent) {
    let input = LegacyTaskStatePublishedHistory::model_builder()
        .payload(payload.clone())
        .build();
    let _ = LegacyTaskStatePublishedHistoryToEvent::apply(input.as_ref());
}

/// Typed boundary between persisted domain facts and the task/board/CI read
/// projection. The projector below consumes this mapped value, so the checked
/// model covers both sides of every durable fact rather than stopping at the
/// command append boundary.
#[derive(ModelOutput)]
struct TiberProjectionEvent {
    event: TiberEvent,
}

macro_rules! tiber_projection_mapping {
    ($variant:ident, $payload:ty, $function:ident, $mapping:ident) => {
        fn $function(payload: &$payload) -> TiberEvent {
            TiberEvent::$variant(payload.clone())
        }
        mapping! { $mapping: TiberEvent.$variant => TiberProjectionEvent.event using $function; }
    };
}

tiber_projection_mapping!(
    RepositoryInitialized,
    RepositoryInitializedEvent,
    project_repository_initialized,
    ProjectRepositoryInitialized
);
tiber_projection_mapping!(
    TaskCreated,
    TaskCreatedEvent,
    project_task_created,
    ProjectTaskCreated
);
tiber_projection_mapping!(
    TaskTransitioned,
    TaskTransitionedEvent,
    project_task_transitioned,
    ProjectTaskTransitioned
);
tiber_projection_mapping!(
    TaskPriorityChanged,
    TaskOrderEvent,
    project_task_priority_changed,
    ProjectTaskPriorityChanged
);
tiber_projection_mapping!(
    TaskLinksChanged,
    TaskLinksChangedEvent,
    project_task_links_changed,
    ProjectTaskLinksChanged
);
tiber_projection_mapping!(
    TaskSubtaskAdded,
    TaskSubtaskAddedEvent,
    project_task_subtask_added,
    ProjectTaskSubtaskAdded
);
tiber_projection_mapping!(
    TaskSubtaskChecked,
    TaskSubtaskCheckedEvent,
    project_task_subtask_checked,
    ProjectTaskSubtaskChecked
);
tiber_projection_mapping!(
    TaskDetailsUpdated,
    TaskDetailsUpdatedEvent,
    project_task_details_updated,
    ProjectTaskDetailsUpdated
);
tiber_projection_mapping!(
    TaskPullRequestChanged,
    TaskPullRequestChangedEvent,
    project_task_pull_request_changed,
    ProjectTaskPullRequestChanged
);
tiber_projection_mapping!(
    TaskAcceptanceAdded,
    TaskAcceptanceAddedEvent,
    project_task_acceptance_added,
    ProjectTaskAcceptanceAdded
);
tiber_projection_mapping!(
    TaskAcceptanceChecked,
    TaskAcceptanceCheckedEvent,
    project_task_acceptance_checked,
    ProjectTaskAcceptanceChecked
);
tiber_projection_mapping!(
    TaskAcceptanceRemoved,
    TaskAcceptanceRemovedEvent,
    project_task_acceptance_removed,
    ProjectTaskAcceptanceRemoved
);
tiber_projection_mapping!(
    TaskNoteAdded,
    TaskNoteAddedEvent,
    project_task_note_added,
    ProjectTaskNoteAdded
);
tiber_projection_mapping!(
    TaskFinalReviewRecorded,
    TaskFinalReviewRecordedEvent,
    project_task_final_review_recorded,
    ProjectTaskFinalReviewRecorded
);
tiber_projection_mapping!(
    LegacyTaskClaimChanged,
    TaskClaimChangedEvent,
    project_task_claim_changed,
    ProjectLegacyTaskClaimChanged
);
tiber_projection_mapping!(
    TaskValidationRepaired,
    TaskValidationRepairedEvent,
    project_task_validation_repaired,
    ProjectTaskValidationRepaired
);
tiber_projection_mapping!(
    TasksClosedFromCommitTrailers,
    TasksClosedFromCommitTrailersEvent,
    project_tasks_closed_from_commit_trailers,
    ProjectTasksClosedFromCommitTrailers
);
tiber_projection_mapping!(
    LegacyTaskClosedFromTrailer,
    TaskStemEvent,
    project_task_closed_from_trailer,
    ProjectLegacyTaskClosedFromTrailer
);
tiber_projection_mapping!(
    LegacyTaskRemoved,
    TaskStemEvent,
    project_task_removed,
    ProjectLegacyTaskRemoved
);
tiber_projection_mapping!(
    BoardReordered,
    TaskOrderEvent,
    project_board_reordered,
    ProjectBoardReordered
);
tiber_projection_mapping!(
    LegacyTaskStatePublished,
    TaskStatePublishedEvent,
    project_legacy_task_state_published,
    ProjectLegacyTaskStatePublished
);
tiber_projection_mapping!(
    CiRecoveryClaimed,
    CiRecoveryClaimedEvent,
    project_ci_recovery_claimed,
    ProjectCiRecoveryClaimed
);
tiber_projection_mapping!(
    CiRecoveryJoined,
    CiRecoveryJoinedEvent,
    project_ci_recovery_joined,
    ProjectCiRecoveryJoined
);
tiber_projection_mapping!(
    CiRecoveryTransferred,
    CiRecoveryTransferredEvent,
    project_ci_recovery_transferred,
    ProjectCiRecoveryTransferred
);
tiber_projection_mapping!(
    CiRecoveryTakenOver,
    CiRecoveryTakenOverEvent,
    project_ci_recovery_taken_over,
    ProjectCiRecoveryTakenOver
);
tiber_projection_mapping!(
    CiRecoveryAssigned,
    CiRecoveryAssignedEvent,
    project_ci_recovery_assigned,
    ProjectCiRecoveryAssigned
);
tiber_projection_mapping!(
    CiRecoveryReported,
    CiRecoveryReportedEvent,
    project_ci_recovery_reported,
    ProjectCiRecoveryReported
);
tiber_projection_mapping!(
    CiRecoveryHeartbeatRecorded,
    CiRecoveryHeartbeatRecordedEvent,
    project_ci_recovery_heartbeat,
    ProjectCiRecoveryHeartbeat
);
tiber_projection_mapping!(
    CiRecoveryDiagnosed,
    CiRecoveryDiagnosedEvent,
    project_ci_recovery_diagnosed,
    ProjectCiRecoveryDiagnosed
);
tiber_projection_mapping!(
    CiRecoveryActionChosen,
    CiRecoveryActionChosenEvent,
    project_ci_recovery_action,
    ProjectCiRecoveryAction
);
tiber_projection_mapping!(
    CiRecoveryReplacementRecorded,
    CiRecoveryReplacementRecordedEvent,
    project_ci_recovery_replacement,
    ProjectCiRecoveryReplacement
);
tiber_projection_mapping!(
    CiRecoveryResolved,
    CiRecoveryResolvedEvent,
    project_ci_recovery_resolved,
    ProjectCiRecoveryResolved
);
tiber_projection_mapping!(
    LegacyRecoveryStatePublished,
    CiRecoveryEvent,
    project_legacy_recovery_state,
    ProjectLegacyRecoveryState
);

fn projection_event(event: &TiberEvent) -> TiberEvent {
    macro_rules! project {
        ($mapping:ident) => {
            TiberProjectionEvent::model_builder()
                .event($mapping::apply(event))
                .build()
                .into_inner()
                .event
        };
    }
    match event {
        TiberEvent::RepositoryInitialized(_) => project!(ProjectRepositoryInitialized),
        TiberEvent::TaskCreated(_) => project!(ProjectTaskCreated),
        TiberEvent::TaskTransitioned(_) => project!(ProjectTaskTransitioned),
        TiberEvent::TaskPriorityChanged(_) => project!(ProjectTaskPriorityChanged),
        TiberEvent::TaskLinksChanged(_) => project!(ProjectTaskLinksChanged),
        TiberEvent::TaskSubtaskAdded(_) => project!(ProjectTaskSubtaskAdded),
        TiberEvent::TaskSubtaskChecked(_) => project!(ProjectTaskSubtaskChecked),
        TiberEvent::TaskDetailsUpdated(_) => project!(ProjectTaskDetailsUpdated),
        TiberEvent::LegacyTaskClaimChanged(_) => project!(ProjectLegacyTaskClaimChanged),
        TiberEvent::TaskPullRequestChanged(_) => project!(ProjectTaskPullRequestChanged),
        TiberEvent::TaskAcceptanceAdded(_) => project!(ProjectTaskAcceptanceAdded),
        TiberEvent::TaskAcceptanceChecked(_) => project!(ProjectTaskAcceptanceChecked),
        TiberEvent::TaskAcceptanceRemoved(_) => project!(ProjectTaskAcceptanceRemoved),
        TiberEvent::TaskNoteAdded(_) => project!(ProjectTaskNoteAdded),
        TiberEvent::TaskFinalReviewRecorded(_) => project!(ProjectTaskFinalReviewRecorded),
        TiberEvent::TaskValidationRepaired(_) => project!(ProjectTaskValidationRepaired),
        TiberEvent::TasksClosedFromCommitTrailers(_) => {
            project!(ProjectTasksClosedFromCommitTrailers)
        }
        TiberEvent::LegacyTaskClosedFromTrailer(_) => {
            project!(ProjectLegacyTaskClosedFromTrailer)
        }
        TiberEvent::LegacyTaskRemoved(_) => project!(ProjectLegacyTaskRemoved),
        TiberEvent::BoardReordered(_) => project!(ProjectBoardReordered),
        TiberEvent::LegacyTaskStatePublished(_) => project!(ProjectLegacyTaskStatePublished),
        TiberEvent::CiRecoveryClaimed(_) => project!(ProjectCiRecoveryClaimed),
        TiberEvent::CiRecoveryJoined(_) => project!(ProjectCiRecoveryJoined),
        TiberEvent::CiRecoveryTransferred(_) => project!(ProjectCiRecoveryTransferred),
        TiberEvent::CiRecoveryTakenOver(_) => project!(ProjectCiRecoveryTakenOver),
        TiberEvent::CiRecoveryAssigned(_) => project!(ProjectCiRecoveryAssigned),
        TiberEvent::CiRecoveryReported(_) => project!(ProjectCiRecoveryReported),
        TiberEvent::CiRecoveryHeartbeatRecorded(_) => project!(ProjectCiRecoveryHeartbeat),
        TiberEvent::CiRecoveryDiagnosed(_) => project!(ProjectCiRecoveryDiagnosed),
        TiberEvent::CiRecoveryActionChosen(_) => project!(ProjectCiRecoveryAction),
        TiberEvent::CiRecoveryReplacementRecorded(_) => project!(ProjectCiRecoveryReplacement),
        TiberEvent::CiRecoveryResolved(_) => project!(ProjectCiRecoveryResolved),
        TiberEvent::LegacyRecoveryStatePublished(_) => project!(ProjectLegacyRecoveryState),
    }
}

/// Run compatibility mappings only for explicit legacy variants. Current facts
/// have their own command and projection mappings and must never be routed
/// through an adapter whose sole purpose is historical replay.
fn fold_explicit_legacy_tiber_fact(event: &TiberEvent) {
    match event {
        TiberEvent::LegacyTaskStatePublished(payload) => {
            validate_historical_task_state_publication(payload);
        }
        TiberEvent::LegacyRecoveryStatePublished(payload) => {
            validate_historical_recovery_state_publication(payload);
        }
        TiberEvent::LegacyTaskClaimChanged(payload) => {
            let input = LegacyTaskClaimChangedHistory::model_builder()
                .payload(payload.clone())
                .build();
            let _ = LegacyTaskClaimChangedHistoryToEvent::apply(input.as_ref());
        }
        TiberEvent::LegacyTaskRemoved(payload) => {
            let input = LegacyTaskRemovedHistory::model_builder()
                .payload(payload.clone())
                .build();
            let _ = LegacyTaskRemovedHistoryToEvent::apply(input.as_ref());
        }
        TiberEvent::LegacyTaskClosedFromTrailer(payload) => {
            let input = LegacyTaskClosedFromTrailerHistory::model_builder()
                .payload(payload.clone())
                .build();
            let _ = LegacyTaskClosedFromTrailerHistoryToEvent::apply(input.as_ref());
        }
        _ => {}
    }
}

fn validate_historical_recovery_state_publication(payload: &CiRecoveryEvent) {
    let input = LegacyRecoveryStatePublishedHistory::model_builder()
        .payload(payload.clone())
        .build();
    let _ = LegacyRecoveryStatePublishedHistoryToEvent::apply(input.as_ref());
}

#[derive(Clone)]
struct ClaimCiRecoveryIntent {
    incident_id: String,
    schema_version: u32,
    trigger: CiRecoveryTrigger,
    owner: CiRecoveryParticipant,
    lease_expires_at: u64,
}

#[derive(ModelInput)]
struct ClaimCiRecoveryRequest {
    #[model(origin)]
    stream: CiRecoveryStream,
    #[model(origin)]
    intent: ClaimCiRecoveryIntent,
}

#[derive(ModelCommand)]
struct ClaimCiRecovery {
    #[stream]
    stream: CiRecoveryStream,
    intent: ClaimCiRecoveryIntent,
}

mapping! {
    ClaimCiRecoveryRequestToStream:
        ClaimCiRecoveryRequest.stream => ClaimCiRecovery.stream
        using clone;
}

mapping! {
    ClaimCiRecoveryRequestToIntent:
        ClaimCiRecoveryRequest.intent => ClaimCiRecovery.intent
        using clone;
}

fn claim_ci_recovery_fact(
    stream: &CiRecoveryStream,
    intent: &ClaimCiRecoveryIntent,
    _: &bool,
) -> CiRecoveryClaimedEvent {
    CiRecoveryClaimedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(stream).clone(),
        schema_version: intent.schema_version,
        incident_id: intent.incident_id.clone(),
        trigger: intent.trigger.clone().into(),
        owner: intent.owner.clone().into(),
        lease_expires_at: intent.lease_expires_at,
    }
}

mapping! {
    ClaimCiRecoveryToFact:
        (ClaimCiRecovery.stream, ClaimCiRecovery.intent, ClaimCiRecoveryState.active_incident) => TiberEvent.CiRecoveryClaimed
        using claim_ci_recovery_fact;
}

#[derive(ModelState)]
struct ClaimCiRecoveryState {
    #[model(default)]
    active_incident: bool,
}

impl ModelCommandLogic for ClaimCiRecovery {
    type Event = TiberEvent;
    type State = ClaimCiRecoveryState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        match event {
            TiberEvent::CiRecoveryClaimed(_) => state.active_incident = true,
            TiberEvent::CiRecoveryResolved(_) => state.active_incident = false,
            TiberEvent::LegacyRecoveryStatePublished(payload) => {
                state.active_incident =
                    payload.state.state != tiber_core::events::CiRecoveryPhase::Resolved;
            }
            _ => {}
        }
        Modeled::from_built(state)
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        if state.as_ref().active_incident {
            return Err("ci_recovery_claim_already_active".into());
        }
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_cirecoveryclaimed(ClaimCiRecoveryToFact::apply((
                self,
                self,
                state.as_ref(),
            ))),
        ))
    }
}

#[derive(Clone)]
struct JoinCiRecoveryIntent {
    trigger: Option<CiRecoveryTrigger>,
    participant: Option<CiRecoveryParticipant>,
}

#[derive(ModelInput)]
struct JoinCiRecoveryRequest {
    #[model(origin)]
    stream: CiRecoveryStream,
    #[model(origin)]
    intent: JoinCiRecoveryIntent,
}

#[derive(ModelCommand)]
struct JoinCiRecovery {
    #[stream]
    stream: CiRecoveryStream,
    intent: JoinCiRecoveryIntent,
}

mapping! {
    JoinCiRecoveryRequestToStream:
        JoinCiRecoveryRequest.stream => JoinCiRecovery.stream
        using clone;
}

mapping! {
    JoinCiRecoveryRequestToIntent:
        JoinCiRecoveryRequest.intent => JoinCiRecovery.intent
        using clone;
}

fn join_ci_recovery_fact(
    stream: &CiRecoveryStream,
    intent: &JoinCiRecoveryIntent,
    _: &bool,
    triggers: &[CiRecoveryTrigger],
    participants: &[CiRecoveryParticipant],
    failed_replacement: &Option<CiRecoveryReplacement>,
) -> CiRecoveryJoinedEvent {
    let trigger = intent.trigger.as_ref().filter(|trigger| {
        !triggers.contains(*trigger)
            && failed_replacement.as_ref().is_some_and(|replacement| {
                replacement.run_id == trigger.run_id
                    && replacement.run_url == trigger.run_url
                    && replacement.sha == trigger.failed_sha
            })
    });
    let participant = intent
        .participant
        .as_ref()
        .filter(|participant| !participants.contains(*participant));
    CiRecoveryJoinedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(stream).clone(),
        trigger: trigger.cloned().map(Into::into),
        participant: participant.cloned().map(Into::into),
    }
}

mapping! {
    JoinCiRecoveryToFact:
        (JoinCiRecovery.stream, JoinCiRecovery.intent, JoinCiRecoveryState.active_incident, JoinCiRecoveryState.triggers, JoinCiRecoveryState.participants, JoinCiRecoveryState.failed_replacement) => TiberEvent.CiRecoveryJoined
        using join_ci_recovery_fact;
}

#[derive(ModelState)]
struct JoinCiRecoveryState {
    #[model(default)]
    active_incident: bool,
    #[model(default)]
    triggers: Vec<CiRecoveryTrigger>,
    #[model(default)]
    participants: Vec<CiRecoveryParticipant>,
    #[model(default)]
    failed_replacement: Option<CiRecoveryReplacement>,
}

impl ModelCommandLogic for JoinCiRecovery {
    type Event = TiberEvent;
    type State = JoinCiRecoveryState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        match event {
            TiberEvent::CiRecoveryClaimed(payload) => {
                state.active_incident = true;
                state.triggers = vec![payload.trigger.clone().into()];
                state.participants = vec![payload.owner.clone().into()];
                state.failed_replacement = None;
            }
            TiberEvent::CiRecoveryJoined(payload) => {
                if let Some(trigger) = &payload.trigger {
                    let trigger = trigger.clone().into();
                    if !state.triggers.contains(&trigger) {
                        state.triggers.push(trigger);
                    }
                }
                if let Some(participant) = &payload.participant {
                    let participant = participant.clone().into();
                    if !state.participants.contains(&participant) {
                        state.participants.push(participant);
                    }
                }
            }
            TiberEvent::CiRecoveryReplacementRecorded(payload) => {
                let replacement: CiRecoveryReplacement = payload.replacement.clone().into();
                state.failed_replacement = (replacement.status
                    == CiRecoveryReplacementStatus::Failed)
                    .then_some(replacement);
            }
            TiberEvent::CiRecoveryResolved(_) => state.active_incident = false,
            TiberEvent::LegacyRecoveryStatePublished(payload) => {
                let legacy = CiRecoveryState::from_snapshot(&payload.state);
                state.active_incident = legacy.state != CiRecoveryPhase::Resolved;
                state.triggers = if legacy.triggers.is_empty() {
                    vec![legacy.trigger]
                } else {
                    legacy.triggers
                };
                state.participants = legacy.participants;
                state.failed_replacement = legacy.replacement.filter(|replacement| {
                    replacement.status == CiRecoveryReplacementStatus::Failed
                });
            }
            _ => {}
        }
        Modeled::from_built(state)
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let state = state.as_ref();
        if !state.active_incident {
            return Err("ci_recovery_join_without_active_incident".into());
        }
        let trigger = self.intent.trigger.as_ref().filter(|trigger| {
            !state.triggers.contains(*trigger)
                && state
                    .failed_replacement
                    .as_ref()
                    .is_some_and(|replacement| {
                        replacement.run_id == trigger.run_id
                            && replacement.run_url == trigger.run_url
                            && replacement.sha == trigger.failed_sha
                    })
        });
        let participant = self
            .intent
            .participant
            .as_ref()
            .filter(|participant| !state.participants.contains(*participant));
        if trigger.is_none() && participant.is_none() {
            return Err("ci_recovery_join_empty_or_duplicate_fact".into());
        }
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_cirecoveryjoined(JoinCiRecoveryToFact::apply((
                self, self, state, state, state, state,
            ))),
        ))
    }
}

/// The externally observed clock is an input to this pure command. The command
/// decides the successor epoch and resulting ownership fact from the current
/// incident facts; it does not receive a caller-built event.
#[derive(Clone)]
struct TransferCiRecoveryIntent {
    incident_id: String,
    expected_epoch: u64,
    caller: CiRecoveryParticipant,
    recipient: CiRecoveryParticipant,
    observed_at: u64,
    lease_expires_at: u64,
}

#[derive(ModelInput)]
struct TransferCiRecoveryRequest {
    #[model(origin)]
    stream: CiRecoveryStream,
    #[model(origin)]
    intent: TransferCiRecoveryIntent,
}

#[derive(ModelCommand)]
struct TransferCiRecovery {
    #[stream]
    stream: CiRecoveryStream,
    intent: TransferCiRecoveryIntent,
}

#[derive(Clone)]
struct CiOwnershipDecisionContext {
    incident_id: Option<String>,
    owner: Option<CiRecoveryParticipant>,
    epoch: u64,
    lease_expires_at: u64,
    participants: Vec<CiRecoveryParticipant>,
    resolved: bool,
}

#[derive(ModelOutput)]
struct CiOwnershipDecisionOutput {
    context: CiOwnershipDecisionContext,
}

fn ownership_decision_context(
    incident_id: &Option<String>,
    owner: &Option<CiRecoveryParticipant>,
    epoch: &u64,
    lease_expires_at: &u64,
    participants: &[CiRecoveryParticipant],
    resolved: &bool,
) -> CiOwnershipDecisionContext {
    CiOwnershipDecisionContext {
        incident_id: incident_id.clone(),
        owner: owner.clone(),
        epoch: *epoch,
        lease_expires_at: *lease_expires_at,
        participants: participants.to_vec(),
        resolved: *resolved,
    }
}

mapping! {
    TransferCiRecoveryStateToDecisionContext:
        (TransferCiRecoveryState.incident_id, TransferCiRecoveryState.owner, TransferCiRecoveryState.epoch, TransferCiRecoveryState.lease_expires_at, TransferCiRecoveryState.participants, TransferCiRecoveryState.resolved) => CiOwnershipDecisionOutput.context
        using ownership_decision_context;
}

mapping! {
    TransferCiRecoveryRequestToStream:
        TransferCiRecoveryRequest.stream => TransferCiRecovery.stream
        using clone;
}

mapping! {
    TransferCiRecoveryRequestToIntent:
        TransferCiRecoveryRequest.intent => TransferCiRecovery.intent
        using clone;
}

fn transfer_ci_recovery_candidate_fact(
    stream: &CiRecoveryStream,
    intent: &TransferCiRecoveryIntent,
    context: &CiOwnershipDecisionContext,
) -> CiRecoveryTransferredEvent {
    CiRecoveryTransferredEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(stream).clone(),
        owner: intent.recipient.clone().into(),
        epoch: intent.expected_epoch.saturating_add(1),
        lease_expires_at: intent.lease_expires_at,
        participant: (!context.participants.contains(&intent.recipient))
            .then(|| intent.recipient.clone().into()),
    }
}

// EventCore's checked mapping records the fact fields determined entirely by
// the transfer intent. `decide` below removes the optional participant when
// the folded incident facts show it has already joined.
mapping! {
    TransferCiRecoveryToCandidateFact:
        (TransferCiRecovery.stream, TransferCiRecovery.intent, CiOwnershipDecisionOutput.context) => TiberEvent.CiRecoveryTransferred
        using transfer_ci_recovery_candidate_fact;
}

#[derive(ModelState)]
struct TransferCiRecoveryState {
    #[model(default)]
    incident_id: Option<String>,
    #[model(default)]
    owner: Option<CiRecoveryParticipant>,
    #[model(default)]
    epoch: u64,
    #[model(default)]
    lease_expires_at: u64,
    #[model(default)]
    participants: Vec<CiRecoveryParticipant>,
    #[model(default)]
    resolved: bool,
}

impl ModelCommandLogic for TransferCiRecovery {
    type Event = TiberEvent;
    type State = TransferCiRecoveryState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        match event {
            TiberEvent::CiRecoveryClaimed(payload) => {
                state.incident_id = Some(payload.incident_id.clone());
                state.owner = Some(payload.owner.clone().into());
                state.epoch = 1;
                state.lease_expires_at = payload.lease_expires_at;
                state.participants = vec![payload.owner.clone().into()];
                state.resolved = false;
            }
            TiberEvent::CiRecoveryJoined(payload) => {
                if let Some(participant) = &payload.participant {
                    let participant = participant.clone().into();
                    if !state.participants.contains(&participant) {
                        state.participants.push(participant);
                    }
                }
            }
            TiberEvent::CiRecoveryTransferred(payload) => {
                let owner: CiRecoveryParticipant = payload.owner.clone().into();
                state.owner = Some(owner.clone());
                state.epoch = payload.epoch;
                state.lease_expires_at = payload.lease_expires_at;
                if let Some(participant) = &payload.participant {
                    let participant = participant.clone().into();
                    if !state.participants.contains(&participant) {
                        state.participants.push(participant);
                    }
                }
            }
            TiberEvent::CiRecoveryTakenOver(payload) => {
                let owner: CiRecoveryParticipant = payload.owner.clone().into();
                state.owner = Some(owner.clone());
                state.epoch = payload.epoch;
                state.lease_expires_at = payload.lease_expires_at;
                if let Some(participant) = &payload.participant {
                    let participant = participant.clone().into();
                    if !state.participants.contains(&participant) {
                        state.participants.push(participant);
                    }
                }
            }
            TiberEvent::CiRecoveryResolved(_) => state.resolved = true,
            TiberEvent::LegacyRecoveryStatePublished(payload) => {
                let legacy = CiRecoveryState::from_snapshot(&payload.state);
                state.incident_id = Some(legacy.incident_id);
                state.owner = Some(legacy.owner);
                state.epoch = legacy.epoch;
                state.lease_expires_at = legacy.lease_expires_at;
                state.participants = legacy.participants;
                state.resolved = legacy.state == CiRecoveryPhase::Resolved;
            }
            _ => {}
        }
        Modeled::from_built(state)
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let state = state.as_ref();
        if state.resolved {
            return Err("ci_recovery_incident_resolved".into());
        }
        if state.incident_id.as_deref() != Some(&self.intent.incident_id) {
            return Err("ci_recovery_incident_mismatch".into());
        }
        if state.epoch != self.intent.expected_epoch {
            return Err("ci_recovery_stale_epoch".into());
        }
        if state.owner.as_ref() != Some(&self.intent.caller) {
            return Err("ci_recovery_not_owner".into());
        }
        if state.lease_expires_at <= self.intent.observed_at {
            return Err("ci_recovery_lease_expired".into());
        }
        let decision = CiOwnershipDecisionOutput::model_builder()
            .context(TransferCiRecoveryStateToDecisionContext::apply((
                state, state, state, state, state, state,
            )))
            .build();
        debug_assert_eq!(decision.as_ref().context.incident_id, state.incident_id);
        debug_assert_eq!(decision.as_ref().context.owner, state.owner);
        debug_assert_eq!(decision.as_ref().context.epoch, state.epoch);
        debug_assert_eq!(
            decision.as_ref().context.lease_expires_at,
            state.lease_expires_at
        );
        debug_assert_eq!(decision.as_ref().context.resolved, state.resolved);
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_cirecoverytransferred(
                TransferCiRecoveryToCandidateFact::apply((self, self, decision.as_ref())),
            ),
        ))
    }
}

#[derive(Clone)]
struct TakeOverCiRecoveryIntent {
    incident_id: String,
    expected_epoch: u64,
    successor: CiRecoveryParticipant,
    observed_at: u64,
    lease_expires_at: u64,
}

#[derive(ModelInput)]
struct TakeOverCiRecoveryRequest {
    #[model(origin)]
    stream: CiRecoveryStream,
    #[model(origin)]
    intent: TakeOverCiRecoveryIntent,
}

#[derive(ModelCommand)]
struct TakeOverCiRecovery {
    #[stream]
    stream: CiRecoveryStream,
    intent: TakeOverCiRecoveryIntent,
}

mapping! {
    TakeOverCiRecoveryRequestToStream:
        TakeOverCiRecoveryRequest.stream => TakeOverCiRecovery.stream
        using clone;
}

mapping! {
    TakeOverCiRecoveryRequestToIntent:
        TakeOverCiRecoveryRequest.intent => TakeOverCiRecovery.intent
        using clone;
}

fn takeover_ci_recovery_candidate_fact(
    stream: &CiRecoveryStream,
    intent: &TakeOverCiRecoveryIntent,
    context: &CiOwnershipDecisionContext,
) -> CiRecoveryTakenOverEvent {
    CiRecoveryTakenOverEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(stream).clone(),
        owner: intent.successor.clone().into(),
        epoch: intent.expected_epoch.saturating_add(1),
        lease_expires_at: intent.lease_expires_at,
        participant: (!context.participants.contains(&intent.successor))
            .then(|| intent.successor.clone().into()),
    }
}

mapping! {
    TakeOverCiRecoveryToCandidateFact:
        (TakeOverCiRecovery.stream, TakeOverCiRecovery.intent, CiOwnershipDecisionOutput.context) => TiberEvent.CiRecoveryTakenOver
        using takeover_ci_recovery_candidate_fact;
}

#[derive(ModelState)]
struct TakeOverCiRecoveryState {
    #[model(default)]
    incident_id: Option<String>,
    #[model(default)]
    owner: Option<CiRecoveryParticipant>,
    #[model(default)]
    epoch: u64,
    #[model(default)]
    lease_expires_at: u64,
    #[model(default)]
    participants: Vec<CiRecoveryParticipant>,
    #[model(default)]
    resolved: bool,
}

mapping! {
    TakeOverCiRecoveryStateToDecisionContext:
        (TakeOverCiRecoveryState.incident_id, TakeOverCiRecoveryState.owner, TakeOverCiRecoveryState.epoch, TakeOverCiRecoveryState.lease_expires_at, TakeOverCiRecoveryState.participants, TakeOverCiRecoveryState.resolved) => CiOwnershipDecisionOutput.context
        using ownership_decision_context;
}

impl ModelCommandLogic for TakeOverCiRecovery {
    type Event = TiberEvent;
    type State = TakeOverCiRecoveryState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        match event {
            TiberEvent::CiRecoveryClaimed(payload) => {
                state.incident_id = Some(payload.incident_id.clone());
                state.owner = Some(payload.owner.clone().into());
                state.epoch = 1;
                state.lease_expires_at = payload.lease_expires_at;
                state.participants = vec![payload.owner.clone().into()];
                state.resolved = false;
            }
            TiberEvent::CiRecoveryJoined(payload) => {
                if let Some(participant) = &payload.participant {
                    let participant = participant.clone().into();
                    if !state.participants.contains(&participant) {
                        state.participants.push(participant);
                    }
                }
            }
            TiberEvent::CiRecoveryTransferred(payload) => {
                state.owner = Some(payload.owner.clone().into());
                state.epoch = payload.epoch;
                state.lease_expires_at = payload.lease_expires_at;
                if let Some(participant) = &payload.participant {
                    let participant = participant.clone().into();
                    if !state.participants.contains(&participant) {
                        state.participants.push(participant);
                    }
                }
            }
            TiberEvent::CiRecoveryTakenOver(payload) => {
                state.owner = Some(payload.owner.clone().into());
                state.epoch = payload.epoch;
                state.lease_expires_at = payload.lease_expires_at;
                if let Some(participant) = &payload.participant {
                    let participant = participant.clone().into();
                    if !state.participants.contains(&participant) {
                        state.participants.push(participant);
                    }
                }
            }
            TiberEvent::CiRecoveryResolved(_) => state.resolved = true,
            TiberEvent::LegacyRecoveryStatePublished(payload) => {
                let legacy = CiRecoveryState::from_snapshot(&payload.state);
                state.incident_id = Some(legacy.incident_id);
                state.owner = Some(legacy.owner);
                state.epoch = legacy.epoch;
                state.lease_expires_at = legacy.lease_expires_at;
                state.participants = legacy.participants;
                state.resolved = legacy.state == CiRecoveryPhase::Resolved;
            }
            _ => {}
        }
        Modeled::from_built(state)
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let state = state.as_ref();
        if state.resolved {
            return Err("ci_recovery_incident_resolved".into());
        }
        if state.incident_id.as_deref() != Some(&self.intent.incident_id) {
            return Err("ci_recovery_incident_mismatch".into());
        }
        if state.epoch != self.intent.expected_epoch {
            return Err("ci_recovery_stale_epoch".into());
        }
        if state.owner.as_ref() == Some(&self.intent.successor) {
            return Err("ci_recovery_already_owner".into());
        }
        if state.lease_expires_at > self.intent.observed_at {
            return Err("ci_recovery_lease_active".into());
        }
        let decision = CiOwnershipDecisionOutput::model_builder()
            .context(TakeOverCiRecoveryStateToDecisionContext::apply((
                state, state, state, state, state, state,
            )))
            .build();
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_cirecoverytakenover(
                TakeOverCiRecoveryToCandidateFact::apply((self, self, decision.as_ref())),
            ),
        ))
    }
}

#[derive(Clone)]
struct AssignCiRecoveryWorkIntent {
    incident_id: String,
    expected_epoch: u64,
    caller: CiRecoveryParticipant,
    assignment: CiRecoveryAssignment,
    observed_at: u64,
}

#[derive(ModelInput)]
struct AssignCiRecoveryWorkRequest {
    #[model(origin)]
    stream: CiRecoveryStream,
    #[model(origin)]
    intent: AssignCiRecoveryWorkIntent,
}

#[derive(ModelCommand)]
struct AssignCiRecoveryWork {
    #[stream]
    stream: CiRecoveryStream,
    intent: AssignCiRecoveryWorkIntent,
}

#[derive(Clone)]
struct CiAssignmentDecisionContext {
    incident_id: Option<String>,
    owner: Option<CiRecoveryParticipant>,
    epoch: u64,
    lease_expires_at: u64,
    participants: Vec<CiRecoveryParticipant>,
    assignment_count: usize,
    resolved: bool,
}

#[derive(ModelOutput)]
struct CiAssignmentDecisionOutput {
    context: CiAssignmentDecisionContext,
}

fn assignment_decision_context(
    incident_id: &Option<String>,
    owner: &Option<CiRecoveryParticipant>,
    epoch: &u64,
    lease_expires_at: &u64,
    participants: &[CiRecoveryParticipant],
    assignment_count: &usize,
    resolved: &bool,
) -> CiAssignmentDecisionContext {
    CiAssignmentDecisionContext {
        incident_id: incident_id.clone(),
        owner: owner.clone(),
        epoch: *epoch,
        lease_expires_at: *lease_expires_at,
        participants: participants.to_vec(),
        assignment_count: *assignment_count,
        resolved: *resolved,
    }
}

mapping! {
    AssignCiRecoveryWorkStateToDecisionContext:
        (AssignCiRecoveryWorkState.incident_id, AssignCiRecoveryWorkState.owner, AssignCiRecoveryWorkState.epoch, AssignCiRecoveryWorkState.lease_expires_at, AssignCiRecoveryWorkState.participants, AssignCiRecoveryWorkState.assignment_count, AssignCiRecoveryWorkState.resolved) => CiAssignmentDecisionOutput.context
        using assignment_decision_context;
}

mapping! {
    AssignCiRecoveryWorkRequestToStream:
        AssignCiRecoveryWorkRequest.stream => AssignCiRecoveryWork.stream
        using clone;
}

mapping! {
    AssignCiRecoveryWorkRequestToIntent:
        AssignCiRecoveryWorkRequest.intent => AssignCiRecoveryWork.intent
        using clone;
}

fn assign_ci_recovery_work_fact(
    stream: &CiRecoveryStream,
    intent: &AssignCiRecoveryWorkIntent,
    context: &CiAssignmentDecisionContext,
) -> CiRecoveryAssignedEvent {
    debug_assert_eq!(
        context.incident_id.as_deref(),
        Some(intent.incident_id.as_str())
    );
    debug_assert_eq!(context.owner.as_ref(), Some(&intent.caller));
    debug_assert_eq!(context.epoch, intent.expected_epoch);
    debug_assert!(context.lease_expires_at > intent.observed_at);
    debug_assert!(context.participants.contains(&intent.assignment.assignee));
    debug_assert!(!context.resolved);
    let mut assignment: tiber_core::events::CiRecoveryAssignment = intent.assignment.clone().into();
    assignment.id = format!("a{}", context.assignment_count.saturating_add(1));
    CiRecoveryAssignedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(stream).clone(),
        assignment,
    }
}

mapping! {
    AssignCiRecoveryWorkToFact:
        (AssignCiRecoveryWork.stream, AssignCiRecoveryWork.intent, CiAssignmentDecisionOutput.context) => TiberEvent.CiRecoveryAssigned
        using assign_ci_recovery_work_fact;
}

#[derive(ModelState)]
struct AssignCiRecoveryWorkState {
    #[model(default)]
    incident_id: Option<String>,
    #[model(default)]
    owner: Option<CiRecoveryParticipant>,
    #[model(default)]
    epoch: u64,
    #[model(default)]
    lease_expires_at: u64,
    #[model(default)]
    participants: Vec<CiRecoveryParticipant>,
    #[model(default)]
    assignment_count: usize,
    #[model(default)]
    resolved: bool,
}

impl ModelCommandLogic for AssignCiRecoveryWork {
    type Event = TiberEvent;
    type State = AssignCiRecoveryWorkState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        match event {
            TiberEvent::CiRecoveryClaimed(payload) => {
                state.incident_id = Some(payload.incident_id.clone());
                state.owner = Some(payload.owner.clone().into());
                state.epoch = 1;
                state.lease_expires_at = payload.lease_expires_at;
                state.participants = vec![payload.owner.clone().into()];
                state.assignment_count = 0;
                state.resolved = false;
            }
            TiberEvent::CiRecoveryJoined(payload) => {
                if let Some(participant) = &payload.participant {
                    let participant = participant.clone().into();
                    if !state.participants.contains(&participant) {
                        state.participants.push(participant);
                    }
                }
            }
            TiberEvent::CiRecoveryTransferred(payload) => {
                state.owner = Some(payload.owner.clone().into());
                state.epoch = payload.epoch;
                state.lease_expires_at = payload.lease_expires_at;
                if let Some(participant) = &payload.participant {
                    let participant = participant.clone().into();
                    if !state.participants.contains(&participant) {
                        state.participants.push(participant);
                    }
                }
            }
            TiberEvent::CiRecoveryTakenOver(payload) => {
                state.owner = Some(payload.owner.clone().into());
                state.epoch = payload.epoch;
                state.lease_expires_at = payload.lease_expires_at;
                if let Some(participant) = &payload.participant {
                    let participant = participant.clone().into();
                    if !state.participants.contains(&participant) {
                        state.participants.push(participant);
                    }
                }
            }
            TiberEvent::CiRecoveryAssigned(_) => {
                state.assignment_count = state.assignment_count.saturating_add(1);
            }
            TiberEvent::CiRecoveryResolved(_) => state.resolved = true,
            TiberEvent::LegacyRecoveryStatePublished(payload) => {
                let legacy = CiRecoveryState::from_snapshot(&payload.state);
                state.incident_id = Some(legacy.incident_id);
                state.owner = Some(legacy.owner);
                state.epoch = legacy.epoch;
                state.lease_expires_at = legacy.lease_expires_at;
                state.participants = legacy.participants;
                state.assignment_count = legacy.assignments.len();
                state.resolved = legacy.state == CiRecoveryPhase::Resolved;
            }
            _ => {}
        }
        Modeled::from_built(state)
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let state = state.as_ref();
        if state.resolved {
            return Err("ci_recovery_incident_resolved".into());
        }
        if state.incident_id.as_deref() != Some(&self.intent.incident_id) {
            return Err("ci_recovery_incident_mismatch".into());
        }
        if state.epoch != self.intent.expected_epoch {
            return Err("ci_recovery_stale_epoch".into());
        }
        if state.owner.as_ref() != Some(&self.intent.caller) {
            return Err("ci_recovery_not_owner".into());
        }
        if state.lease_expires_at <= self.intent.observed_at {
            return Err("ci_recovery_lease_expired".into());
        }
        if !state
            .participants
            .contains(&self.intent.assignment.assignee)
        {
            return Err("ci_recovery_assignee_not_joined".into());
        }
        if self.intent.assignment.owner_epoch != state.epoch
            || self.intent.assignment.report.is_some()
        {
            return Err("ci_recovery_assignment_authorization_invalid".into());
        }
        let decision = CiAssignmentDecisionOutput::model_builder()
            .context(AssignCiRecoveryWorkStateToDecisionContext::apply((
                state, state, state, state, state, state, state,
            )))
            .build();
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_cirecoveryassigned(AssignCiRecoveryWorkToFact::apply((
                self,
                self,
                decision.as_ref(),
            ))),
        ))
    }
}

/// Records the assignee's immutable result for one issued CI-recovery task.
/// The folded state contains only the incident identity, current epoch, and
/// issued assignments needed to decide whether that report is admissible.
#[derive(Clone)]
struct ReportCiRecoveryWorkIntent {
    incident_id: String,
    assignment_id: String,
    assignee: CiRecoveryParticipant,
    report: CiRecoveryReport,
}

#[derive(ModelInput)]
struct ReportCiRecoveryWorkRequest {
    #[model(origin)]
    stream: CiRecoveryStream,
    #[model(origin)]
    intent: ReportCiRecoveryWorkIntent,
}

#[derive(ModelCommand)]
struct ReportCiRecoveryWork {
    #[stream]
    stream: CiRecoveryStream,
    intent: ReportCiRecoveryWorkIntent,
}

mapping! {
    ReportCiRecoveryWorkRequestToStream:
        ReportCiRecoveryWorkRequest.stream => ReportCiRecoveryWork.stream
        using clone;
}

mapping! {
    ReportCiRecoveryWorkRequestToIntent:
        ReportCiRecoveryWorkRequest.intent => ReportCiRecoveryWork.intent
        using clone;
}

fn report_ci_recovery_work_fact(
    stream: &CiRecoveryStream,
    intent: &ReportCiRecoveryWorkIntent,
    incident_id: &Option<String>,
    epoch: &u64,
    assignments: &[CiRecoveryAssignment],
    resolved: &bool,
) -> CiRecoveryReportedEvent {
    debug_assert_eq!(incident_id.as_deref(), Some(intent.incident_id.as_str()));
    debug_assert!(!resolved);
    debug_assert!(assignments.iter().any(|assignment| {
        assignment.id == intent.assignment_id
            && assignment.owner_epoch == *epoch
            && assignment.assignee == intent.assignee
            && assignment.report.is_none()
    }));
    CiRecoveryReportedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(stream).clone(),
        assignment_id: intent.assignment_id.clone(),
        assignee: intent.assignee.clone().into(),
        report: intent.report.clone().into(),
    }
}

mapping! {
    ReportCiRecoveryWorkToFact:
        (ReportCiRecoveryWork.stream, ReportCiRecoveryWork.intent, ReportCiRecoveryWorkState.incident_id, ReportCiRecoveryWorkState.epoch, ReportCiRecoveryWorkState.assignments, ReportCiRecoveryWorkState.resolved) => TiberEvent.CiRecoveryReported
        using report_ci_recovery_work_fact;
}

#[derive(ModelState)]
struct ReportCiRecoveryWorkState {
    #[model(default)]
    incident_id: Option<String>,
    #[model(default)]
    epoch: u64,
    #[model(default)]
    assignments: Vec<CiRecoveryAssignment>,
    #[model(default)]
    resolved: bool,
}

impl ModelCommandLogic for ReportCiRecoveryWork {
    type Event = TiberEvent;
    type State = ReportCiRecoveryWorkState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        match event {
            TiberEvent::CiRecoveryClaimed(payload) => {
                state.incident_id = Some(payload.incident_id.clone());
                state.epoch = 1;
                state.assignments.clear();
                state.resolved = false;
            }
            TiberEvent::CiRecoveryTransferred(payload) => state.epoch = payload.epoch,
            TiberEvent::CiRecoveryTakenOver(payload) => state.epoch = payload.epoch,
            TiberEvent::CiRecoveryAssigned(payload) => {
                state.assignments.push(payload.assignment.clone().into());
            }
            TiberEvent::CiRecoveryReported(payload) => {
                if let Some(assignment) = state
                    .assignments
                    .iter_mut()
                    .find(|assignment| assignment.id == payload.assignment_id)
                {
                    assignment.report = Some(payload.report.clone().into());
                }
            }
            TiberEvent::CiRecoveryResolved(_) => state.resolved = true,
            TiberEvent::LegacyRecoveryStatePublished(payload) => {
                let legacy = CiRecoveryState::from_snapshot(&payload.state);
                state.incident_id = Some(legacy.incident_id);
                state.epoch = legacy.epoch;
                state.assignments = legacy.assignments;
                state.resolved = legacy.state == CiRecoveryPhase::Resolved;
            }
            _ => {}
        }
        Modeled::from_built(state)
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let state = state.as_ref();
        if state.resolved {
            return Err("ci_recovery_incident_resolved".into());
        }
        if state.incident_id.as_deref() != Some(&self.intent.incident_id) {
            return Err("ci_recovery_incident_mismatch".into());
        }
        let Some(assignment) = state
            .assignments
            .iter()
            .find(|assignment| assignment.id == self.intent.assignment_id)
        else {
            return Err("ci_recovery_assignment_missing".into());
        };
        if assignment.owner_epoch != state.epoch {
            return Err("ci_recovery_assignment_stale".into());
        }
        if assignment.assignee != self.intent.assignee {
            return Err("ci_recovery_assignment_not_assignee".into());
        }
        if assignment.report.is_some() {
            return Err("ci_recovery_assignment_already_reported".into());
        }
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_cirecoveryreported(ReportCiRecoveryWorkToFact::apply((
                self, self, state, state, state, state,
            ))),
        ))
    }
}

/// Extends the active owner's lease using the observed clock supplied as input.
#[derive(Clone)]
struct RenewCiRecoveryLeaseIntent {
    incident_id: String,
    expected_epoch: u64,
    owner: CiRecoveryParticipant,
    observed_at: u64,
    lease_expires_at: u64,
}

#[derive(ModelInput)]
struct RenewCiRecoveryLeaseRequest {
    #[model(origin)]
    stream: CiRecoveryStream,
    #[model(origin)]
    intent: RenewCiRecoveryLeaseIntent,
}

#[derive(ModelCommand)]
struct RenewCiRecoveryLease {
    #[stream]
    stream: CiRecoveryStream,
    intent: RenewCiRecoveryLeaseIntent,
}

mapping! {
    RenewCiRecoveryLeaseRequestToStream:
        RenewCiRecoveryLeaseRequest.stream => RenewCiRecoveryLease.stream
        using clone;
}

mapping! {
    RenewCiRecoveryLeaseRequestToIntent:
        RenewCiRecoveryLeaseRequest.intent => RenewCiRecoveryLease.intent
        using clone;
}

fn renew_ci_recovery_lease_fact(
    stream: &CiRecoveryStream,
    intent: &RenewCiRecoveryLeaseIntent,
    incident_id: &Option<String>,
    owner: &Option<CiRecoveryParticipant>,
    epoch: &u64,
    lease_expires_at: &u64,
    resolved: &bool,
) -> CiRecoveryHeartbeatRecordedEvent {
    debug_assert_eq!(incident_id.as_deref(), Some(intent.incident_id.as_str()));
    debug_assert_eq!(owner.as_ref(), Some(&intent.owner));
    debug_assert_eq!(*epoch, intent.expected_epoch);
    debug_assert!(*lease_expires_at > intent.observed_at);
    debug_assert!(!resolved);
    CiRecoveryHeartbeatRecordedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(stream).clone(),
        epoch: intent.expected_epoch,
        owner: intent.owner.clone().into(),
        lease_expires_at: intent.lease_expires_at,
    }
}

mapping! {
    RenewCiRecoveryLeaseToFact:
        (RenewCiRecoveryLease.stream, RenewCiRecoveryLease.intent, RenewCiRecoveryLeaseState.incident_id, RenewCiRecoveryLeaseState.owner, RenewCiRecoveryLeaseState.epoch, RenewCiRecoveryLeaseState.lease_expires_at, RenewCiRecoveryLeaseState.resolved) => TiberEvent.CiRecoveryHeartbeatRecorded
        using renew_ci_recovery_lease_fact;
}

#[derive(ModelState)]
struct RenewCiRecoveryLeaseState {
    #[model(default)]
    incident_id: Option<String>,
    #[model(default)]
    owner: Option<CiRecoveryParticipant>,
    #[model(default)]
    epoch: u64,
    #[model(default)]
    lease_expires_at: u64,
    #[model(default)]
    resolved: bool,
}

impl ModelCommandLogic for RenewCiRecoveryLease {
    type Event = TiberEvent;
    type State = RenewCiRecoveryLeaseState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        match event {
            TiberEvent::CiRecoveryClaimed(payload) => {
                state.incident_id = Some(payload.incident_id.clone());
                state.owner = Some(payload.owner.clone().into());
                state.epoch = 1;
                state.lease_expires_at = payload.lease_expires_at;
                state.resolved = false;
            }
            TiberEvent::CiRecoveryTransferred(payload) => {
                state.owner = Some(payload.owner.clone().into());
                state.epoch = payload.epoch;
                state.lease_expires_at = payload.lease_expires_at;
            }
            TiberEvent::CiRecoveryTakenOver(payload) => {
                state.owner = Some(payload.owner.clone().into());
                state.epoch = payload.epoch;
                state.lease_expires_at = payload.lease_expires_at;
            }
            TiberEvent::CiRecoveryHeartbeatRecorded(payload) => {
                state.lease_expires_at = payload.lease_expires_at;
            }
            TiberEvent::CiRecoveryResolved(_) => state.resolved = true,
            TiberEvent::LegacyRecoveryStatePublished(payload) => {
                let legacy = CiRecoveryState::from_snapshot(&payload.state);
                state.incident_id = Some(legacy.incident_id);
                state.owner = Some(legacy.owner);
                state.epoch = legacy.epoch;
                state.lease_expires_at = legacy.lease_expires_at;
                state.resolved = legacy.state == CiRecoveryPhase::Resolved;
            }
            _ => {}
        }
        Modeled::from_built(state)
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let state = state.as_ref();
        if state.resolved {
            return Err("ci_recovery_incident_resolved".into());
        }
        if state.incident_id.as_deref() != Some(&self.intent.incident_id) {
            return Err("ci_recovery_incident_mismatch".into());
        }
        if state.epoch != self.intent.expected_epoch {
            return Err("ci_recovery_stale_epoch".into());
        }
        if state.owner.as_ref() != Some(&self.intent.owner) {
            return Err("ci_recovery_not_owner".into());
        }
        if state.lease_expires_at <= self.intent.observed_at {
            return Err("ci_recovery_lease_expired".into());
        }
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_cirecoveryheartbeatrecorded(
                RenewCiRecoveryLeaseToFact::apply((self, self, state, state, state, state, state)),
            ),
        ))
    }
}

#[derive(Clone)]
struct RecordCiRecoveryDiagnosisIntent {
    incident_id: String,
    expected_epoch: u64,
    owner: CiRecoveryParticipant,
    observed_at: u64,
    failure_record: CiRecoveryFailureRecord,
    diagnosis: CiRecoveryDiagnosis,
}

#[derive(ModelInput)]
struct RecordCiRecoveryDiagnosisRequest {
    #[model(origin)]
    stream: CiRecoveryStream,
    #[model(origin)]
    intent: RecordCiRecoveryDiagnosisIntent,
}

#[derive(ModelCommand)]
struct RecordCiRecoveryDiagnosis {
    #[stream]
    stream: CiRecoveryStream,
    intent: RecordCiRecoveryDiagnosisIntent,
}

mapping! {
    RecordCiRecoveryDiagnosisRequestToStream:
        RecordCiRecoveryDiagnosisRequest.stream => RecordCiRecoveryDiagnosis.stream
        using clone;
}
mapping! {
    RecordCiRecoveryDiagnosisRequestToIntent:
        RecordCiRecoveryDiagnosisRequest.intent => RecordCiRecoveryDiagnosis.intent
        using clone;
}
fn record_ci_recovery_diagnosis_fact(
    stream: &CiRecoveryStream,
    intent: &RecordCiRecoveryDiagnosisIntent,
    incident_id: &Option<String>,
    owner: &Option<CiRecoveryParticipant>,
    epoch: &u64,
    lease_expires_at: &u64,
    resolved: &bool,
) -> CiRecoveryDiagnosedEvent {
    debug_assert_eq!(incident_id.as_deref(), Some(intent.incident_id.as_str()));
    debug_assert_eq!(owner.as_ref(), Some(&intent.owner));
    debug_assert_eq!(*epoch, intent.expected_epoch);
    debug_assert!(*lease_expires_at > intent.observed_at);
    debug_assert!(!resolved);
    CiRecoveryDiagnosedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(stream).clone(),
        epoch: intent.expected_epoch,
        owner: intent.owner.clone().into(),
        failure_record: intent.failure_record.clone().into(),
        diagnosis: intent.diagnosis.clone().into(),
    }
}
mapping! {
    RecordCiRecoveryDiagnosisToFact:
        (RecordCiRecoveryDiagnosis.stream, RecordCiRecoveryDiagnosis.intent, RecordCiRecoveryDiagnosisState.incident_id, RecordCiRecoveryDiagnosisState.owner, RecordCiRecoveryDiagnosisState.epoch, RecordCiRecoveryDiagnosisState.lease_expires_at, RecordCiRecoveryDiagnosisState.resolved) => TiberEvent.CiRecoveryDiagnosed
        using record_ci_recovery_diagnosis_fact;
}

#[derive(ModelState)]
struct RecordCiRecoveryDiagnosisState {
    #[model(default)]
    incident_id: Option<String>,
    #[model(default)]
    owner: Option<CiRecoveryParticipant>,
    #[model(default)]
    epoch: u64,
    #[model(default)]
    lease_expires_at: u64,
    #[model(default)]
    resolved: bool,
}
impl ModelCommandLogic for RecordCiRecoveryDiagnosis {
    type Event = TiberEvent;
    type State = RecordCiRecoveryDiagnosisState;
    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        match event {
            TiberEvent::CiRecoveryClaimed(payload) => {
                state.incident_id = Some(payload.incident_id.clone());
                state.owner = Some(payload.owner.clone().into());
                state.epoch = 1;
                state.lease_expires_at = payload.lease_expires_at;
                state.resolved = false;
            }
            TiberEvent::CiRecoveryTransferred(payload) => {
                state.owner = Some(payload.owner.clone().into());
                state.epoch = payload.epoch;
                state.lease_expires_at = payload.lease_expires_at;
            }
            TiberEvent::CiRecoveryTakenOver(payload) => {
                state.owner = Some(payload.owner.clone().into());
                state.epoch = payload.epoch;
                state.lease_expires_at = payload.lease_expires_at;
            }
            TiberEvent::CiRecoveryHeartbeatRecorded(payload) => {
                state.lease_expires_at = payload.lease_expires_at
            }
            TiberEvent::CiRecoveryResolved(_) => state.resolved = true,
            TiberEvent::LegacyRecoveryStatePublished(payload) => {
                let legacy = CiRecoveryState::from_snapshot(&payload.state);
                state.incident_id = Some(legacy.incident_id);
                state.owner = Some(legacy.owner);
                state.epoch = legacy.epoch;
                state.lease_expires_at = legacy.lease_expires_at;
                state.resolved = legacy.state == CiRecoveryPhase::Resolved;
            }
            _ => {}
        }
        Modeled::from_built(state)
    }
    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let state = state.as_ref();
        if state.resolved {
            return Err("ci_recovery_incident_resolved".into());
        }
        if state.incident_id.as_deref() != Some(&self.intent.incident_id) {
            return Err("ci_recovery_incident_mismatch".into());
        }
        if state.epoch != self.intent.expected_epoch {
            return Err("ci_recovery_stale_epoch".into());
        }
        if state.owner.as_ref() != Some(&self.intent.owner) {
            return Err("ci_recovery_not_owner".into());
        }
        if state.lease_expires_at <= self.intent.observed_at {
            return Err("ci_recovery_lease_expired".into());
        }
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_cirecoverydiagnosed(RecordCiRecoveryDiagnosisToFact::apply(
                (self, self, state, state, state, state, state),
            )),
        ))
    }
}

#[derive(Clone)]
struct SelectCiRecoveryActionIntent {
    incident_id: String,
    expected_epoch: u64,
    owner: CiRecoveryParticipant,
    observed_at: u64,
    action: CiRecoveryAction,
}
#[derive(ModelInput)]
struct SelectCiRecoveryActionRequest {
    #[model(origin)]
    stream: CiRecoveryStream,
    #[model(origin)]
    intent: SelectCiRecoveryActionIntent,
}
#[derive(ModelCommand)]
struct SelectCiRecoveryAction {
    #[stream]
    stream: CiRecoveryStream,
    intent: SelectCiRecoveryActionIntent,
}

#[derive(Clone)]
struct CiOwnerDecisionContext {
    incident_id: Option<String>,
    owner: Option<CiRecoveryParticipant>,
    epoch: u64,
    lease_expires_at: u64,
    resolved: bool,
}

#[derive(ModelOutput)]
struct CiOwnerDecisionOutput {
    context: CiOwnerDecisionContext,
}

fn owner_decision_context(
    incident_id: &Option<String>,
    owner: &Option<CiRecoveryParticipant>,
    epoch: &u64,
    lease_expires_at: &u64,
    resolved: &bool,
) -> CiOwnerDecisionContext {
    CiOwnerDecisionContext {
        incident_id: incident_id.clone(),
        owner: owner.clone(),
        epoch: *epoch,
        lease_expires_at: *lease_expires_at,
        resolved: *resolved,
    }
}
mapping! { SelectCiRecoveryActionRequestToStream: SelectCiRecoveryActionRequest.stream => SelectCiRecoveryAction.stream using clone; }
mapping! { SelectCiRecoveryActionRequestToIntent: SelectCiRecoveryActionRequest.intent => SelectCiRecoveryAction.intent using clone; }
fn select_ci_recovery_action_fact(
    stream: &CiRecoveryStream,
    intent: &SelectCiRecoveryActionIntent,
    context: &CiOwnerDecisionContext,
    classification: &Option<CiRecoveryClassification>,
) -> CiRecoveryActionChosenEvent {
    debug_assert_eq!(
        context.incident_id.as_deref(),
        Some(intent.incident_id.as_str())
    );
    debug_assert_eq!(context.owner.as_ref(), Some(&intent.owner));
    debug_assert_eq!(context.epoch, intent.expected_epoch);
    debug_assert!(context.lease_expires_at > intent.observed_at);
    debug_assert!(!context.resolved);
    debug_assert!(classification.is_some());
    CiRecoveryActionChosenEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(stream).clone(),
        epoch: intent.expected_epoch,
        owner: intent.owner.clone().into(),
        action: intent.action.clone().into(),
    }
}
mapping! { SelectCiRecoveryActionToFact:
    (SelectCiRecoveryAction.stream, SelectCiRecoveryAction.intent, CiOwnerDecisionOutput.context, SelectCiRecoveryActionState.classification) => TiberEvent.CiRecoveryActionChosen
    using select_ci_recovery_action_fact;
}
#[derive(ModelState)]
struct SelectCiRecoveryActionState {
    #[model(default)]
    incident_id: Option<String>,
    #[model(default)]
    owner: Option<CiRecoveryParticipant>,
    #[model(default)]
    epoch: u64,
    #[model(default)]
    lease_expires_at: u64,
    #[model(default)]
    classification: Option<CiRecoveryClassification>,
    #[model(default)]
    resolved: bool,
}
mapping! {
    SelectCiRecoveryActionStateToOwnerContext:
        (SelectCiRecoveryActionState.incident_id, SelectCiRecoveryActionState.owner, SelectCiRecoveryActionState.epoch, SelectCiRecoveryActionState.lease_expires_at, SelectCiRecoveryActionState.resolved) => CiOwnerDecisionOutput.context
        using owner_decision_context;
}
impl ModelCommandLogic for SelectCiRecoveryAction {
    type Event = TiberEvent;
    type State = SelectCiRecoveryActionState;
    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        match event {
            TiberEvent::CiRecoveryClaimed(p) => {
                state.incident_id = Some(p.incident_id.clone());
                state.owner = Some(p.owner.clone().into());
                state.epoch = 1;
                state.lease_expires_at = p.lease_expires_at;
                state.classification = None;
                state.resolved = false;
            }
            TiberEvent::CiRecoveryTransferred(p) => {
                state.owner = Some(p.owner.clone().into());
                state.epoch = p.epoch;
                state.lease_expires_at = p.lease_expires_at;
            }
            TiberEvent::CiRecoveryTakenOver(p) => {
                state.owner = Some(p.owner.clone().into());
                state.epoch = p.epoch;
                state.lease_expires_at = p.lease_expires_at;
            }
            TiberEvent::CiRecoveryHeartbeatRecorded(p) => {
                state.lease_expires_at = p.lease_expires_at
            }
            TiberEvent::CiRecoveryDiagnosed(p) => {
                state.classification = Some(p.diagnosis.classification.into())
            }
            TiberEvent::CiRecoveryResolved(_) => state.resolved = true,
            TiberEvent::LegacyRecoveryStatePublished(p) => {
                let legacy = CiRecoveryState::from_snapshot(&p.state);
                state.incident_id = Some(legacy.incident_id);
                state.owner = Some(legacy.owner);
                state.epoch = legacy.epoch;
                state.lease_expires_at = legacy.lease_expires_at;
                state.classification = legacy.diagnosis.map(|d| d.classification);
                state.resolved = legacy.state == CiRecoveryPhase::Resolved;
            }
            _ => {}
        };
        Modeled::from_built(state)
    }
    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let state = state.as_ref();
        if state.resolved {
            return Err("ci_recovery_incident_resolved".into());
        }
        if state.incident_id.as_deref() != Some(&self.intent.incident_id) {
            return Err("ci_recovery_incident_mismatch".into());
        }
        if state.epoch != self.intent.expected_epoch {
            return Err("ci_recovery_stale_epoch".into());
        }
        if state.owner.as_ref() != Some(&self.intent.owner) {
            return Err("ci_recovery_not_owner".into());
        }
        if state.lease_expires_at <= self.intent.observed_at {
            return Err("ci_recovery_lease_expired".into());
        }
        let Some(classification) = state.classification else {
            return Err("ci_recovery_diagnosis_required".into());
        };
        let allowed = matches!(
            (classification, self.intent.action.kind),
            (
                CiRecoveryClassification::Caused,
                CiRecoveryActionKind::Repair
            ) | (
                CiRecoveryClassification::Unrelated | CiRecoveryClassification::Transient,
                CiRecoveryActionKind::Rerun
            )
        );
        if !allowed {
            return Err("ci_recovery_action_conflicts_diagnosis".into());
        }
        let decision = CiOwnerDecisionOutput::model_builder()
            .context(SelectCiRecoveryActionStateToOwnerContext::apply((
                state, state, state, state, state,
            )))
            .build();
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_cirecoveryactionchosen(SelectCiRecoveryActionToFact::apply(
                (self, self, decision.as_ref(), state),
            )),
        ))
    }
}

#[derive(Clone)]
struct RecordCiRecoveryReplacementIntent {
    incident_id: String,
    expected_epoch: u64,
    owner: CiRecoveryParticipant,
    observed_at: u64,
    replacement: CiRecoveryReplacement,
}
#[derive(ModelInput)]
struct RecordCiRecoveryReplacementRequest {
    #[model(origin)]
    stream: CiRecoveryStream,
    #[model(origin)]
    intent: RecordCiRecoveryReplacementIntent,
}
#[derive(ModelCommand)]
struct RecordCiRecoveryReplacement {
    #[stream]
    stream: CiRecoveryStream,
    intent: RecordCiRecoveryReplacementIntent,
}
mapping! { RecordCiRecoveryReplacementRequestToStream: RecordCiRecoveryReplacementRequest.stream => RecordCiRecoveryReplacement.stream using clone; }
mapping! { RecordCiRecoveryReplacementRequestToIntent: RecordCiRecoveryReplacementRequest.intent => RecordCiRecoveryReplacement.intent using clone; }
fn record_ci_recovery_replacement_fact(
    stream: &CiRecoveryStream,
    intent: &RecordCiRecoveryReplacementIntent,
    context: &CiOwnerDecisionContext,
    action: &Option<CiRecoveryAction>,
    failed_sha: &Option<String>,
) -> CiRecoveryReplacementRecordedEvent {
    debug_assert_eq!(
        context.incident_id.as_deref(),
        Some(intent.incident_id.as_str())
    );
    debug_assert_eq!(context.owner.as_ref(), Some(&intent.owner));
    debug_assert_eq!(context.epoch, intent.expected_epoch);
    debug_assert!(context.lease_expires_at > intent.observed_at);
    debug_assert!(!context.resolved);
    debug_assert!(action.is_some());
    debug_assert!(
        action
            .as_ref()
            .is_some_and(|action| action.kind != CiRecoveryActionKind::Rerun)
            || failed_sha.as_deref() == Some(intent.replacement.sha.as_str())
    );
    CiRecoveryReplacementRecordedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(stream).clone(),
        epoch: intent.expected_epoch,
        owner: intent.owner.clone().into(),
        replacement: intent.replacement.clone().into(),
    }
}
mapping! { RecordCiRecoveryReplacementToFact:
    (RecordCiRecoveryReplacement.stream, RecordCiRecoveryReplacement.intent, CiOwnerDecisionOutput.context, RecordCiRecoveryReplacementState.action, RecordCiRecoveryReplacementState.failed_sha) => TiberEvent.CiRecoveryReplacementRecorded
    using record_ci_recovery_replacement_fact;
}
#[derive(ModelState)]
struct RecordCiRecoveryReplacementState {
    #[model(default)]
    incident_id: Option<String>,
    #[model(default)]
    owner: Option<CiRecoveryParticipant>,
    #[model(default)]
    epoch: u64,
    #[model(default)]
    lease_expires_at: u64,
    #[model(default)]
    action: Option<CiRecoveryAction>,
    #[model(default)]
    failed_sha: Option<String>,
    #[model(default)]
    resolved: bool,
}
mapping! {
    RecordCiRecoveryReplacementStateToOwnerContext:
        (RecordCiRecoveryReplacementState.incident_id, RecordCiRecoveryReplacementState.owner, RecordCiRecoveryReplacementState.epoch, RecordCiRecoveryReplacementState.lease_expires_at, RecordCiRecoveryReplacementState.resolved) => CiOwnerDecisionOutput.context
        using owner_decision_context;
}
impl ModelCommandLogic for RecordCiRecoveryReplacement {
    type Event = TiberEvent;
    type State = RecordCiRecoveryReplacementState;
    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        match event {
            TiberEvent::CiRecoveryClaimed(p) => {
                state.incident_id = Some(p.incident_id.clone());
                state.owner = Some(p.owner.clone().into());
                state.epoch = 1;
                state.lease_expires_at = p.lease_expires_at;
                state.failed_sha = Some(p.trigger.failed_sha.clone());
                state.action = None;
                state.resolved = false;
            }
            TiberEvent::CiRecoveryTransferred(p) => {
                state.owner = Some(p.owner.clone().into());
                state.epoch = p.epoch;
                state.lease_expires_at = p.lease_expires_at;
            }
            TiberEvent::CiRecoveryTakenOver(p) => {
                state.owner = Some(p.owner.clone().into());
                state.epoch = p.epoch;
                state.lease_expires_at = p.lease_expires_at;
            }
            TiberEvent::CiRecoveryHeartbeatRecorded(p) => {
                state.lease_expires_at = p.lease_expires_at
            }
            TiberEvent::CiRecoveryActionChosen(p) => state.action = Some(p.action.clone().into()),
            TiberEvent::CiRecoveryReplacementRecorded(p) => {
                if p.replacement.status == tiber_core::events::CiRecoveryReplacementStatus::Failed {
                    state.action = None;
                }
            }
            TiberEvent::CiRecoveryResolved(_) => state.resolved = true,
            TiberEvent::LegacyRecoveryStatePublished(p) => {
                let legacy = CiRecoveryState::from_snapshot(&p.state);
                state.incident_id = Some(legacy.incident_id);
                state.owner = Some(legacy.owner);
                state.epoch = legacy.epoch;
                state.lease_expires_at = legacy.lease_expires_at;
                state.action = legacy.next_action;
                state.failed_sha = Some(legacy.trigger.failed_sha);
                state.resolved = legacy.state == CiRecoveryPhase::Resolved;
            }
            _ => {}
        }
        Modeled::from_built(state)
    }
    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let state = state.as_ref();
        if state.resolved {
            return Err("ci_recovery_incident_resolved".into());
        }
        if state.incident_id.as_deref() != Some(&self.intent.incident_id) {
            return Err("ci_recovery_incident_mismatch".into());
        }
        if state.epoch != self.intent.expected_epoch {
            return Err("ci_recovery_stale_epoch".into());
        }
        if state.owner.as_ref() != Some(&self.intent.owner) {
            return Err("ci_recovery_not_owner".into());
        }
        if state.lease_expires_at <= self.intent.observed_at {
            return Err("ci_recovery_lease_expired".into());
        }
        let Some(action) = &state.action else {
            return Err("ci_recovery_next_action_required".into());
        };
        if action.kind == CiRecoveryActionKind::Rerun
            && state.failed_sha.as_deref() != Some(&self.intent.replacement.sha)
        {
            return Err("ci_recovery_rerun_sha_mismatch".into());
        }
        let decision = CiOwnerDecisionOutput::model_builder()
            .context(RecordCiRecoveryReplacementStateToOwnerContext::apply((
                state, state, state, state, state,
            )))
            .build();
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_cirecoveryreplacementrecorded(
                RecordCiRecoveryReplacementToFact::apply((
                    self,
                    self,
                    decision.as_ref(),
                    state,
                    state,
                )),
            ),
        ))
    }
}

#[derive(Clone)]
struct ResolveCiRecoveryIntent {
    incident_id: String,
    participant: CiRecoveryParticipant,
    proof: CiRecoveryReleaseProof,
}
#[derive(ModelInput)]
struct ResolveCiRecoveryRequest {
    #[model(origin)]
    stream: CiRecoveryStream,
    #[model(origin)]
    intent: ResolveCiRecoveryIntent,
}
#[derive(ModelCommand)]
struct ResolveCiRecovery {
    #[stream]
    stream: CiRecoveryStream,
    intent: ResolveCiRecoveryIntent,
}
mapping! { ResolveCiRecoveryRequestToStream: ResolveCiRecoveryRequest.stream => ResolveCiRecovery.stream using clone; }
mapping! { ResolveCiRecoveryRequestToIntent: ResolveCiRecoveryRequest.intent => ResolveCiRecovery.intent using clone; }
fn resolve_ci_recovery_fact(
    stream: &CiRecoveryStream,
    intent: &ResolveCiRecoveryIntent,
    incident_id: &Option<String>,
    participants: &[CiRecoveryParticipant],
    replacement: &Option<CiRecoveryReplacement>,
    resolved: &bool,
) -> CiRecoveryResolvedEvent {
    debug_assert_eq!(incident_id.as_deref(), Some(intent.incident_id.as_str()));
    debug_assert!(participants.contains(&intent.participant));
    debug_assert!(!resolved);
    debug_assert!(replacement.as_ref().is_some_and(|replacement| {
        replacement.status != CiRecoveryReplacementStatus::Failed
            && replacement.run_id == intent.proof.replacement_run_id
            && replacement.run_url == intent.proof.replacement_run_url
            && replacement.sha == intent.proof.sha
    }));
    CiRecoveryResolvedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(stream).clone(),
        participant: intent.participant.clone().into(),
        proof: intent.proof.clone().into(),
    }
}
mapping! { ResolveCiRecoveryToFact:
    (ResolveCiRecovery.stream, ResolveCiRecovery.intent, ResolveCiRecoveryState.incident_id, ResolveCiRecoveryState.participants, ResolveCiRecoveryState.replacement, ResolveCiRecoveryState.resolved) => TiberEvent.CiRecoveryResolved
    using resolve_ci_recovery_fact;
}
#[derive(ModelState)]
struct ResolveCiRecoveryState {
    #[model(default)]
    incident_id: Option<String>,
    #[model(default)]
    participants: Vec<CiRecoveryParticipant>,
    #[model(default)]
    replacement: Option<CiRecoveryReplacement>,
    #[model(default)]
    resolved: bool,
}
impl ModelCommandLogic for ResolveCiRecovery {
    type Event = TiberEvent;
    type State = ResolveCiRecoveryState;
    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        match event {
            TiberEvent::CiRecoveryClaimed(p) => {
                state.incident_id = Some(p.incident_id.clone());
                state.participants = vec![p.owner.clone().into()];
                state.replacement = None;
                state.resolved = false;
            }
            TiberEvent::CiRecoveryJoined(p) => {
                if let Some(participant) = &p.participant {
                    let participant = participant.clone().into();
                    if !state.participants.contains(&participant) {
                        state.participants.push(participant);
                    }
                }
            }
            TiberEvent::CiRecoveryTransferred(p) => {
                if let Some(participant) = &p.participant {
                    let participant = participant.clone().into();
                    if !state.participants.contains(&participant) {
                        state.participants.push(participant);
                    }
                }
            }
            TiberEvent::CiRecoveryTakenOver(p) => {
                if let Some(participant) = &p.participant {
                    let participant = participant.clone().into();
                    if !state.participants.contains(&participant) {
                        state.participants.push(participant);
                    }
                }
            }
            TiberEvent::CiRecoveryReplacementRecorded(p) => {
                state.replacement = Some(p.replacement.clone().into())
            }
            TiberEvent::CiRecoveryResolved(_) => state.resolved = true,
            TiberEvent::LegacyRecoveryStatePublished(p) => {
                let legacy = CiRecoveryState::from_snapshot(&p.state);
                state.incident_id = Some(legacy.incident_id);
                state.participants = legacy.participants;
                state.replacement = legacy.replacement;
                state.resolved = legacy.state == CiRecoveryPhase::Resolved;
            }
            _ => {}
        }
        Modeled::from_built(state)
    }
    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let state = state.as_ref();
        if state.resolved {
            return Err("ci_recovery_incident_resolved".into());
        }
        if state.incident_id.as_deref() != Some(&self.intent.incident_id) {
            return Err("ci_recovery_incident_mismatch".into());
        }
        if !state.participants.contains(&self.intent.participant) {
            return Err("ci_recovery_participant_required".into());
        }
        let Some(replacement) = &state.replacement else {
            return Err("ci_recovery_replacement_required".into());
        };
        if replacement.status == CiRecoveryReplacementStatus::Failed {
            return Err("ci_recovery_replacement_failed".into());
        }
        if replacement.run_id != self.intent.proof.replacement_run_id
            || replacement.run_url != self.intent.proof.replacement_run_url
            || replacement.sha != self.intent.proof.sha
        {
            return Err("ci_recovery_release_proof_mismatch".into());
        }
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_cirecoveryresolved(ResolveCiRecoveryToFact::apply((
                self, self, state, state, state, state,
            ))),
        ))
    }
}

#[derive(Clone, Default)]
struct TiberProjection {
    initialized: bool,
    tasks: std::collections::BTreeMap<String, Task>,
    order: Vec<String>,
    ci_recovery: Option<CiRecoveryState>,
}

fn stream_id(value: impl Into<String>) -> Result<StreamId, Error> {
    StreamId::try_new(value.into())
        .map_err(|error| Error::Parse(format!("event_stream_invalid source={error}")))
}

fn run_async<T>(future: impl std::future::Future<Output = T> + Send + 'static) -> T
where
    T: Send + 'static,
{
    let run = move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("Tiber's bundled Tokio runtime must initialize")
            .block_on(future)
    };
    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::spawn(run)
            .join()
            .expect("Tiber's event-store worker must complete")
    } else {
        run()
    }
}

fn event_store_error(error: impl std::fmt::Display) -> Error {
    if std::env::var_os("TIBER_EVENT_STORE_DIAGNOSTICS").is_some() {
        eprintln!("tiber.event_store_failure source={error}");
    }
    Error::Parse("event_store_failed source_redacted=true".to_string())
}

/// Preserve deliberately typed domain rejections while ensuring infrastructure
/// errors cannot disclose remote URLs, local paths, or credential-bearing
/// process output through a CLI/MCP response.
fn eventcore_command_error(error: eventcore::CommandError) -> Error {
    match error {
        eventcore::CommandError::BusinessRuleViolation(error) => Error::Parse(error.to_string()),
        eventcore::CommandError::ValidationError(error) => Error::Parse(error),
        eventcore::CommandError::ConcurrencyError(_) => {
            Error::Parse("event_version_conflict=true".to_string())
        }
        eventcore::CommandError::EventStoreError(error) => event_store_error(error),
    }
}

fn load_tiber_projection(root: &Path) -> Result<TiberProjection, Error> {
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    run_async(async move {
        let mut projection = TiberProjection::default();
        let mut page = EventPage::first(BatchSize::new(1024));
        loop {
            let events = store
                .read_events::<TiberEvent>(EventFilter::all(), page)
                .await
                .map_err(event_store_error)?;
            if events.is_empty() {
                break;
            }
            for (event, _) in &events {
                apply_tiber_event(&mut projection, event)?;
            }
            page = page.next(events.last().expect("nonempty page").1);
            if events.len() < 1024 {
                break;
            }
        }
        Ok(projection)
    })
}

fn task_mut<'a>(projection: &'a mut TiberProjection, stem: &str) -> Result<&'a mut Task, Error> {
    projection
        .tasks
        .get_mut(stem)
        .ok_or_else(|| Error::Parse(format!("task_event_without_creation ref={stem}")))
}

fn apply_tiber_event(projection: &mut TiberProjection, event: &TiberEvent) -> Result<(), Error> {
    fold_explicit_legacy_tiber_fact(event);
    let projected_event = projection_event(event);
    let event = &projected_event;
    match event {
        TiberEvent::RepositoryInitialized(_) => projection.initialized = true,
        TiberEvent::TaskCreated(TaskCreatedEvent { task, .. }) => {
            projection.tasks.insert(task.stem.clone(), (**task).clone());
        }
        TiberEvent::TaskTransitioned(TaskTransitionedEvent {
            stem,
            status,
            claim,
            completion_snapshot,
            ..
        }) => {
            let task = task_mut(projection, stem)?;
            task.status.clone_from(status);
            task.claim.clone_from(claim);
            if completion_snapshot.is_some() {
                task.final_review
                    .completion_snapshot
                    .clone_from(completion_snapshot);
            }
        }
        TiberEvent::TaskPriorityChanged(TaskOrderEvent { order, .. })
        | TiberEvent::BoardReordered(TaskOrderEvent { order, .. }) => {
            projection.order.clone_from(order)
        }
        TiberEvent::TaskLinksChanged(TaskLinksChangedEvent {
            stem,
            blocks,
            blocked_by,
            ..
        }) => {
            let task = task_mut(projection, stem)?;
            task.blocks.clone_from(blocks);
            task.blocked_by.clone_from(blocked_by);
        }
        TiberEvent::TaskSubtaskAdded(TaskSubtaskAddedEvent { stem, subtask, .. }) => {
            task_mut(projection, stem)?.subtasks.push(subtask.clone())
        }
        TiberEvent::TaskSubtaskChecked(TaskSubtaskCheckedEvent {
            stem,
            subtask_id,
            checked,
            ..
        }) => {
            let item = task_mut(projection, stem)?
                .subtasks
                .iter_mut()
                .find(|item| &item.id == subtask_id)
                .ok_or_else(|| Error::Parse(format!("subtask_ref_missing ref={subtask_id}")))?;
            item.checked = *checked;
        }
        TiberEvent::TaskDetailsUpdated(TaskDetailsUpdatedEvent {
            stem,
            title,
            tags,
            summary,
            context,
            ..
        }) => {
            let task = task_mut(projection, stem)?;
            task.title.clone_from(title);
            task.tags.clone_from(tags);
            task.summary.clone_from(summary);
            task.context.clone_from(context);
        }
        TiberEvent::LegacyTaskClaimChanged(TaskClaimChangedEvent { stem, claim, .. }) => {
            task_mut(projection, stem)?.claim.clone_from(claim)
        }
        TiberEvent::TaskPullRequestChanged(TaskPullRequestChangedEvent {
            stem,
            url,
            status,
            ..
        }) => {
            let task = task_mut(projection, stem)?;
            task.pr_mr_url.clone_from(url);
            task.pr_mr_status.clone_from(status);
        }
        TiberEvent::TaskAcceptanceAdded(TaskAcceptanceAddedEvent { stem, item, .. }) => {
            task_mut(projection, stem)?.acceptance.push(item.clone())
        }
        TiberEvent::TaskAcceptanceChecked(TaskAcceptanceCheckedEvent {
            stem,
            index,
            checked,
            ..
        }) => {
            let item = task_mut(projection, stem)?
                .acceptance
                .get_mut(*index)
                .ok_or_else(|| {
                    Error::Parse(format!("acceptance_index_missing index={}", index + 1))
                })?;
            item.checked = *checked;
        }
        TiberEvent::TaskAcceptanceRemoved(TaskAcceptanceRemovedEvent { stem, index, .. }) => {
            let task = task_mut(projection, stem)?;
            if *index >= task.acceptance.len() {
                return Err(Error::Parse(format!(
                    "acceptance_index_missing index={}",
                    index + 1
                )));
            }
            task.acceptance.remove(*index);
        }
        TiberEvent::TaskNoteAdded(TaskNoteAddedEvent { stem, note, .. }) => {
            task_mut(projection, stem)?.notes.push(note.clone())
        }
        TiberEvent::TaskFinalReviewRecorded(TaskFinalReviewRecordedEvent {
            stem, review, ..
        }) => {
            let state = &mut task_mut(projection, stem)?.final_review;
            state.reviews.push(review.clone());
            apply_final_review_record_to_clean_sequence(&mut state.clean_reviews, review);
        }
        TiberEvent::TaskValidationRepaired(TaskValidationRepairedEvent {
            link_changes,
            order_change,
            ..
        }) => {
            for change in link_changes {
                apply_tiber_event(projection, &TiberEvent::TaskLinksChanged(change.clone()))?;
            }
            if let Some(change) = order_change {
                apply_tiber_event(projection, &TiberEvent::BoardReordered(change.clone()))?;
            }
        }
        TiberEvent::LegacyTaskStatePublished(_) => {}
        TiberEvent::TasksClosedFromCommitTrailers(TasksClosedFromCommitTrailersEvent {
            stems,
            order,
            completion_snapshots,
            ..
        }) => {
            for stem in stems {
                let task = task_mut(projection, stem)?;
                task.status = "done".into();
                task.claim = None;
            }
            for (stem, snapshot) in completion_snapshots {
                task_mut(projection, stem)?.final_review.completion_snapshot =
                    Some(snapshot.clone());
            }
            projection.order = order.clone();
        }
        TiberEvent::LegacyTaskClosedFromTrailer(TaskStemEvent { stem, .. }) => {
            let task = task_mut(projection, stem)?;
            task.status = "done".into();
            task.claim = None;
        }
        TiberEvent::LegacyTaskRemoved(TaskStemEvent { stem, .. }) => {
            projection.tasks.remove(stem);
        }
        TiberEvent::CiRecoveryClaimed(CiRecoveryClaimedEvent {
            schema_version,
            incident_id,
            trigger,
            owner,
            lease_expires_at,
            ..
        }) => {
            projection.ci_recovery = Some(CiRecoveryState {
                schema_version: *schema_version,
                incident_id: incident_id.clone(),
                state: CiRecoveryPhase::Diagnosing,
                epoch: 1,
                trigger: trigger.clone().into(),
                triggers: vec![trigger.clone().into()],
                owner: owner.clone().into(),
                lease_expires_at: *lease_expires_at,
                participants: vec![owner.clone().into()],
                assignments: Vec::new(),
                failure_record: None,
                diagnosis: None,
                next_action: None,
                replacement: None,
                release_proof: None,
            });
        }
        TiberEvent::CiRecoveryJoined(CiRecoveryJoinedEvent {
            trigger,
            participant,
            ..
        }) => {
            let state = projection
                .ci_recovery
                .as_mut()
                .ok_or_else(|| Error::Parse("ci_recovery_join_without_claim=true".to_string()))?;
            if trigger.is_none() && participant.is_none() {
                return Err(Error::Parse("ci_recovery_join_empty_fact=true".to_string()));
            }
            if let Some(trigger) = trigger {
                let trigger: CiRecoveryTrigger = trigger.clone().into();
                state.trigger = trigger.clone();
                if !state.triggers.contains(&trigger) {
                    state.triggers.push(trigger);
                }
            }
            if let Some(participant) = participant {
                let participant: CiRecoveryParticipant = participant.clone().into();
                if !state.participants.contains(&participant) {
                    state.participants.push(participant);
                }
            }
        }
        TiberEvent::CiRecoveryTransferred(CiRecoveryTransferredEvent {
            owner,
            epoch,
            lease_expires_at,
            participant,
            ..
        })
        | TiberEvent::CiRecoveryTakenOver(CiRecoveryTakenOverEvent {
            owner,
            epoch,
            lease_expires_at,
            participant,
            ..
        }) => {
            let state = projection.ci_recovery.as_mut().ok_or_else(|| {
                Error::Parse("ci_recovery_ownership_change_without_claim=true".to_string())
            })?;
            state.owner = owner.clone().into();
            state.epoch = *epoch;
            state.lease_expires_at = *lease_expires_at;
            if let Some(participant) = participant {
                let participant: CiRecoveryParticipant = participant.clone().into();
                if !state.participants.contains(&participant) {
                    state.participants.push(participant);
                }
            }
        }
        TiberEvent::CiRecoveryAssigned(CiRecoveryAssignedEvent { assignment, .. }) => {
            let state = projection.ci_recovery.as_mut().ok_or_else(|| {
                Error::Parse("ci_recovery_assignment_without_claim=true".to_string())
            })?;
            let assignment: CiRecoveryAssignment = assignment.clone().into();
            if state
                .assignments
                .iter()
                .any(|current| current.id == assignment.id)
            {
                return Err(Error::Parse(
                    "ci_recovery_assignment_duplicate=true".to_string(),
                ));
            }
            state.assignments.push(assignment);
        }
        TiberEvent::CiRecoveryReported(CiRecoveryReportedEvent {
            assignment_id,
            assignee,
            report,
            ..
        }) => {
            let state = projection
                .ci_recovery
                .as_mut()
                .ok_or_else(|| Error::Parse("ci_recovery_report_without_claim=true".to_string()))?;
            let assignment = state
                .assignments
                .iter_mut()
                .find(|assignment| assignment.id == *assignment_id)
                .ok_or_else(|| Error::Parse("ci_recovery_report_assignment_missing=true".into()))?;
            if assignment.assignee != CiRecoveryParticipant::from(assignee.clone()) {
                return Err(Error::Parse(
                    "ci_recovery_report_assignee_invalid=true".into(),
                ));
            }
            if assignment.report.is_some() {
                return Err(Error::Parse("ci_recovery_report_duplicate=true".into()));
            }
            assignment.report = Some(report.clone().into());
        }
        TiberEvent::CiRecoveryHeartbeatRecorded(CiRecoveryHeartbeatRecordedEvent {
            epoch,
            owner,
            lease_expires_at,
            ..
        }) => {
            let state = projection.ci_recovery.as_mut().ok_or_else(|| {
                Error::Parse("ci_recovery_heartbeat_without_claim=true".to_string())
            })?;
            if state.epoch != *epoch || state.owner != CiRecoveryParticipant::from(owner.clone()) {
                return Err(Error::Parse(
                    "ci_recovery_heartbeat_owner_invalid=true".into(),
                ));
            }
            state.lease_expires_at = *lease_expires_at;
        }
        TiberEvent::CiRecoveryDiagnosed(CiRecoveryDiagnosedEvent {
            epoch,
            owner,
            failure_record,
            diagnosis,
            ..
        }) => {
            let state = projection.ci_recovery.as_mut().ok_or_else(|| {
                Error::Parse("ci_recovery_diagnosis_without_claim=true".to_string())
            })?;
            if state.epoch != *epoch || state.owner != CiRecoveryParticipant::from(owner.clone()) {
                return Err(Error::Parse(
                    "ci_recovery_diagnosis_owner_invalid=true".into(),
                ));
            }
            state.failure_record = Some(failure_record.clone().into());
            state.diagnosis = Some(diagnosis.clone().into());
            state.next_action = None;
            state.replacement = None;
            state.release_proof = None;
            state.state = CiRecoveryPhase::Diagnosing;
        }
        TiberEvent::CiRecoveryActionChosen(CiRecoveryActionChosenEvent {
            epoch,
            owner,
            action,
            ..
        }) => {
            let state = projection
                .ci_recovery
                .as_mut()
                .ok_or_else(|| Error::Parse("ci_recovery_action_without_claim=true".to_string()))?;
            if state.epoch != *epoch || state.owner != CiRecoveryParticipant::from(owner.clone()) {
                return Err(Error::Parse("ci_recovery_action_owner_invalid=true".into()));
            }
            if state.diagnosis.is_none() {
                return Err(Error::Parse(
                    "ci_recovery_action_without_diagnosis=true".into(),
                ));
            }
            state.next_action = Some(action.clone().into());
            state.state = CiRecoveryPhase::ActionSelected;
        }
        TiberEvent::CiRecoveryReplacementRecorded(CiRecoveryReplacementRecordedEvent {
            epoch,
            owner,
            replacement,
            ..
        }) => {
            let state = projection.ci_recovery.as_mut().ok_or_else(|| {
                Error::Parse("ci_recovery_replacement_without_claim=true".to_string())
            })?;
            if state.epoch != *epoch || state.owner != CiRecoveryParticipant::from(owner.clone()) {
                return Err(Error::Parse(
                    "ci_recovery_replacement_owner_invalid=true".into(),
                ));
            }
            if state.next_action.is_none() {
                return Err(Error::Parse(
                    "ci_recovery_replacement_without_action=true".into(),
                ));
            }
            let replacement: CiRecoveryReplacement = replacement.clone().into();
            let failed = replacement.status == CiRecoveryReplacementStatus::Failed;
            state.replacement = Some(replacement);
            if failed {
                state.state = CiRecoveryPhase::Diagnosing;
                state.failure_record = None;
                state.diagnosis = None;
                state.next_action = None;
            } else {
                state.state = CiRecoveryPhase::WaitingCi;
            }
        }
        TiberEvent::CiRecoveryResolved(CiRecoveryResolvedEvent {
            participant, proof, ..
        }) => {
            let state = projection.ci_recovery.as_mut().ok_or_else(|| {
                Error::Parse("ci_recovery_resolution_without_claim=true".to_string())
            })?;
            let participant: CiRecoveryParticipant = participant.clone().into();
            if !state.participants.contains(&participant) {
                return Err(Error::Parse(
                    "ci_recovery_resolution_participant_invalid=true".into(),
                ));
            }
            let replacement = state.replacement.as_ref().ok_or_else(|| {
                Error::Parse("ci_recovery_resolution_without_replacement=true".into())
            })?;
            let proof: CiRecoveryReleaseProof = proof.clone().into();
            if replacement.status == CiRecoveryReplacementStatus::Failed
                || replacement.run_id != proof.replacement_run_id
                || replacement.run_url != proof.replacement_run_url
                || replacement.sha != proof.sha
                || proof.terminal_status != "success"
            {
                return Err(Error::Parse(
                    "ci_recovery_resolution_proof_invalid=true".into(),
                ));
            }
            state.release_proof = Some(proof);
            state.state = CiRecoveryPhase::Resolved;
        }
        TiberEvent::LegacyRecoveryStatePublished(payload) => {
            projection.ci_recovery = Some(CiRecoveryState::from_snapshot(&payload.state));
        }
    }
    Ok(())
}

/// A request to establish Tiber's repository authority. The request contains
/// no event payload because the successful fact is fully determined by the
/// domain intent and its repository stream.
#[derive(ModelInput)]
struct InitializeTiberRepositoryRequest {
    #[model(origin)]
    stream: TiberRepositoryStream,
}

#[derive(Clone, Debug, Eq, PartialEq, StreamIdentity)]
struct TiberRepositoryStream(StreamId);

#[derive(ModelCommand)]
struct InitializeTiberRepository {
    #[stream]
    stream: TiberRepositoryStream,
}

mapping! { InitializeTiberRepositoryRequestToStream:
InitializeTiberRepositoryRequest.stream => InitializeTiberRepository.stream using clone; }

fn repository_initialized_fact(
    stream: &TiberRepositoryStream,
    _: &bool,
) -> RepositoryInitializedEvent {
    RepositoryInitializedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(stream).clone(),
    }
}

mapping! { InitializeTiberRepositoryToFact:
(InitializeTiberRepository.stream, InitializeTiberRepositoryState.initialized) => TiberEvent.RepositoryInitialized using repository_initialized_fact; }

/// Only the initialization fact can decide whether this intent is valid.
#[derive(ModelState)]
struct InitializeTiberRepositoryState {
    #[model(default)]
    initialized: bool,
}

/// Stable identity of the board authority stream. New task facts live here so
/// task commands can make a complete decision from one declared stream while
/// still discovering and replaying the older per-task streams.
#[derive(Clone, Debug, Eq, PartialEq, StreamIdentity)]
struct TiberBoardStream(StreamId);

/// A creation request is domain intent, not a pre-computed task event. The
/// command chooses the final human-facing stem after folding the current board
/// and its related legacy task streams.
#[derive(Clone)]
struct CreateTaskIntent {
    title: TaskTitle,
    task_id: String,
    recorded_at: String,
    max_queued: Option<usize>,
}

#[derive(ModelInput)]
struct CreateTaskRequest {
    #[model(origin)]
    board: TiberBoardStream,
    #[model(origin)]
    intent: CreateTaskIntent,
}

#[derive(ModelCommand)]
struct CreateTask {
    #[stream]
    board: TiberBoardStream,
    intent: CreateTaskIntent,
}

mapping! { CreateTaskRequestToBoard:
CreateTaskRequest.board => CreateTask.board using clone; }
mapping! { CreateTaskRequestToIntent:
CreateTaskRequest.intent => CreateTask.intent using clone; }

/// The narrow state required by task creation. `board_task_stems` identifies
/// current facts already stored in the board stream; any ordered task absent
/// from it belongs to a legacy per-task stream and is discovered before the
/// command decides.
#[derive(ModelState)]
struct CreateTaskState {
    #[model(default)]
    board_order: Vec<String>,
    #[model(default)]
    board_task_stems: BTreeSet<String>,
    #[model(default)]
    task_statuses: BTreeMap<String, String>,
}

fn task_stem_for_creation(state: &CreateTaskState, intent: &CreateTaskIntent) -> String {
    let base = intent.title.file_stem();
    let mut nickname = base.clone();
    let mut suffix = 2;
    while state
        .task_statuses
        .keys()
        .any(|stem| stem.ends_with(&format!("-{nickname}")))
    {
        nickname = format!("{base}-{suffix}");
        suffix += 1;
    }
    format!("{}-{nickname}", intent.task_id)
}

fn created_task_fact(
    board: &TiberBoardStream,
    intent: &CreateTaskIntent,
    board_order: &[String],
    board_task_stems: &BTreeSet<String>,
    task_statuses: &BTreeMap<String, String>,
) -> TaskCreatedEvent {
    let state = CreateTaskState {
        board_order: board_order.to_vec(),
        board_task_stems: board_task_stems.clone(),
        task_statuses: task_statuses.clone(),
    };
    let stem = task_stem_for_creation(&state, intent);
    debug_assert!(!state.task_statuses.contains_key(&stem));
    TaskCreatedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        task: Box::new(Task::new(
            stem,
            intent.title.as_str().to_string(),
            intent.recorded_at.clone(),
        )),
    }
}

fn created_task_order_fact(
    board: &TiberBoardStream,
    intent: &CreateTaskIntent,
    board_order: &[String],
    board_task_stems: &BTreeSet<String>,
    task_statuses: &BTreeMap<String, String>,
) -> TaskOrderEvent {
    let state = CreateTaskState {
        board_order: board_order.to_vec(),
        board_task_stems: board_task_stems.clone(),
        task_statuses: task_statuses.clone(),
    };
    let mut order = state.board_order.clone();
    order.push(task_stem_for_creation(&state, intent));
    TaskOrderEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        order,
    }
}

mapping! { CreateTaskToCreatedFact:
    (CreateTask.board, CreateTask.intent, CreateTaskState.board_order, CreateTaskState.board_task_stems, CreateTaskState.task_statuses) => TiberEvent.TaskCreated
    using created_task_fact;
}
mapping! { CreateTaskToOrderFact:
    (CreateTask.board, CreateTask.intent, CreateTaskState.board_order, CreateTaskState.board_task_stems, CreateTaskState.task_statuses) => TiberEvent.BoardReordered
    using created_task_order_fact;
}

impl ModelCommandLogic for CreateTask {
    type Event = TiberEvent;
    type State = CreateTaskState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        match event {
            TiberEvent::TaskCreated(TaskCreatedEvent { stream_id, task }) => {
                if stream_id.as_ref() == BOARD_STREAM {
                    state.board_task_stems.insert(task.stem.clone());
                }
                state
                    .task_statuses
                    .insert(task.stem.clone(), task.status.clone());
            }
            TiberEvent::TaskTransitioned(TaskTransitionedEvent { stem, status, .. }) => {
                state.task_statuses.insert(stem.clone(), status.clone());
            }
            TiberEvent::LegacyTaskClosedFromTrailer(TaskStemEvent { stem, .. }) => {
                state.task_statuses.insert(stem.clone(), "done".into());
            }
            TiberEvent::TasksClosedFromCommitTrailers(event) => {
                for stem in &event.stems {
                    state.task_statuses.insert(stem.clone(), "done".into());
                }
                state.board_order.clone_from(&event.order);
            }
            TiberEvent::LegacyTaskRemoved(TaskStemEvent { stem, .. }) => {
                state.task_statuses.remove(stem);
                state.board_task_stems.remove(stem);
            }
            TiberEvent::TaskPriorityChanged(TaskOrderEvent { order, .. })
            | TiberEvent::BoardReordered(TaskOrderEvent { order, .. }) => {
                state.board_order.clone_from(order);
            }
            _ => {}
        }
        Modeled::from_built(state)
    }

    fn discover_related_streams(&self, state: &Modeled<Self::State>) -> Vec<StreamId> {
        state
            .as_ref()
            .board_order
            .iter()
            .filter(|stem| !state.as_ref().board_task_stems.contains(*stem))
            .filter_map(|stem| stream_id(format!("tiber:task:{stem}")).ok())
            .collect()
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let state = state.as_ref();
        let queued = state
            .task_statuses
            .values()
            .filter(|status| status.as_str() == "backlog")
            .count();
        if let Some(max_queued) = self.intent.max_queued {
            if !backlog_admission_allowed(queued, max_queued) {
                return Err(format!(
                    "tiber_backlog_capacity_exceeded queued={queued} max_queued={max_queued}"
                )
                .into());
            }
        }

        let stem = task_stem_for_creation(state, &self.intent);
        if state.task_statuses.contains_key(&stem) {
            return Err("tiber_task_already_exists".into());
        }
        let mut facts = ModeledEvents::one(TiberEvent::model_variant_taskcreated(
            CreateTaskToCreatedFact::apply((self, self, state, state, state)),
        ));
        facts.push(TiberEvent::model_variant_boardreordered(
            CreateTaskToOrderFact::apply((self, self, state, state, state)),
        ));
        Ok(facts)
    }
}

fn execute_create_task(root: &Path, title: TaskTitle) -> Result<TaskPath, Error> {
    let repository = GitRepository::at(root);
    let _lock = repository.acquire_lock()?;
    execute_initialize_tiber_repository(root)?;
    let task_id = new_task_id();
    let max_queued = repository.project_config()?.backlog.max_queued;
    let intent = CreateTaskIntent {
        title,
        task_id: task_id.clone(),
        recorded_at: command_recorded_at(),
        max_queued,
    };
    let request = CreateTaskRequest::model_builder()
        .board(TiberBoardStream(stream_id(BOARD_STREAM)?))
        .intent(intent)
        .build();
    let command = CreateTask::model_builder()
        .board(CreateTaskRequestToBoard::apply(request.as_ref()))
        .intent(CreateTaskRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => load_tiber_projection(root)?
            .tasks
            .keys()
            .find(|stem| stem.starts_with(&format!("{task_id}-")))
            .cloned()
            .map(|path| TaskPath { path })
            .ok_or_else(|| Error::Parse("eventcore_created_task_missing=true".into())),
        Err(eventcore::CommandError::BusinessRuleViolation(error)) => {
            let message = error.to_string();
            let Some((queued, max_queued)) = message
                .strip_prefix("tiber_backlog_capacity_exceeded queued=")
                .and_then(|value| value.split_once(" max_queued="))
                .and_then(|(queued, max_queued)| {
                    Some((queued.parse().ok()?, max_queued.parse().ok()?))
                })
            else {
                return Err(Error::Parse("eventcore_command_rejected=true".to_string()));
            };
            Err(Error::BacklogCapacityExceeded { queued, max_queued })
        }
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

#[derive(Clone)]
struct TransitionTaskIntent {
    stem: String,
    status: String,
    claim: Option<Claim>,
    max_queued: Option<usize>,
    minimum_clean_reviews: usize,
    expected_clean_reviews: Vec<FinalReviewRecord>,
    completion_snapshot: Option<FinalReviewCompletionSnapshot>,
}

#[derive(ModelInput)]
struct TransitionTaskRequest {
    #[model(origin)]
    board: TiberBoardStream,
    #[model(origin)]
    intent: TransitionTaskIntent,
}

#[derive(ModelCommand)]
struct TransitionTask {
    #[stream]
    board: TiberBoardStream,
    intent: TransitionTaskIntent,
}

mapping! { TransitionTaskRequestToBoard:
TransitionTaskRequest.board => TransitionTask.board using clone; }
mapping! { TransitionTaskRequestToIntent:
TransitionTaskRequest.intent => TransitionTask.intent using clone; }

/// Transitioning a task needs its current lifecycle fact, claim, and board
/// membership—nothing from unrelated task fields.
#[derive(ModelState)]
struct TransitionTaskState {
    #[model(default)]
    board_order: Vec<String>,
    #[model(default)]
    board_task_stems: BTreeSet<String>,
    #[model(default)]
    task_statuses: BTreeMap<String, String>,
    #[model(default)]
    target_claim: Option<Option<Claim>>,
    #[model(default)]
    final_review_clean_reviews: BTreeMap<String, Vec<FinalReviewRecord>>,
}

fn transitioned_task_fact(
    board: &TiberBoardStream,
    intent: &TransitionTaskIntent,
    board_order: &[String],
    board_task_stems: &BTreeSet<String>,
    task_statuses: &BTreeMap<String, String>,
    target_claim: &Option<Option<Claim>>,
    _final_review_clean_reviews: &BTreeMap<String, Vec<FinalReviewRecord>>,
) -> TaskTransitionedEvent {
    let _ = board_order;
    debug_assert!(
        board_task_stems.contains(&intent.stem) || task_statuses.contains_key(&intent.stem)
    );
    debug_assert!(task_statuses.contains_key(&intent.stem));
    debug_assert!(target_claim.is_some());
    TaskTransitionedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        stem: intent.stem.clone(),
        status: intent.status.clone(),
        claim: intent.claim.clone(),
        completion_snapshot: intent.completion_snapshot.clone(),
    }
}

fn transitioned_task_order_fact(
    board: &TiberBoardStream,
    intent: &TransitionTaskIntent,
    board_order: &[String],
    board_task_stems: &BTreeSet<String>,
    task_statuses: &BTreeMap<String, String>,
    target_claim: &Option<Option<Claim>>,
    _final_review_clean_reviews: &BTreeMap<String, Vec<FinalReviewRecord>>,
) -> TaskOrderEvent {
    debug_assert!(
        board_task_stems.contains(&intent.stem) || task_statuses.contains_key(&intent.stem)
    );
    debug_assert!(target_claim.is_some());
    let mut order = board_order.to_vec();
    if is_open_status(&intent.status) {
        if !order.contains(&intent.stem) {
            order.push(intent.stem.clone());
        }
    } else {
        order.retain(|entry| entry != &intent.stem);
    }
    TaskOrderEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        order,
    }
}

mapping! { TransitionTaskToLifecycleFact:
    (TransitionTask.board, TransitionTask.intent, TransitionTaskState.board_order, TransitionTaskState.board_task_stems, TransitionTaskState.task_statuses, TransitionTaskState.target_claim, TransitionTaskState.final_review_clean_reviews) => TiberEvent.TaskTransitioned
    using transitioned_task_fact;
}
mapping! { TransitionTaskToOrderFact:
    (TransitionTask.board, TransitionTask.intent, TransitionTaskState.board_order, TransitionTaskState.board_task_stems, TransitionTaskState.task_statuses, TransitionTaskState.target_claim, TransitionTaskState.final_review_clean_reviews) => TiberEvent.BoardReordered
    using transitioned_task_order_fact;
}

impl ModelCommandLogic for TransitionTask {
    type Event = TiberEvent;
    type State = TransitionTaskState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        match event {
            TiberEvent::TaskCreated(TaskCreatedEvent { stream_id, task }) => {
                if stream_id.as_ref() == BOARD_STREAM {
                    state.board_task_stems.insert(task.stem.clone());
                }
                state
                    .task_statuses
                    .insert(task.stem.clone(), task.status.clone());
                if task.stem == self.intent.stem {
                    state.target_claim = Some(task.claim.clone());
                }
            }
            TiberEvent::TaskTransitioned(TaskTransitionedEvent {
                stem,
                status,
                claim,
                ..
            }) => {
                state.task_statuses.insert(stem.clone(), status.clone());
                if stem == &self.intent.stem {
                    state.target_claim = Some(claim.clone());
                }
            }
            TiberEvent::LegacyTaskClosedFromTrailer(TaskStemEvent { stem, .. }) => {
                state.task_statuses.insert(stem.clone(), "done".into());
                if stem == &self.intent.stem {
                    state.target_claim = Some(None);
                }
            }
            TiberEvent::TasksClosedFromCommitTrailers(event) => {
                for stem in &event.stems {
                    state.task_statuses.insert(stem.clone(), "done".into());
                    if stem == &self.intent.stem {
                        state.target_claim = Some(None);
                    }
                }
                state.board_order.clone_from(&event.order);
            }
            TiberEvent::TaskFinalReviewRecorded(TaskFinalReviewRecordedEvent {
                stem,
                review,
                ..
            }) => {
                apply_final_review_record_to_clean_sequence(
                    state
                        .final_review_clean_reviews
                        .entry(stem.clone())
                        .or_default(),
                    review,
                );
            }
            TiberEvent::LegacyTaskRemoved(TaskStemEvent { stem, .. }) => {
                state.task_statuses.remove(stem);
                if stem == &self.intent.stem {
                    state.target_claim = None;
                }
                state.board_task_stems.remove(stem);
            }
            TiberEvent::TaskPriorityChanged(TaskOrderEvent { order, .. })
            | TiberEvent::BoardReordered(TaskOrderEvent { order, .. }) => {
                state.board_order.clone_from(order);
            }
            _ => {}
        }
        Modeled::from_built(state)
    }

    fn discover_related_streams(&self, state: &Modeled<Self::State>) -> Vec<StreamId> {
        state
            .as_ref()
            .board_order
            .iter()
            .filter(|stem| !state.as_ref().board_task_stems.contains(*stem))
            .filter_map(|stem| stream_id(format!("tiber:task:{stem}")).ok())
            .collect()
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let state = state.as_ref();
        let old_status = state
            .task_statuses
            .get(&self.intent.stem)
            .ok_or("tiber_task_missing")?;
        let old_claim = state.target_claim.clone().ok_or("tiber_task_missing")?;
        if self.intent.status == "done" && self.intent.minimum_clean_reviews > 0 {
            enforce_final_review_authority_snapshot(
                &self.intent.stem,
                state
                    .final_review_clean_reviews
                    .get(&self.intent.stem)
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
                &self.intent.expected_clean_reviews,
                self.intent.minimum_clean_reviews,
                false,
            )?;
            let snapshot = self.intent.completion_snapshot.as_ref().ok_or_else(|| {
                eventcore::CommandError::from(format!(
                    "tiber.final_review_completion_snapshot_missing task={} action=\"retry completion against an immutable commit snapshot\"",
                    self.intent.stem
                ))
            })?;
            let latest = state
                .final_review_clean_reviews
                .get(&self.intent.stem)
                .and_then(|reviews| reviews.last())
                .ok_or("tiber.final_review_completion_snapshot_missing")?;
            if snapshot.source_fingerprint != latest.source_fingerprint
                || snapshot.verification_fingerprint != latest.verification_fingerprint
            {
                return Err(format!(
                    "tiber.final_review_completion_snapshot_changed task={} required={} clean=0 missing={} action=\"record fresh reviews for the immutable completion snapshot\"",
                    self.intent.stem,
                    self.intent.minimum_clean_reviews,
                    self.intent.minimum_clean_reviews
                )
                .into());
            }
        }
        let admits_to_backlog = self.intent.status == "backlog" && old_status != "backlog";
        if admits_to_backlog {
            let queued = state
                .task_statuses
                .values()
                .filter(|status| status.as_str() == "backlog")
                .count();
            if let Some(max_queued) = self.intent.max_queued {
                if !backlog_admission_allowed(queued, max_queued) {
                    return Err(format!(
                        "tiber_backlog_capacity_exceeded queued={queued} max_queued={max_queued}"
                    )
                    .into());
                }
            }
        }

        let mut order = state.board_order.clone();
        if is_open_status(&self.intent.status) {
            if !order.contains(&self.intent.stem) {
                order.push(self.intent.stem.clone());
            }
        } else {
            order.retain(|entry| entry != &self.intent.stem);
        }
        let status_changed = old_status != &self.intent.status || old_claim != self.intent.claim;
        let order_changed = order != state.board_order;
        if !status_changed && !order_changed {
            return Ok(ModeledEvents::none(
                "task already has requested lifecycle state",
            ));
        }

        let mut facts = ModeledEvents::none("transition facts initialized");
        if status_changed {
            facts.push(TiberEvent::model_variant_tasktransitioned(
                TransitionTaskToLifecycleFact::apply((
                    self, self, state, state, state, state, state,
                )),
            ));
        }
        if order_changed {
            facts.push(TiberEvent::model_variant_boardreordered(
                TransitionTaskToOrderFact::apply((self, self, state, state, state, state, state)),
            ));
        }
        Ok(facts)
    }
}

fn resolve_task_stem_from_projection(
    projection: &TiberProjection,
    task_ref: &str,
) -> Result<String, Error> {
    if task_ref.contains('/') || task_ref.ends_with(".md") || task_ref.trim().is_empty() {
        return Err(Error::Parse(format!("invalid_task_ref ref={task_ref}")));
    }
    let mut matches = projection
        .tasks
        .keys()
        .filter(|stem| {
            let id = stem
                .split_once('-')
                .and_then(|(date, rest)| {
                    rest.split_once('-')
                        .map(|(code, _)| format!("{date}-{code}"))
                })
                .unwrap_or_default();
            let nickname = stem
                .split_once('-')
                .and_then(|(_, rest)| rest.split_once('-'))
                .map(|(_, nickname)| nickname)
                .unwrap_or_default();
            stem.as_str() == task_ref || id == task_ref || nickname == task_ref
        })
        .cloned()
        .collect::<Vec<_>>();
    matches.sort();
    match matches.as_slice() {
        [resolved] => Ok(resolved.clone()),
        [] => Err(Error::Parse(format!("task_ref_missing ref={task_ref}"))),
        _ => Err(Error::Parse(format!(
            "ambiguous_task_ref ref={task_ref} matches={}",
            matches.join(",")
        ))),
    }
}

fn execute_transition_task(root: &Path, task_ref: &str, status: &str) -> Result<TaskPath, Error> {
    let repository = GitRepository::at(root);
    let _lock = repository.acquire_lock()?;
    execute_initialize_tiber_repository(root)?;
    let projection = load_tiber_projection(root)?;
    let stem = resolve_task_stem_from_projection(&projection, task_ref)?;
    let status = parse_status(status)?.to_string();
    let project_config = if status == "done" {
        repository.project_config_for_lifecycle_gate()?
    } else {
        repository.project_config()?
    };
    let completion_snapshot =
        if status == "done" && project_config.final_review.minimum_clean_reviews > 0 {
            let task = projection
                .tasks
                .get(&stem)
                .ok_or_else(|| Error::Parse(format!("task_ref_missing ref={task_ref}")))?;
            enforce_final_review_gate(
                root,
                task,
                project_config.final_review.minimum_clean_reviews,
                false,
            )?;
            let latest = task
                .final_review
                .clean_reviews
                .last()
                .expect("gate requires reviews");
            Some(completion_snapshot_for_review(
                root,
                "HEAD",
                latest,
                project_config.final_review.minimum_clean_reviews,
                false,
            )?)
        } else {
            None
        };
    if completion_snapshot.is_some() {
        wait_at_completion_snapshot_test_barrier()?;
    }
    let intent = TransitionTaskIntent {
        stem: stem.clone(),
        claim: (status == "in-progress").then(|| Claim {
            host: claim_host(),
            session: claim_session(),
        }),
        status,
        max_queued: project_config.backlog.max_queued,
        minimum_clean_reviews: project_config.final_review.minimum_clean_reviews,
        expected_clean_reviews: projection
            .tasks
            .get(&stem)
            .map(|task| task.final_review.clean_reviews.clone())
            .unwrap_or_default(),
        completion_snapshot,
    };
    let request = TransitionTaskRequest::model_builder()
        .board(TiberBoardStream(stream_id(BOARD_STREAM)?))
        .intent(intent)
        .build();
    let command = TransitionTask::model_builder()
        .board(TransitionTaskRequestToBoard::apply(request.as_ref()))
        .intent(TransitionTaskRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(TaskPath { path: stem }),
        Err(eventcore::CommandError::BusinessRuleViolation(error)) => {
            let message = error.to_string();
            if message.starts_with("tiber.final_review_") {
                return Err(Error::Usage(message));
            }
            let Some((queued, max_queued)) = message
                .strip_prefix("tiber_backlog_capacity_exceeded queued=")
                .and_then(|value| value.split_once(" max_queued="))
                .and_then(|(queued, max_queued)| {
                    Some((queued.parse().ok()?, max_queued.parse().ok()?))
                })
            else {
                return Err(Error::Parse("eventcore_command_rejected=true".to_string()));
            };
            Err(Error::BacklogCapacityExceeded { queued, max_queued })
        }
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

#[derive(Clone)]
struct PrioritizeTaskIntent {
    task_stem: String,
    before_stem: String,
}

#[derive(ModelInput)]
struct PrioritizeTaskRequest {
    #[model(origin)]
    board: TiberBoardStream,
    #[model(origin)]
    intent: PrioritizeTaskIntent,
}

#[derive(ModelCommand)]
struct PrioritizeTask {
    #[stream]
    board: TiberBoardStream,
    intent: PrioritizeTaskIntent,
}

mapping! { PrioritizeTaskRequestToBoard:
PrioritizeTaskRequest.board => PrioritizeTask.board using clone; }
mapping! { PrioritizeTaskRequestToIntent:
PrioritizeTaskRequest.intent => PrioritizeTask.intent using clone; }

#[derive(ModelState)]
struct PrioritizeTaskState {
    #[model(default)]
    board_order: Vec<String>,
}

fn prioritized_task_fact(
    board: &TiberBoardStream,
    intent: &PrioritizeTaskIntent,
    board_order: &[String],
) -> TaskOrderEvent {
    let mut order = board_order.to_vec();
    order.retain(|entry| entry != &intent.task_stem);
    let before_index = order
        .iter()
        .position(|entry| entry == &intent.before_stem)
        .expect("validated prioritization target must remain on the board");
    order.insert(before_index, intent.task_stem.clone());
    TaskOrderEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        order,
    }
}

mapping! { PrioritizeTaskToFact:
    (PrioritizeTask.board, PrioritizeTask.intent, PrioritizeTaskState.board_order) => TiberEvent.TaskPriorityChanged
    using prioritized_task_fact;
}

impl ModelCommandLogic for PrioritizeTask {
    type Event = TiberEvent;
    type State = PrioritizeTaskState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        if let TiberEvent::TaskPriorityChanged(TaskOrderEvent { order, .. })
        | TiberEvent::BoardReordered(TaskOrderEvent { order, .. }) = event
        {
            state.board_order.clone_from(order);
        }
        Modeled::from_built(state)
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let mut order = state.as_ref().board_order.clone();
        order.retain(|entry| entry != &self.intent.task_stem);
        let before_index = order
            .iter()
            .position(|entry| entry == &self.intent.before_stem)
            .ok_or("tiber_prioritization_target_not_on_board")?;
        order.insert(before_index, self.intent.task_stem.clone());
        if order == state.as_ref().board_order {
            return Ok(ModeledEvents::none("task already has requested priority"));
        }
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_taskprioritychanged(PrioritizeTaskToFact::apply((
                self,
                self,
                state.as_ref(),
            ))),
        ))
    }
}

fn execute_prioritize_task(root: &Path, task_ref: &str, before_ref: &str) -> Result<(), Error> {
    let repository = GitRepository::at(root);
    let _lock = repository.acquire_lock()?;
    execute_initialize_tiber_repository(root)?;
    let projection = load_tiber_projection(root)?;
    let intent = PrioritizeTaskIntent {
        task_stem: resolve_task_stem_from_projection(&projection, task_ref)?,
        before_stem: resolve_task_stem_from_projection(&projection, before_ref)?,
    };
    let request = PrioritizeTaskRequest::model_builder()
        .board(TiberBoardStream(stream_id(BOARD_STREAM)?))
        .intent(intent)
        .build();
    let command = PrioritizeTask::model_builder()
        .board(PrioritizeTaskRequestToBoard::apply(request.as_ref()))
        .intent(PrioritizeTaskRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

#[derive(Clone)]
struct TaskLinkIntent {
    from_stem: String,
    to_stem: String,
}

/// Linking decisions need only each endpoint's current links. Board membership
/// is retained solely to discover legacy per-task streams.
#[derive(ModelState)]
struct TaskLinkState {
    #[model(default)]
    board_order: Vec<String>,
    #[model(default)]
    board_task_stems: BTreeSet<String>,
    #[model(default)]
    source_links: Option<(Vec<String>, Vec<String>)>,
    #[model(default)]
    target_links: Option<(Vec<String>, Vec<String>)>,
}

fn evolve_task_links(
    mut state: TaskLinkState,
    intent: &TaskLinkIntent,
    event: &TiberEvent,
) -> TaskLinkState {
    match event {
        TiberEvent::TaskCreated(TaskCreatedEvent { stream_id, task }) => {
            if stream_id.as_ref() == BOARD_STREAM {
                state.board_task_stems.insert(task.stem.clone());
            }
            let links = (task.blocks.clone(), task.blocked_by.clone());
            if task.stem == intent.from_stem {
                state.source_links = Some(links.clone());
            }
            if task.stem == intent.to_stem {
                state.target_links = Some(links);
            }
        }
        TiberEvent::TaskLinksChanged(TaskLinksChangedEvent {
            stem,
            blocks,
            blocked_by,
            ..
        }) => {
            let links = (blocks.clone(), blocked_by.clone());
            if stem == &intent.from_stem {
                state.source_links = Some(links.clone());
            }
            if stem == &intent.to_stem {
                state.target_links = Some(links);
            }
        }
        TiberEvent::LegacyTaskRemoved(TaskStemEvent { stem, .. }) => {
            if stem == &intent.from_stem {
                state.source_links = None;
            }
            if stem == &intent.to_stem {
                state.target_links = None;
            }
            state.board_task_stems.remove(stem);
        }
        TiberEvent::TaskPriorityChanged(TaskOrderEvent { order, .. })
        | TiberEvent::BoardReordered(TaskOrderEvent { order, .. }) => {
            state.board_order.clone_from(order);
        }
        _ => {}
    }
    state
}

fn discover_legacy_task_link_streams(state: &TaskLinkState) -> Vec<StreamId> {
    state
        .board_order
        .iter()
        .filter(|stem| !state.board_task_stems.contains(*stem))
        .filter_map(|stem| stream_id(format!("tiber:task:{stem}")).ok())
        .collect()
}

#[derive(ModelInput)]
struct LinkTasksRequest {
    #[model(origin)]
    board: TiberBoardStream,
    #[model(origin)]
    intent: TaskLinkIntent,
}

#[derive(ModelCommand)]
struct LinkTasks {
    #[stream]
    board: TiberBoardStream,
    intent: TaskLinkIntent,
}

mapping! { LinkTasksRequestToBoard:
LinkTasksRequest.board => LinkTasks.board using clone; }
mapping! { LinkTasksRequestToIntent:
LinkTasksRequest.intent => LinkTasks.intent using clone; }

enum TaskLinkFactChange {
    LinkSource,
    LinkTarget,
    UnlinkSource,
    UnlinkTarget,
}

fn changed_task_link_fact(
    board: &TiberBoardStream,
    intent: &TaskLinkIntent,
    board_order: &[String],
    board_task_stems: &BTreeSet<String>,
    source_links: &Option<(Vec<String>, Vec<String>)>,
    target_links: &Option<(Vec<String>, Vec<String>)>,
    change: TaskLinkFactChange,
) -> TaskLinksChangedEvent {
    let (link, target) = match change {
        TaskLinkFactChange::LinkSource => (true, false),
        TaskLinkFactChange::LinkTarget => (true, true),
        TaskLinkFactChange::UnlinkSource => (false, false),
        TaskLinkFactChange::UnlinkTarget => (false, true),
    };
    debug_assert!(board_order.contains(&intent.from_stem) || source_links.is_some());
    debug_assert!(board_order.contains(&intent.to_stem) || target_links.is_some());
    debug_assert!(board_task_stems.contains(&intent.from_stem) || source_links.is_some());
    let stem = if target {
        &intent.to_stem
    } else {
        &intent.from_stem
    };
    let (mut blocks, mut blocked_by) = if target { target_links } else { source_links }
        .as_ref()
        .cloned()
        .expect("validated task link endpoint must exist");
    if target || intent.from_stem == intent.to_stem {
        if link {
            if !blocked_by.contains(&intent.from_stem) {
                blocked_by.push(intent.from_stem.clone());
            }
        } else {
            blocked_by.retain(|entry| entry != &intent.from_stem);
        }
    }
    if !target {
        if link {
            if !blocks.contains(&intent.to_stem) {
                blocks.push(intent.to_stem.clone());
            }
        } else {
            blocks.retain(|entry| entry != &intent.to_stem);
        }
    }
    TaskLinksChangedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        stem: stem.clone(),
        blocks,
        blocked_by,
    }
}

fn linked_source_fact(
    board: &TiberBoardStream,
    intent: &TaskLinkIntent,
    board_order: &[String],
    board_task_stems: &BTreeSet<String>,
    source_links: &Option<(Vec<String>, Vec<String>)>,
    target_links: &Option<(Vec<String>, Vec<String>)>,
) -> TaskLinksChangedEvent {
    changed_task_link_fact(
        board,
        intent,
        board_order,
        board_task_stems,
        source_links,
        target_links,
        TaskLinkFactChange::LinkSource,
    )
}
fn linked_target_fact(
    board: &TiberBoardStream,
    intent: &TaskLinkIntent,
    board_order: &[String],
    board_task_stems: &BTreeSet<String>,
    source_links: &Option<(Vec<String>, Vec<String>)>,
    target_links: &Option<(Vec<String>, Vec<String>)>,
) -> TaskLinksChangedEvent {
    changed_task_link_fact(
        board,
        intent,
        board_order,
        board_task_stems,
        source_links,
        target_links,
        TaskLinkFactChange::LinkTarget,
    )
}
fn unlinked_source_fact(
    board: &TiberBoardStream,
    intent: &TaskLinkIntent,
    board_order: &[String],
    board_task_stems: &BTreeSet<String>,
    source_links: &Option<(Vec<String>, Vec<String>)>,
    target_links: &Option<(Vec<String>, Vec<String>)>,
) -> TaskLinksChangedEvent {
    changed_task_link_fact(
        board,
        intent,
        board_order,
        board_task_stems,
        source_links,
        target_links,
        TaskLinkFactChange::UnlinkSource,
    )
}
fn unlinked_target_fact(
    board: &TiberBoardStream,
    intent: &TaskLinkIntent,
    board_order: &[String],
    board_task_stems: &BTreeSet<String>,
    source_links: &Option<(Vec<String>, Vec<String>)>,
    target_links: &Option<(Vec<String>, Vec<String>)>,
) -> TaskLinksChangedEvent {
    changed_task_link_fact(
        board,
        intent,
        board_order,
        board_task_stems,
        source_links,
        target_links,
        TaskLinkFactChange::UnlinkTarget,
    )
}

mapping! { LinkTasksToSourceFact:
    (LinkTasks.board, LinkTasks.intent, TaskLinkState.board_order, TaskLinkState.board_task_stems, TaskLinkState.source_links, TaskLinkState.target_links) => TiberEvent.TaskLinksChanged
    using linked_source_fact;
}
mapping! { LinkTasksToTargetFact:
    (LinkTasks.board, LinkTasks.intent, TaskLinkState.board_order, TaskLinkState.board_task_stems, TaskLinkState.source_links, TaskLinkState.target_links) => TiberEvent.TaskLinksChanged
    using linked_target_fact;
}

impl ModelCommandLogic for LinkTasks {
    type Event = TiberEvent;
    type State = TaskLinkState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        Modeled::from_built(evolve_task_links(state.into_inner(), &self.intent, event))
    }

    fn discover_related_streams(&self, state: &Modeled<Self::State>) -> Vec<StreamId> {
        discover_legacy_task_link_streams(state.as_ref())
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let state = state.as_ref();
        let (from_blocks, _from_blocked_by) = state
            .source_links
            .as_ref()
            .ok_or("tiber_link_source_missing")?;
        let (_to_blocks, to_blocked_by) = state
            .target_links
            .as_ref()
            .ok_or("tiber_link_target_missing")?;
        let mut next_from_blocks = from_blocks.clone();
        if !next_from_blocks.contains(&self.intent.to_stem) {
            next_from_blocks.push(self.intent.to_stem.clone());
        }
        let mut next_to_blocked_by = to_blocked_by.clone();
        if !next_to_blocked_by.contains(&self.intent.from_stem) {
            next_to_blocked_by.push(self.intent.from_stem.clone());
        }
        if &next_from_blocks == from_blocks && &next_to_blocked_by == to_blocked_by {
            return Ok(ModeledEvents::none("dependency already exists"));
        }
        if self.intent.from_stem == self.intent.to_stem {
            return Ok(ModeledEvents::one(
                TiberEvent::model_variant_tasklinkschanged(LinkTasksToSourceFact::apply((
                    self, self, state, state, state, state,
                ))),
            ));
        }
        let mut facts = ModeledEvents::none("link facts initialized");
        if &next_from_blocks != from_blocks {
            facts.push(TiberEvent::model_variant_tasklinkschanged(
                LinkTasksToSourceFact::apply((self, self, state, state, state, state)),
            ));
        }
        if &next_to_blocked_by != to_blocked_by {
            facts.push(TiberEvent::model_variant_tasklinkschanged(
                LinkTasksToTargetFact::apply((self, self, state, state, state, state)),
            ));
        }
        Ok(facts)
    }
}

#[derive(ModelInput)]
struct UnlinkTasksRequest {
    #[model(origin)]
    board: TiberBoardStream,
    #[model(origin)]
    intent: TaskLinkIntent,
}

#[derive(ModelCommand)]
struct UnlinkTasks {
    #[stream]
    board: TiberBoardStream,
    intent: TaskLinkIntent,
}

mapping! { UnlinkTasksRequestToBoard:
UnlinkTasksRequest.board => UnlinkTasks.board using clone; }
mapping! { UnlinkTasksRequestToIntent:
UnlinkTasksRequest.intent => UnlinkTasks.intent using clone; }
mapping! { UnlinkTasksToSourceFact:
    (UnlinkTasks.board, UnlinkTasks.intent, TaskLinkState.board_order, TaskLinkState.board_task_stems, TaskLinkState.source_links, TaskLinkState.target_links) => TiberEvent.TaskLinksChanged
    using unlinked_source_fact;
}
mapping! { UnlinkTasksToTargetFact:
    (UnlinkTasks.board, UnlinkTasks.intent, TaskLinkState.board_order, TaskLinkState.board_task_stems, TaskLinkState.source_links, TaskLinkState.target_links) => TiberEvent.TaskLinksChanged
    using unlinked_target_fact;
}

impl ModelCommandLogic for UnlinkTasks {
    type Event = TiberEvent;
    type State = TaskLinkState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        Modeled::from_built(evolve_task_links(state.into_inner(), &self.intent, event))
    }

    fn discover_related_streams(&self, state: &Modeled<Self::State>) -> Vec<StreamId> {
        discover_legacy_task_link_streams(state.as_ref())
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let state = state.as_ref();
        let (from_blocks, _from_blocked_by) = state
            .source_links
            .as_ref()
            .ok_or("tiber_link_source_missing")?;
        let (_to_blocks, to_blocked_by) = state
            .target_links
            .as_ref()
            .ok_or("tiber_link_target_missing")?;
        let next_from_blocks = from_blocks
            .iter()
            .filter(|entry| *entry != &self.intent.to_stem)
            .cloned()
            .collect::<Vec<_>>();
        let next_to_blocked_by = to_blocked_by
            .iter()
            .filter(|entry| *entry != &self.intent.from_stem)
            .cloned()
            .collect::<Vec<_>>();
        if &next_from_blocks == from_blocks && &next_to_blocked_by == to_blocked_by {
            return Ok(ModeledEvents::none("dependency does not exist"));
        }
        if self.intent.from_stem == self.intent.to_stem {
            return Ok(ModeledEvents::one(
                TiberEvent::model_variant_tasklinkschanged(UnlinkTasksToSourceFact::apply((
                    self, self, state, state, state, state,
                ))),
            ));
        }
        let mut facts = ModeledEvents::none("unlink facts initialized");
        if &next_from_blocks != from_blocks {
            facts.push(TiberEvent::model_variant_tasklinkschanged(
                UnlinkTasksToSourceFact::apply((self, self, state, state, state, state)),
            ));
        }
        if &next_to_blocked_by != to_blocked_by {
            facts.push(TiberEvent::model_variant_tasklinkschanged(
                UnlinkTasksToTargetFact::apply((self, self, state, state, state, state)),
            ));
        }
        Ok(facts)
    }
}

fn execute_link_blocks(root: &Path, from_ref: &str, to_ref: &str) -> Result<(), Error> {
    let repository = GitRepository::at(root);
    let _lock = repository.acquire_lock()?;
    execute_initialize_tiber_repository(root)?;
    let projection = load_tiber_projection(root)?;
    let intent = TaskLinkIntent {
        from_stem: resolve_task_stem_from_projection(&projection, from_ref)?,
        to_stem: resolve_task_stem_from_projection(&projection, to_ref)?,
    };
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    let request = LinkTasksRequest::model_builder()
        .board(TiberBoardStream(stream_id(BOARD_STREAM)?))
        .intent(intent)
        .build();
    let command = LinkTasks::model_builder()
        .board(LinkTasksRequestToBoard::apply(request.as_ref()))
        .intent(LinkTasksRequestToIntent::apply(request.as_ref()))
        .build();
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

fn execute_unlink_blocks(root: &Path, from_ref: &str, to_ref: &str) -> Result<(), Error> {
    let repository = GitRepository::at(root);
    let _lock = repository.acquire_lock()?;
    execute_initialize_tiber_repository(root)?;
    let projection = load_tiber_projection(root)?;
    let intent = TaskLinkIntent {
        from_stem: resolve_task_stem_from_projection(&projection, from_ref)?,
        to_stem: resolve_task_stem_from_projection(&projection, to_ref)?,
    };
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    let request = UnlinkTasksRequest::model_builder()
        .board(TiberBoardStream(stream_id(BOARD_STREAM)?))
        .intent(intent)
        .build();
    let command = UnlinkTasks::model_builder()
        .board(UnlinkTasksRequestToBoard::apply(request.as_ref()))
        .intent(UnlinkTasksRequestToIntent::apply(request.as_ref()))
        .build();
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

#[derive(Clone)]
struct AddSubtaskIntent {
    stem: String,
    title: String,
    after: Vec<String>,
}

#[derive(ModelInput)]
struct AddSubtaskRequest {
    #[model(origin)]
    board: TiberBoardStream,
    #[model(origin)]
    intent: AddSubtaskIntent,
}

#[derive(ModelCommand)]
struct AddSubtask {
    #[stream]
    board: TiberBoardStream,
    intent: AddSubtaskIntent,
}

mapping! { AddSubtaskRequestToBoard:
AddSubtaskRequest.board => AddSubtask.board using clone; }
mapping! { AddSubtaskRequestToIntent:
AddSubtaskRequest.intent => AddSubtask.intent using clone; }

/// Subtask decisions fold only the addressed task's list. Board membership is
/// retained solely to discover a legacy per-task stream when necessary.
#[derive(ModelState)]
struct AddSubtaskState {
    #[model(default)]
    board_order: Vec<String>,
    #[model(default)]
    board_task_stems: BTreeSet<String>,
    #[model(default)]
    target_subtasks: Option<Vec<Subtask>>,
}

fn added_subtask_fact(
    board: &TiberBoardStream,
    intent: &AddSubtaskIntent,
    board_order: &[String],
    board_task_stems: &BTreeSet<String>,
    target_subtasks: &Option<Vec<Subtask>>,
) -> TaskSubtaskAddedEvent {
    debug_assert!(board_order.contains(&intent.stem) || target_subtasks.is_some());
    debug_assert!(board_task_stems.contains(&intent.stem) || target_subtasks.is_some());
    let items = target_subtasks
        .as_ref()
        .expect("validated subtask target must exist");
    let next_id = items
        .iter()
        .filter_map(|item| item.id.strip_prefix('s')?.parse::<usize>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    TaskSubtaskAddedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        stem: intent.stem.clone(),
        subtask: Subtask {
            id: format!("s{next_id}"),
            checked: false,
            title: intent.title.clone(),
            after: intent.after.clone(),
        },
    }
}

mapping! { AddSubtaskToFact:
    (AddSubtask.board, AddSubtask.intent, AddSubtaskState.board_order, AddSubtaskState.board_task_stems, AddSubtaskState.target_subtasks) => TiberEvent.TaskSubtaskAdded
    using added_subtask_fact;
}

fn evolve_subtask_state(
    mut state: AddSubtaskState,
    target_stem: &str,
    event: &TiberEvent,
) -> AddSubtaskState {
    match event {
        TiberEvent::TaskCreated(TaskCreatedEvent { stream_id, task }) => {
            if stream_id.as_ref() == BOARD_STREAM {
                state.board_task_stems.insert(task.stem.clone());
            }
            if task.stem == target_stem {
                state.target_subtasks = Some(task.subtasks.clone());
            }
        }
        TiberEvent::TaskSubtaskAdded(TaskSubtaskAddedEvent { stem, subtask, .. }) => {
            if stem == target_stem {
                state
                    .target_subtasks
                    .get_or_insert_with(Vec::new)
                    .push(subtask.clone());
            }
        }
        TiberEvent::TaskSubtaskChecked(TaskSubtaskCheckedEvent {
            stem,
            subtask_id,
            checked,
            ..
        }) => {
            if stem == target_stem {
                if let Some(item) = state
                    .target_subtasks
                    .get_or_insert_with(Vec::new)
                    .iter_mut()
                    .find(|item| item.id == *subtask_id)
                {
                    item.checked = *checked;
                }
            }
        }
        TiberEvent::LegacyTaskRemoved(TaskStemEvent { stem, .. }) => {
            if stem == target_stem {
                state.target_subtasks = None;
            }
            state.board_task_stems.remove(stem);
        }
        TiberEvent::TaskPriorityChanged(TaskOrderEvent { order, .. })
        | TiberEvent::BoardReordered(TaskOrderEvent { order, .. }) => {
            state.board_order.clone_from(order);
        }
        _ => {}
    }
    state
}

impl ModelCommandLogic for AddSubtask {
    type Event = TiberEvent;
    type State = AddSubtaskState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        Modeled::from_built(evolve_subtask_state(
            state.into_inner(),
            &self.intent.stem,
            event,
        ))
    }

    fn discover_related_streams(&self, state: &Modeled<Self::State>) -> Vec<StreamId> {
        state
            .as_ref()
            .board_order
            .iter()
            .filter(|stem| !state.as_ref().board_task_stems.contains(*stem))
            .filter_map(|stem| stream_id(format!("tiber:task:{stem}")).ok())
            .collect()
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        if state.as_ref().target_subtasks.is_none() {
            return Err("tiber_subtask_task_missing".into());
        }
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_tasksubtaskadded(AddSubtaskToFact::apply((
                self,
                self,
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
            ))),
        ))
    }
}

fn execute_add_subtask(
    root: &Path,
    task_ref: &str,
    title: &str,
    after_refs: &[String],
) -> Result<(), Error> {
    let title = title.trim();
    if title.is_empty() {
        return Err(Error::Parse("subtask_title_empty=true".into()));
    }
    if title.chars().any(char::is_control) {
        return Err(Error::Parse("subtask_title_invalid=true".into()));
    }
    let after = after_refs
        .iter()
        .map(|value| parse_subtask_ref(value))
        .collect::<Result<Vec<_>, Error>>()?;
    let repository = GitRepository::at(root);
    let _lock = repository.acquire_lock()?;
    execute_initialize_tiber_repository(root)?;
    let projection = load_tiber_projection(root)?;
    let request = AddSubtaskRequest::model_builder()
        .board(TiberBoardStream(stream_id(BOARD_STREAM)?))
        .intent(AddSubtaskIntent {
            stem: resolve_task_stem_from_projection(&projection, task_ref)?,
            title: title.to_string(),
            after,
        })
        .build();
    let command = AddSubtask::model_builder()
        .board(AddSubtaskRequestToBoard::apply(request.as_ref()))
        .intent(AddSubtaskRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

#[derive(Clone)]
struct SetSubtaskCheckedIntent {
    stem: String,
    subtask_id: String,
    checked: bool,
}

#[derive(ModelInput)]
struct SetSubtaskCheckedRequest {
    #[model(origin)]
    board: TiberBoardStream,
    #[model(origin)]
    intent: SetSubtaskCheckedIntent,
}

#[derive(ModelCommand)]
struct SetSubtaskChecked {
    #[stream]
    board: TiberBoardStream,
    intent: SetSubtaskCheckedIntent,
}

mapping! { SetSubtaskCheckedRequestToBoard:
SetSubtaskCheckedRequest.board => SetSubtaskChecked.board using clone; }
mapping! { SetSubtaskCheckedRequestToIntent:
SetSubtaskCheckedRequest.intent => SetSubtaskChecked.intent using clone; }

fn checked_subtask_fact(
    board: &TiberBoardStream,
    intent: &SetSubtaskCheckedIntent,
) -> TaskSubtaskCheckedEvent {
    TaskSubtaskCheckedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        stem: intent.stem.clone(),
        subtask_id: intent.subtask_id.clone(),
        checked: intent.checked,
    }
}

mapping! { SetSubtaskCheckedToFact:
    (SetSubtaskChecked.board, SetSubtaskChecked.intent) => TiberEvent.TaskSubtaskChecked
    using checked_subtask_fact;
}

impl ModelCommandLogic for SetSubtaskChecked {
    type Event = TiberEvent;
    type State = AddSubtaskState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        Modeled::from_built(evolve_subtask_state(
            state.into_inner(),
            &self.intent.stem,
            event,
        ))
    }

    fn discover_related_streams(&self, state: &Modeled<Self::State>) -> Vec<StreamId> {
        state
            .as_ref()
            .board_order
            .iter()
            .filter(|stem| !state.as_ref().board_task_stems.contains(*stem))
            .filter_map(|stem| stream_id(format!("tiber:task:{stem}")).ok())
            .collect()
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let item = state
            .as_ref()
            .target_subtasks
            .as_ref()
            .and_then(|items| items.iter().find(|item| item.id == self.intent.subtask_id))
            .ok_or("tiber_subtask_missing")?;
        if item.checked == self.intent.checked {
            return Ok(ModeledEvents::none(
                "subtask already has requested checked state",
            ));
        }
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_tasksubtaskchecked(SetSubtaskCheckedToFact::apply((
                self, self,
            ))),
        ))
    }
}

fn execute_set_subtask_checked(
    root: &Path,
    task_ref: &str,
    index: &str,
    checked: bool,
) -> Result<(), Error> {
    let repository = GitRepository::at(root);
    let _lock = repository.acquire_lock()?;
    execute_initialize_tiber_repository(root)?;
    let projection = load_tiber_projection(root)?;
    let request = SetSubtaskCheckedRequest::model_builder()
        .board(TiberBoardStream(stream_id(BOARD_STREAM)?))
        .intent(SetSubtaskCheckedIntent {
            stem: resolve_task_stem_from_projection(&projection, task_ref)?,
            subtask_id: parse_subtask_ref(index)?,
            checked,
        })
        .build();
    let command = SetSubtaskChecked::model_builder()
        .board(SetSubtaskCheckedRequestToBoard::apply(request.as_ref()))
        .intent(SetSubtaskCheckedRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

#[derive(Clone)]
struct UpdateTaskIntent {
    stem: String,
    title: Option<String>,
    tags: Option<Vec<String>>,
    summary: Option<String>,
    context: Option<String>,
    pr_mr_url: Option<Option<String>>,
    pr_mr_status: Option<Option<String>>,
}

#[derive(Clone)]
struct TaskUpdateFacts {
    title: String,
    tags: Vec<String>,
    summary: String,
    context: String,
    pr_mr_url: Option<String>,
    pr_mr_status: Option<String>,
}

#[derive(ModelInput)]
struct UpdateTaskRequest {
    #[model(origin)]
    board: TiberBoardStream,
    #[model(origin)]
    intent: UpdateTaskIntent,
}

#[derive(ModelCommand)]
struct UpdateTask {
    #[stream]
    board: TiberBoardStream,
    intent: UpdateTaskIntent,
}

mapping! { UpdateTaskRequestToBoard:
UpdateTaskRequest.board => UpdateTask.board using clone; }
mapping! { UpdateTaskRequestToIntent:
UpdateTaskRequest.intent => UpdateTask.intent using clone; }

/// Updating task metadata requires only the addressed task's mutable facts.
/// Board membership is retained solely for legacy stream discovery.
#[derive(ModelState)]
struct UpdateTaskState {
    #[model(default)]
    board_order: Vec<String>,
    #[model(default)]
    board_task_stems: BTreeSet<String>,
    #[model(default)]
    target_task: Option<TaskUpdateFacts>,
}

fn updated_task_details_fact(
    board: &TiberBoardStream,
    intent: &UpdateTaskIntent,
    board_order: &[String],
    board_task_stems: &BTreeSet<String>,
    target_task: &Option<TaskUpdateFacts>,
) -> TaskDetailsUpdatedEvent {
    debug_assert!(board_order.contains(&intent.stem) || target_task.is_some());
    debug_assert!(board_task_stems.contains(&intent.stem) || target_task.is_some());
    let current = target_task
        .as_ref()
        .expect("validated update target must exist");
    TaskDetailsUpdatedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        stem: intent.stem.clone(),
        title: intent
            .title
            .clone()
            .unwrap_or_else(|| current.title.clone()),
        tags: intent.tags.clone().unwrap_or_else(|| current.tags.clone()),
        summary: intent
            .summary
            .clone()
            .unwrap_or_else(|| current.summary.clone()),
        context: intent
            .context
            .clone()
            .unwrap_or_else(|| current.context.clone()),
    }
}

fn updated_task_pull_request_fact(
    board: &TiberBoardStream,
    intent: &UpdateTaskIntent,
    board_order: &[String],
    board_task_stems: &BTreeSet<String>,
    target_task: &Option<TaskUpdateFacts>,
) -> TaskPullRequestChangedEvent {
    debug_assert!(board_order.contains(&intent.stem) || target_task.is_some());
    debug_assert!(board_task_stems.contains(&intent.stem) || target_task.is_some());
    let current = target_task
        .as_ref()
        .expect("validated update target must exist");
    TaskPullRequestChangedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        stem: intent.stem.clone(),
        url: intent
            .pr_mr_url
            .clone()
            .unwrap_or_else(|| current.pr_mr_url.clone()),
        status: intent
            .pr_mr_status
            .clone()
            .unwrap_or_else(|| current.pr_mr_status.clone()),
    }
}

mapping! { UpdateTaskToDetailsFact:
    (UpdateTask.board, UpdateTask.intent, UpdateTaskState.board_order, UpdateTaskState.board_task_stems, UpdateTaskState.target_task) => TiberEvent.TaskDetailsUpdated
    using updated_task_details_fact;
}
mapping! { UpdateTaskToPullRequestFact:
    (UpdateTask.board, UpdateTask.intent, UpdateTaskState.board_order, UpdateTaskState.board_task_stems, UpdateTaskState.target_task) => TiberEvent.TaskPullRequestChanged
    using updated_task_pull_request_fact;
}

impl ModelCommandLogic for UpdateTask {
    type Event = TiberEvent;
    type State = UpdateTaskState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        match event {
            TiberEvent::TaskCreated(TaskCreatedEvent { stream_id, task }) => {
                if stream_id.as_ref() == BOARD_STREAM {
                    state.board_task_stems.insert(task.stem.clone());
                }
                if task.stem == self.intent.stem {
                    state.target_task = Some(TaskUpdateFacts {
                        title: task.title.clone(),
                        tags: task.tags.clone(),
                        summary: task.summary.clone(),
                        context: task.context.clone(),
                        pr_mr_url: task.pr_mr_url.clone(),
                        pr_mr_status: task.pr_mr_status.clone(),
                    });
                }
            }
            TiberEvent::TaskDetailsUpdated(TaskDetailsUpdatedEvent {
                stem,
                title,
                tags,
                summary,
                context,
                ..
            }) => {
                if stem == &self.intent.stem {
                    if let Some(task) = state.target_task.as_mut() {
                        task.title.clone_from(title);
                        task.tags.clone_from(tags);
                        task.summary.clone_from(summary);
                        task.context.clone_from(context);
                    }
                }
            }
            TiberEvent::TaskPullRequestChanged(TaskPullRequestChangedEvent {
                stem,
                url,
                status,
                ..
            }) => {
                if stem == &self.intent.stem {
                    if let Some(task) = state.target_task.as_mut() {
                        task.pr_mr_url.clone_from(url);
                        task.pr_mr_status.clone_from(status);
                    }
                }
            }
            TiberEvent::LegacyTaskRemoved(TaskStemEvent { stem, .. }) => {
                if stem == &self.intent.stem {
                    state.target_task = None;
                }
                state.board_task_stems.remove(stem);
            }
            TiberEvent::TaskPriorityChanged(TaskOrderEvent { order, .. })
            | TiberEvent::BoardReordered(TaskOrderEvent { order, .. }) => {
                state.board_order.clone_from(order);
            }
            _ => {}
        }
        Modeled::from_built(state)
    }

    fn discover_related_streams(&self, state: &Modeled<Self::State>) -> Vec<StreamId> {
        state
            .as_ref()
            .board_order
            .iter()
            .filter(|stem| !state.as_ref().board_task_stems.contains(*stem))
            .filter_map(|stem| stream_id(format!("tiber:task:{stem}")).ok())
            .collect()
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let current = state
            .as_ref()
            .target_task
            .as_ref()
            .ok_or("tiber_update_task_missing")?;
        let next_title = self.intent.title.as_ref().unwrap_or(&current.title);
        let next_tags = self.intent.tags.as_ref().unwrap_or(&current.tags);
        let next_summary = self.intent.summary.as_ref().unwrap_or(&current.summary);
        let next_context = self.intent.context.as_ref().unwrap_or(&current.context);
        let next_url = self.intent.pr_mr_url.as_ref().unwrap_or(&current.pr_mr_url);
        let next_status = self
            .intent
            .pr_mr_status
            .as_ref()
            .unwrap_or(&current.pr_mr_status);
        let details_changed = next_title != &current.title
            || next_tags != &current.tags
            || next_summary != &current.summary
            || next_context != &current.context;
        let pr_changed = next_url != &current.pr_mr_url || next_status != &current.pr_mr_status;
        if !details_changed && !pr_changed {
            return Ok(ModeledEvents::none(
                "task metadata already has requested values",
            ));
        }
        let mut facts = ModeledEvents::none("task update facts initialized");
        if details_changed {
            facts.push(TiberEvent::model_variant_taskdetailsupdated(
                UpdateTaskToDetailsFact::apply((
                    self,
                    self,
                    state.as_ref(),
                    state.as_ref(),
                    state.as_ref(),
                )),
            ));
        }
        if pr_changed {
            facts.push(TiberEvent::model_variant_taskpullrequestchanged(
                UpdateTaskToPullRequestFact::apply((
                    self,
                    self,
                    state.as_ref(),
                    state.as_ref(),
                    state.as_ref(),
                )),
            ));
        }
        Ok(facts)
    }
}

fn execute_update_task(root: &Path, task_ref: &str, update: TaskUpdate<'_>) -> Result<(), Error> {
    let title = update.title.map(TaskTitle::parse).transpose()?;
    let summary = update.summary.map(parse_task_section_body).transpose()?;
    let context = update.context.map(parse_task_section_body).transpose()?;
    let repository = GitRepository::at(root);
    let _lock = repository.acquire_lock()?;
    execute_initialize_tiber_repository(root)?;
    let projection = load_tiber_projection(root)?;
    let request = UpdateTaskRequest::model_builder()
        .board(TiberBoardStream(stream_id(BOARD_STREAM)?))
        .intent(UpdateTaskIntent {
            stem: resolve_task_stem_from_projection(&projection, task_ref)?,
            title: title.map(|value| value.as_str().to_string()),
            tags: update.tags,
            summary,
            context,
            pr_mr_url: update.pr_mr_url.map(nonempty_option),
            pr_mr_status: update.pr_mr_status.map(nonempty_option),
        })
        .build();
    let command = UpdateTask::model_builder()
        .board(UpdateTaskRequestToBoard::apply(request.as_ref()))
        .intent(UpdateTaskRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

#[derive(Clone)]
struct AddAcceptanceIntent {
    stem: String,
    text: String,
}

#[derive(ModelInput)]
struct AddAcceptanceRequest {
    #[model(origin)]
    board: TiberBoardStream,
    #[model(origin)]
    intent: AddAcceptanceIntent,
}

#[derive(ModelCommand)]
struct AddAcceptance {
    #[stream]
    board: TiberBoardStream,
    intent: AddAcceptanceIntent,
}

mapping! { AddAcceptanceRequestToBoard:
AddAcceptanceRequest.board => AddAcceptance.board using clone; }
mapping! { AddAcceptanceRequestToIntent:
AddAcceptanceRequest.intent => AddAcceptance.intent using clone; }

/// Acceptance decisions fold only the addressed task's checklist. Board
/// membership is retained solely for legacy stream discovery.
#[derive(ModelState)]
struct AddAcceptanceState {
    #[model(default)]
    board_order: Vec<String>,
    #[model(default)]
    board_task_stems: BTreeSet<String>,
    #[model(default)]
    target_acceptance: Option<Vec<ChecklistItem>>,
}

fn added_acceptance_fact(
    board: &TiberBoardStream,
    intent: &AddAcceptanceIntent,
    board_order: &[String],
    board_task_stems: &BTreeSet<String>,
    target_acceptance: &Option<Vec<ChecklistItem>>,
) -> TaskAcceptanceAddedEvent {
    debug_assert!(board_order.contains(&intent.stem) || target_acceptance.is_some());
    debug_assert!(board_task_stems.contains(&intent.stem) || target_acceptance.is_some());
    debug_assert!(target_acceptance.is_some());
    TaskAcceptanceAddedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        stem: intent.stem.clone(),
        item: ChecklistItem {
            checked: false,
            text: intent.text.clone(),
        },
    }
}

mapping! { AddAcceptanceToFact:
    (AddAcceptance.board, AddAcceptance.intent, AddAcceptanceState.board_order, AddAcceptanceState.board_task_stems, AddAcceptanceState.target_acceptance) => TiberEvent.TaskAcceptanceAdded
    using added_acceptance_fact;
}

impl ModelCommandLogic for AddAcceptance {
    type Event = TiberEvent;
    type State = AddAcceptanceState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        Modeled::from_built(evolve_acceptance_state(
            state.into_inner(),
            &self.intent.stem,
            event,
        ))
    }

    fn discover_related_streams(&self, state: &Modeled<Self::State>) -> Vec<StreamId> {
        acceptance_related_streams(state.as_ref())
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        if state.as_ref().target_acceptance.is_none() {
            return Err("tiber_acceptance_task_missing".into());
        }
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_taskacceptanceadded(AddAcceptanceToFact::apply((
                self,
                self,
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
            ))),
        ))
    }
}

fn evolve_acceptance_state(
    mut state: AddAcceptanceState,
    target_stem: &str,
    event: &TiberEvent,
) -> AddAcceptanceState {
    match event {
        TiberEvent::TaskCreated(TaskCreatedEvent { stream_id, task }) => {
            if stream_id.as_ref() == BOARD_STREAM {
                state.board_task_stems.insert(task.stem.clone());
            }
            if task.stem == target_stem {
                state.target_acceptance = Some(task.acceptance.clone());
            }
        }
        TiberEvent::TaskAcceptanceAdded(TaskAcceptanceAddedEvent { stem, item, .. }) => {
            if stem == target_stem {
                state
                    .target_acceptance
                    .get_or_insert_with(Vec::new)
                    .push(item.clone());
            }
        }
        TiberEvent::TaskAcceptanceChecked(TaskAcceptanceCheckedEvent {
            stem,
            index,
            checked,
            ..
        }) => {
            if stem == target_stem {
                if let Some(item) = state
                    .target_acceptance
                    .get_or_insert_with(Vec::new)
                    .get_mut(*index)
                {
                    item.checked = *checked;
                }
            }
        }
        TiberEvent::TaskAcceptanceRemoved(TaskAcceptanceRemovedEvent { stem, index, .. }) => {
            if stem == target_stem {
                let items = state.target_acceptance.get_or_insert_with(Vec::new);
                if *index < items.len() {
                    items.remove(*index);
                }
            }
        }
        TiberEvent::LegacyTaskRemoved(TaskStemEvent { stem, .. }) => {
            if stem == target_stem {
                state.target_acceptance = None;
            }
            state.board_task_stems.remove(stem);
        }
        TiberEvent::TaskPriorityChanged(TaskOrderEvent { order, .. })
        | TiberEvent::BoardReordered(TaskOrderEvent { order, .. }) => {
            state.board_order.clone_from(order);
        }
        _ => {}
    }
    state
}

fn acceptance_related_streams(state: &AddAcceptanceState) -> Vec<StreamId> {
    state
        .board_order
        .iter()
        .filter(|stem| !state.board_task_stems.contains(*stem))
        .filter_map(|stem| stream_id(format!("tiber:task:{stem}")).ok())
        .collect()
}

fn execute_add_acceptance(root: &Path, task_ref: &str, criterion: &str) -> Result<(), Error> {
    let criterion = parse_nonempty_text(criterion, "acceptance")?;
    let repository = GitRepository::at(root);
    let _lock = repository.acquire_lock()?;
    execute_initialize_tiber_repository(root)?;
    let projection = load_tiber_projection(root)?;
    let request = AddAcceptanceRequest::model_builder()
        .board(TiberBoardStream(stream_id(BOARD_STREAM)?))
        .intent(AddAcceptanceIntent {
            stem: resolve_task_stem_from_projection(&projection, task_ref)?,
            text: criterion.to_string(),
        })
        .build();
    let command = AddAcceptance::model_builder()
        .board(AddAcceptanceRequestToBoard::apply(request.as_ref()))
        .intent(AddAcceptanceRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

#[derive(Clone)]
struct SetAcceptanceCheckedIntent {
    stem: String,
    index: usize,
    checked: bool,
}

#[derive(ModelInput)]
struct SetAcceptanceCheckedRequest {
    #[model(origin)]
    board: TiberBoardStream,
    #[model(origin)]
    intent: SetAcceptanceCheckedIntent,
}

#[derive(ModelCommand)]
struct SetAcceptanceChecked {
    #[stream]
    board: TiberBoardStream,
    intent: SetAcceptanceCheckedIntent,
}

mapping! { SetAcceptanceCheckedRequestToBoard:
SetAcceptanceCheckedRequest.board => SetAcceptanceChecked.board using clone; }
mapping! { SetAcceptanceCheckedRequestToIntent:
SetAcceptanceCheckedRequest.intent => SetAcceptanceChecked.intent using clone; }

fn checked_acceptance_fact(
    board: &TiberBoardStream,
    intent: &SetAcceptanceCheckedIntent,
) -> TaskAcceptanceCheckedEvent {
    TaskAcceptanceCheckedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        stem: intent.stem.clone(),
        index: intent.index,
        checked: intent.checked,
    }
}

mapping! { SetAcceptanceCheckedToFact:
    (SetAcceptanceChecked.board, SetAcceptanceChecked.intent) => TiberEvent.TaskAcceptanceChecked
    using checked_acceptance_fact;
}

impl ModelCommandLogic for SetAcceptanceChecked {
    type Event = TiberEvent;
    type State = AddAcceptanceState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        Modeled::from_built(evolve_acceptance_state(
            state.into_inner(),
            &self.intent.stem,
            event,
        ))
    }

    fn discover_related_streams(&self, state: &Modeled<Self::State>) -> Vec<StreamId> {
        acceptance_related_streams(state.as_ref())
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let item = state
            .as_ref()
            .target_acceptance
            .as_ref()
            .and_then(|items| items.get(self.intent.index))
            .ok_or("tiber_acceptance_missing")?;
        if item.checked == self.intent.checked {
            return Ok(ModeledEvents::none(
                "acceptance already has requested checked state",
            ));
        }
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_taskacceptancechecked(SetAcceptanceCheckedToFact::apply((
                self, self,
            ))),
        ))
    }
}

fn execute_set_acceptance_checked(
    root: &Path,
    task_ref: &str,
    index: &str,
    checked: bool,
) -> Result<(), Error> {
    let index = parse_one_based_usize(index, "acceptance")? - 1;
    let repository = GitRepository::at(root);
    let _lock = repository.acquire_lock()?;
    execute_initialize_tiber_repository(root)?;
    let projection = load_tiber_projection(root)?;
    let request = SetAcceptanceCheckedRequest::model_builder()
        .board(TiberBoardStream(stream_id(BOARD_STREAM)?))
        .intent(SetAcceptanceCheckedIntent {
            stem: resolve_task_stem_from_projection(&projection, task_ref)?,
            index,
            checked,
        })
        .build();
    let command = SetAcceptanceChecked::model_builder()
        .board(SetAcceptanceCheckedRequestToBoard::apply(request.as_ref()))
        .intent(SetAcceptanceCheckedRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

#[derive(Clone)]
struct RemoveAcceptanceIntent {
    stem: String,
    index: usize,
}

#[derive(ModelInput)]
struct RemoveAcceptanceRequest {
    #[model(origin)]
    board: TiberBoardStream,
    #[model(origin)]
    intent: RemoveAcceptanceIntent,
}

#[derive(ModelCommand)]
struct RemoveAcceptance {
    #[stream]
    board: TiberBoardStream,
    intent: RemoveAcceptanceIntent,
}

mapping! { RemoveAcceptanceRequestToBoard:
RemoveAcceptanceRequest.board => RemoveAcceptance.board using clone; }
mapping! { RemoveAcceptanceRequestToIntent:
RemoveAcceptanceRequest.intent => RemoveAcceptance.intent using clone; }

fn removed_acceptance_fact(
    board: &TiberBoardStream,
    intent: &RemoveAcceptanceIntent,
) -> TaskAcceptanceRemovedEvent {
    TaskAcceptanceRemovedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        stem: intent.stem.clone(),
        index: intent.index,
    }
}

mapping! { RemoveAcceptanceToFact:
    (RemoveAcceptance.board, RemoveAcceptance.intent) => TiberEvent.TaskAcceptanceRemoved
    using removed_acceptance_fact;
}

impl ModelCommandLogic for RemoveAcceptance {
    type Event = TiberEvent;
    type State = AddAcceptanceState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        Modeled::from_built(evolve_acceptance_state(
            state.into_inner(),
            &self.intent.stem,
            event,
        ))
    }

    fn discover_related_streams(&self, state: &Modeled<Self::State>) -> Vec<StreamId> {
        acceptance_related_streams(state.as_ref())
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        if state
            .as_ref()
            .target_acceptance
            .as_ref()
            .and_then(|items| items.get(self.intent.index))
            .is_none()
        {
            return Err("tiber_acceptance_missing".into());
        }
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_taskacceptanceremoved(RemoveAcceptanceToFact::apply((
                self, self,
            ))),
        ))
    }
}

fn execute_remove_acceptance(root: &Path, task_ref: &str, index: &str) -> Result<(), Error> {
    let index = parse_one_based_usize(index, "acceptance")? - 1;
    let repository = GitRepository::at(root);
    let _lock = repository.acquire_lock()?;
    execute_initialize_tiber_repository(root)?;
    let projection = load_tiber_projection(root)?;
    let request = RemoveAcceptanceRequest::model_builder()
        .board(TiberBoardStream(stream_id(BOARD_STREAM)?))
        .intent(RemoveAcceptanceIntent {
            stem: resolve_task_stem_from_projection(&projection, task_ref)?,
            index,
        })
        .build();
    let command = RemoveAcceptance::model_builder()
        .board(RemoveAcceptanceRequestToBoard::apply(request.as_ref()))
        .intent(RemoveAcceptanceRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

#[derive(Clone)]
struct AddTaskNoteIntent {
    stem: String,
    note: Note,
}

#[derive(ModelInput)]
struct AddTaskNoteRequest {
    #[model(origin)]
    board: TiberBoardStream,
    #[model(origin)]
    intent: AddTaskNoteIntent,
}

#[derive(ModelCommand)]
struct AddTaskNote {
    #[stream]
    board: TiberBoardStream,
    intent: AddTaskNoteIntent,
}

mapping! { AddTaskNoteRequestToBoard:
AddTaskNoteRequest.board => AddTaskNote.board using clone; }
mapping! { AddTaskNoteRequestToIntent:
AddTaskNoteRequest.intent => AddTaskNote.intent using clone; }

#[derive(ModelState)]
struct AddTaskNoteState {
    #[model(default)]
    board_order: Vec<String>,
    #[model(default)]
    board_task_stems: BTreeSet<String>,
    #[model(default)]
    task_stems: BTreeSet<String>,
}

fn added_task_note_fact(
    board: &TiberBoardStream,
    intent: &AddTaskNoteIntent,
    board_order: &[String],
    board_task_stems: &BTreeSet<String>,
    task_stems: &BTreeSet<String>,
) -> TaskNoteAddedEvent {
    debug_assert!(board_order.contains(&intent.stem) || task_stems.contains(&intent.stem));
    debug_assert!(board_task_stems.contains(&intent.stem) || task_stems.contains(&intent.stem));
    debug_assert!(task_stems.contains(&intent.stem));
    TaskNoteAddedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        stem: intent.stem.clone(),
        note: intent.note.clone(),
    }
}

mapping! { AddTaskNoteToFact:
    (AddTaskNote.board, AddTaskNote.intent, AddTaskNoteState.board_order, AddTaskNoteState.board_task_stems, AddTaskNoteState.task_stems) => TiberEvent.TaskNoteAdded
    using added_task_note_fact;
}

impl ModelCommandLogic for AddTaskNote {
    type Event = TiberEvent;
    type State = AddTaskNoteState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        match event {
            TiberEvent::TaskCreated(TaskCreatedEvent { stream_id, task }) => {
                state.task_stems.insert(task.stem.clone());
                if stream_id.as_ref() == BOARD_STREAM {
                    state.board_task_stems.insert(task.stem.clone());
                }
            }
            TiberEvent::LegacyTaskRemoved(TaskStemEvent { stem, .. }) => {
                state.task_stems.remove(stem);
                state.board_task_stems.remove(stem);
            }
            TiberEvent::TaskPriorityChanged(TaskOrderEvent { order, .. })
            | TiberEvent::BoardReordered(TaskOrderEvent { order, .. }) => {
                state.board_order.clone_from(order);
            }
            _ => {}
        }
        Modeled::from_built(state)
    }

    fn discover_related_streams(&self, state: &Modeled<Self::State>) -> Vec<StreamId> {
        state
            .as_ref()
            .board_order
            .iter()
            .filter(|stem| !state.as_ref().board_task_stems.contains(*stem))
            .filter_map(|stem| stream_id(format!("tiber:task:{stem}")).ok())
            .collect()
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        if !state.as_ref().task_stems.contains(&self.intent.stem) {
            return Err("tiber_note_task_missing".into());
        }
        Ok(ModeledEvents::one(TiberEvent::model_variant_tasknoteadded(
            AddTaskNoteToFact::apply((self, self, state.as_ref(), state.as_ref(), state.as_ref())),
        )))
    }
}

fn execute_add_task_note(root: &Path, task_ref: &str, note: &str) -> Result<(), Error> {
    let text = parse_nonempty_text(note, "note")?;
    let repository = GitRepository::at(root);
    let _lock = repository.acquire_lock()?;
    execute_initialize_tiber_repository(root)?;
    let projection = load_tiber_projection(root)?;
    let request = AddTaskNoteRequest::model_builder()
        .board(TiberBoardStream(stream_id(BOARD_STREAM)?))
        .intent(AddTaskNoteIntent {
            stem: resolve_task_stem_from_projection(&projection, task_ref)?,
            note: Note {
                date: current_date_string(),
                text: text.to_string(),
            },
        })
        .build();
    let command = AddTaskNote::model_builder()
        .board(AddTaskNoteRequestToBoard::apply(request.as_ref()))
        .intent(AddTaskNoteRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

#[derive(Clone)]
struct RecordFinalReviewIntent {
    stem: String,
    review: FinalReviewRecord,
}

#[derive(Clone, Copy)]
struct FinalReviewScopeLimits {
    git_output_bytes: usize,
    paths: usize,
    content_bytes: u64,
}

const FINAL_REVIEW_SCOPE_LIMITS: FinalReviewScopeLimits = FinalReviewScopeLimits {
    git_output_bytes: FINAL_REVIEW_GIT_OUTPUT_MAX_BYTES,
    paths: FINAL_REVIEW_SCOPE_MAX_PATHS,
    content_bytes: FINAL_REVIEW_SCOPE_MAX_CONTENT_BYTES,
};

fn bounded_final_review_git_output(
    root: &Path,
    args: &[&str],
    scope: &[String],
    scope_kind: &str,
    limits: FinalReviewScopeLimits,
) -> Result<Vec<u8>, Error> {
    bounded_final_review_git_output_with_index(root, args, scope, scope_kind, limits, None, None)
}

fn bounded_final_review_git_output_with_index(
    root: &Path,
    args: &[&str],
    scope: &[String],
    scope_kind: &str,
    limits: FinalReviewScopeLimits,
    index_file: Option<&Path>,
    attribute_source: Option<&str>,
) -> Result<Vec<u8>, Error> {
    let isolated_attributes = if attribute_source.is_some() {
        let git_dir = tempfile::Builder::new()
            .prefix("tiber-final-review-attributes-")
            .tempdir()
            .map_err(Error::Io)?;
        let init = Command::new("git")
            .args(["init", "--bare", "--quiet"])
            .arg(git_dir.path())
            .current_dir(root)
            .output()
            .map_err(Error::Io)?;
        if !init.status.success() {
            return Err(Error::Parse(
                "tiber.final_review_attribute_isolation_failed=true".into(),
            ));
        }
        let attributes_file = git_dir.path().join("empty-attributes");
        fs::write(&attributes_file, []).map_err(Error::Io)?;
        let object_directory = GitRepository::at(root).git_common_dir()?.join("objects");
        Some((git_dir, attributes_file, object_directory))
    } else {
        None
    };
    let mut command = Command::new("git");
    command.args(args).args(scope).current_dir(root);
    if let Some(index_file) = index_file {
        command.env("GIT_INDEX_FILE", index_file);
    }
    if let Some(attribute_source) = attribute_source {
        command.env("GIT_ATTR_SOURCE", attribute_source);
    }
    if let Some((git_dir, attributes_file, object_directory)) = isolated_attributes.as_ref() {
        command
            .env("GIT_DIR", git_dir.path())
            .env("GIT_WORK_TREE", root)
            .env("GIT_OBJECT_DIRECTORY", object_directory)
            .env("GIT_ATTR_NOSYSTEM", "1")
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "core.attributesFile")
            .env("GIT_CONFIG_VALUE_0", attributes_file);
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(Error::Io)?;
    let mut stdout = child
        .stdout
        .take()
        .expect("piped Git stdout should be available");
    let stderr = child
        .stderr
        .take()
        .expect("piped Git stderr should be available");
    let stderr_limit = limits.git_output_bytes;
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr
            .take(stderr_limit.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = stdout.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        if bytes.len().saturating_add(read) > limits.git_output_bytes {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stderr_reader.join();
            return Err(Error::Usage(format!(
                "tiber.final_review_{scope_kind}_scope_too_large condition=git_output_bytes max_bytes={} action=\"narrow the declared review scope and record fresh independent reviews\"",
                limits.git_output_bytes
            )));
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    let status = child.wait()?;
    let mut stderr = stderr_reader
        .join()
        .map_err(|_| Error::Parse("tiber.final_review_git_stderr_reader_failed=true".into()))??;
    stderr.truncate(limits.git_output_bytes);
    if !status.success() {
        return Err(Error::CommandFailed {
            program: "git".into(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            status: status.code().unwrap_or(1).to_string(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        });
    }
    Ok(bytes)
}

fn bounded_final_review_paths(
    bytes: &[u8],
    scope_kind: &str,
    limits: FinalReviewScopeLimits,
) -> Result<Vec<String>, Error> {
    let mut paths = bytes
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| String::from_utf8(path.to_vec()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| Error::Usage("tiber.final_review_scope_path_invalid_utf8=true".into()))?;
    if paths.len() > limits.paths {
        return Err(Error::Usage(format!(
            "tiber.final_review_{scope_kind}_scope_too_large condition=path_count paths={} max_paths={} action=\"narrow the declared review scope and record fresh independent reviews\"",
            paths.len(), limits.paths
        )));
    }
    paths.shrink_to_fit();
    Ok(paths)
}

fn final_review_gitlink_is_dirty(
    root: &Path,
    limits: FinalReviewScopeLimits,
) -> Result<Option<bool>, Error> {
    let mut command = Command::new("git");
    command
        .args(["status", "--porcelain=v1", "-z", "--untracked-files=all"])
        .current_dir(root);
    final_review_gitlink_is_dirty_from_command(&mut command, limits)
}

fn final_review_gitlink_is_dirty_from_command(
    command: &mut Command,
    limits: FinalReviewScopeLimits,
) -> Result<Option<bool>, Error> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(Error::Io)?;
    let mut stdout = child
        .stdout
        .take()
        .expect("piped Git stdout should be available");
    let stderr = child
        .stderr
        .take()
        .expect("piped Git stderr should be available");
    let stderr_limit = limits.git_output_bytes;
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr
            .take(stderr_limit.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let mut first_byte = [0_u8; 1];
    match stdout.read(&mut first_byte) {
        Ok(0) => {
            let status = child.wait()?;
            let _ = stderr_reader.join().map_err(|_| {
                Error::Parse("tiber.final_review_git_stderr_reader_failed=true".into())
            })??;
            Ok(status.success().then_some(false))
        }
        Ok(_) => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stderr_reader.join();
            Ok(Some(true))
        }
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stderr_reader.join();
            Err(error.into())
        }
    }
}

fn source_scope_fingerprint(
    root: &Path,
    scope: &[String],
    commit_range: Option<&str>,
    scope_kind: &str,
) -> Result<String, Error> {
    source_scope_fingerprint_with_limits(
        root,
        scope,
        commit_range,
        scope_kind,
        FINAL_REVIEW_SCOPE_LIMITS,
    )
}

fn source_scope_fingerprint_with_limits(
    root: &Path,
    scope: &[String],
    commit_range: Option<&str>,
    scope_kind: &str,
    limits: FinalReviewScopeLimits,
) -> Result<String, Error> {
    let snapshot = final_review_scope_snapshot(root, scope, commit_range, scope_kind, limits)?;
    let paths = snapshot.paths;
    let index = bounded_final_review_git_output(
        root,
        &["ls-files", "--stage", "-z", "--"],
        scope,
        scope_kind,
        limits,
    )?;
    let index_modes = index
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .filter_map(|record| {
            let separator = record.iter().position(|byte| *byte == b'\t')?;
            let mode = record[..separator].split(|byte| *byte == b' ').next()?;
            Some(
                String::from_utf8(record[separator + 1..].to_vec())
                    .map(|path| (path, mode.to_vec())),
            )
        })
        .collect::<Result<BTreeMap<_, _>, _>>()
        .map_err(|_| Error::Usage("tiber.final_review_scope_path_invalid_utf8=true".into()))?;
    let gitlinks = index_modes
        .iter()
        .filter_map(|(path, mode)| (mode == b"160000").then_some(path.as_str()))
        .collect::<BTreeSet<_>>();
    let mut hasher = Sha256::new();
    hasher.update(b"tiber-final-review-scope-v5\0");
    let mut content_bytes = 0_u64;
    for path in paths {
        let absolute = root.join(&path);
        hasher.update(path.as_bytes());
        hasher.update([0]);
        let metadata = match fs::symlink_metadata(&absolute) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if gitlinks.contains(path.as_str()) {
                    return Err(Error::Usage(format!(
                        "tiber.final_review_gitlink_unavailable path={path:?} action=\"initialize the scoped submodule checkout before recording or completing final review\""
                    )));
                }
                hasher.update(b"deleted\0deleted\0");
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() {
            hasher.update(b"120000\0symlink\0");
            hasher.update(fs::read_link(&absolute)?.as_os_str().as_encoded_bytes());
        } else if metadata.is_dir() && gitlinks.contains(path.as_str()) {
            let top_level = Command::new("git")
                .args(["rev-parse", "--show-toplevel"])
                .current_dir(&absolute)
                .output()
                .map_err(Error::Io)?;
            let nested_root = String::from_utf8_lossy(&top_level.stdout);
            let nested_root = nested_root.trim_end_matches(['\r', '\n']);
            let is_nested_checkout = top_level.status.success()
                && matches!(
                    (fs::canonicalize(nested_root), fs::canonicalize(&absolute)),
                    (Ok(discovered), Ok(expected)) if discovered == expected
                );
            if !is_nested_checkout {
                return Err(Error::Usage(format!(
                    "tiber.final_review_gitlink_unavailable path={path:?} action=\"initialize the scoped submodule checkout before recording or completing final review\""
                )));
            }
            let head = Command::new("git")
                .args(["rev-parse", "--verify", "HEAD"])
                .current_dir(&absolute)
                .output()
                .map_err(Error::Io)?;
            if !head.status.success() {
                return Err(Error::Usage(format!(
                    "tiber.final_review_gitlink_unavailable path={path:?} action=\"initialize the scoped submodule checkout before recording or completing final review\""
                )));
            }
            let Some(is_dirty) = final_review_gitlink_is_dirty(&absolute, limits)? else {
                return Err(Error::Usage(format!(
                    "tiber.final_review_gitlink_unavailable path={path:?} action=\"repair the scoped submodule checkout before recording or completing final review\""
                )));
            };
            if is_dirty {
                return Err(Error::Usage(format!(
                    "tiber.final_review_gitlink_dirty path={path:?} action=\"commit or discard nested submodule changes before recording or completing final review\""
                )));
            }
            hasher.update(b"160000\0gitlink\0");
            hasher.update(String::from_utf8_lossy(&head.stdout).trim().as_bytes());
        } else if metadata.is_dir() {
            hasher.update(b"040000\0directory\0");
        } else {
            #[cfg(unix)]
            let mode = if metadata.permissions().mode() & 0o111 == 0 {
                b"100644".as_slice()
            } else {
                b"100755".as_slice()
            };
            #[cfg(not(unix))]
            let mode = if index_modes.get(&path).is_some_and(|mode| mode == b"100755") {
                b"100755".as_slice()
            } else {
                b"100644".as_slice()
            };
            hasher.update(mode);
            hasher.update(b"\0blob\0");
            let mut file = fs::File::open(&absolute)?;
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let read = file.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                content_bytes = content_bytes.saturating_add(read as u64);
                if content_bytes > limits.content_bytes {
                    return Err(Error::Usage(format!(
                        "tiber.final_review_{scope_kind}_scope_too_large condition=content_bytes max_bytes={} action=\"narrow the declared review scope and record fresh independent reviews\"",
                        limits.content_bytes
                    )));
                }
                hasher.update(&buffer[..read]);
            }
        }
        hasher.update([0]);
    }
    Ok(format!("sha256:v5:{:x}", hasher.finalize()))
}

struct FinalReviewScopeSnapshot {
    paths: Vec<String>,
}

fn final_review_scope_snapshot(
    root: &Path,
    scope: &[String],
    commit_range: Option<&str>,
    scope_kind: &str,
    limits: FinalReviewScopeLimits,
) -> Result<FinalReviewScopeSnapshot, Error> {
    if scope.is_empty()
        || scope.iter().any(|pathspec| {
            pathspec.is_empty()
                || pathspec.starts_with('-')
                || pathspec.starts_with('/')
                || pathspec.contains('\0')
        })
    {
        return Err(Error::Usage(format!(
            "tiber.final_review_{scope_kind}_scope_invalid action=\"provide one or more nonempty repository-relative Git pathspecs\""
        )));
    }
    let output = bounded_final_review_git_output(
        root,
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
            "--",
        ],
        scope,
        scope_kind,
        limits,
    )?;
    let mut paths = bounded_final_review_paths(&output, scope_kind, limits)?;
    let staged = bounded_final_review_git_output(
        root,
        &["diff", "--cached", "--name-only", "-z", "--"],
        scope,
        scope_kind,
        limits,
    )?;
    let staged_paths = bounded_final_review_paths(&staged, scope_kind, limits)?
        .into_iter()
        .collect::<BTreeSet<_>>();
    paths.extend(staged_paths.iter().cloned());
    let unstaged = bounded_final_review_git_output(
        root,
        &["diff", "--name-only", "-z", "--"],
        scope,
        scope_kind,
        limits,
    )?;
    let unstaged_paths = bounded_final_review_paths(&unstaged, scope_kind, limits)?
        .into_iter()
        .collect::<BTreeSet<_>>();
    paths.extend(unstaged_paths.iter().cloned());
    if let Some(commit_range) = commit_range {
        let output = match bounded_final_review_git_output(
            root,
            &["diff", "--name-only", "-z", commit_range, "--"],
            scope,
            scope_kind,
            limits,
        ) {
            Ok(output) => output,
            Err(Error::CommandFailed { .. }) => {
                return Err(Error::Usage(
                    "tiber.final_review_commit_range_invalid action=\"provide a resolvable commit range\""
                        .into(),
                ));
            }
            Err(error) => return Err(error),
        };
        if output.is_empty() && canonical_commit_range(root, commit_range).is_err() {
            return Err(Error::Usage(
                "tiber.final_review_commit_range_invalid action=\"provide a resolvable commit range\""
                    .into(),
            ));
        }
        paths.extend(bounded_final_review_paths(&output, scope_kind, limits)?);
    }
    for pathspec in scope {
        if let Some(path) = pathspec.strip_prefix(":(literal)") {
            let path = Path::new(path);
            if path.as_os_str().is_empty()
                || path.is_absolute()
                || path.components().any(|component| {
                    matches!(
                        component,
                        std::path::Component::ParentDir
                            | std::path::Component::RootDir
                            | std::path::Component::Prefix(_)
                    )
                })
            {
                return Err(Error::Usage(format!(
                    "tiber.final_review_{scope_kind}_scope_invalid action=\"provide repository-contained literal paths\""
                )));
            }
            paths.push(path.to_string_lossy().into_owned());
        }
    }
    paths.sort();
    paths.dedup();
    if paths.len() > limits.paths {
        return Err(Error::Usage(format!(
            "tiber.final_review_{scope_kind}_scope_too_large condition=path_count paths={} max_paths={} action=\"narrow the declared review scope and record fresh independent reviews\"",
            paths.len(), limits.paths
        )));
    }
    if paths.is_empty() {
        return Err(Error::Usage(format!(
            "tiber.final_review_{scope_kind}_scope_empty action=\"correct the declared pathspecs so they select at least one tracked, untracked, or range-changed repository file\""
        )));
    }
    Ok(FinalReviewScopeSnapshot { paths })
}

fn canonical_commit_range(root: &Path, commit_range: &str) -> Result<String, Error> {
    let Some((start, end)) = commit_range.split_once("..") else {
        return Err(Error::Usage(
            "tiber.final_review_commit_range_invalid expected=<start>..<end>".into(),
        ));
    };
    if start.is_empty() || end.is_empty() || end.contains("..") {
        return Err(Error::Usage(
            "tiber.final_review_commit_range_invalid expected=<start>..<end>".into(),
        ));
    }
    let resolve = |revision: &str| {
        git_output(
            [
                "rev-parse",
                "--verify",
                "--end-of-options",
                &format!("{revision}^{{commit}}"),
            ],
            Some(root),
        )
        .map(|value| value.trim().to_string())
        .map_err(|_| {
            Error::Usage(format!(
                "tiber.final_review_commit_range_invalid revision={revision} action=\"provide a range whose endpoints resolve to commits\""
            ))
        })
    };
    Ok(format!("{}..{}", resolve(start)?, resolve(end)?))
}

fn canonical_commit_snapshot(root: &Path, revision: &str) -> Result<(String, String), Error> {
    let resolve = |suffix: &str| {
        git_output(
            [
                "rev-parse",
                "--verify",
                "--end-of-options",
                &format!("{revision}^{{{suffix}}}"),
            ],
            Some(root),
        )
        .map(|value| value.trim().to_string())
        .map_err(|_| {
            Error::Usage(format!(
                "tiber.final_review_completion_commit_invalid revision={revision:?} action=\"provide a revision that resolves to an immutable commit\""
            ))
        })
    };
    let commit_oid = resolve("commit")?;
    let tree_oid = git_output(
        [
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{commit_oid}^{{tree}}"),
        ],
        Some(root),
    )
    .map(|value| value.trim().to_string())
    .map_err(|_| Error::Parse("tiber.final_review_completion_tree_invalid=true".into()))?;
    Ok((commit_oid, tree_oid))
}

#[derive(Clone)]
struct TreeManifestEntry {
    mode: String,
    object_type: String,
    oid: String,
}

#[derive(Default)]
struct FinalReviewBlobBatch {
    bytes: Vec<u8>,
    ranges: BTreeMap<String, (usize, usize)>,
}

impl FinalReviewBlobBatch {
    fn get(&self, oid: &str) -> Option<&[u8]> {
        let (start, end) = self.ranges.get(oid)?;
        self.bytes.get(*start..*end)
    }
}

fn bounded_final_review_blob_batch(
    root: &Path,
    object_ids: &BTreeSet<String>,
    scope_kind: &str,
) -> Result<FinalReviewBlobBatch, Error> {
    if object_ids.is_empty() {
        return Ok(FinalReviewBlobBatch::default());
    }
    #[cfg(test)]
    count_final_review_tree_git_process();
    let mut child = Command::new("git")
        .args(["cat-file", "--batch"])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(Error::Io)?;
    let requested_oids = object_ids.iter().cloned().collect::<Vec<_>>();
    let mut stdin = child.stdin.take().expect("piped Git stdin");
    let writer = thread::spawn(move || -> std::io::Result<()> {
        for oid in requested_oids {
            writeln!(stdin, "{oid}")?;
        }
        Ok(())
    });
    let stdout = child.stdout.take().expect("piped Git stdout");
    let stderr = child.stderr.take().expect("piped Git stderr");
    let stderr_reader = thread::spawn(move || -> std::io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        stderr
            .take((FINAL_REVIEW_GIT_OUTPUT_MAX_BYTES as u64).saturating_add(1))
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    });
    let maximum = FINAL_REVIEW_SCOPE_LIMITS
        .content_bytes
        .saturating_add(FINAL_REVIEW_SCOPE_LIMITS.git_output_bytes as u64);
    let mut stdout_bytes = Vec::new();
    stdout
        .take(maximum.saturating_add(1))
        .read_to_end(&mut stdout_bytes)
        .map_err(Error::Io)?;
    if stdout_bytes.len() as u64 > maximum {
        let _ = child.kill();
        let _ = child.wait();
        let _ = writer.join();
        let _ = stderr_reader.join();
        return Err(Error::Usage(format!(
            "tiber.final_review_{scope_kind}_scope_too_large condition=content_bytes max_bytes={} action=\"narrow the declared review scope and record fresh independent reviews\"",
            FINAL_REVIEW_SCOPE_LIMITS.content_bytes
        )));
    }
    writer
        .join()
        .map_err(|_| Error::Parse("tiber.final_review_blob_batch_writer_failed=true".into()))?
        .map_err(Error::Io)?;
    let status = child.wait().map_err(Error::Io)?;
    let mut stderr_bytes = stderr_reader
        .join()
        .map_err(|_| Error::Parse("tiber.final_review_git_stderr_reader_failed=true".into()))?
        .map_err(Error::Io)?;
    stderr_bytes.truncate(FINAL_REVIEW_GIT_OUTPUT_MAX_BYTES);
    if !status.success() {
        return Err(Error::CommandFailed {
            program: "git".into(),
            args: vec!["cat-file".into(), "--batch".into()],
            status: status.code().unwrap_or(1).to_string(),
            stderr: String::from_utf8_lossy(&stderr_bytes).into_owned(),
        });
    }
    let mut cursor = 0_usize;
    let mut ranges = BTreeMap::new();
    for requested_oid in object_ids {
        let header_end = stdout_bytes[cursor..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|offset| cursor + offset)
            .ok_or_else(|| Error::Parse("tiber.final_review_blob_batch_invalid=true".into()))?;
        let header = std::str::from_utf8(&stdout_bytes[cursor..header_end])
            .map_err(|_| Error::Parse("tiber.final_review_blob_batch_invalid=true".into()))?;
        let mut fields = header.split_whitespace();
        let oid = fields.next().unwrap_or_default();
        let object_type = fields.next().unwrap_or_default();
        let size = fields
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .ok_or_else(|| Error::Parse("tiber.final_review_blob_batch_invalid=true".into()))?;
        if oid != requested_oid || object_type != "blob" {
            return Err(Error::Parse(
                "tiber.final_review_blob_batch_invalid=true".into(),
            ));
        }
        let content_start = header_end + 1;
        let content_end = content_start
            .checked_add(size)
            .ok_or_else(|| Error::Parse("tiber.final_review_blob_batch_invalid=true".into()))?;
        if content_end >= stdout_bytes.len() || stdout_bytes[content_end] != b'\n' {
            return Err(Error::Parse(
                "tiber.final_review_blob_batch_invalid=true".into(),
            ));
        }
        ranges.insert(oid.to_string(), (content_start, content_end));
        cursor = content_end + 1;
    }
    if cursor != stdout_bytes.len() {
        return Err(Error::Parse(
            "tiber.final_review_blob_batch_invalid=true".into(),
        ));
    }
    Ok(FinalReviewBlobBatch {
        bytes: stdout_bytes,
        ranges,
    })
}

fn final_review_pathspec_uses_attributes(pathspec: &str) -> bool {
    pathspec
        .strip_prefix(":(")
        .and_then(|suffix| suffix.split_once(')'))
        .is_some_and(|(magic, _)| {
            magic
                .split(',')
                .any(|component| component.starts_with("attr:"))
        })
}

fn final_review_attribute_source_support_from_command(
    command: &mut Command,
    scope_kind: &str,
) -> Result<(), Error> {
    let status = command.status().map_err(Error::Io)?;
    if status.success() {
        return Ok(());
    }
    Err(Error::Usage(format!(
        "tiber.final_review_{scope_kind}_attribute_source_unsupported action=\"upgrade Git to 2.41 or newer and record fresh independent reviews\""
    )))
}

fn ensure_final_review_attribute_source_support(
    root: &Path,
    tree_oid: &str,
    scope_kind: &str,
) -> Result<(), Error> {
    let mut command = Command::new("git");
    command
        .arg("check-attr")
        .arg(format!("--source={tree_oid}"))
        .args(["--all", "--", "."])
        .current_dir(root)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    final_review_attribute_source_support_from_command(&mut command, scope_kind)
}

fn tree_scope_fingerprint(
    root: &Path,
    tree_oid: &str,
    requested_scope: &[String],
    retained_scope: &[String],
    scope_kind: &str,
) -> Result<(Vec<String>, String), Error> {
    if requested_scope.is_empty()
        || retained_scope.is_empty()
        || requested_scope
            .iter()
            .chain(retained_scope)
            .any(|path| path.is_empty() || path.starts_with('/') || path.contains('\0'))
    {
        return Err(Error::Usage(format!(
            "tiber.final_review_{scope_kind}_scope_invalid action=\"record fresh v5 reviews with repository-contained materialized paths\""
        )));
    }
    let index_directory = tempfile::Builder::new()
        .prefix("tiber-final-review-tree-index-")
        .tempdir()
        .map_err(Error::Io)?;
    let index_file = index_directory.path().join("index");
    #[cfg(test)]
    count_final_review_tree_git_process();
    let read_tree = Command::new("git")
        .args(["read-tree", tree_oid])
        .env("GIT_INDEX_FILE", &index_file)
        .current_dir(root)
        .output()
        .map_err(Error::Io)?;
    if !read_tree.status.success() {
        return Err(Error::Parse(
            "tiber.final_review_completion_tree_invalid=true".into(),
        ));
    }
    if requested_scope
        .iter()
        .any(|pathspec| final_review_pathspec_uses_attributes(pathspec))
    {
        ensure_final_review_attribute_source_support(root, tree_oid, scope_kind)?;
    }
    #[cfg(test)]
    count_final_review_tree_git_process();
    let materialized = bounded_final_review_git_output_with_index(
        root,
        &["ls-files", "--cached", "-z", "--"],
        requested_scope,
        scope_kind,
        FINAL_REVIEW_SCOPE_LIMITS,
        Some(&index_file),
        Some(tree_oid),
    )?;
    let mut paths =
        bounded_final_review_paths(&materialized, scope_kind, FINAL_REVIEW_SCOPE_LIMITS)?
            .into_iter()
            .collect::<BTreeSet<_>>();
    let retained_paths = retained_scope
        .iter()
        .map(|path| path.strip_prefix(":(literal)").unwrap_or(path).to_string())
        .collect::<BTreeSet<_>>();
    paths.extend(retained_paths.iter().cloned());
    let literal_paths = paths
        .iter()
        .map(|path| format!(":(literal){path}"))
        .collect::<Vec<_>>();
    #[cfg(test)]
    count_final_review_tree_git_process();
    let output = bounded_final_review_git_output(
        root,
        &["ls-tree", "-rzt", "--full-tree", tree_oid, "--"],
        &literal_paths,
        scope_kind,
        FINAL_REVIEW_SCOPE_LIMITS,
    )?;
    let mut entries = BTreeMap::new();
    for record in output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        let separator = record
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or_else(|| Error::Parse("tiber.final_review_tree_entry_invalid=true".into()))?;
        let metadata = std::str::from_utf8(&record[..separator])
            .map_err(|_| Error::Parse("tiber.final_review_tree_entry_invalid=true".into()))?;
        let path = String::from_utf8(record[separator + 1..].to_vec())
            .map_err(|_| Error::Usage("tiber.final_review_scope_path_invalid_utf8=true".into()))?;
        let mut fields = metadata.split_whitespace();
        entries.insert(
            path,
            TreeManifestEntry {
                mode: fields.next().unwrap_or_default().to_string(),
                object_type: fields.next().unwrap_or_default().to_string(),
                oid: fields.next().unwrap_or_default().to_string(),
            },
        );
    }
    if paths.len() > FINAL_REVIEW_SCOPE_LIMITS.paths {
        return Err(Error::Usage(format!(
            "tiber.final_review_{scope_kind}_scope_too_large condition=path_count paths={} max_paths={} action=\"narrow the declared review scope and record fresh independent reviews\"",
            paths.len(), FINAL_REVIEW_SCOPE_LIMITS.paths
        )));
    }
    let blob_oids = paths
        .iter()
        .filter_map(|path| entries.get(path))
        .filter(|entry| matches!(entry.mode.as_str(), "120000" | "100644" | "100755"))
        .map(|entry| entry.oid.clone())
        .collect::<BTreeSet<_>>();
    let blobs = bounded_final_review_blob_batch(root, &blob_oids, scope_kind)?;
    let mut missing = Vec::new();
    let mut content_bytes = 0_u64;
    let mut hasher = Sha256::new();
    hasher.update(b"tiber-final-review-scope-v5\0");
    for materialized_path in paths {
        if materialized_path.is_empty() || materialized_path.starts_with('/') {
            return Err(Error::Usage(format!(
                "tiber.final_review_{scope_kind}_scope_invalid action=\"record fresh v5 reviews with repository-contained materialized paths\""
            )));
        }
        hasher.update(materialized_path.as_bytes());
        hasher.update([0]);
        let Some(entry) = entries.get(&materialized_path) else {
            missing.push(materialized_path);
            hasher.update(b"deleted\0deleted\0");
            continue;
        };
        hasher.update(entry.mode.as_bytes());
        hasher.update([0]);
        match entry.mode.as_str() {
            "160000" => {
                hasher.update(b"gitlink\0");
                hasher.update(entry.oid.as_bytes());
            }
            "040000" if entry.object_type == "tree" => hasher.update(b"directory\0"),
            "120000" | "100644" | "100755" if entry.object_type == "blob" => {
                if entry.mode == "120000" {
                    hasher.update(b"symlink\0");
                } else {
                    hasher.update(b"blob\0");
                }
                let blob = blobs.get(&entry.oid).ok_or_else(|| {
                    Error::Parse("tiber.final_review_blob_batch_invalid=true".into())
                })?;
                content_bytes = content_bytes.saturating_add(blob.len() as u64);
                if content_bytes > FINAL_REVIEW_SCOPE_LIMITS.content_bytes {
                    return Err(Error::Usage(format!(
                        "tiber.final_review_{scope_kind}_scope_too_large condition=content_bytes max_bytes={} action=\"narrow the declared review scope and record fresh independent reviews\"",
                        FINAL_REVIEW_SCOPE_LIMITS.content_bytes
                    )));
                }
                hasher.update(blob);
            }
            _ => {
                return Err(Error::Usage(format!(
                    "tiber.final_review_completion_tree_entry_unsupported path={materialized_path:?} mode={:?} type={:?}", entry.mode, entry.object_type
                )));
            }
        }
        hasher.update([0]);
    }
    Ok((missing, format!("sha256:v5:{:x}", hasher.finalize())))
}

struct TreeFinalReviewFingerprintCache {
    entries: BTreeMap<FinalReviewFingerprintCacheKey, (Vec<String>, String)>,
    recency: VecDeque<FinalReviewFingerprintCacheKey>,
    max_entries: usize,
}

impl Default for TreeFinalReviewFingerprintCache {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            recency: VecDeque::new(),
            max_entries: FINAL_REVIEW_FINGERPRINT_CACHE_MAX_ENTRIES,
        }
    }
}

#[cfg(test)]
impl TreeFinalReviewFingerprintCache {
    fn with_max_entries(max_entries: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            recency: VecDeque::new(),
            max_entries,
        }
    }
}

fn tree_final_review_fingerprint_cache_key(
    tree_oid: &str,
    scope_kind: &str,
    requested_scope: &[String],
    retained_scope: &[String],
) -> FinalReviewFingerprintCacheKey {
    let mut hasher = Sha256::new();
    hasher.update(b"tiber-final-review-tree-cache-v1\0");
    for value in [tree_oid, scope_kind] {
        hasher.update((value.len() as u64).to_le_bytes());
        hasher.update(value.as_bytes());
    }
    for scope in [requested_scope, retained_scope] {
        hasher.update((scope.len() as u64).to_le_bytes());
        for path in scope {
            hasher.update((path.len() as u64).to_le_bytes());
            hasher.update(path.as_bytes());
        }
    }
    hasher.finalize().into()
}

fn cached_tree_scope_fingerprint(
    root: &Path,
    tree_oid: &str,
    requested_scope: &[String],
    retained_scope: &[String],
    scope_kind: &str,
    cache: &mut TreeFinalReviewFingerprintCache,
) -> Result<(Vec<String>, String), Error> {
    let key = tree_final_review_fingerprint_cache_key(
        tree_oid,
        scope_kind,
        requested_scope,
        retained_scope,
    );
    if let Some(value) = cache.entries.get(&key) {
        let value = value.clone();
        cache.recency.retain(|candidate| candidate != &key);
        cache.recency.push_back(key);
        return Ok(value);
    }
    let value =
        tree_scope_fingerprint(root, tree_oid, requested_scope, retained_scope, scope_kind)?;
    if cache.max_entries > 0 {
        if cache.entries.len() == cache.max_entries {
            if let Some(oldest) = cache.recency.pop_front() {
                cache.entries.remove(&oldest);
            }
        }
        cache.entries.insert(key, value.clone());
        cache.recency.push_back(key);
    }
    Ok(value)
}

fn completion_snapshot_for_review(
    root: &Path,
    revision: &str,
    latest: &FinalReviewRecord,
    required: usize,
    delivery: bool,
) -> Result<FinalReviewCompletionSnapshot, Error> {
    let (commit_oid, tree_oid) = canonical_commit_snapshot(root, revision)?;
    completion_snapshot_for_review_at(
        root,
        &commit_oid,
        &tree_oid,
        latest,
        required,
        delivery,
        &mut TreeFinalReviewFingerprintCache::default(),
    )
}

fn completion_snapshot_for_review_at(
    root: &Path,
    commit_oid: &str,
    tree_oid: &str,
    latest: &FinalReviewRecord,
    required: usize,
    delivery: bool,
    cache: &mut TreeFinalReviewFingerprintCache,
) -> Result<FinalReviewCompletionSnapshot, Error> {
    let delivery_field = if delivery {
        " delivery_blocked=true"
    } else {
        ""
    };
    if !latest.source_fingerprint.starts_with("sha256:v5:")
        || !latest.verification_fingerprint.starts_with("sha256:v5:")
    {
        return Err(Error::Usage(format!(
            "tiber.final_review_completion_snapshot_mutable{delivery_field} condition=fingerprint_version_legacy required={required} clean=0 missing={required} action=\"record {required} fresh clean independent final reviews using v5 immutable-compatible fingerprints\""
        )));
    }
    let canonical_range = canonical_commit_range(root, &latest.commit_range)?;
    if canonical_range != latest.commit_range {
        return Err(Error::Usage(format!(
            "tiber.final_review_evidence_stale{delivery_field} condition=commit_range_changed required={required} clean=0 missing={required} reviewed={} current={} action=\"record {required} fresh clean independent final reviews for the pinned commit range\"",
            latest.commit_range, canonical_range
        )));
    }
    let (missing_source, source_fingerprint) = cached_tree_scope_fingerprint(
        root,
        tree_oid,
        &latest.requested_scope,
        &latest.scope,
        "source",
        cache,
    )?;
    if source_fingerprint != latest.source_fingerprint {
        let condition = if missing_source.is_empty() {
            "source_changed"
        } else {
            "source_not_committed"
        };
        let path = missing_source
            .first()
            .map_or(String::new(), |path| format!(" path={path:?}"));
        return Err(Error::Usage(format!(
            "tiber.final_review_completion_snapshot_mutable{delivery_field} condition={condition}{path} required={required} clean=0 missing={required} action=\"commit the reviewed source snapshot and complete {required} fresh clean independent final reviews\""
        )));
    }
    let (missing_verification, verification_fingerprint) = cached_tree_scope_fingerprint(
        root,
        tree_oid,
        &latest.requested_verification_scope,
        &latest.verification_scope,
        "verification",
        cache,
    )?;
    if verification_fingerprint != latest.verification_fingerprint {
        let condition = if missing_verification.is_empty() {
            "verification_changed"
        } else {
            "verification_not_committed"
        };
        let path = missing_verification
            .first()
            .map_or(String::new(), |path| format!(" path={path:?}"));
        return Err(Error::Usage(format!(
            "tiber.final_review_completion_snapshot_mutable{delivery_field} condition={condition}{path} required={required} clean=0 missing={required} action=\"commit the reviewed verification receipt or configure an immutable committed receipt before completion\""
        )));
    }
    Ok(FinalReviewCompletionSnapshot {
        commit_oid: commit_oid.to_string(),
        tree_oid: tree_oid.to_string(),
        source_fingerprint,
        verification_fingerprint,
    })
}

#[cfg(debug_assertions)]
fn wait_at_completion_snapshot_test_barrier() -> Result<(), Error> {
    let Some(directory) = std::env::var_os("TIBER_TEST_COMPLETION_SNAPSHOT_BARRIER") else {
        return Ok(());
    };
    let directory = PathBuf::from(directory);
    fs::create_dir_all(&directory)?;
    fs::write(directory.join("ready"), b"snapshot-resolved\n")?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while !directory.join("release").exists() {
        if Instant::now() >= deadline {
            return Err(Error::Parse(
                "tiber.final_review_test_barrier_timeout=true".into(),
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

#[cfg(not(debug_assertions))]
fn wait_at_completion_snapshot_test_barrier() -> Result<(), Error> {
    Ok(())
}

type FinalReviewFingerprintCacheKey = [u8; 32];

fn apply_final_review_record_to_clean_sequence(
    clean_reviews: &mut Vec<FinalReviewRecord>,
    review: &FinalReviewRecord,
) {
    match review.outcome {
        FinalReviewOutcome::Finding => clean_reviews.clear(),
        FinalReviewOutcome::Clean => {
            let fingerprints_changed = clean_reviews.last().is_some_and(|previous| {
                previous.scope != review.scope
                    || previous.requested_scope != review.requested_scope
                    || previous.commit_range != review.commit_range
                    || previous.source_fingerprint != review.source_fingerprint
                    || previous.verification_scope != review.verification_scope
                    || previous.requested_verification_scope != review.requested_verification_scope
                    || previous.verification_fingerprint != review.verification_fingerprint
            });
            if fingerprints_changed {
                clean_reviews.clear();
            }
            clean_reviews.push(review.clone());
        }
    }
}

fn enforce_final_review_authority_snapshot(
    stem: &str,
    authoritative: &[FinalReviewRecord],
    expected: &[FinalReviewRecord],
    required: usize,
    delivery: bool,
) -> Result<(), eventcore::CommandError> {
    let delivery_field = if delivery {
        " delivery_blocked=true"
    } else {
        ""
    };
    if authoritative != expected {
        return Err(format!(
            "tiber.final_review_evidence_changed_during_completion{delivery_field} task={stem} required={required} clean={} action=\"retry completion so the final-review gate evaluates the latest authoritative evidence\"",
            authoritative.len()
        )
        .into());
    }
    let clean = authoritative.len();
    if clean < required {
        return Err(format!(
            "tiber.final_review_evidence_incomplete{delivery_field} task={stem} required={required} clean={clean} missing={} action=\"record {} additional consecutive clean independent final review(s) for the current declared scope and verification evidence\"",
            required - clean,
            required - clean
        )
        .into());
    }
    let sequence = &authoritative[clean - required..];
    let distinct_reviewers = sequence
        .iter()
        .map(|review| review.reviewer_identity.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let distinct_reviews = sequence
        .iter()
        .map(|review| review.review_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let independent_review_types = sequence
        .iter()
        .filter(|review| review.reviewer_type == "independent-final-review")
        .count();
    if distinct_reviewers < required
        || distinct_reviews < required
        || independent_review_types < required
    {
        return Err(format!(
            "tiber.final_review_independence_incomplete{delivery_field} task={stem} required={required} distinct_reviewers={distinct_reviewers} distinct_reviews={distinct_reviews} independent_review_types={independent_review_types} expected_reviewer_type=independent-final-review action=\"record clean reviews with unique review and reviewer identities using the independent-final-review reviewer type\""
        )
        .into());
    }
    Ok(())
}

fn enforce_final_review_gate(
    root: &Path,
    task: &Task,
    required: usize,
    delivery: bool,
) -> Result<(), Error> {
    enforce_final_review_gate_with_cache(root, task, required, delivery)
}

fn enforce_final_review_gate_with_cache(
    root: &Path,
    task: &Task,
    required: usize,
    delivery: bool,
) -> Result<(), Error> {
    let stem = &task.stem;
    let delivery_field = if delivery {
        " delivery_blocked=true"
    } else {
        ""
    };
    let clean = task.final_review.clean_reviews.len();
    if clean < required {
        return Err(Error::Usage(format!(
            "tiber.final_review_evidence_incomplete{delivery_field} task={stem} required={required} clean={clean} missing={} action=\"record {} additional consecutive clean independent final review(s) for the current declared scope and verification evidence\"",
            required - clean,
            required - clean
        )));
    }
    let sequence = &task.final_review.clean_reviews[clean - required..];
    let distinct_reviewers = sequence
        .iter()
        .map(|review| review.reviewer_identity.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let distinct_reviews = sequence
        .iter()
        .map(|review| review.review_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let independent_review_types = sequence
        .iter()
        .filter(|review| review.reviewer_type == "independent-final-review")
        .count();
    if distinct_reviewers < required
        || distinct_reviews < required
        || independent_review_types < required
    {
        return Err(Error::Usage(format!(
            "tiber.final_review_independence_incomplete{delivery_field} task={stem} required={required} distinct_reviewers={distinct_reviewers} distinct_reviews={distinct_reviews} independent_review_types={independent_review_types} expected_reviewer_type=independent-final-review action=\"record clean reviews with unique review and reviewer identities using the independent-final-review reviewer type\""
        )));
    }
    let latest = sequence
        .last()
        .expect("required final review sequence is nonempty");
    let canonical_range = canonical_commit_range(root, &latest.commit_range)?;
    if canonical_range != latest.commit_range {
        return Err(Error::Usage(format!(
            "tiber.final_review_evidence_stale{delivery_field} task={stem} condition=commit_range_changed required={required} clean=0 missing={required} reviewed={} current={} action=\"record {required} fresh clean independent final reviews for the pinned commit range\"",
            latest.commit_range, canonical_range
        )));
    }
    Ok(())
}

#[derive(ModelInput)]
struct RecordFinalReviewRequest {
    #[model(origin)]
    board: TiberBoardStream,
    #[model(origin)]
    intent: RecordFinalReviewIntent,
}

#[derive(ModelCommand)]
struct RecordFinalReview {
    #[stream]
    board: TiberBoardStream,
    intent: RecordFinalReviewIntent,
}

mapping! { RecordFinalReviewRequestToBoard:
RecordFinalReviewRequest.board => RecordFinalReview.board using clone; }
mapping! { RecordFinalReviewRequestToIntent:
RecordFinalReviewRequest.intent => RecordFinalReview.intent using clone; }

fn recorded_final_review_fact(
    board: &TiberBoardStream,
    intent: &RecordFinalReviewIntent,
    _: &[String],
    _: &BTreeSet<String>,
    task_stems: &BTreeSet<String>,
) -> TaskFinalReviewRecordedEvent {
    debug_assert!(task_stems.contains(&intent.stem));
    TaskFinalReviewRecordedEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        stem: intent.stem.clone(),
        review: intent.review.clone(),
    }
}

mapping! { RecordFinalReviewToFact:
    (RecordFinalReview.board, RecordFinalReview.intent, AddTaskNoteState.board_order, AddTaskNoteState.board_task_stems, AddTaskNoteState.task_stems) => TiberEvent.TaskFinalReviewRecorded
    using recorded_final_review_fact;
}

impl ModelCommandLogic for RecordFinalReview {
    type Event = TiberEvent;
    type State = AddTaskNoteState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        match event {
            TiberEvent::TaskCreated(TaskCreatedEvent { stream_id, task }) => {
                state.task_stems.insert(task.stem.clone());
                if stream_id.as_ref() == BOARD_STREAM {
                    state.board_task_stems.insert(task.stem.clone());
                }
            }
            TiberEvent::LegacyTaskRemoved(TaskStemEvent { stem, .. }) => {
                state.task_stems.remove(stem);
                state.board_task_stems.remove(stem);
            }
            TiberEvent::TaskPriorityChanged(TaskOrderEvent { order, .. })
            | TiberEvent::BoardReordered(TaskOrderEvent { order, .. }) => {
                state.board_order.clone_from(order);
            }
            _ => {}
        }
        Modeled::from_built(state)
    }

    fn discover_related_streams(&self, state: &Modeled<Self::State>) -> Vec<StreamId> {
        state
            .as_ref()
            .board_order
            .iter()
            .filter(|stem| !state.as_ref().board_task_stems.contains(*stem))
            .filter_map(|stem| stream_id(format!("tiber:task:{stem}")).ok())
            .collect()
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        if !state.as_ref().task_stems.contains(&self.intent.stem) {
            return Err("tiber_final_review_task_missing".into());
        }
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_taskfinalreviewrecorded(RecordFinalReviewToFact::apply((
                self,
                self,
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
            ))),
        ))
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_record_final_review(
    root: &Path,
    task_ref: &str,
    review_id: &str,
    reviewer_identity: &str,
    reviewer_type: &str,
    scope: &[String],
    commit_range: &str,
    outcome: &str,
    evidence: &str,
    timestamp: &str,
    source_fingerprint: &str,
    verification_scope: &[String],
    verification_fingerprint: &str,
) -> Result<(), Error> {
    let outcome = FinalReviewOutcome::parse(outcome).ok_or_else(|| {
        Error::Usage("tiber.final_review_outcome_invalid expected=clean,finding".into())
    })?;
    let review_id = parse_nonempty_text(review_id, "review_id")?;
    let reviewer_identity = parse_nonempty_text(reviewer_identity, "reviewer_identity")?;
    let reviewer_type = parse_nonempty_text(reviewer_type, "reviewer_type")?;
    if reviewer_type != "independent-final-review" {
        return Err(Error::Usage(
            "tiber.final_review_reviewer_type_invalid expected=independent-final-review action=\"record the result of a fresh independent final review\""
                .into(),
        ));
    }
    let commit_range = parse_nonempty_text(commit_range, "commit_range")?;
    let evidence = parse_nonempty_text(evidence, "evidence")?;
    let timestamp = parse_nonempty_text(timestamp, "timestamp")?;
    let source_fingerprint = parse_nonempty_text(source_fingerprint, "source_fingerprint")?;
    let verification_fingerprint =
        parse_nonempty_text(verification_fingerprint, "verification_fingerprint")?;
    chrono::DateTime::parse_from_rfc3339(timestamp).map_err(|_| {
        Error::Usage(
            "tiber.final_review_timestamp_invalid expected=RFC3339 action=\"provide the review completion timestamp with an explicit offset\""
                .into(),
        )
    })?;
    let repository = GitRepository::at(root);
    let _lock = repository.acquire_lock()?;
    execute_initialize_tiber_repository(root)?;
    let projection = load_tiber_projection(root)?;
    let mut requested_scope = scope.to_vec();
    requested_scope.sort();
    requested_scope.dedup();
    let mut requested_verification_scope = verification_scope.to_vec();
    requested_verification_scope.sort();
    requested_verification_scope.dedup();
    let commit_range = canonical_commit_range(root, commit_range)?;
    let scope: Vec<String> = final_review_scope_snapshot(
        root,
        &requested_scope,
        Some(&commit_range),
        "source",
        FINAL_REVIEW_SCOPE_LIMITS,
    )?
    .paths
    .into_iter()
    .map(|path| format!(":(literal){path}"))
    .collect();
    let verification_scope: Vec<String> = final_review_scope_snapshot(
        root,
        &requested_verification_scope,
        None,
        "verification",
        FINAL_REVIEW_SCOPE_LIMITS,
    )?
    .paths
    .into_iter()
    .map(|path| format!(":(literal){path}"))
    .collect();
    let computed_source_fingerprint =
        source_scope_fingerprint(root, &scope, Some(&commit_range), "source")?;
    if source_fingerprint != "auto" && source_fingerprint != computed_source_fingerprint {
        return Err(Error::Usage(format!(
            "tiber.final_review_source_fingerprint_mismatch supplied={source_fingerprint} current={computed_source_fingerprint} action=\"rerun with --source-fingerprint auto or the reported current fingerprint\""
        )));
    }
    let computed_verification_fingerprint =
        source_scope_fingerprint(root, &verification_scope, None, "verification")?;
    if verification_fingerprint != "auto"
        && verification_fingerprint != computed_verification_fingerprint
    {
        return Err(Error::Usage(format!(
            "tiber.final_review_verification_fingerprint_mismatch supplied={verification_fingerprint} current={computed_verification_fingerprint} action=\"rerun with --verification-fingerprint auto or the reported current fingerprint\""
        )));
    }
    let request = RecordFinalReviewRequest::model_builder()
        .board(TiberBoardStream(stream_id(BOARD_STREAM)?))
        .intent(RecordFinalReviewIntent {
            stem: resolve_task_stem_from_projection(&projection, task_ref)?,
            review: FinalReviewRecord {
                review_id: review_id.into(),
                reviewer_identity: reviewer_identity.into(),
                reviewer_type: reviewer_type.into(),
                requested_scope,
                scope,
                commit_range,
                outcome,
                evidence: evidence.into(),
                timestamp: timestamp.into(),
                source_fingerprint: computed_source_fingerprint,
                requested_verification_scope,
                verification_scope,
                verification_fingerprint: computed_verification_fingerprint,
            },
        })
        .build();
    let command = RecordFinalReview::model_builder()
        .board(RecordFinalReviewRequestToBoard::apply(request.as_ref()))
        .intent(RecordFinalReviewRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

#[derive(ModelInput)]
struct ValidateTaskBoardRequest {
    #[model(origin)]
    board: TiberBoardStream,
}

#[derive(ModelCommand)]
struct ValidateTaskBoard {
    #[stream]
    board: TiberBoardStream,
}

mapping! { ValidateTaskBoardRequestToBoard:
ValidateTaskBoardRequest.board => ValidateTaskBoard.board using clone; }

/// Board reconciliation needs lifecycle status, dependency links, ordering,
/// and enough membership information to discover legacy per-task streams.
#[derive(ModelState)]
struct ValidateTaskBoardState {
    #[model(default)]
    board_order: Vec<String>,
    #[model(default)]
    board_task_stems: BTreeSet<String>,
    #[model(default)]
    task_statuses: BTreeMap<String, String>,
    #[model(default)]
    task_links: BTreeMap<String, (Vec<String>, Vec<String>)>,
}

#[derive(Clone)]
struct TaskBoardRepairPlan {
    stream_id: StreamId,
    link_changes: Vec<TaskLinksChangedEvent>,
    order_change: Option<TaskOrderEvent>,
    repairs: Vec<ValidationRepair>,
}

#[derive(ModelOutput)]
struct TaskBoardRepairOutput {
    plan: TaskBoardRepairPlan,
}

fn task_board_repair_plan(
    board: &TiberBoardStream,
    board_order: &[String],
    board_task_stems: &BTreeSet<String>,
    task_statuses: &BTreeMap<String, String>,
    task_links: &BTreeMap<String, (Vec<String>, Vec<String>)>,
) -> TaskBoardRepairPlan {
    debug_assert!(board_task_stems
        .iter()
        .all(|stem| task_statuses.contains_key(stem) && task_links.contains_key(stem)));
    let stream_id = eventcore::model::StreamIdentity::as_stream_id(board).clone();
    let mut repaired_links = task_links.clone();
    let link_snapshot = task_links.clone();
    for (stem, (blocks, blocked_by)) in link_snapshot {
        for blocked in blocks {
            if let Some((_target_blocks, target_blocked_by)) = repaired_links.get_mut(&blocked) {
                if !target_blocked_by.contains(&stem) {
                    target_blocked_by.push(stem.clone());
                }
            }
        }
        for blocker in blocked_by {
            if let Some((target_blocks, _target_blocked_by)) = repaired_links.get_mut(&blocker) {
                if !target_blocks.contains(&stem) {
                    target_blocks.push(stem.clone());
                }
            }
        }
    }
    let open = task_statuses
        .iter()
        .filter(|(_stem, status)| is_open_status(status))
        .map(|(stem, _status)| stem.clone())
        .collect::<Vec<_>>();
    let reconciliation = OrderReconciliation::reconcile(board_order.to_vec(), open);
    let mut link_changes = Vec::new();
    let mut repairs = Vec::new();
    for (stem, (repaired_blocks, repaired_blocked_by)) in &repaired_links {
        let (original_blocks, original_blocked_by) = task_links
            .get(stem)
            .expect("repaired task must originate in folded task state");
        if original_blocks != repaired_blocks || original_blocked_by != repaired_blocked_by {
            for target in repaired_blocks
                .iter()
                .filter(|target| !original_blocks.contains(*target))
            {
                repairs.push(ValidationRepair::ReciprocalLinkAdded {
                    task: stem.clone(),
                    field: "blocks".into(),
                    target: target.clone(),
                });
            }
            for target in repaired_blocked_by
                .iter()
                .filter(|target| !original_blocked_by.contains(*target))
            {
                repairs.push(ValidationRepair::ReciprocalLinkAdded {
                    task: stem.clone(),
                    field: "blocked_by".into(),
                    target: target.clone(),
                });
            }
            link_changes.push(TaskLinksChangedEvent {
                stream_id: stream_id.clone(),
                stem: stem.clone(),
                blocks: repaired_blocks.clone(),
                blocked_by: repaired_blocked_by.clone(),
            });
        }
    }
    let order_change = (board_order != reconciliation.entries()).then(|| {
        for task in reconciliation
            .entries()
            .iter()
            .filter(|task| !board_order.contains(*task))
        {
            repairs.push(ValidationRepair::BoardEntryAdded { task: task.clone() });
        }
        for task in board_order
            .iter()
            .filter(|task| !reconciliation.entries().contains(*task))
        {
            repairs.push(ValidationRepair::BoardEntryRemoved { task: task.clone() });
        }
        TaskOrderEvent {
            stream_id,
            order: reconciliation.entries().to_vec(),
        }
    });
    TaskBoardRepairPlan {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        link_changes,
        order_change,
        repairs,
    }
}

fn task_validation_repaired_fact(plan: &TaskBoardRepairPlan) -> TaskValidationRepairedEvent {
    TaskValidationRepairedEvent {
        stream_id: plan.stream_id.clone(),
        link_changes: plan.link_changes.clone(),
        order_change: plan.order_change.clone(),
        repairs: plan.repairs.clone(),
    }
}

mapping! { ValidateTaskBoardStateToRepairPlan:
    (ValidateTaskBoard.board, ValidateTaskBoardState.board_order, ValidateTaskBoardState.board_task_stems, ValidateTaskBoardState.task_statuses, ValidateTaskBoardState.task_links) => TaskBoardRepairOutput.plan
    using task_board_repair_plan;
}
mapping! { TaskBoardRepairPlanToFact:
    TaskBoardRepairOutput.plan => TiberEvent.TaskValidationRepaired
    using task_validation_repaired_fact;
}

impl ModelCommandLogic for ValidateTaskBoard {
    type Event = TiberEvent;
    type State = ValidateTaskBoardState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        Modeled::from_built(evolve_task_board_state(state.into_inner(), event))
    }

    fn discover_related_streams(&self, state: &Modeled<Self::State>) -> Vec<StreamId> {
        state
            .as_ref()
            .board_order
            .iter()
            .filter(|stem| !state.as_ref().board_task_stems.contains(*stem))
            .filter_map(|stem| stream_id(format!("tiber:task:{stem}")).ok())
            .collect()
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        let output = TaskBoardRepairOutput::model_builder()
            .plan(ValidateTaskBoardStateToRepairPlan::apply((
                self,
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
            )))
            .build();
        if output.as_ref().plan.repairs.is_empty() {
            return Ok(ModeledEvents::none("task board is already reconciled"));
        }
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_taskvalidationrepaired(TaskBoardRepairPlanToFact::apply(
                output.as_ref(),
            )),
        ))
    }
}

fn evolve_task_board_state(
    mut state: ValidateTaskBoardState,
    event: &TiberEvent,
) -> ValidateTaskBoardState {
    match event {
        TiberEvent::TaskCreated(TaskCreatedEvent { stream_id, task }) => {
            state
                .task_statuses
                .insert(task.stem.clone(), task.status.clone());
            state.task_links.insert(
                task.stem.clone(),
                (task.blocks.clone(), task.blocked_by.clone()),
            );
            if stream_id.as_ref() == BOARD_STREAM {
                state.board_task_stems.insert(task.stem.clone());
            }
        }
        TiberEvent::TaskTransitioned(TaskTransitionedEvent { stem, status, .. }) => {
            if let Some(current) = state.task_statuses.get_mut(stem) {
                current.clone_from(status);
            }
        }
        TiberEvent::LegacyTaskClosedFromTrailer(TaskStemEvent { stem, .. }) => {
            if let Some(current) = state.task_statuses.get_mut(stem) {
                *current = "done".into();
            }
        }
        TiberEvent::TasksClosedFromCommitTrailers(event) => {
            for stem in &event.stems {
                if let Some(current) = state.task_statuses.get_mut(stem) {
                    *current = "done".into();
                }
            }
            state.board_order.clone_from(&event.order);
        }
        TiberEvent::TaskLinksChanged(TaskLinksChangedEvent {
            stem,
            blocks,
            blocked_by,
            ..
        }) => {
            if let Some((current_blocks, current_blocked_by)) = state.task_links.get_mut(stem) {
                current_blocks.clone_from(blocks);
                current_blocked_by.clone_from(blocked_by);
            }
        }
        TiberEvent::TaskValidationRepaired(TaskValidationRepairedEvent {
            link_changes,
            order_change,
            ..
        }) => {
            for change in link_changes {
                if let Some((current_blocks, current_blocked_by)) =
                    state.task_links.get_mut(&change.stem)
                {
                    current_blocks.clone_from(&change.blocks);
                    current_blocked_by.clone_from(&change.blocked_by);
                }
            }
            if let Some(change) = order_change {
                state.board_order.clone_from(&change.order);
            }
        }
        TiberEvent::LegacyTaskRemoved(TaskStemEvent { stem, .. }) => {
            state.task_statuses.remove(stem);
            state.task_links.remove(stem);
            state.board_task_stems.remove(stem);
        }
        TiberEvent::TaskPriorityChanged(TaskOrderEvent { order, .. })
        | TiberEvent::BoardReordered(TaskOrderEvent { order, .. }) => {
            state.board_order.clone_from(order);
        }
        _ => {}
    }
    state
}

fn execute_validate_task_board(root: &Path) -> Result<Vec<ValidationMessage>, Error> {
    let repository = GitRepository::at(root);
    let _lock = repository.acquire_lock()?;
    execute_initialize_tiber_repository(root)?;
    let request = ValidateTaskBoardRequest::model_builder()
        .board(TiberBoardStream(stream_id(BOARD_STREAM)?))
        .build();
    let command = ValidateTaskBoard::model_builder()
        .board(ValidateTaskBoardRequestToBoard::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(Vec::new()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

#[derive(Clone)]
struct CloseTasksFromCommitTrailersIntent {
    stems: Vec<String>,
    minimum_clean_reviews: usize,
    expected_clean_reviews: BTreeMap<String, Vec<FinalReviewRecord>>,
    completion_snapshots: BTreeMap<String, FinalReviewCompletionSnapshot>,
}

#[derive(ModelInput)]
struct CloseTasksFromCommitTrailersRequest {
    #[model(origin)]
    board: TiberBoardStream,
    #[model(origin)]
    intent: CloseTasksFromCommitTrailersIntent,
}

#[derive(ModelCommand)]
struct CloseTasksFromCommitTrailers {
    #[stream]
    board: TiberBoardStream,
    intent: CloseTasksFromCommitTrailersIntent,
}

mapping! { CloseTasksFromCommitTrailersRequestToBoard:
CloseTasksFromCommitTrailersRequest.board => CloseTasksFromCommitTrailers.board using clone; }
mapping! { CloseTasksFromCommitTrailersRequestToIntent:
CloseTasksFromCommitTrailersRequest.intent => CloseTasksFromCommitTrailers.intent using clone; }

/// Trailer closure needs current board ordering, task lifecycle state, and the
/// authoritative clean-review sequence used by the completion invariant.
/// Keeping unrelated task fields out avoids coupling this command to full task
/// document reconciliation.
#[derive(ModelState)]
struct CloseTasksFromCommitTrailersState {
    #[model(default)]
    board_order: Vec<String>,
    #[model(default)]
    board_task_stems: BTreeSet<String>,
    #[model(default)]
    task_statuses: BTreeMap<String, String>,
    #[model(default)]
    final_review_clean_reviews: BTreeMap<String, Vec<FinalReviewRecord>>,
}

fn evolve_close_tasks_from_commit_trailers_state(
    mut state: CloseTasksFromCommitTrailersState,
    event: &TiberEvent,
) -> CloseTasksFromCommitTrailersState {
    match event {
        TiberEvent::TaskCreated(TaskCreatedEvent { stream_id, task }) => {
            state
                .task_statuses
                .insert(task.stem.clone(), task.status.clone());
            if stream_id.as_ref() == BOARD_STREAM {
                state.board_task_stems.insert(task.stem.clone());
            }
        }
        TiberEvent::TaskTransitioned(TaskTransitionedEvent { stem, status, .. }) => {
            if let Some(current) = state.task_statuses.get_mut(stem) {
                current.clone_from(status);
            }
        }
        TiberEvent::LegacyTaskClosedFromTrailer(TaskStemEvent { stem, .. }) => {
            if let Some(current) = state.task_statuses.get_mut(stem) {
                *current = "done".into();
            }
        }
        TiberEvent::TasksClosedFromCommitTrailers(event) => {
            for stem in &event.stems {
                if let Some(current) = state.task_statuses.get_mut(stem) {
                    *current = "done".into();
                }
            }
            state.board_order.clone_from(&event.order);
        }
        TiberEvent::TaskFinalReviewRecorded(TaskFinalReviewRecordedEvent {
            stem, review, ..
        }) => {
            apply_final_review_record_to_clean_sequence(
                state
                    .final_review_clean_reviews
                    .entry(stem.clone())
                    .or_default(),
                review,
            );
        }
        TiberEvent::LegacyTaskRemoved(TaskStemEvent { stem, .. }) => {
            state.task_statuses.remove(stem);
            state.board_task_stems.remove(stem);
        }
        TiberEvent::TaskPriorityChanged(TaskOrderEvent { order, .. })
        | TiberEvent::BoardReordered(TaskOrderEvent { order, .. }) => {
            state.board_order.clone_from(order);
        }
        _ => {}
    }
    state
}

fn closed_tasks_from_trailers_fact(
    intent: &CloseTasksFromCommitTrailersIntent,
    board: &TiberBoardStream,
    board_order: &[String],
    _board_task_stems: &BTreeSet<String>,
    task_statuses: &BTreeMap<String, String>,
    _final_review_clean_reviews: &BTreeMap<String, Vec<FinalReviewRecord>>,
) -> TasksClosedFromCommitTrailersEvent {
    let closed = intent
        .stems
        .iter()
        .filter(|stem| {
            task_statuses
                .get(*stem)
                .is_some_and(|status| status != "done")
        })
        .cloned()
        .collect::<Vec<_>>();
    let closed_set = closed.iter().cloned().collect::<BTreeSet<_>>();
    TasksClosedFromCommitTrailersEvent {
        stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
        stems: closed,
        order: board_order
            .iter()
            .filter(|stem| !closed_set.contains(*stem))
            .cloned()
            .collect(),
        completion_snapshots: intent.completion_snapshots.clone(),
    }
}
mapping! { CloseTasksFromCommitTrailersToFact:
(CloseTasksFromCommitTrailers.intent, CloseTasksFromCommitTrailers.board, CloseTasksFromCommitTrailersState.board_order, CloseTasksFromCommitTrailersState.board_task_stems, CloseTasksFromCommitTrailersState.task_statuses, CloseTasksFromCommitTrailersState.final_review_clean_reviews) => TiberEvent.TasksClosedFromCommitTrailers using closed_tasks_from_trailers_fact; }

impl ModelCommandLogic for CloseTasksFromCommitTrailers {
    type Event = TiberEvent;
    type State = CloseTasksFromCommitTrailersState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        Modeled::from_built(evolve_close_tasks_from_commit_trailers_state(
            state.into_inner(),
            event,
        ))
    }

    fn discover_related_streams(&self, state: &Modeled<Self::State>) -> Vec<StreamId> {
        state
            .as_ref()
            .board_order
            .iter()
            .filter(|stem| !state.as_ref().board_task_stems.contains(*stem))
            .filter_map(|stem| stream_id(format!("tiber:task:{stem}")).ok())
            .collect()
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        for stem in &self.intent.stems {
            state
                .as_ref()
                .task_statuses
                .get(stem)
                .ok_or("tiber_trailer_task_missing")?;
            if self.intent.minimum_clean_reviews > 0 {
                enforce_final_review_authority_snapshot(
                    stem,
                    state
                        .as_ref()
                        .final_review_clean_reviews
                        .get(stem)
                        .map(Vec::as_slice)
                        .unwrap_or_default(),
                    self.intent
                        .expected_clean_reviews
                        .get(stem)
                        .map(Vec::as_slice)
                        .unwrap_or_default(),
                    self.intent.minimum_clean_reviews,
                    true,
                )?;
                let snapshot = self.intent.completion_snapshots.get(stem).ok_or_else(|| {
                    eventcore::CommandError::from(format!(
                        "tiber.final_review_completion_snapshot_missing delivery_blocked=true task={stem} action=\"retry delivery against the immutable HEAD commit snapshot\""
                    ))
                })?;
                let latest = state
                    .as_ref()
                    .final_review_clean_reviews
                    .get(stem)
                    .and_then(|reviews| reviews.last())
                    .ok_or("tiber.final_review_completion_snapshot_missing")?;
                if snapshot.source_fingerprint != latest.source_fingerprint
                    || snapshot.verification_fingerprint != latest.verification_fingerprint
                {
                    return Err(format!(
                        "tiber.final_review_completion_snapshot_changed delivery_blocked=true task={stem} required={} clean=0 missing={} action=\"record fresh reviews for the immutable delivery snapshot\"",
                        self.intent.minimum_clean_reviews,
                        self.intent.minimum_clean_reviews
                    )
                    .into());
                }
            }
        }
        let has_open_task = self.intent.stems.iter().any(|stem| {
            state
                .as_ref()
                .task_statuses
                .get(stem)
                .is_some_and(|status| status != "done")
        });
        if !has_open_task && self.intent.completion_snapshots.is_empty() {
            return Ok(ModeledEvents::none(
                "trailer-referenced tasks are already closed",
            ));
        }
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_tasksclosedfromcommittrailers(
                CloseTasksFromCommitTrailersToFact::apply((
                    self,
                    self,
                    state.as_ref(),
                    state.as_ref(),
                    state.as_ref(),
                    state.as_ref(),
                )),
            ),
        ))
    }
}

fn execute_close_tasks_from_commit_trailers(root: &Path) -> Result<Vec<String>, Error> {
    let repository = GitRepository::at(root);
    let _lock = repository.acquire_lock()?;
    let log = repository.git(["log", "-1", "--format=%B"])?;
    let requested = closes_trailers(&log);
    if requested.is_empty() {
        return Ok(Vec::new());
    }
    execute_initialize_tiber_repository(root)?;
    let projection = load_tiber_projection(root)?;
    let mut stems = requested
        .iter()
        .map(|task_ref| resolve_task_stem_from_projection(&projection, task_ref))
        .collect::<Result<Vec<_>, _>>()?;
    stems.sort();
    stems.dedup();
    let config = repository.project_config_for_lifecycle_gate()?;
    let mut completion_snapshots = BTreeMap::new();
    if config.final_review.minimum_clean_reviews > 0 {
        let required = config.final_review.minimum_clean_reviews;
        let (completion_commit_oid, completion_tree_oid) = canonical_commit_snapshot(root, "HEAD")?;
        let mut tree_fingerprint_cache = TreeFinalReviewFingerprintCache::default();
        for stem in &stems {
            let task = projection
                .tasks
                .get(stem)
                .ok_or_else(|| Error::Parse(format!("task_ref_missing ref={stem}")))?;
            enforce_final_review_gate_with_cache(root, task, required, true)?;
            let latest = task
                .final_review
                .clean_reviews
                .last()
                .expect("gate requires reviews");
            completion_snapshots.insert(
                stem.clone(),
                completion_snapshot_for_review_at(
                    root,
                    &completion_commit_oid,
                    &completion_tree_oid,
                    latest,
                    required,
                    true,
                    &mut tree_fingerprint_cache,
                )?,
            );
        }
        wait_at_completion_snapshot_test_barrier()?;
    }
    let request = CloseTasksFromCommitTrailersRequest::model_builder()
        .board(TiberBoardStream(stream_id(BOARD_STREAM)?))
        .intent(CloseTasksFromCommitTrailersIntent {
            stems: stems.clone(),
            minimum_clean_reviews: config.final_review.minimum_clean_reviews,
            expected_clean_reviews: stems
                .iter()
                .filter_map(|stem| {
                    projection
                        .tasks
                        .get(stem)
                        .map(|task| (stem.clone(), task.final_review.clean_reviews.clone()))
                })
                .collect(),
            completion_snapshots,
        })
        .build();
    let command = CloseTasksFromCommitTrailers::model_builder()
        .board(CloseTasksFromCommitTrailersRequestToBoard::apply(
            request.as_ref(),
        ))
        .intent(CloseTasksFromCommitTrailersRequestToIntent::apply(
            request.as_ref(),
        ))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(stems),
        Err(eventcore::CommandError::BusinessRuleViolation(error)) => {
            let message = error.to_string();
            if message.starts_with("tiber.final_review_") {
                Err(Error::Usage(message))
            } else {
                Err(Error::Parse("eventcore_command_rejected=true".into()))
            }
        }
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

impl ModelCommandLogic for InitializeTiberRepository {
    type Event = TiberEvent;
    type State = InitializeTiberRepositoryState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut state = state.into_inner();
        if matches!(event, TiberEvent::RepositoryInitialized(_)) {
            state.initialized = true;
        }
        Modeled::from_built(state)
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, eventcore::CommandError> {
        if state.as_ref().initialized {
            return Err("tiber_repository_already_initialized".into());
        }
        Ok(ModeledEvents::one(
            TiberEvent::model_variant_repositoryinitialized(
                InitializeTiberRepositoryToFact::apply((self, state.as_ref())),
            ),
        ))
    }
}

fn execute_initialize_tiber_repository(root: &Path) -> Result<(), Error> {
    let request = InitializeTiberRepositoryRequest::model_builder()
        .stream(TiberRepositoryStream(stream_id(REPOSITORY_STREAM)?))
        .build();
    let command = InitializeTiberRepository::model_builder()
        .stream(InitializeTiberRepositoryRequestToStream::apply(
            request.as_ref(),
        ))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(()),
        // Initialization is idempotent. A retry may reload a transaction that
        // was durably published before this caller observed its outcome.
        Err(eventcore::CommandError::ValidationError(message))
            if message == "tiber_repository_already_initialized" =>
        {
            Ok(())
        }
        Err(eventcore::CommandError::BusinessRuleViolation(error))
            if error.to_string() == "tiber_repository_already_initialized" =>
        {
            Ok(())
        }
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

fn execute_claim_ci_recovery(root: &Path, intent: ClaimCiRecoveryIntent) -> Result<(), Error> {
    let request = ClaimCiRecoveryRequest::model_builder()
        .stream(CiRecoveryStream(stream_id(CI_RECOVERY_STREAM)?))
        .intent(intent)
        .build();
    let command = ClaimCiRecovery::model_builder()
        .stream(ClaimCiRecoveryRequestToStream::apply(request.as_ref()))
        .intent(ClaimCiRecoveryRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    let outcome =
        run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await });
    match outcome {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

fn execute_join_ci_recovery(root: &Path, intent: JoinCiRecoveryIntent) -> Result<(), Error> {
    let request = JoinCiRecoveryRequest::model_builder()
        .stream(CiRecoveryStream(stream_id(CI_RECOVERY_STREAM)?))
        .intent(intent)
        .build();
    let command = JoinCiRecovery::model_builder()
        .stream(JoinCiRecoveryRequestToStream::apply(request.as_ref()))
        .intent(JoinCiRecoveryRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    let outcome =
        run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await });
    match outcome {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

fn execute_transfer_ci_recovery(
    root: &Path,
    intent: TransferCiRecoveryIntent,
) -> Result<(), Error> {
    let request = TransferCiRecoveryRequest::model_builder()
        .stream(CiRecoveryStream(stream_id(CI_RECOVERY_STREAM)?))
        .intent(intent)
        .build();
    let command = TransferCiRecovery::model_builder()
        .stream(TransferCiRecoveryRequestToStream::apply(request.as_ref()))
        .intent(TransferCiRecoveryRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    let outcome =
        run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await });
    match outcome {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

fn execute_takeover_ci_recovery(
    root: &Path,
    intent: TakeOverCiRecoveryIntent,
) -> Result<(), Error> {
    let request = TakeOverCiRecoveryRequest::model_builder()
        .stream(CiRecoveryStream(stream_id(CI_RECOVERY_STREAM)?))
        .intent(intent)
        .build();
    let command = TakeOverCiRecovery::model_builder()
        .stream(TakeOverCiRecoveryRequestToStream::apply(request.as_ref()))
        .intent(TakeOverCiRecoveryRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    let outcome =
        run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await });
    match outcome {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

fn execute_assign_ci_recovery_work(
    root: &Path,
    intent: AssignCiRecoveryWorkIntent,
) -> Result<String, Error> {
    let expected_assignment = intent.assignment.clone();
    let request = AssignCiRecoveryWorkRequest::model_builder()
        .stream(CiRecoveryStream(stream_id(CI_RECOVERY_STREAM)?))
        .intent(intent)
        .build();
    let command = AssignCiRecoveryWork::model_builder()
        .stream(AssignCiRecoveryWorkRequestToStream::apply(request.as_ref()))
        .intent(AssignCiRecoveryWorkRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    let outcome =
        run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await });
    match outcome {
        Ok(_) => load_tiber_projection(root)?
            .ci_recovery
            .ok_or_else(|| Error::Parse("ci_recovery_incident_missing active=false".into()))?
            .assignments
            .iter()
            .rev()
            .find(|assignment| {
                assignment.owner_epoch == expected_assignment.owner_epoch
                    && assignment.assignee == expected_assignment.assignee
                    && assignment.capabilities == expected_assignment.capabilities
                    && assignment.scope == expected_assignment.scope
                    && assignment.report.is_none()
            })
            .map(|assignment| assignment.id.clone())
            .ok_or_else(|| Error::Parse("ci_recovery_assignment_commit_missing=true".into())),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

fn execute_report_ci_recovery_work(
    root: &Path,
    intent: ReportCiRecoveryWorkIntent,
) -> Result<(), Error> {
    let request = ReportCiRecoveryWorkRequest::model_builder()
        .stream(CiRecoveryStream(stream_id(CI_RECOVERY_STREAM)?))
        .intent(intent)
        .build();
    let command = ReportCiRecoveryWork::model_builder()
        .stream(ReportCiRecoveryWorkRequestToStream::apply(request.as_ref()))
        .intent(ReportCiRecoveryWorkRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    let outcome =
        run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await });
    match outcome {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

fn execute_renew_ci_recovery_lease(
    root: &Path,
    intent: RenewCiRecoveryLeaseIntent,
) -> Result<(), Error> {
    let request = RenewCiRecoveryLeaseRequest::model_builder()
        .stream(CiRecoveryStream(stream_id(CI_RECOVERY_STREAM)?))
        .intent(intent)
        .build();
    let command = RenewCiRecoveryLease::model_builder()
        .stream(RenewCiRecoveryLeaseRequestToStream::apply(request.as_ref()))
        .intent(RenewCiRecoveryLeaseRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    let outcome =
        run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await });
    match outcome {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

fn execute_record_ci_recovery_diagnosis(
    root: &Path,
    intent: RecordCiRecoveryDiagnosisIntent,
) -> Result<(), Error> {
    let request = RecordCiRecoveryDiagnosisRequest::model_builder()
        .stream(CiRecoveryStream(stream_id(CI_RECOVERY_STREAM)?))
        .intent(intent)
        .build();
    let command = RecordCiRecoveryDiagnosis::model_builder()
        .stream(RecordCiRecoveryDiagnosisRequestToStream::apply(
            request.as_ref(),
        ))
        .intent(RecordCiRecoveryDiagnosisRequestToIntent::apply(
            request.as_ref(),
        ))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    let outcome =
        run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await });
    match outcome {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

fn execute_select_ci_recovery_action(
    root: &Path,
    intent: SelectCiRecoveryActionIntent,
) -> Result<(), Error> {
    let request = SelectCiRecoveryActionRequest::model_builder()
        .stream(CiRecoveryStream(stream_id(CI_RECOVERY_STREAM)?))
        .intent(intent)
        .build();
    let command = SelectCiRecoveryAction::model_builder()
        .stream(SelectCiRecoveryActionRequestToStream::apply(
            request.as_ref(),
        ))
        .intent(SelectCiRecoveryActionRequestToIntent::apply(
            request.as_ref(),
        ))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

fn execute_record_ci_recovery_replacement(
    root: &Path,
    intent: RecordCiRecoveryReplacementIntent,
) -> Result<(), Error> {
    let request = RecordCiRecoveryReplacementRequest::model_builder()
        .stream(CiRecoveryStream(stream_id(CI_RECOVERY_STREAM)?))
        .intent(intent)
        .build();
    let command = RecordCiRecoveryReplacement::model_builder()
        .stream(RecordCiRecoveryReplacementRequestToStream::apply(
            request.as_ref(),
        ))
        .intent(RecordCiRecoveryReplacementRequestToIntent::apply(
            request.as_ref(),
        ))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

fn execute_resolve_ci_recovery(root: &Path, intent: ResolveCiRecoveryIntent) -> Result<(), Error> {
    let request = ResolveCiRecoveryRequest::model_builder()
        .stream(CiRecoveryStream(stream_id(CI_RECOVERY_STREAM)?))
        .intent(intent)
        .build();
    let command = ResolveCiRecovery::model_builder()
        .stream(ResolveCiRecoveryRequestToStream::apply(request.as_ref()))
        .intent(ResolveCiRecoveryRequestToIntent::apply(request.as_ref()))
        .build();
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    match run_async(async move { eventcore::execute(store, command, RetryPolicy::new()).await }) {
        Ok(_) => Ok(()),
        Err(eventcore::CommandError::ConcurrencyError(_))
        | Err(eventcore::CommandError::EventStoreError(EventStoreError::VersionConflict {
            ..
        })) => Err(Error::Parse("event_version_conflict=true".into())),
        Err(error) => Err(eventcore_command_error(error)),
    }
}

const WORKFLOW_BLOCKER_FILE: &str = "workflow-blocker.json";

thread_local! {
    static MCP_CI_RECOVERY_SESSION: RefCell<Option<String>> = const { RefCell::new(None) };
    static MCP_REPOSITORY_ROOT: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkflowBlocker {
    schema_version: u8,
    kind: String,
    error_code: String,
    required_action: String,
    created_at: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct WorkflowBlockerData {
    pub error_code: &'static str,
    pub required_action: &'static str,
}

pub fn with_mcp_ci_recovery_session<T>(session: &str, operation: impl FnOnce() -> T) -> T {
    MCP_CI_RECOVERY_SESSION.with(|slot| {
        let previous = slot.replace(Some(session.to_string()));
        let result = operation();
        slot.replace(previous);
        result
    })
}

pub fn with_mcp_repository_root<T>(root: Option<&Path>, operation: impl FnOnce() -> T) -> T {
    MCP_REPOSITORY_ROOT.with(|slot| {
        let previous = slot.replace(root.map(Path::to_path_buf));
        let result = operation();
        slot.replace(previous);
        result
    })
}

fn record_workflow_blocker(repo: &GitRepository, blocker: WorkflowBlocker) -> Result<(), Error> {
    let directory = repo.git_common_dir()?.join("tiber");
    fs::create_dir_all(&directory)?;
    let path = directory.join(WORKFLOW_BLOCKER_FILE);
    let temporary = directory.join(format!(".{WORKFLOW_BLOCKER_FILE}.{}", std::process::id()));
    let bytes = serde_json::to_vec(&blocker)
        .map_err(|error| Error::Parse(format!("workflow_blocker_json_invalid source={error}")))?;
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn clear_workflow_blocker(repo: &GitRepository, kind: &str) -> Result<(), Error> {
    let path = repo
        .git_common_dir()?
        .join("tiber")
        .join(WORKFLOW_BLOCKER_FILE);
    let Ok(contents) = fs::read_to_string(&path) else {
        return Ok(());
    };
    let blocker: WorkflowBlocker = serde_json::from_str(&contents)
        .map_err(|_| Error::Parse("workflow_blocker_invalid workflow_blocked=true".to_string()))?;
    if blocker.kind == kind {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Error::Io(error)),
        }
    }
    Ok(())
}

pub fn init_repository() -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.init_repository()
}

pub fn dashboard_runtime_dir() -> Result<PathBuf, Error> {
    let repo = GitRepository::discover()?;
    Ok(repo.git_common_dir()?.join("tiber"))
}

pub fn acquire_dashboard_startup_lock() -> Result<DashboardStartupLock, Error> {
    let repo = GitRepository::discover()?;
    Ok(DashboardStartupLock {
        _lock: repo.acquire_named_lock("dashboard-startup.lock")?,
    })
}

pub struct DashboardStartupLock {
    _lock: TiberLock,
}

pub fn init_repository_at(root: impl Into<PathBuf>) -> Result<(), Error> {
    let repo = GitRepository::at(root);
    repo.init_repository()
}

#[doc(hidden)]
pub fn sync_repository_at(root: impl Into<PathBuf>) -> Result<(), Error> {
    GitRepository::at(root).sync_repository()
}

pub fn claim_ci_recovery(input: CiRecoveryTrigger) -> Result<CiRecoveryClaim, Error> {
    let repo = GitRepository::discover()?;
    match repo.claim_ci_recovery(input) {
        Ok(claim) => {
            clear_workflow_blocker(&repo, "ci_claim_failed")?;
            Ok(claim)
        }
        Err(source) => {
            record_workflow_blocker(
                &repo,
                WorkflowBlocker {
                    schema_version: 1,
                    kind: "ci_claim_failed".to_string(),
                    error_code: "tiber.ci_recovery_claim_failed".to_string(),
                    required_action: "retry the shared Tiber CI-recovery claim; use status or sync only as needed to restore it".to_string(),
                    created_at: unix_timestamp()?,
                },
            )?;
            Err(Error::WorkflowBlocked {
                code: "tiber.ci_recovery_claim_failed",
                required_action: "retry the shared Tiber CI-recovery claim",
                source: Box::new(source),
            })
        }
    }
}

#[doc(hidden)]
pub fn claim_ci_recovery_at(
    root: impl Into<PathBuf>,
    input: CiRecoveryTrigger,
) -> Result<CiRecoveryClaim, Error> {
    GitRepository::at(root).claim_ci_recovery(input)
}

#[doc(hidden)]
pub fn ci_recovery_status_at(root: impl Into<PathBuf>) -> Result<CiRecoveryStatus, Error> {
    GitRepository::at(root).ci_recovery_status()
}

/// Reads only whether the shared CI-recovery authority currently blocks
/// delivery. Unlike `ci_recovery_status_at`, an absent incident is a normal
/// `false` result rather than an error; store and replay failures remain typed
/// errors so callers cannot silently fail open.
pub fn ci_recovery_hold_at(root: impl AsRef<Path>) -> Result<bool, Error> {
    Ok(load_tiber_projection(root.as_ref())?
        .ci_recovery
        .is_some_and(|state| state.state != CiRecoveryPhase::Resolved))
}

pub fn assert_ci_recovery_owner(
    incident_id: &str,
    epoch: u64,
) -> Result<CiRecoveryAssertion, Error> {
    let repo = GitRepository::discover()?;
    repo.assert_ci_recovery_owner(incident_id, epoch)
}

pub fn transfer_ci_recovery(
    incident_id: &str,
    epoch: u64,
    to_host: &str,
    to_session: &str,
) -> Result<CiRecoveryTransfer, Error> {
    let repo = GitRepository::discover()?;
    repo.transfer_ci_recovery(incident_id, epoch, to_host, to_session)
}

pub fn takeover_ci_recovery(incident_id: &str, epoch: u64) -> Result<CiRecoveryTransfer, Error> {
    let repo = GitRepository::discover()?;
    repo.takeover_ci_recovery(incident_id, epoch)
}

pub fn assign_ci_recovery(
    incident_id: &str,
    epoch: u64,
    input: CiRecoveryAssignmentInput,
) -> Result<CiRecoveryAssignmentResult, Error> {
    let repo = GitRepository::discover()?;
    repo.assign_ci_recovery(incident_id, epoch, input)
}

pub fn report_ci_recovery(
    incident_id: &str,
    assignment_id: &str,
    summary: &str,
    evidence: &str,
) -> Result<CiRecoveryAssignmentResult, Error> {
    let repo = GitRepository::discover()?;
    repo.report_ci_recovery(incident_id, assignment_id, summary, evidence)
}

pub fn heartbeat_ci_recovery(incident_id: &str, epoch: u64) -> Result<CiRecoveryAssertion, Error> {
    let repo = GitRepository::discover()?;
    repo.heartbeat_ci_recovery(incident_id, epoch)
}

pub fn wait_for_ci_recovery(
    incident_id: &str,
    epoch: u64,
    timeout_seconds: u64,
) -> Result<CiRecoveryWait, Error> {
    let repo = GitRepository::discover()?;
    repo.wait_for_ci_recovery(incident_id, epoch, timeout_seconds)
}

pub fn diagnose_ci_recovery(
    incident_id: &str,
    epoch: u64,
    record: CiRecoveryDiagnosisInput,
) -> Result<CiRecoveryStatus, Error> {
    let repo = GitRepository::discover()?;
    repo.diagnose_ci_recovery(incident_id, epoch, record)
}

pub fn choose_ci_recovery_action(
    incident_id: &str,
    epoch: u64,
    kind: &str,
    description: &str,
) -> Result<CiRecoveryStatus, Error> {
    let repo = GitRepository::discover()?;
    repo.choose_ci_recovery_action(incident_id, epoch, kind, description)
}

pub fn record_ci_recovery_replacement(
    incident_id: &str,
    epoch: u64,
    replacement: CiRecoveryReplacementInput,
) -> Result<CiRecoveryStatus, Error> {
    let repo = GitRepository::discover()?;
    repo.record_ci_recovery_replacement(incident_id, epoch, replacement)
}

pub fn resolve_ci_recovery(
    incident_id: &str,
    proof: CiRecoveryReleaseInput,
) -> Result<CiRecoveryStatus, Error> {
    let repo = GitRepository::discover()?;
    repo.resolve_ci_recovery(incident_id, proof)
}

pub fn ci_recovery_status() -> Result<CiRecoveryStatus, Error> {
    let repo = GitRepository::discover()?;
    repo.ci_recovery_status()
}

pub fn create_task_at(root: impl Into<PathBuf>, title: &str) -> Result<TaskPath, Error> {
    let repo = GitRepository::at(root);
    execute_create_task(&repo.root, TaskTitle::parse(title)?)
}

pub fn list_tasks_at(root: impl Into<PathBuf>) -> Result<Vec<TaskSummary>, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_snapshot_workspace(|repo| repo.list_tasks())
}

pub fn list_tasks_by_status_at(
    root: impl Into<PathBuf>,
    status: &str,
) -> Result<Vec<TaskSummary>, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_snapshot_workspace(|repo| repo.list_tasks_by_status(status))
}

pub fn search_tasks_at(
    root: impl Into<PathBuf>,
    query: &str,
) -> Result<Vec<TaskSearchResult>, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_snapshot_workspace(|repo| repo.search_tasks(query))
}

pub fn show_task_at(root: impl Into<PathBuf>, task_ref: &str) -> Result<String, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_snapshot_workspace(|repo| repo.show_task(task_ref))
}

pub fn task_metadata_at(root: impl Into<PathBuf>, task_ref: &str) -> Result<TaskMetadata, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_snapshot_workspace(|repo| repo.task_metadata(task_ref))
}

pub fn prioritize_before_at(
    root: impl Into<PathBuf>,
    task_ref: &str,
    before_ref: &str,
) -> Result<(), Error> {
    let repo = GitRepository::at(root);
    execute_prioritize_task(&repo.root, task_ref, before_ref)
}

#[doc(hidden)]
pub fn transition_task_at(
    root: impl Into<PathBuf>,
    task_ref: &str,
    status: &str,
) -> Result<TaskPath, Error> {
    let repo = GitRepository::at(root);
    execute_transition_task(&repo.root, task_ref, status)
}

#[doc(hidden)]
pub fn link_blocks_at(root: impl Into<PathBuf>, from_ref: &str, to_ref: &str) -> Result<(), Error> {
    let repo = GitRepository::at(root);
    execute_link_blocks(&repo.root, from_ref, to_ref)
}

#[doc(hidden)]
pub fn update_task_at(
    root: impl Into<PathBuf>,
    task_ref: &str,
    update: TaskUpdate<'_>,
) -> Result<(), Error> {
    let repo = GitRepository::at(root);
    execute_update_task(&repo.root, task_ref, update)
}

pub fn task_documents_at(root: impl Into<PathBuf>) -> Result<Vec<TaskDocument>, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_snapshot_workspace(|repo| repo.task_documents_snapshot())
}

pub fn list_docs_at(root: impl Into<PathBuf>) -> Result<Vec<String>, Error> {
    let repo = GitRepository::at(root);
    repo.list_docs()
}

pub fn read_doc_at(root: impl Into<PathBuf>, doc_ref: &str) -> Result<String, Error> {
    let repo = GitRepository::at(root);
    repo.read_doc(doc_ref)
}

impl GitRepository {
    fn init_repository(&self) -> Result<(), Error> {
        let _lock = self.acquire_lock()?;
        let projection = load_tiber_projection(&self.root)?;
        if !projection.initialized {
            execute_initialize_tiber_repository(&self.root)?;
        }
        Ok(())
    }

    fn claim_ci_recovery(&self, input: CiRecoveryTrigger) -> Result<CiRecoveryClaim, Error> {
        let _lock = self.acquire_lock()?;
        let participant = ci_recovery_participant()?;
        let input = CiRecoveryTrigger {
            run_id: required_ci_recovery_text("run_id", &input.run_id)?,
            run_url: required_ci_recovery_text("run_url", &input.run_url)?,
            failed_sha: required_ci_recovery_text("failed_sha", &input.failed_sha)?,
            workflow: required_ci_recovery_text("workflow", &input.workflow)?,
            git_ref: required_ci_recovery_text("git_ref", &input.git_ref)?,
        };
        let claim_time = unix_timestamp()?;

        for attempt in 1..=MAX_SYNC_ATTEMPTS {
            let remote_parent = self.fetch_coordination_branch()?;
            let active_state = self.read_active_ci_recovery(remote_parent.as_deref())?;
            if remote_parent.is_some() && active_state.is_none() {
                return Err(Error::Parse(
                    "ci_recovery_coordination_ref_invalid active_json=missing mutation=false"
                        .to_string(),
                ));
            }
            if let Some(mut state) = active_state {
                if state.state != CiRecoveryPhase::Resolved {
                    let role = if state.owner == participant {
                        CiRecoveryRole::Owner
                    } else {
                        CiRecoveryRole::Waiting
                    };
                    let mut changed = false;
                    if state.triggers.is_empty() {
                        state.triggers.push(state.trigger.clone());
                    }
                    let join_intent = JoinCiRecoveryIntent {
                        trigger: (!state.triggers.contains(&input)).then(|| input.clone()),
                        participant: (!state.participants.contains(&participant))
                            .then(|| participant.clone()),
                    };
                    if !state.triggers.contains(&input) {
                        let matches_failed_replacement =
                            state.replacement.as_ref().is_some_and(|replacement| {
                                replacement.status == CiRecoveryReplacementStatus::Failed
                                    && replacement.run_id == input.run_id
                                    && replacement.run_url == input.run_url
                                    && replacement.sha == input.failed_sha
                            });
                        if !matches_failed_replacement {
                            return Err(Error::Parse(format!(
                                "ci_recovery_distinct_trigger_requires_separate_incident active_incident_id={}",
                                state.incident_id
                            )));
                        }
                        state.trigger = input.clone();
                        state.triggers.push(input.clone());
                        changed = true;
                    }
                    if !state.participants.contains(&participant) {
                        state.participants.push(participant.clone());
                        changed = true;
                    }
                    if !changed {
                        return Ok(CiRecoveryClaim::from_state(state, role));
                    }
                    match execute_join_ci_recovery(&self.root, join_intent) {
                        Ok(()) => {
                            return Ok(CiRecoveryClaim::from_state(state, role));
                        }
                        Err(error)
                            if is_retryable_push_failure(&error) && attempt < MAX_SYNC_ATTEMPTS =>
                        {
                            continue;
                        }
                        Err(error) => return Err(error),
                    }
                }
            }

            let state = CiRecoveryState {
                schema_version: 1,
                incident_id: ci_recovery_incident_id(&input.run_id),
                state: CiRecoveryPhase::Diagnosing,
                epoch: 1,
                trigger: input.clone(),
                triggers: vec![input.clone()],
                owner: participant.clone(),
                lease_expires_at: claim_time.saturating_add(CI_RECOVERY_LEASE_SECONDS),
                participants: vec![participant.clone()],
                assignments: Vec::new(),
                failure_record: None,
                diagnosis: None,
                next_action: None,
                replacement: None,
                release_proof: None,
            };
            let intent = ClaimCiRecoveryIntent {
                incident_id: state.incident_id.clone(),
                schema_version: state.schema_version,
                trigger: state.trigger.clone(),
                owner: state.owner.clone(),
                lease_expires_at: state.lease_expires_at,
            };
            match execute_claim_ci_recovery(&self.root, intent) {
                Ok(()) => {
                    return Ok(CiRecoveryClaim::from_state(state, CiRecoveryRole::Owner));
                }
                Err(error)
                    if (is_retryable_push_failure(&error)
                        || is_coordination_branch_creation_race(&error))
                        && attempt < MAX_SYNC_ATTEMPTS =>
                {
                    continue;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("CI recovery claim attempts always return")
    }

    fn assert_ci_recovery_owner(
        &self,
        incident_id: &str,
        epoch: u64,
    ) -> Result<CiRecoveryAssertion, Error> {
        let _lock = self.acquire_lock()?;
        let participant = ci_recovery_participant()?;
        let coordination_ref = self
            .fetch_coordination_branch()?
            .ok_or_else(|| Error::Parse("ci_recovery_incident_missing active=false".to_string()))?;
        let state = self
            .read_active_ci_recovery(Some(&coordination_ref))?
            .ok_or_else(|| Error::Parse("ci_recovery_incident_missing active=false".to_string()))?;
        if state.incident_id != incident_id {
            return Err(Error::Parse(format!(
                "ci_recovery_incident_mismatch expected={} actual={incident_id}",
                state.incident_id
            )));
        }
        if state.epoch != epoch {
            return Err(Error::Parse(format!(
                "ci_recovery_stale_epoch expected={} actual={epoch}",
                state.epoch
            )));
        }
        if state.owner != participant {
            return Err(Error::Parse(format!(
                "ci_recovery_not_owner incident_id={} epoch={}",
                state.incident_id, state.epoch
            )));
        }
        ensure_ci_recovery_lease_active(&state, unix_timestamp()?)?;
        Ok(CiRecoveryAssertion {
            allowed: true,
            incident_id: state.incident_id,
            epoch: state.epoch,
            lease_expires_at: state.lease_expires_at,
        })
    }

    fn transfer_ci_recovery(
        &self,
        incident_id: &str,
        epoch: u64,
        to_host: &str,
        to_session: &str,
    ) -> Result<CiRecoveryTransfer, Error> {
        let _lock = self.acquire_lock()?;
        let caller = ci_recovery_participant()?;
        let recipient = ci_recovery_participant_from(to_host, to_session)?;
        let now = unix_timestamp()?;
        let lease_expires_at = now.saturating_add(CI_RECOVERY_LEASE_SECONDS);
        let intent = TransferCiRecoveryIntent {
            incident_id: incident_id.to_string(),
            expected_epoch: epoch,
            caller,
            recipient,
            observed_at: now,
            lease_expires_at,
        };
        execute_transfer_ci_recovery(&self.root, intent)?;
        Ok(CiRecoveryTransfer {
            incident_id: incident_id.to_string(),
            epoch: epoch.saturating_add(1),
            lease_expires_at,
        })
    }

    fn takeover_ci_recovery(
        &self,
        incident_id: &str,
        epoch: u64,
    ) -> Result<CiRecoveryTransfer, Error> {
        let _lock = self.acquire_lock()?;
        let successor = ci_recovery_participant()?;
        let now = unix_timestamp()?;
        let lease_expires_at = now.saturating_add(CI_RECOVERY_LEASE_SECONDS);
        let intent = TakeOverCiRecoveryIntent {
            incident_id: incident_id.to_string(),
            expected_epoch: epoch,
            successor,
            observed_at: now,
            lease_expires_at,
        };
        execute_takeover_ci_recovery(&self.root, intent)?;
        Ok(CiRecoveryTransfer {
            incident_id: incident_id.to_string(),
            epoch: epoch.saturating_add(1),
            lease_expires_at,
        })
    }

    fn assign_ci_recovery(
        &self,
        incident_id: &str,
        epoch: u64,
        input: CiRecoveryAssignmentInput,
    ) -> Result<CiRecoveryAssignmentResult, Error> {
        let caller = ci_recovery_participant()?;
        let assignee = ci_recovery_participant_from(&input.to_host, &input.to_session)?;
        let capabilities = input
            .capabilities
            .split(',')
            .map(str::trim)
            .filter(|capability| !capability.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        if capabilities.is_empty()
            || capabilities.iter().any(|capability| {
                !matches!(
                    capability.as_str(),
                    "inspect" | "reproduce" | "edit" | "test"
                )
            })
        {
            return Err(Error::Parse(
                "ci_recovery_capability_invalid allowed=inspect,reproduce,edit,test".to_string(),
            ));
        }
        let scope = required_ci_recovery_text("assignment_scope", &input.scope)?;
        let now = unix_timestamp()?;
        let intent = AssignCiRecoveryWorkIntent {
            incident_id: incident_id.to_string(),
            expected_epoch: epoch,
            caller,
            assignment: CiRecoveryAssignment {
                // The command's folded assignment count decides the durable
                // sequential identifier. This placeholder is never emitted.
                id: String::new(),
                owner_epoch: epoch,
                assignee,
                capabilities,
                scope,
                report: None,
            },
            observed_at: now,
        };
        let assignment_id = execute_assign_ci_recovery_work(&self.root, intent)?;
        Ok(CiRecoveryAssignmentResult {
            incident_id: incident_id.to_string(),
            assignment_id,
            epoch,
        })
    }

    fn report_ci_recovery(
        &self,
        incident_id: &str,
        assignment_id: &str,
        summary: &str,
        evidence: &str,
    ) -> Result<CiRecoveryAssignmentResult, Error> {
        let caller = ci_recovery_participant()?;
        let summary = required_ci_recovery_text("assignment_summary", summary)?;
        let evidence = required_ci_recovery_text("assignment_evidence", evidence)?;
        let epoch = load_tiber_projection(&self.root)?
            .ci_recovery
            .as_ref()
            .filter(|state| state.incident_id == incident_id)
            .map(|state| state.epoch)
            .ok_or_else(|| Error::Parse("ci_recovery_incident_missing active=false".into()))?;
        execute_report_ci_recovery_work(
            &self.root,
            ReportCiRecoveryWorkIntent {
                incident_id: incident_id.to_string(),
                assignment_id: assignment_id.to_string(),
                assignee: caller,
                report: CiRecoveryReport { summary, evidence },
            },
        )?;
        Ok(CiRecoveryAssignmentResult {
            incident_id: incident_id.to_string(),
            assignment_id: assignment_id.to_string(),
            epoch,
        })
    }

    fn heartbeat_ci_recovery(
        &self,
        incident_id: &str,
        epoch: u64,
    ) -> Result<CiRecoveryAssertion, Error> {
        let caller = ci_recovery_participant()?;
        let now = unix_timestamp()?;
        let lease_expires_at = now.saturating_add(CI_RECOVERY_LEASE_SECONDS);
        execute_renew_ci_recovery_lease(
            &self.root,
            RenewCiRecoveryLeaseIntent {
                incident_id: incident_id.to_string(),
                expected_epoch: epoch,
                owner: caller,
                observed_at: now,
                lease_expires_at,
            },
        )?;
        Ok(CiRecoveryAssertion {
            allowed: true,
            incident_id: incident_id.to_string(),
            epoch,
            lease_expires_at,
        })
    }

    fn wait_for_ci_recovery(
        &self,
        incident_id: &str,
        epoch: u64,
        timeout_seconds: u64,
    ) -> Result<CiRecoveryWait, Error> {
        if timeout_seconds > 60 {
            return Err(Error::Parse(
                "ci_recovery_wait_timeout_invalid maximum_seconds=60".to_string(),
            ));
        }
        let participant = ci_recovery_participant()?;
        if timeout_seconds == 0 {
            return Ok(CiRecoveryWait::timeout(incident_id, epoch));
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(timeout_seconds);
        let mut last_state = None;
        loop {
            if let Some(state) = last_state.as_ref() {
                if std::time::Instant::now() >= deadline {
                    return Ok(CiRecoveryWait::from_state(state, "timeout", None));
                }
            }
            let _lock = match self.try_acquire_task_lock_once() {
                Ok(lock) => lock,
                Err(error) if is_tiber_lock_busy(&error) => {
                    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                    if remaining.is_zero() {
                        return Ok(last_state
                            .as_ref()
                            .map(|state| CiRecoveryWait::from_state(state, "timeout", None))
                            .unwrap_or_else(|| CiRecoveryWait::timeout(incident_id, epoch)));
                    }
                    std::thread::sleep(Duration::from_millis(50).min(remaining));
                    continue;
                }
                Err(error) => return Err(error),
            };
            let state = {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                if remaining.is_zero() {
                    return Ok(last_state
                        .as_ref()
                        .map(|state| CiRecoveryWait::from_state(state, "timeout", None))
                        .unwrap_or_else(|| CiRecoveryWait::timeout(incident_id, epoch)));
                }
                let parent = self
                    .fetch_coordination_branch_with_timeout(remaining.min(Duration::from_secs(10)))?
                    .ok_or_else(|| {
                        Error::Parse("ci_recovery_incident_missing active=false".to_string())
                    })?;
                self.read_active_ci_recovery(Some(&parent))?
                    .ok_or_else(|| {
                        Error::Parse("ci_recovery_incident_missing active=false".to_string())
                    })?
            };
            if state.incident_id != incident_id {
                return Err(Error::Parse(format!(
                    "ci_recovery_incident_mismatch expected={} actual={incident_id}",
                    state.incident_id
                )));
            }
            if !state.participants.contains(&participant) {
                return Err(Error::Parse(format!(
                    "ci_recovery_participant_required incident_id={incident_id}"
                )));
            }
            if state.epoch != epoch {
                return Ok(CiRecoveryWait::from_state(&state, "epoch-changed", None));
            }
            if state.state == CiRecoveryPhase::Resolved {
                return Ok(CiRecoveryWait::from_state(&state, "resolved", None));
            }
            if let Some(assignment) = state.assignments.iter().find(|assignment| {
                assignment.owner_epoch == state.epoch && assignment.assignee == participant
            }) {
                return Ok(CiRecoveryWait::from_state(
                    &state,
                    "assignment",
                    Some(assignment.id.clone()),
                ));
            }
            if std::time::Instant::now() >= deadline {
                return Ok(CiRecoveryWait::from_state(&state, "timeout", None));
            }
            last_state = Some(state);
            drop(_lock);
            ci_recovery_signal_wait_ready()?;
            std::thread::sleep(
                Duration::from_millis(250)
                    .min(deadline.saturating_duration_since(std::time::Instant::now())),
            );
        }
    }

    fn diagnose_ci_recovery(
        &self,
        incident_id: &str,
        epoch: u64,
        record: CiRecoveryDiagnosisInput,
    ) -> Result<CiRecoveryStatus, Error> {
        let caller = ci_recovery_participant()?;
        let classification = CiRecoveryClassification::parse(&record.classification)?;
        let job = required_ci_recovery_text("job", &record.job)?;
        let step = required_ci_recovery_text("step", &record.step)?;
        let log_evidence = required_ci_recovery_text("log_evidence", &record.log_evidence)?;
        let cause = required_ci_recovery_text("cause", &record.cause)?;
        execute_record_ci_recovery_diagnosis(
            &self.root,
            RecordCiRecoveryDiagnosisIntent {
                incident_id: incident_id.to_string(),
                expected_epoch: epoch,
                owner: caller,
                observed_at: unix_timestamp()?,
                failure_record: CiRecoveryFailureRecord {
                    job: job.clone(),
                    step: step.clone(),
                    log_evidence: log_evidence.clone(),
                },
                diagnosis: CiRecoveryDiagnosis {
                    cause,
                    classification,
                },
            },
        )?;
        load_tiber_projection(&self.root)?
            .ci_recovery
            .as_ref()
            .filter(|state| state.incident_id == incident_id)
            .map(CiRecoveryStatus::from_state)
            .ok_or_else(|| Error::Parse("ci_recovery_incident_missing active=false".into()))
    }

    fn choose_ci_recovery_action(
        &self,
        incident_id: &str,
        epoch: u64,
        kind: &str,
        description: &str,
    ) -> Result<CiRecoveryStatus, Error> {
        let caller = ci_recovery_participant()?;
        let kind = CiRecoveryActionKind::parse(kind)?;
        let description = required_ci_recovery_text("description", description)?;
        execute_select_ci_recovery_action(
            &self.root,
            SelectCiRecoveryActionIntent {
                incident_id: incident_id.to_string(),
                expected_epoch: epoch,
                owner: caller,
                observed_at: unix_timestamp()?,
                action: CiRecoveryAction { kind, description },
            },
        )?;
        load_tiber_projection(&self.root)?
            .ci_recovery
            .as_ref()
            .filter(|state| state.incident_id == incident_id)
            .map(CiRecoveryStatus::from_state)
            .ok_or_else(|| Error::Parse("ci_recovery_incident_missing active=false".into()))
    }

    fn record_ci_recovery_replacement(
        &self,
        incident_id: &str,
        epoch: u64,
        replacement: CiRecoveryReplacementInput,
    ) -> Result<CiRecoveryStatus, Error> {
        let caller = ci_recovery_participant()?;
        let status = CiRecoveryReplacementStatus::parse(&replacement.status)?;
        let run_id = required_ci_recovery_text("replacement_run_id", &replacement.run_id)?;
        let run_url = required_ci_recovery_text("replacement_run_url", &replacement.run_url)?;
        let sha = required_ci_recovery_text("replacement_sha", &replacement.sha)?;
        execute_record_ci_recovery_replacement(
            &self.root,
            RecordCiRecoveryReplacementIntent {
                incident_id: incident_id.to_string(),
                expected_epoch: epoch,
                owner: caller,
                observed_at: unix_timestamp()?,
                replacement: CiRecoveryReplacement {
                    run_id,
                    run_url,
                    sha,
                    status,
                },
            },
        )?;
        load_tiber_projection(&self.root)?
            .ci_recovery
            .as_ref()
            .filter(|state| state.incident_id == incident_id)
            .map(CiRecoveryStatus::from_state)
            .ok_or_else(|| Error::Parse("ci_recovery_incident_missing active=false".into()))
    }

    fn resolve_ci_recovery(
        &self,
        incident_id: &str,
        proof: CiRecoveryReleaseInput,
    ) -> Result<CiRecoveryStatus, Error> {
        let participant = ci_recovery_participant()?;
        if proof.terminal_status != "success" {
            return Err(Error::Parse(format!(
                "ci_recovery_terminal_success_required actual={}",
                proof.terminal_status
            )));
        }
        let replacement_run_id =
            required_ci_recovery_text("replacement_run_id", &proof.replacement_run_id)?;
        let replacement_run_url =
            required_ci_recovery_text("replacement_run_url", &proof.replacement_run_url)?;
        let sha = required_ci_recovery_text("replacement_sha", &proof.sha)?;
        execute_resolve_ci_recovery(
            &self.root,
            ResolveCiRecoveryIntent {
                incident_id: incident_id.to_string(),
                participant,
                proof: CiRecoveryReleaseProof {
                    replacement_run_id,
                    replacement_run_url,
                    sha,
                    terminal_status: "success".to_string(),
                },
            },
        )?;
        load_tiber_projection(&self.root)?
            .ci_recovery
            .as_ref()
            .filter(|state| state.incident_id == incident_id)
            .map(CiRecoveryStatus::from_state)
            .ok_or_else(|| Error::Parse("ci_recovery_incident_missing active=false".into()))
    }

    fn ci_recovery_status(&self) -> Result<CiRecoveryStatus, Error> {
        let _lock = self.acquire_lock()?;
        let parent = self
            .fetch_coordination_branch()?
            .ok_or_else(|| Error::Parse("ci_recovery_incident_missing active=false".to_string()))?;
        let state = self
            .read_active_ci_recovery(Some(&parent))?
            .ok_or_else(|| Error::Parse("ci_recovery_incident_missing active=false".to_string()))?;
        Ok(CiRecoveryStatus::from_state(&state))
    }

    fn fetch_coordination_branch(&self) -> Result<Option<String>, Error> {
        self.fetch_coordination_branch_with_timeout(Duration::from_secs(10))
    }

    fn fetch_coordination_branch_with_timeout(
        &self,
        _timeout: Duration,
    ) -> Result<Option<String>, Error> {
        if git_status(["remote", "get-url", "origin"], Some(&self.root)).is_err() {
            return Err(Error::Parse(
                "ci_recovery_remote_required remote=origin".to_string(),
            ));
        }
        let projection = load_tiber_projection(&self.root)?;
        Ok(projection
            .ci_recovery
            .as_ref()
            .map(|state| state.incident_id.clone()))
    }

    fn read_active_ci_recovery(
        &self,
        coordination_ref: Option<&str>,
    ) -> Result<Option<CiRecoveryState>, Error> {
        let Some(_coordination_ref) = coordination_ref else {
            return Ok(None);
        };
        Ok(load_tiber_projection(&self.root)?.ci_recovery)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CiRecoveryTrigger {
    pub run_id: String,
    pub run_url: String,
    pub failed_sha: String,
    pub workflow: String,
    pub git_ref: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CiRecoveryParticipant {
    pub host: String,
    pub session: String,
}

/// Closed lifecycle vocabulary folded by the repository-wide CI-recovery commands.
/// The wire adapter retains the established lower-case strings, while the
/// domain and eventual EventCore fold cannot manufacture an unknown phase.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum CiRecoveryPhase {
    Diagnosing,
    ActionSelected,
    WaitingCi,
    Resolved,
}

impl CiRecoveryPhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Diagnosing => "diagnosing",
            Self::ActionSelected => "action-selected",
            Self::WaitingCi => "waiting-ci",
            Self::Resolved => "resolved",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CiRecoveryClassification {
    Caused,
    Unrelated,
    Transient,
}

impl CiRecoveryClassification {
    fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "caused" => Ok(Self::Caused),
            "unrelated" => Ok(Self::Unrelated),
            "transient" => Ok(Self::Transient),
            _ => Err(Error::Parse(format!(
                "ci_recovery_choice_invalid field=classification value={value} allowed=caused,unrelated,transient"
            ))),
        }
    }
}

impl std::fmt::Display for CiRecoveryClassification {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Caused => "caused",
            Self::Unrelated => "unrelated",
            Self::Transient => "transient",
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CiRecoveryActionKind {
    Repair,
    Rerun,
}

impl CiRecoveryActionKind {
    fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "repair" => Ok(Self::Repair),
            "rerun" => Ok(Self::Rerun),
            _ => Err(Error::Parse(format!(
                "ci_recovery_choice_invalid field=action value={value} allowed=repair,rerun"
            ))),
        }
    }
}

impl std::fmt::Display for CiRecoveryActionKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Repair => "repair",
            Self::Rerun => "rerun",
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CiRecoveryReplacementStatus {
    Queued,
    Running,
    Failed,
}

impl CiRecoveryReplacementStatus {
    fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "failed" => Ok(Self::Failed),
            _ => Err(Error::Parse(format!(
                "ci_recovery_choice_invalid field=replacement_status value={value} allowed=queued,running,failed"
            ))),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CiRecoveryState {
    schema_version: u32,
    incident_id: String,
    state: CiRecoveryPhase,
    epoch: u64,
    trigger: CiRecoveryTrigger,
    #[serde(default)]
    triggers: Vec<CiRecoveryTrigger>,
    owner: CiRecoveryParticipant,
    lease_expires_at: u64,
    #[serde(default)]
    participants: Vec<CiRecoveryParticipant>,
    #[serde(default)]
    assignments: Vec<CiRecoveryAssignment>,
    #[serde(default)]
    failure_record: Option<CiRecoveryFailureRecord>,
    #[serde(default)]
    diagnosis: Option<CiRecoveryDiagnosis>,
    #[serde(default)]
    next_action: Option<CiRecoveryAction>,
    #[serde(default)]
    replacement: Option<CiRecoveryReplacement>,
    #[serde(default)]
    release_proof: Option<CiRecoveryReleaseProof>,
}

impl CiRecoveryState {
    #[cfg(test)]
    fn snapshot(&self) -> tiber_core::events::CiRecoverySnapshot {
        use tiber_core::events as core;
        core::CiRecoverySnapshot {
            schema_version: self.schema_version,
            incident_id: self.incident_id.clone(),
            state: self.state.into(),
            epoch: self.epoch,
            trigger: self.trigger.clone().into(),
            triggers: self.triggers.iter().cloned().map(Into::into).collect(),
            owner: self.owner.clone().into(),
            lease_expires_at: self.lease_expires_at,
            participants: self.participants.iter().cloned().map(Into::into).collect(),
            assignments: self.assignments.iter().cloned().map(Into::into).collect(),
            failure_record: self.failure_record.clone().map(Into::into),
            diagnosis: self.diagnosis.clone().map(Into::into),
            next_action: self.next_action.clone().map(Into::into),
            replacement: self.replacement.clone().map(Into::into),
            release_proof: self.release_proof.clone().map(Into::into),
        }
    }

    fn from_snapshot(value: &tiber_core::events::CiRecoverySnapshot) -> Self {
        Self {
            schema_version: value.schema_version,
            incident_id: value.incident_id.clone(),
            state: value.state.into(),
            epoch: value.epoch,
            trigger: value.trigger.clone().into(),
            triggers: value.triggers.iter().cloned().map(Into::into).collect(),
            owner: value.owner.clone().into(),
            lease_expires_at: value.lease_expires_at,
            participants: value.participants.iter().cloned().map(Into::into).collect(),
            assignments: value.assignments.iter().cloned().map(Into::into).collect(),
            failure_record: value.failure_record.clone().map(Into::into),
            diagnosis: value.diagnosis.clone().map(Into::into),
            next_action: value.next_action.clone().map(Into::into),
            replacement: value.replacement.clone().map(Into::into),
            release_proof: value.release_proof.clone().map(Into::into),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiRecoveryFailureRecord {
    pub job: String,
    pub step: String,
    pub log_evidence: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiRecoveryDiagnosis {
    pub cause: String,
    pub classification: CiRecoveryClassification,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiRecoveryAction {
    pub kind: CiRecoveryActionKind,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiRecoveryReplacement {
    pub run_id: String,
    pub run_url: String,
    pub sha: String,
    pub status: CiRecoveryReplacementStatus,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiRecoveryReleaseProof {
    pub replacement_run_id: String,
    pub replacement_run_url: String,
    pub sha: String,
    pub terminal_status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiRecoveryAssignment {
    pub id: String,
    pub owner_epoch: u64,
    pub assignee: CiRecoveryParticipant,
    pub capabilities: Vec<String>,
    pub scope: String,
    pub report: Option<CiRecoveryReport>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiRecoveryReport {
    pub summary: String,
    pub evidence: String,
}

macro_rules! ci_recovery_enum_conversion {
    ($local:ty, $core:path, { $($local_variant:ident => $core_variant:ident),+ $(,)? }) => {
        impl From<$local> for $core {
            fn from(value: $local) -> Self {
                match value { $(<$local>::$local_variant => <$core>::$core_variant),+ }
            }
        }
        impl From<$core> for $local {
            fn from(value: $core) -> Self {
                match value { $(<$core>::$core_variant => <$local>::$local_variant),+ }
            }
        }
    };
}

ci_recovery_enum_conversion!(
    CiRecoveryPhase,
    tiber_core::events::CiRecoveryPhase,
    { Diagnosing => Diagnosing, ActionSelected => ActionSelected, WaitingCi => WaitingCi, Resolved => Resolved }
);
ci_recovery_enum_conversion!(
    CiRecoveryClassification,
    tiber_core::events::CiRecoveryClassification,
    { Caused => Caused, Unrelated => Unrelated, Transient => Transient }
);
ci_recovery_enum_conversion!(
    CiRecoveryActionKind,
    tiber_core::events::CiRecoveryActionKind,
    { Repair => Repair, Rerun => Rerun }
);
ci_recovery_enum_conversion!(
    CiRecoveryReplacementStatus,
    tiber_core::events::CiRecoveryReplacementStatus,
    { Queued => Queued, Running => Running, Failed => Failed }
);

impl From<CiRecoveryTrigger> for tiber_core::events::CiRecoveryTrigger {
    fn from(value: CiRecoveryTrigger) -> Self {
        Self {
            run_id: value.run_id,
            run_url: value.run_url,
            failed_sha: value.failed_sha,
            workflow: value.workflow,
            git_ref: value.git_ref,
        }
    }
}
impl From<tiber_core::events::CiRecoveryTrigger> for CiRecoveryTrigger {
    fn from(value: tiber_core::events::CiRecoveryTrigger) -> Self {
        Self {
            run_id: value.run_id,
            run_url: value.run_url,
            failed_sha: value.failed_sha,
            workflow: value.workflow,
            git_ref: value.git_ref,
        }
    }
}
impl From<CiRecoveryParticipant> for tiber_core::events::CiRecoveryParticipant {
    fn from(value: CiRecoveryParticipant) -> Self {
        Self {
            host: value.host,
            session: value.session,
        }
    }
}
impl From<tiber_core::events::CiRecoveryParticipant> for CiRecoveryParticipant {
    fn from(value: tiber_core::events::CiRecoveryParticipant) -> Self {
        Self {
            host: value.host,
            session: value.session,
        }
    }
}
impl From<CiRecoveryReport> for tiber_core::events::CiRecoveryReport {
    fn from(value: CiRecoveryReport) -> Self {
        Self {
            summary: value.summary,
            evidence: value.evidence,
        }
    }
}
impl From<tiber_core::events::CiRecoveryReport> for CiRecoveryReport {
    fn from(value: tiber_core::events::CiRecoveryReport) -> Self {
        Self {
            summary: value.summary,
            evidence: value.evidence,
        }
    }
}
impl From<CiRecoveryAssignment> for tiber_core::events::CiRecoveryAssignment {
    fn from(value: CiRecoveryAssignment) -> Self {
        Self {
            id: value.id,
            owner_epoch: value.owner_epoch,
            assignee: value.assignee.into(),
            capabilities: value.capabilities,
            scope: value.scope,
            report: value.report.map(Into::into),
        }
    }
}
impl From<tiber_core::events::CiRecoveryAssignment> for CiRecoveryAssignment {
    fn from(value: tiber_core::events::CiRecoveryAssignment) -> Self {
        Self {
            id: value.id,
            owner_epoch: value.owner_epoch,
            assignee: value.assignee.into(),
            capabilities: value.capabilities,
            scope: value.scope,
            report: value.report.map(Into::into),
        }
    }
}
impl From<CiRecoveryFailureRecord> for tiber_core::events::CiRecoveryFailureRecord {
    fn from(value: CiRecoveryFailureRecord) -> Self {
        Self {
            job: value.job,
            step: value.step,
            log_evidence: value.log_evidence,
        }
    }
}
impl From<tiber_core::events::CiRecoveryFailureRecord> for CiRecoveryFailureRecord {
    fn from(value: tiber_core::events::CiRecoveryFailureRecord) -> Self {
        Self {
            job: value.job,
            step: value.step,
            log_evidence: value.log_evidence,
        }
    }
}
impl From<CiRecoveryDiagnosis> for tiber_core::events::CiRecoveryDiagnosis {
    fn from(value: CiRecoveryDiagnosis) -> Self {
        Self {
            cause: value.cause,
            classification: value.classification.into(),
        }
    }
}
impl From<tiber_core::events::CiRecoveryDiagnosis> for CiRecoveryDiagnosis {
    fn from(value: tiber_core::events::CiRecoveryDiagnosis) -> Self {
        Self {
            cause: value.cause,
            classification: value.classification.into(),
        }
    }
}
impl From<CiRecoveryAction> for tiber_core::events::CiRecoveryAction {
    fn from(value: CiRecoveryAction) -> Self {
        Self {
            kind: value.kind.into(),
            description: value.description,
        }
    }
}
impl From<tiber_core::events::CiRecoveryAction> for CiRecoveryAction {
    fn from(value: tiber_core::events::CiRecoveryAction) -> Self {
        Self {
            kind: value.kind.into(),
            description: value.description,
        }
    }
}
impl From<CiRecoveryReplacement> for tiber_core::events::CiRecoveryReplacement {
    fn from(value: CiRecoveryReplacement) -> Self {
        Self {
            run_id: value.run_id,
            run_url: value.run_url,
            sha: value.sha,
            status: value.status.into(),
        }
    }
}
impl From<tiber_core::events::CiRecoveryReplacement> for CiRecoveryReplacement {
    fn from(value: tiber_core::events::CiRecoveryReplacement) -> Self {
        Self {
            run_id: value.run_id,
            run_url: value.run_url,
            sha: value.sha,
            status: value.status.into(),
        }
    }
}
impl From<CiRecoveryReleaseProof> for tiber_core::events::CiRecoveryReleaseProof {
    fn from(value: CiRecoveryReleaseProof) -> Self {
        Self {
            replacement_run_id: value.replacement_run_id,
            replacement_run_url: value.replacement_run_url,
            sha: value.sha,
            terminal_status: value.terminal_status,
        }
    }
}
impl From<tiber_core::events::CiRecoveryReleaseProof> for CiRecoveryReleaseProof {
    fn from(value: tiber_core::events::CiRecoveryReleaseProof) -> Self {
        Self {
            replacement_run_id: value.replacement_run_id,
            replacement_run_url: value.replacement_run_url,
            sha: value.sha,
            terminal_status: value.terminal_status,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CiRecoveryAssignmentInput {
    pub to_host: String,
    pub to_session: String,
    pub capabilities: String,
    pub scope: String,
}

#[derive(Clone, Debug)]
pub struct CiRecoveryDiagnosisInput {
    pub job: String,
    pub step: String,
    pub log_evidence: String,
    pub cause: String,
    pub classification: String,
}

#[derive(Clone, Debug)]
pub struct CiRecoveryReplacementInput {
    pub run_id: String,
    pub run_url: String,
    pub sha: String,
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct CiRecoveryReleaseInput {
    pub replacement_run_id: String,
    pub replacement_run_url: String,
    pub sha: String,
    pub terminal_status: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CiRecoveryRole {
    Owner,
    Waiting,
}

#[derive(Clone, Debug, Serialize)]
pub struct CiRecoveryClaim {
    pub incident_id: String,
    pub state: String,
    pub role: CiRecoveryRole,
    pub epoch: u64,
    pub lease_expires_at: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CiRecoveryAssertion {
    pub allowed: bool,
    pub incident_id: String,
    pub epoch: u64,
    pub lease_expires_at: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CiRecoveryTransfer {
    pub incident_id: String,
    pub epoch: u64,
    pub lease_expires_at: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CiRecoveryAssignmentResult {
    pub incident_id: String,
    pub assignment_id: String,
    pub epoch: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CiRecoveryStatus {
    pub schema_version: u32,
    pub incident_id: String,
    pub state: String,
    pub epoch: u64,
    pub lease_expires_at: u64,
    pub hold_released: bool,
    pub trigger_count: usize,
    pub trigger: CiRecoveryTrigger,
    pub triggers: Vec<CiRecoveryTrigger>,
    pub owner: CiRecoveryParticipant,
    pub participants: Vec<CiRecoveryParticipant>,
    pub assignments: Vec<CiRecoveryAssignment>,
    pub failure_record: Option<CiRecoveryFailureRecord>,
    pub diagnosis: Option<CiRecoveryDiagnosis>,
    pub next_action: Option<CiRecoveryAction>,
    pub replacement: Option<CiRecoveryReplacement>,
    pub release_proof: Option<CiRecoveryReleaseProof>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CiRecoveryWait {
    pub incident_id: String,
    pub state: String,
    pub epoch: u64,
    pub wake_reason: String,
    pub assignment_id: Option<String>,
}

impl CiRecoveryWait {
    fn timeout(incident_id: &str, epoch: u64) -> Self {
        Self {
            incident_id: incident_id.to_string(),
            state: "unknown".to_string(),
            epoch,
            wake_reason: "timeout".to_string(),
            assignment_id: None,
        }
    }

    fn from_state(
        state: &CiRecoveryState,
        wake_reason: &str,
        assignment_id: Option<String>,
    ) -> Self {
        Self {
            incident_id: state.incident_id.clone(),
            state: state.state.as_str().to_string(),
            epoch: state.epoch,
            wake_reason: wake_reason.to_string(),
            assignment_id,
        }
    }
}

impl CiRecoveryStatus {
    fn from_state(state: &CiRecoveryState) -> Self {
        Self {
            schema_version: state.schema_version,
            incident_id: state.incident_id.clone(),
            state: state.state.as_str().to_string(),
            epoch: state.epoch,
            lease_expires_at: state.lease_expires_at,
            hold_released: state.state == CiRecoveryPhase::Resolved,
            trigger_count: if state.triggers.is_empty() {
                1
            } else {
                state.triggers.len()
            },
            trigger: state.trigger.clone(),
            triggers: state.triggers.clone(),
            owner: state.owner.clone(),
            participants: state.participants.clone(),
            assignments: state.assignments.clone(),
            failure_record: state.failure_record.clone(),
            diagnosis: state.diagnosis.clone(),
            next_action: state.next_action.clone(),
            replacement: state.replacement.clone(),
            release_proof: state.release_proof.clone(),
        }
    }
}

impl CiRecoveryClaim {
    fn from_state(state: CiRecoveryState, role: CiRecoveryRole) -> Self {
        Self {
            incident_id: state.incident_id,
            state: state.state.as_str().to_string(),
            role,
            epoch: state.epoch,
            lease_expires_at: state.lease_expires_at,
        }
    }
}

fn ci_recovery_incident_id(run_id: &str) -> String {
    let run_id = run_id
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .collect::<String>();
    format!("ci-{}", if run_id.is_empty() { "run" } else { &run_id })
}

fn ci_recovery_signal_wait_ready() -> Result<(), Error> {
    #[cfg(debug_assertions)]
    if let Some(path) = std::env::var_os("TIBER_CI_RECOVERY_TEST_WAIT_READY") {
        std::fs::write(path, b"ready").map_err(Error::Io)?;
    }
    Ok(())
}

fn unix_timestamp() -> Result<u64, Error> {
    #[cfg(debug_assertions)]
    if let Ok(value) = std::env::var("TIBER_CI_RECOVERY_TEST_NOW") {
        return value.parse::<u64>().map_err(|error| {
            Error::Parse(format!(
                "ci_recovery_test_clock_invalid value={value} source={error}"
            ))
        });
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| Error::Parse(format!("system_clock_before_epoch source={error}")))
}

fn ci_recovery_participant() -> Result<CiRecoveryParticipant, Error> {
    let session = std::env::var("TIBER_CLAIM_SESSION")
        .or_else(|_| std::env::var("CODEX_SESSION_ID"))
        .or_else(|_| std::env::var("CLAUDE_SESSION_ID"))
        .or_else(|_| MCP_CI_RECOVERY_SESSION.with(|slot| slot.borrow().clone().ok_or(std::env::VarError::NotPresent)))
        .map_err(|_| {
            Error::Parse(
                "ci_recovery_session_required env=TIBER_CLAIM_SESSION|CODEX_SESSION_ID|CLAUDE_SESSION_ID"
                    .to_string(),
            )
        })?;
    let session = frontmatter_scalar_value(&session);
    if session == "unknown" {
        return Err(Error::Parse(
            "ci_recovery_session_required value=non-empty".to_string(),
        ));
    }
    Ok(CiRecoveryParticipant {
        host: claim_host(),
        session,
    })
}

fn ci_recovery_participant_from(host: &str, session: &str) -> Result<CiRecoveryParticipant, Error> {
    let host = frontmatter_scalar_value(host);
    let session = frontmatter_scalar_value(session);
    if session == "unknown" {
        return Err(Error::Parse(
            "ci_recovery_recipient_session_required value=non-empty".to_string(),
        ));
    }
    Ok(CiRecoveryParticipant { host, session })
}

fn ensure_ci_recovery_lease_active(state: &CiRecoveryState, now: u64) -> Result<(), Error> {
    if state.lease_expires_at <= now {
        return Err(Error::Parse(format!(
            "ci_recovery_lease_expired incident_id={} epoch={}",
            state.incident_id, state.epoch
        )));
    }
    Ok(())
}

fn required_ci_recovery_text(field: &str, value: &str) -> Result<String, Error> {
    let value = value.trim();
    let lower = value.to_ascii_lowercase();
    let likely_credential = [
        "ghp_",
        "github_pat_",
        "authorization:",
        "bearer ",
        "password=",
        "token=",
        "secret=",
        "-----begin private key",
        "-----begin rsa private key",
        "-----begin openssh private key",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    if value.is_empty()
        || value.len() > CI_RECOVERY_TEXT_MAX_BYTES
        || value.chars().any(char::is_control)
        || likely_credential
    {
        return Err(Error::Parse(format!(
            "ci_recovery_field_invalid field={field}"
        )));
    }
    Ok(value.to_string())
}

pub fn sync_repository() -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.sync_repository()?;
    clear_workflow_blocker(&repo, "publication_failed")
}

/// Persist a fail-closed operational hold when the authoritative Git
/// publication boundary cannot be confirmed. The Git-backed EventStore calls
/// this before returning its publication error.
pub fn record_publication_failure() -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    record_publication_failure_for(&repo)
}

fn record_publication_failure_for(repo: &GitRepository) -> Result<(), Error> {
    record_workflow_blocker(
        repo,
        WorkflowBlocker {
            schema_version: 1,
            kind: "publication_failed".to_string(),
            error_code: "tiber.publication_failed".to_string(),
            required_action: "run Tiber sync until authoritative publication is resolved"
                .to_string(),
            created_at: unix_timestamp()?,
        },
    )
}

pub fn create_task(title: &str) -> Result<TaskPath, Error> {
    let repo = GitRepository::discover()?;
    execute_create_task(&repo.root, TaskTitle::parse(title)?)
}

pub fn list_tasks() -> Result<Vec<TaskSummary>, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_snapshot_workspace(|repo| repo.list_tasks())
}

pub fn list_tasks_by_status(status: &str) -> Result<Vec<TaskSummary>, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_snapshot_workspace(|repo| repo.list_tasks_by_status(status))
}

pub fn search_tasks(query: &str) -> Result<Vec<TaskSearchResult>, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_snapshot_workspace(|repo| repo.search_tasks(query))
}

pub fn show_task(task_ref: &str) -> Result<String, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_snapshot_workspace(|repo| repo.show_task(task_ref))
}

pub fn task_metadata(task_ref: &str) -> Result<TaskMetadata, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_snapshot_workspace(|repo| repo.task_metadata(task_ref))
}

pub fn list_docs() -> Result<Vec<String>, Error> {
    let repo = GitRepository::discover()?;
    repo.list_docs()
}

pub fn read_doc(doc_ref: &str) -> Result<String, Error> {
    let repo = GitRepository::discover()?;
    repo.read_doc(doc_ref)
}

pub fn next_task() -> Result<Option<TaskSummary>, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_snapshot_workspace(|repo| repo.next_task())
}

pub fn transition_task(task_ref: &str, status: &str) -> Result<TaskPath, Error> {
    let repo = GitRepository::discover()?;
    execute_transition_task(&repo.root, task_ref, status)
}

#[allow(clippy::too_many_arguments)]
pub fn record_final_review(
    task_ref: &str,
    review_id: &str,
    reviewer_identity: &str,
    reviewer_type: &str,
    scope: &[String],
    commit_range: &str,
    outcome: &str,
    evidence: &str,
    timestamp: &str,
    source_fingerprint: &str,
    verification_scope: &[String],
    verification_fingerprint: &str,
) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    execute_record_final_review(
        &repo.root,
        task_ref,
        review_id,
        reviewer_identity,
        reviewer_type,
        scope,
        commit_range,
        outcome,
        evidence,
        timestamp,
        source_fingerprint,
        verification_scope,
        verification_fingerprint,
    )
}

pub fn prioritize_before(task_ref: &str, before_ref: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    execute_prioritize_task(&repo.root, task_ref, before_ref)
}

pub fn link_blocks(from_ref: &str, to_ref: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    execute_link_blocks(&repo.root, from_ref, to_ref)
}

pub fn unlink_blocks(from_ref: &str, to_ref: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    execute_unlink_blocks(&repo.root, from_ref, to_ref)
}

pub fn add_subtask(task_ref: &str, title: &str, after_refs: &[String]) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    execute_add_subtask(&repo.root, task_ref, title, after_refs)
}

pub fn set_subtask_checked(task_ref: &str, index: &str, checked: bool) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    execute_set_subtask_checked(&repo.root, task_ref, index, checked)
}

pub fn update_task(task_ref: &str, update: TaskUpdate<'_>) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    execute_update_task(&repo.root, task_ref, update)
}

pub fn add_acceptance(task_ref: &str, criterion: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    execute_add_acceptance(&repo.root, task_ref, criterion)
}

pub fn set_acceptance_checked(task_ref: &str, index: &str, checked: bool) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    execute_set_acceptance_checked(&repo.root, task_ref, index, checked)
}

pub fn remove_acceptance(task_ref: &str, index: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    execute_remove_acceptance(&repo.root, task_ref, index)
}

pub fn add_note(task_ref: &str, note: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    execute_add_task_note(&repo.root, task_ref, note)
}

pub fn validate_fix() -> Result<Vec<ValidationMessage>, Error> {
    let repo = GitRepository::discover()?;
    execute_validate_task_board(&repo.root)
}

pub fn close_from_trailers() -> Result<Vec<String>, Error> {
    let repo = GitRepository::discover()?;
    execute_close_tasks_from_commit_trailers(&repo.root)
}

pub fn scaffold_repo(apply: bool, replace_conflicts: bool) -> Result<Vec<String>, Error> {
    let repo = GitRepository::discover()?;
    repo.scaffold_repo(apply, replace_conflicts)
}

pub fn install_bin(target_dir: &str, apply: bool) -> Result<String, Error> {
    let target_dir = expand_home(Path::new(target_dir))?;
    let launcher = tiber_launcher_path()?;
    let installed = target_dir.join("tiber");
    if apply {
        fs::create_dir_all(&target_dir)?;
        if installed.exists() || installed.symlink_metadata().is_ok() {
            return Err(Error::Parse(format!(
                "install_target_exists path={}",
                path_to_entry(&installed)?
            )));
        }
        install_launcher(&launcher, &installed)?;
    }
    Ok(format!("{} -> {}", installed.display(), launcher.display()))
}

#[derive(Debug, Eq, PartialEq)]
pub struct TaskPath {
    pub path: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct TaskSummary {
    pub path: String,
    pub title: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TaskSearchResult {
    pub id: String,
    pub status: String,
    pub title: String,
    pub summary: String,
    pub context: String,
}

impl From<&TaskSnapshot> for TaskSummary {
    fn from(snapshot: &TaskSnapshot) -> Self {
        Self {
            path: snapshot.path().to_string(),
            title: snapshot.title().to_string(),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct TaskMetadata {
    pub path: String,
    pub title: String,
    pub committed_at: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct TaskDocument {
    pub stem: String,
    pub status: String,
    pub rank: Option<usize>,
    pub contents: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ValidationMessage(String);

impl fmt::Display for ValidationMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct TaskUpdate<'a> {
    pub title: Option<&'a str>,
    pub summary: Option<&'a str>,
    pub context: Option<&'a str>,
    pub tags: Option<Vec<String>>,
    pub pr_mr_url: Option<&'a str>,
    pub pr_mr_status: Option<&'a str>,
}

#[derive(Debug)]
pub enum Error {
    CommandFailed {
        program: String,
        args: Vec<String>,
        status: String,
        stderr: String,
    },
    BacklogCapacityExceeded {
        queued: usize,
        max_queued: usize,
    },
    Io(std::io::Error),
    Parse(String),
    Core(tiber_core::CoreError),
    Usage(String),
    WorkflowBlocked {
        code: &'static str,
        required_action: &'static str,
        source: Box<Error>,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandFailed {
                program,
                args,
                status,
                stderr,
            } => write!(
                formatter,
                "tiber.command_failed program={program} args={} status={status} stderr={}",
                args.join(" "),
                stderr.trim()
            ),
            Self::BacklogCapacityExceeded {
                queued,
                max_queued,
            } => write!(
                formatter,
                "tiber.backlog_capacity_exceeded queued={queued} max_queued={max_queued} action=\"replace a lower-value queued ticket, combine genuinely overlapping tickets, or reject the candidate\""
            ),
            Self::Io(error) => write!(formatter, "tiber.io_error source={error}"),
            Self::Parse(message) => write!(formatter, "tiber.parse_error {message}"),
            Self::Core(error) => write!(formatter, "{error}"),
            Self::Usage(message) => write!(formatter, "{message}"),
            Self::WorkflowBlocked {
                code,
                required_action,
                source,
            } => write!(
                formatter,
                "{code} workflow_blocked=true required_action=\"{required_action}\" prohibited_actions=\"diagnose,edit,test,rerun,push,unrelated-work\" source={}",
                source.sanitized_workflow_blocker_source()
            ),
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    pub fn workflow_blocker_data(&self) -> Option<WorkflowBlockerData> {
        match self {
            Self::WorkflowBlocked {
                code,
                required_action,
                ..
            } => Some(WorkflowBlockerData {
                error_code: code,
                required_action,
            }),
            _ => None,
        }
    }
    fn sanitized_workflow_blocker_source(&self) -> String {
        match self {
            Self::Parse(message) => format!("tiber.parse_error {message}"),
            _ => self.sanitized_sync_source(),
        }
    }

    fn sanitized_sync_source(&self) -> String {
        match self {
            Self::CommandFailed {
                program,
                status,
                stderr,
                ..
            } => format!(
                "tiber.command_failed program={program} args_redacted=true status={status} stderr_redacted={}",
                !stderr.trim().is_empty()
            ),
            Self::BacklogCapacityExceeded {
                queued,
                max_queued,
            } => format!(
                "tiber.backlog_capacity_exceeded queued={queued} max_queued={max_queued} action=\"replace a lower-value queued ticket, combine genuinely overlapping tickets, or reject the candidate\""
            ),
            Self::Io(_) => "tiber.io_error source_redacted=true".to_string(),
            Self::Parse(message) if message.starts_with("sync_conflict ") => {
                format!("tiber.parse_error {message}")
            }
            Self::Parse(_) => "tiber.parse_error source_redacted=true".to_string(),
            Self::Core(error) => error.to_string(),
            Self::Usage(message) => message.to_string(),
            Self::WorkflowBlocked { code, .. } => format!("{code} workflow_blocked=true"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<tiber_core::CoreError> for Error {
    fn from(error: tiber_core::CoreError) -> Self {
        Self::Core(error)
    }
}

struct GitRepository {
    root: PathBuf,
    task_projection: Option<Rc<RefCell<TiberProjection>>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectConfig {
    #[serde(default)]
    backlog: BacklogConfig,
    #[serde(default)]
    final_review: FinalReviewConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct BacklogConfig {
    max_queued: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FinalReviewConfig {
    #[serde(default)]
    minimum_clean_reviews: usize,
}

impl GitRepository {
    fn at(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            task_projection: None,
        }
    }

    fn discover() -> Result<Self, Error> {
        if let Some(repository_root) =
            MCP_REPOSITORY_ROOT.with(|slot| slot.borrow().as_ref().cloned())
        {
            return Self::discover_from(Some(&repository_root)).map_err(|error| {
                Error::Usage(format!(
                    "tiber.repository_root_invalid source=mcp_sandbox_cwd error={error}"
                ))
            });
        }

        let current_directory_error = match Self::discover_from(None) {
            Ok(repository) => return Ok(repository),
            Err(error) => error,
        };

        if let Some(configured_root) = std::env::var_os("TIBER_REPOSITORY_ROOT") {
            let configured_root = PathBuf::from(configured_root);
            return Self::discover_from(Some(&configured_root)).map_err(|error| {
                Error::Usage(format!(
                    "tiber.repository_root_invalid source=TIBER_REPOSITORY_ROOT error={error}"
                ))
            });
        }

        let launched_from_plugin_root = std::env::current_dir()
            .map(|directory| directory.join(".codex-plugin/plugin.json").is_file())
            .unwrap_or(false);
        if launched_from_plugin_root {
            if let Some(inherited_working_directory) = std::env::var_os("PWD") {
                let inherited_working_directory = PathBuf::from(inherited_working_directory);
                if inherited_working_directory.is_absolute() {
                    if let Ok(repository) = Self::discover_from(Some(&inherited_working_directory))
                    {
                        return Ok(repository);
                    }
                }
            }
        }

        Err(current_directory_error)
    }

    fn discover_from(working_directory: Option<&Path>) -> Result<Self, Error> {
        if let Ok(root) = git_output(["rev-parse", "--show-toplevel"], working_directory) {
            return Ok(Self::at(PathBuf::from(root.trim())));
        }

        let git_dir = git_output(
            ["rev-parse", "--absolute-git-dir"],
            working_directory,
        )
        .map_err(|_| {
                Error::Usage(
                    "tiber.repository_not_found action=\"run from a repository checkout or configure TIBER_REPOSITORY_ROOT\""
                        .to_string(),
                )
            })?;
        if let Ok(root) = git_output(
            ["config", "--path", "--get", "core.worktree"],
            working_directory,
        ) {
            let root = PathBuf::from(root.trim());
            let root = if root.is_absolute() {
                root
            } else {
                PathBuf::from(git_dir.trim()).join(root)
            };
            return Ok(Self::at(root));
        }

        Err(Error::Usage(
            "tiber.repository_root_unresolved action=\"run from a repository checkout or configure TIBER_REPOSITORY_ROOT\""
                .to_string(),
        ))
    }

    fn with_task_projection(&self, projection: Rc<RefCell<TiberProjection>>) -> Self {
        Self {
            root: self.root.clone(),
            task_projection: Some(projection),
        }
    }

    fn with_task_snapshot_workspace<T>(
        &self,
        operation: impl FnOnce(&GitRepository) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let projection = Rc::new(RefCell::new(load_tiber_projection(&self.root)?));
        let repo = self.with_task_projection(projection);
        operation(&repo)
    }

    fn current_branch(&self) -> Result<String, Error> {
        let branch = git_output(["branch", "--show-current"], Some(&self.root))?;
        let branch = branch.trim();
        if branch.is_empty() {
            return Err(Error::Parse("detached_head=true".to_string()));
        }
        Ok(branch.to_string())
    }

    fn commit_signing_enabled(&self) -> Result<bool, Error> {
        match self.git(["config", "--bool", "commit.gpgsign"]) {
            Ok(value) => Ok(value.trim() == "true"),
            Err(Error::CommandFailed { .. }) => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn sync_repository(&self) -> Result<(), Error> {
        let store = GitEventStore::open(&self.root).map_err(event_store_error)?;
        let outcome: SynchronizeOutcome =
            run_async(async move { store.synchronize().await }).map_err(event_store_error)?;
        match outcome {
            SynchronizeOutcome::Current | SynchronizeOutcome::PublishedPending => Ok(()),
            SynchronizeOutcome::DiscardedUnpublished => Err(Error::Parse(
                "event_transaction_discarded reissue_required=true workflow_blocked=true"
                    .to_string(),
            )),
        }
    }

    fn project_config(&self) -> Result<ProjectConfig, Error> {
        let path = self.root.join(CONFIG_FILE);
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ProjectConfig::default());
            }
            Err(error) => return Err(error.into()),
        };
        parse_project_config(&contents)
    }

    fn project_config_for_lifecycle_gate(&self) -> Result<ProjectConfig, Error> {
        let live = match fs::read_to_string(self.root.join(CONFIG_FILE)) {
            Ok(contents) => Some(contents),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let output = Command::new("git")
            .args(["show", &format!("HEAD:{CONFIG_FILE}")])
            .current_dir(&self.root)
            .output()
            .map_err(Error::Io)?;
        let committed = output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).into_owned());
        let live_config = match live.as_deref() {
            Some(contents) => parse_project_config(contents)?,
            None => ProjectConfig::default(),
        };
        let committed_config = match committed.as_deref() {
            Some(contents) => parse_project_config(contents)?,
            None => ProjectConfig::default(),
        };
        if live_config.final_review.minimum_clean_reviews
            != committed_config.final_review.minimum_clean_reviews
        {
            return Err(Error::Usage(format!(
                "tiber.final_review_config_uncommitted file={CONFIG_FILE} action=\"commit the intended final-review policy configuration before completion or delivery\""
            )));
        }
        Ok(committed_config)
    }
}

fn parse_project_config(contents: &str) -> Result<ProjectConfig, Error> {
    let config: ProjectConfig = toml::from_str(contents).map_err(|error| {
        Error::Parse(format!("config_invalid file={CONFIG_FILE} source={error}"))
    })?;
    let minimum = config.final_review.minimum_clean_reviews;
    if (1..3).contains(&minimum) {
        return Err(Error::Parse(format!(
                "config_invalid file={CONFIG_FILE} field=final_review.minimum_clean_reviews expected=0_or_at_least_3 actual={minimum}"
            )));
    }
    Ok(config)
}

impl GitRepository {
    fn list_tasks(&self) -> Result<Vec<TaskSummary>, Error> {
        Ok(self
            .board_snapshot()?
            .ordered_tasks()
            .iter()
            .map(TaskSummary::from)
            .collect())
    }

    fn list_tasks_by_status(&self, status: &str) -> Result<Vec<TaskSummary>, Error> {
        let status = parse_status(status)?;
        let projection = self.task_projection()?;
        let tasks = projection
            .borrow()
            .tasks
            .values()
            .filter(|task| task.status == status)
            .map(|task| TaskSummary {
                path: task.stem.clone(),
                title: task.title.clone(),
            })
            .collect();
        Ok(tasks)
    }

    fn search_tasks(&self, query: &str) -> Result<Vec<TaskSearchResult>, Error> {
        let query = query.to_lowercase();
        let projection = self.task_projection()?;
        let mut results = Vec::new();
        for task in projection.borrow().tasks.values() {
            if [
                task.title.as_str(),
                task.summary.as_str(),
                task.context.as_str(),
            ]
            .iter()
            .any(|field| field.to_lowercase().contains(&query))
            {
                results.push(TaskSearchResult {
                    id: task.stem.clone(),
                    status: task.status.clone(),
                    title: task.title.clone(),
                    summary: task.summary.clone(),
                    context: task.context.clone(),
                });
            }
        }
        results.sort_by(|left, right| {
            (left.status.as_str(), left.id.as_str())
                .cmp(&(right.status.as_str(), right.id.as_str()))
        });
        Ok(results)
    }

    fn board_snapshot(&self) -> Result<BoardSnapshot, Error> {
        let projection = self.task_projection()?;
        let projection = projection.borrow();
        let ordered_tasks = projection
            .order
            .iter()
            .filter_map(|stem| {
                projection
                    .tasks
                    .get(stem)
                    .map(|task| TaskSnapshot::new(stem, &task.title))
            })
            .collect();
        Ok(BoardSnapshot::from_ordered_tasks(ordered_tasks))
    }

    fn show_task(&self, task_ref: &str) -> Result<String, Error> {
        let stem = self.resolve_task_stem(task_ref)?;
        Ok(self
            .task_projection()?
            .borrow()
            .tasks
            .get(&stem)
            .expect("resolved task")
            .render_markdown())
    }

    fn task_metadata(&self, task_ref: &str) -> Result<TaskMetadata, Error> {
        let stem = self.resolve_task_stem(task_ref)?;
        let projection = self.task_projection()?;
        let projection = projection.borrow();
        let task = projection.tasks.get(&stem).expect("resolved task");
        Ok(TaskMetadata {
            path: stem,
            title: task.title.clone(),
            committed_at: Some(task.committed_at.clone()),
        })
    }

    fn task_documents_snapshot(&self) -> Result<Vec<TaskDocument>, Error> {
        let projection = self.task_projection()?;
        let projection = projection.borrow();
        let ranks = projection
            .order
            .iter()
            .enumerate()
            .map(|(index, stem)| (stem.clone(), index + 1))
            .collect::<std::collections::BTreeMap<_, _>>();
        Ok(projection
            .tasks
            .values()
            .map(|task| TaskDocument {
                rank: ranks.get(&task.stem).copied(),
                stem: task.stem.clone(),
                status: task.status.clone(),
                contents: task.render_markdown(),
            })
            .collect())
    }

    fn list_docs(&self) -> Result<Vec<String>, Error> {
        let docs_dir = self.root.join("docs");
        let mut docs = Vec::new();
        if docs_dir.exists() {
            collect_docs(&docs_dir, &docs_dir, &mut docs)?;
        }
        docs.sort();
        Ok(docs.into_iter().map(|doc| format!("docs/{doc}")).collect())
    }

    fn read_doc(&self, doc_ref: &str) -> Result<String, Error> {
        let doc_ref = parse_doc_ref(doc_ref)?;
        fs::read_to_string(self.root.join(doc_ref)).map_err(Error::Io)
    }

    fn next_task(&self) -> Result<Option<TaskSummary>, Error> {
        let projection = self.task_projection()?;
        let projection = projection.borrow();
        for stem in &projection.order {
            let Some(task) = projection.tasks.get(stem) else {
                continue;
            };
            if task.blocked_by.iter().all(|blocker| {
                projection
                    .tasks
                    .get(blocker)
                    .is_some_and(|item| item.status == "done")
            }) {
                return Ok(Some(TaskSummary {
                    path: stem.clone(),
                    title: task.title.clone(),
                }));
            }
        }
        Ok(None)
    }

    fn scaffold_repo(&self, apply: bool, replace_conflicts: bool) -> Result<Vec<String>, Error> {
        let _lock = if apply {
            Some(self.acquire_lock()?)
        } else {
            None
        };
        let mut files = Vec::new();
        let mut integration_conflicts = Vec::new();
        let mut integration_messages = Vec::new();
        let equivalent_hook = self.equivalent_task_closing_hook()?;
        if equivalent_hook.is_none() {
            let active_hook = self.active_post_commit_hook()?;
            if self.hook_dispatches_tiber_snippet(&active_hook)? {
                files.push((
                    ".githooks/post-commit.tiber",
                    "#!/usr/bin/env bash\nset -euo pipefail\n\ntiber close-from-trailers\n"
                        .to_string(),
                    true,
                ));
            } else if self.explicit_hooks_path()?.is_some() {
                integration_conflicts.push(format!(
                    "hook-dispatch active={} resolution=the active hook must invoke .githooks/post-commit.tiber",
                    active_hook.display()
                ));
            } else {
                integration_messages.push(format!(
                    "skipped hook-dispatch active={} resolution=configure an active post-commit dispatcher before adding .githooks/post-commit.tiber",
                    active_hook.display()
                ));
            }
        }
        let equivalent_workflow = self.equivalent_task_closing_workflow()?;
        if equivalent_workflow.is_none() {
            if self.commit_signing_enabled()? {
                integration_conflicts.push(
                    "signed-publication generated GitHub workflow cannot access a signing key resolution=provide repository-owned signed tasks-branch automation and rerun scaffold"
                        .to_string(),
                );
            } else {
                let publication_branch = yaml_single_quoted(&self.publication_branch()?);
                files.push((
                    ".github/workflows/tiber-close-from-trailers.yml",
                    format!(
                        "name: tiber close from trailers\n\non:\n  push:\n    branches: [{publication_branch}]\n\npermissions:\n  contents: write\n\njobs:\n  close:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683\n      - name: Install Tiber\n        run: |\n          git clone --no-checkout https://github.com/jwilger/ai-plugins.git .tiber-src\n          git -C .tiber-src checkout e975c7eb5bf66a3bb93095e65343955e30f4eedc\n          cargo install --locked --path .tiber-src/plugins/development-system/components/tiber/rust/crates/tiber-cli --bin tiber --root .tiber-install\n          echo \"$PWD/.tiber-install/bin\" >> \"$GITHUB_PATH\"\n      - run: tiber close-from-trailers\n"
                    ),
                    true,
                ));
            }
        }
        let justfile_exists = self.root.join("justfile").exists();
        let planned_justfile = self.show_tasks_justfile()?;
        if let Some(justfile) = planned_justfile.as_ref() {
            files.push(("justfile", justfile.clone(), false));
        }
        let mut messages = integration_messages;
        messages.extend(
            integration_conflicts
                .iter()
                .map(|conflict| format!("conflict {conflict}")),
        );
        if justfile_exists && planned_justfile.is_none() {
            messages.push("already configured justfile".to_string());
        }
        let mut pending_files = Vec::new();
        let mut conflicts = Vec::new();
        for (path, contents, conflict_on_difference) in files {
            let destination = self.root.join(path);
            match fs::read_to_string(&destination) {
                Ok(existing) if existing == contents => {
                    messages.push(format!("already configured {path}"));
                }
                Ok(_) if conflict_on_difference => conflicts.push((path, contents)),
                Ok(_) => pending_files.push((path, contents)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    pending_files.push((path, contents));
                }
                Err(error) => return Err(error.into()),
            }
        }
        if apply
            && (!integration_conflicts.is_empty() || (!replace_conflicts && !conflicts.is_empty()))
        {
            if !integration_conflicts.is_empty() {
                return Err(Error::Parse(format!(
                    "scaffold_integration_conflicts {}",
                    integration_conflicts.join(";")
                )));
            }
            return Err(Error::Parse(format!(
                "scaffold_conflicts paths={} resolution=--replace-conflicts",
                conflicts
                    .iter()
                    .map(|(path, _contents)| *path)
                    .collect::<Vec<_>>()
                    .join(",")
            )));
        }
        if replace_conflicts {
            pending_files.extend(conflicts.iter().cloned());
        }
        if apply {
            for (path, _contents) in &pending_files {
                let destination = self.root.join(path);
                reject_symlinked_ancestors(&self.root, &destination)?;
                match fs::symlink_metadata(&destination) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        return Err(Error::Parse(format!(
                            "scaffold_destination_symlink path={path}"
                        )));
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
            }
            for (path, contents) in &pending_files {
                let destination = self.root.join(path);
                atomic_write(&destination, contents.as_bytes())?;
                messages.push(format!("wrote {path}"));
            }
        } else {
            messages.extend(
                pending_files
                    .iter()
                    .map(|(path, _contents)| format!("would write {path}")),
            );
            messages.extend(conflicts.iter().map(|(path, _contents)| {
                format!("conflict {path} resolution=--replace-conflicts")
            }));
        }
        for path in [equivalent_hook, equivalent_workflow].into_iter().flatten() {
            messages.push(format!("already configured {path}"));
        }
        Ok(messages)
    }

    fn show_tasks_justfile(&self) -> Result<Option<String>, Error> {
        let path = self.root.join("justfile");
        if !path.exists() {
            return Ok(None);
        }
        let mut contents = fs::read_to_string(path)?;
        if contents.lines().any(|line| line.trim() == "show-tasks:") {
            return Ok(None);
        }
        if !contents.ends_with('\n') {
            contents.push('\n');
        }
        contents.push_str("\nshow-tasks:\n  tiber list\n");
        Ok(Some(contents))
    }

    fn equivalent_task_closing_workflow(&self) -> Result<Option<String>, Error> {
        let workflows = self.root.join(".github").join("workflows");
        let entries = match fs::read_dir(&workflows) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut entries = entries.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let supported_extension = path
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|extension| matches!(extension, "yml" | "yaml"));
            if !entry.file_type()?.is_file() || !supported_extension {
                continue;
            }
            let relative = path
                .strip_prefix(&self.root)
                .map_err(|_| Error::Parse("scaffold_path_outside_repository".to_string()))?;
            let relative = path_to_entry(relative)?;
            let contents = fs::read(&path)?;
            let contents = std::str::from_utf8(&contents).map_err(|_| {
                Error::Parse(format!("scaffold_workflow_invalid_utf8 path={relative}"))
            })?;
            if workflow_invokes_task_closer(contents) {
                return Ok(Some(relative));
            }
        }
        Ok(None)
    }

    fn equivalent_task_closing_hook(&self) -> Result<Option<String>, Error> {
        let hooks = PathBuf::from(
            self.git(["rev-parse", "--path-format=absolute", "--git-path", "hooks"])?
                .trim(),
        );
        let hook = hooks.join("post-commit");
        if !hook.is_file() || !is_executable(&hook)? {
            return Ok(None);
        }
        let contents = fs::read(&hook)?;
        let Ok(contents) = std::str::from_utf8(&contents) else {
            return Ok(None);
        };
        if contents.lines().any(shell_line_invokes_task_closer) {
            let path = match hook.strip_prefix(&self.root) {
                Ok(relative) => path_to_entry(relative)?,
                Err(_) => hook.display().to_string(),
            };
            return Ok(Some(path));
        }
        Ok(None)
    }

    fn explicit_hooks_path(&self) -> Result<Option<PathBuf>, Error> {
        match self.git(["config", "--path", "core.hooksPath"]) {
            Ok(path) => {
                let path = PathBuf::from(path.trim());
                Ok(Some(if path.is_absolute() {
                    path
                } else {
                    self.root.join(path)
                }))
            }
            Err(Error::CommandFailed { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn active_post_commit_hook(&self) -> Result<PathBuf, Error> {
        Ok(PathBuf::from(
            self.git([
                "rev-parse",
                "--path-format=absolute",
                "--git-path",
                "hooks/post-commit",
            ])?
            .trim(),
        ))
    }

    fn publication_branch(&self) -> Result<String, Error> {
        match self.git([
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ]) {
            Ok(reference) => reference
                .trim()
                .strip_prefix("origin/")
                .filter(|branch| !branch.is_empty())
                .map(str::to_string)
                .ok_or_else(|| Error::Parse("origin_default_branch_invalid=true".to_string())),
            Err(Error::CommandFailed { .. }) => self.current_branch(),
            Err(error) => Err(error),
        }
    }

    fn hook_dispatches_tiber_snippet(&self, hook: &Path) -> Result<bool, Error> {
        if !hook.is_file() || !is_executable(hook)? {
            return Ok(false);
        }
        let contents = fs::read(hook)?;
        Ok(std::str::from_utf8(&contents).is_ok_and(hook_contents_dispatch_tiber_snippet))
    }

    fn task_file_refs(&self) -> Result<Vec<String>, Error> {
        let projection = self.task_projection()?;
        let mut refs = projection
            .borrow()
            .tasks
            .values()
            .map(|task| format!("{}/{}.md", task.status, task.stem))
            .collect::<Vec<_>>();
        refs.sort();
        Ok(refs)
    }

    fn task_projection(&self) -> Result<Rc<RefCell<TiberProjection>>, Error> {
        self.task_projection
            .clone()
            .ok_or_else(|| Error::Parse("task_projection_unavailable=true".into()))
    }

    fn resolve_task_stem(&self, task_ref: &str) -> Result<String, Error> {
        let path = self.resolve_task_ref(task_ref)?;
        task_stem(&path)
    }

    fn resolve_task_ref(&self, task_ref: &str) -> Result<PathBuf, Error> {
        if task_ref.contains('/') || task_ref.ends_with(".md") || task_ref.trim().is_empty() {
            return Err(Error::Parse(format!("invalid_task_ref ref={task_ref}")));
        }
        let mut matches = self
            .task_file_refs()?
            .into_iter()
            .filter(|candidate| {
                let stem = candidate.trim_end_matches(".md");
                let file_stem = Path::new(candidate)
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default();
                let id = file_stem
                    .split_once('-')
                    .and_then(|(date, rest)| {
                        rest.split_once('-')
                            .map(|(code, _nickname)| format!("{date}-{code}"))
                    })
                    .unwrap_or_default();
                let nickname = file_stem
                    .split_once('-')
                    .and_then(|(_date, rest)| rest.split_once('-'))
                    .map(|(_code, nickname)| nickname)
                    .unwrap_or_default();
                stem == task_ref || file_stem == task_ref || id == task_ref || nickname == task_ref
            })
            .collect::<Vec<_>>();
        matches.sort();
        match matches.as_slice() {
            [resolved] => Ok(PathBuf::from(resolved)),
            [] => Err(Error::Parse(format!("task_ref_missing ref={task_ref}"))),
            _ => Err(Error::Parse(format!(
                "ambiguous_task_ref ref={task_ref} matches={}",
                matches.join(",")
            ))),
        }
    }

    fn acquire_lock(&self) -> Result<TiberLock, Error> {
        let timeout =
            lock_retry_duration("TIBER_LOCK_RETRY_TIMEOUT_MS", DEFAULT_LOCK_RETRY_TIMEOUT);
        let interval =
            lock_retry_duration("TIBER_LOCK_RETRY_INTERVAL_MS", DEFAULT_LOCK_RETRY_INTERVAL);
        let interval = if interval.is_zero() {
            DEFAULT_LOCK_RETRY_INTERVAL
        } else {
            interval
        };
        let started_at = Instant::now();
        loop {
            match self.try_acquire_task_lock_once() {
                Ok(lock) => return Ok(lock),
                Err(error)
                    if is_tiber_lock_busy(&error) && lock_retry_remaining(started_at, timeout) =>
                {
                    thread::sleep(interval);
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn acquire_named_lock(&self, filename: &str) -> Result<TiberLock, Error> {
        let timeout =
            lock_retry_duration("TIBER_LOCK_RETRY_TIMEOUT_MS", DEFAULT_LOCK_RETRY_TIMEOUT);
        let interval =
            lock_retry_duration("TIBER_LOCK_RETRY_INTERVAL_MS", DEFAULT_LOCK_RETRY_INTERVAL);
        let interval = if interval.is_zero() {
            DEFAULT_LOCK_RETRY_INTERVAL
        } else {
            interval
        };
        let started_at = Instant::now();
        loop {
            match self.try_acquire_named_lock_once(filename) {
                Ok(lock) => return Ok(lock),
                Err(error)
                    if is_tiber_lock_busy(&error) && lock_retry_remaining(started_at, timeout) =>
                {
                    thread::sleep(interval);
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn try_acquire_named_lock_once(&self, filename: &str) -> Result<TiberLock, Error> {
        let lock_dir = self.git_common_dir()?.join("tiber");
        fs::create_dir_all(&lock_dir)?;
        let lock_path = lock_dir.join(filename);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        match file.try_lock() {
            Ok(()) => {
                file.set_len(0)?;
                file.write_all(lock_metadata().as_bytes())?;
                file.sync_data()?;
                Ok(TiberLock {
                    _file: file,
                    _legacy_sentinel: None,
                })
            }
            Err(TryLockError::WouldBlock) => Err(Error::Parse(format!(
                "tiber_lock_busy path={}",
                path_to_entry(&lock_path)?
            ))),
            Err(TryLockError::Error(error)) => Err(Error::Io(error)),
        }
    }

    fn try_acquire_task_lock_once(&self) -> Result<TiberLock, Error> {
        let lock_dir = self.git_common_dir()?.join("tiber");
        fs::create_dir_all(&lock_dir)?;
        let legacy_path = lock_dir.join("tiber.lock");
        let advisory_path = lock_dir.join("tiber.advisory.lock");
        let advisory_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&advisory_path)?;
        match advisory_file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(Error::Parse(format!(
                    "tiber_lock_busy path={}",
                    path_to_entry(&legacy_path)?
                )));
            }
            Err(TryLockError::Error(error)) => return Err(Error::Io(error)),
        }

        if let Some(stale_contents) = stale_lock_contents(&legacy_path)? {
            if fs::read_to_string(&legacy_path)
                .ok()
                .as_deref()
                .is_some_and(|contents| contents == stale_contents)
            {
                let _ = fs::remove_file(&legacy_path);
            }
        }
        let metadata = lock_metadata();
        let legacy_file = match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&legacy_path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(Error::Parse(format!(
                    "tiber_lock_busy path={}",
                    path_to_entry(&legacy_path)?
                )));
            }
            Err(error) => return Err(Error::Io(error)),
        };
        let mut legacy_sentinel = LegacySentinel {
            file: legacy_file,
            path: legacy_path,
            metadata: None,
        };
        legacy_sentinel.file.write_all(metadata.as_bytes())?;
        legacy_sentinel.file.sync_data()?;
        legacy_sentinel.metadata = Some(metadata);

        Ok(TiberLock {
            _file: advisory_file,
            _legacy_sentinel: Some(legacy_sentinel),
        })
    }

    fn git<I, S>(&self, args: I) -> Result<String, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        git_output(args, Some(&self.root))
    }

    fn git_common_dir(&self) -> Result<PathBuf, Error> {
        let git_common_dir = self.git(["rev-parse", "--git-common-dir"])?;
        let git_common_dir = PathBuf::from(git_common_dir.trim());
        if git_common_dir.is_absolute() {
            Ok(git_common_dir)
        } else {
            Ok(self.root.join(git_common_dir))
        }
    }
}

struct TiberLock {
    _file: fs::File,
    _legacy_sentinel: Option<LegacySentinel>,
}

struct LegacySentinel {
    file: fs::File,
    path: PathBuf,
    metadata: Option<String>,
}

impl Drop for LegacySentinel {
    fn drop(&mut self) {
        let still_owned = self.metadata.as_ref().is_none_or(|metadata| {
            fs::read_to_string(&self.path)
                .ok()
                .as_deref()
                .is_some_and(|contents| contents == metadata)
        });
        if still_owned {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn claim_host() -> String {
    std::env::var("TIBER_CLAIM_HOST")
        .or_else(|_| std::env::var("HOSTNAME"))
        .map(|value| frontmatter_scalar_value(&value))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn claim_session() -> String {
    std::env::var("TIBER_CLAIM_SESSION")
        .or_else(|_| std::env::var("CODEX_SESSION_ID"))
        .or_else(|_| std::env::var("CLAUDE_SESSION_ID"))
        .map(|value| frontmatter_scalar_value(&value))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn frontmatter_scalar_value(value: &str) -> String {
    let sanitized = value
        .trim()
        .chars()
        .map(|character| {
            if character.is_control() {
                '-'
            } else {
                character
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "unknown".to_string()
    } else {
        sanitized
    }
}

fn task_stem(task_path: &Path) -> Result<String, Error> {
    task_path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .ok_or_else(|| Error::Parse("task_stem_missing=true".to_string()))
}

fn is_open_status(status: &str) -> bool {
    OPEN_STATUS_DIRS.contains(&status)
}

fn nonempty_option(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn atomic_write(destination: &Path, contents: &[u8]) -> Result<(), Error> {
    static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    let parent = destination
        .parent()
        .ok_or_else(|| Error::Parse("scaffold_destination_parent_missing".to_string()))?;
    let file_name = destination
        .file_name()
        .ok_or_else(|| Error::Parse("scaffold_destination_name_missing".to_string()))?
        .to_string_lossy();
    let temporary_prefix = format!(".tiber-tmp-{file_name}-");
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        "{temporary_prefix}{}-{sequence}",
        std::process::id()
    ));
    let result = (|| -> Result<(), Error> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(contents)?;
        if let Ok(metadata) = fs::metadata(destination) {
            fs::set_permissions(&temporary, metadata.permissions())?;
        }
        file.sync_all()?;
        fs::rename(&temporary, destination)?;
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn reject_symlinked_ancestors(root: &Path, destination: &Path) -> Result<(), Error> {
    let relative = destination
        .strip_prefix(root)
        .map_err(|_| Error::Parse("scaffold_destination_outside_repository".to_string()))?;
    let mut ancestor = root.to_path_buf();
    for component in relative.parent().into_iter().flat_map(Path::components) {
        ancestor.push(component);
        match fs::symlink_metadata(&ancestor) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                let path = ancestor.strip_prefix(root).unwrap_or(&ancestor).display();
                return Err(Error::Parse(format!(
                    "scaffold_destination_ancestor_symlink path={path}"
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn workflow_invokes_task_closer(contents: &str) -> bool {
    if !workflow_has_push_trigger(contents) {
        return false;
    }
    let lines = contents.lines().collect::<Vec<_>>();
    for (jobs_index, line) in lines.iter().enumerate() {
        let jobs_trimmed = line.trim_start();
        if trim_unquoted_comment(jobs_trimmed) != "jobs:" {
            continue;
        }
        let jobs_indentation = line.len() - jobs_trimmed.len();
        for (steps_index, steps_line) in lines.iter().enumerate().skip(jobs_index + 1) {
            let steps_trimmed = steps_line.trim_start();
            if steps_trimmed.is_empty() || steps_trimmed.starts_with('#') {
                continue;
            }
            let steps_indentation = steps_line.len() - steps_trimmed.len();
            if steps_indentation <= jobs_indentation {
                break;
            }
            if trim_unquoted_comment(steps_trimmed) != "steps:" {
                continue;
            }
            if steps_invoke_task_closer(&lines, steps_index + 1, steps_indentation) {
                return true;
            }
        }
    }
    false
}

fn workflow_has_push_trigger(contents: &str) -> bool {
    let lines = contents.lines().collect::<Vec<_>>();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if line.len() != trimmed.len() {
            continue;
        }
        let Some(value) = trim_unquoted_comment(trimmed).strip_prefix("on:") else {
            continue;
        };
        let value = value.trim();
        if value == "push"
            || value
                .strip_prefix('[')
                .and_then(|value| value.strip_suffix(']'))
                .is_some_and(|events| events.split(',').any(|event| event.trim() == "push"))
        {
            return true;
        }
        if !value.is_empty() {
            return false;
        }
        for event_line in &lines[index + 1..] {
            let event = event_line.trim_start();
            let indentation = event_line.len() - event.len();
            if event.is_empty() || event.starts_with('#') {
                continue;
            }
            if indentation == 0 {
                return false;
            }
            let event = trim_unquoted_comment(event).trim();
            if event.starts_with("push:")
                || event.strip_prefix("- ").is_some_and(|item| item == "push")
            {
                return true;
            }
        }
        return false;
    }
    false
}

fn steps_invoke_task_closer(lines: &[&str], start: usize, steps_indentation: usize) -> bool {
    let mut step_indentation = None;
    let mut property_indentation = None;
    for (index, line) in lines.iter().enumerate().skip(start) {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indentation = line.len() - trimmed.len();
        if indentation <= steps_indentation {
            break;
        }
        if let Some(field) = trimmed.strip_prefix("- ") {
            step_indentation = Some(indentation);
            property_indentation = None;
            if run_field_invokes_task_closer(lines, index, indentation, field) {
                return true;
            }
            continue;
        }
        let Some(current_step_indentation) = step_indentation else {
            continue;
        };
        if indentation <= current_step_indentation {
            step_indentation = None;
            property_indentation = None;
            continue;
        }
        let current_property_indentation = *property_indentation.get_or_insert(indentation);
        if indentation == current_property_indentation
            && run_field_invokes_task_closer(lines, index, indentation, trimmed)
        {
            return true;
        }
    }
    false
}

fn run_field_invokes_task_closer(
    lines: &[&str],
    index: usize,
    indentation: usize,
    field: &str,
) -> bool {
    let Some(value) = field.strip_prefix("run:") else {
        return false;
    };
    let value = trim_unquoted_comment(value.trim());
    if value.starts_with('|') || value.starts_with('>') {
        for block_line in &lines[index + 1..] {
            let block_trimmed = block_line.trim_start();
            if block_trimmed.is_empty() {
                continue;
            }
            let block_indentation = block_line.len() - block_trimmed.len();
            if block_indentation <= indentation {
                break;
            }
            if shell_line_invokes_task_closer(block_trimmed) {
                return true;
            }
        }
        false
    } else {
        shell_line_invokes_task_closer(value)
    }
}

fn shell_line_invokes_task_closer(line: &str) -> bool {
    let line = trim_shell_comment(line.trim())
        .trim()
        .trim_matches(|character| matches!(character, '"' | '\''));
    shell_command_segments(line)
        .into_iter()
        .any(shell_command_invokes_task_closer)
}

fn trim_shell_comment(value: &str) -> &str {
    let mut quote = None;
    let mut escaped = false;
    let mut at_word_start = true;
    for (index, character) in value.char_indices() {
        if escaped {
            escaped = false;
            at_word_start = false;
            continue;
        }
        if quote != Some('\'') && character == '\\' {
            escaped = true;
            continue;
        }
        if matches!(character, '"' | '\'') {
            quote = if quote == Some(character) {
                None
            } else if quote.is_none() {
                Some(character)
            } else {
                quote
            };
            at_word_start = false;
            continue;
        }
        if character == '#' && quote.is_none() && at_word_start {
            return value[..index].trim_end();
        }
        at_word_start = quote.is_none()
            && (character.is_whitespace()
                || matches!(character, '|' | '&' | ';' | '(' | ')' | '<' | '>'));
    }
    value
}

fn shell_command_segments(line: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut escaped = false;
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let character = bytes[index] as char;
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        if quote != Some('\'') && character == '\\' {
            escaped = true;
            index += 1;
            continue;
        }
        if matches!(character, '"' | '\'') {
            quote = if quote == Some(character) {
                None
            } else if quote.is_none() {
                Some(character)
            } else {
                quote
            };
            index += 1;
            continue;
        }
        let operator_length = if quote.is_none() && character == ';' {
            1
        } else if quote.is_none()
            && index + 1 < bytes.len()
            && matches!(&bytes[index..index + 2], b"&&" | b"||")
        {
            2
        } else {
            index += 1;
            continue;
        };
        segments.push(&line[start..index]);
        index += operator_length;
        start = index;
    }
    segments.push(&line[start..]);
    segments
}

fn shell_command_invokes_task_closer(line: &str) -> bool {
    let line = line.trim();
    let line = line.strip_prefix("exec ").unwrap_or(line);
    let line = line.strip_prefix("nix develop -c ").unwrap_or(line);
    let Some(remainder) = line.strip_prefix("tiber close-from-trailers") else {
        return false;
    };
    let remainder = remainder.trim_start();
    remainder.is_empty()
        || [">", "1>", "2>"]
            .iter()
            .any(|operator| remainder.starts_with(operator))
}

fn shell_line_invokes_tiber_snippet(line: &str) -> bool {
    if line != line.trim_start() {
        return false;
    }
    shell_command_invokes_tiber_snippet(trim_shell_comment(line.trim()).trim())
}

fn hook_contents_dispatch_tiber_snippet(contents: &str) -> bool {
    let mut meaningful = contents.lines().filter_map(|line| {
        let line = trim_shell_comment(line).trim_end();
        (!line.trim().is_empty()).then_some(line)
    });
    let mut dispatched = false;
    for line in meaningful.by_ref() {
        if line.starts_with("#!") || (!dispatched && line.starts_with("set ")) {
            continue;
        }
        if dispatched || !shell_line_invokes_tiber_snippet(line) {
            return false;
        }
        dispatched = true;
    }
    dispatched
}

fn shell_command_invokes_tiber_snippet(command: &str) -> bool {
    let command = command.trim();
    let command = command.strip_prefix("exec ").unwrap_or(command);
    let command = ["source ", ". ", "bash ", "sh "]
        .iter()
        .find_map(|prefix| command.strip_prefix(prefix))
        .unwrap_or(command)
        .trim_start();
    let Some(token) = command.split_whitespace().next() else {
        return false;
    };
    let token = token.trim_matches(|character| matches!(character, '"' | '\''));
    matches!(
        token,
        ".githooks/post-commit.tiber" | "./.githooks/post-commit.tiber"
    ) || token.ends_with("/.githooks/post-commit.tiber")
}

fn yaml_single_quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn trim_unquoted_comment(value: &str) -> &str {
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quote == Some('"') && character == '\\' {
            escaped = true;
            continue;
        }
        if matches!(character, '"' | '\'') {
            quote = if quote == Some(character) {
                None
            } else if quote.is_none() {
                Some(character)
            } else {
                quote
            };
            continue;
        }
        if character == '#'
            && quote.is_none()
            && value[..index]
                .chars()
                .next_back()
                .is_none_or(char::is_whitespace)
        {
            return value[..index].trim_end();
        }
    }
    value
}

fn new_task_id() -> String {
    generate_task_id()
}

fn command_recorded_at() -> String {
    std::env::var("GIT_AUTHOR_DATE")
        .ok()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(&value).ok())
        .map(|value| {
            value
                .with_timezone(&chrono::Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        })
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339())
}

fn generate_task_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let days = (now.as_secs() / 86_400) as i64;
    let (year, month, day) = civil_from_days(days);
    let mut entropy = now.as_nanos() ^ u128::from(std::process::id());
    let mut code = String::new();
    for _ in 0..4 {
        let index = (entropy % TASK_ID_ALPHABET.len() as u128) as usize;
        code.push(TASK_ID_ALPHABET[index] as char);
        entropy /= TASK_ID_ALPHABET.len() as u128;
    }
    format!("{year:04}{month:02}{day:02}-{code}")
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

fn closes_trailers(log: &str) -> Vec<String> {
    log.lines()
        .filter_map(|line| line.trim().strip_prefix("Closes:"))
        .map(str::trim)
        .filter(|task_ref| !task_ref.is_empty())
        .map(str::to_string)
        .collect()
}

fn is_retryable_push_failure(error: &Error) -> bool {
    match error {
        Error::Parse(message)
            if message.starts_with("event_version_conflict=")
                || message.contains("event_store_authoritative_ref_retry=true")
                // EventCore may have retried a concurrent claim internally.
                // The outer claim protocol then reloads and joins the winner.
                || message.contains("ci_recovery_claim_already_active") =>
        {
            true
        }
        Error::CommandFailed { args, stderr, .. } => {
            args.iter().any(|arg| arg == "push")
                && (stderr.contains("non-fast-forward")
                    || stderr.contains("fetch first")
                    || stderr.contains("incorrect old value provided")
                    || stderr.contains("stale info"))
        }
        _ => false,
    }
}

fn is_coordination_branch_creation_race(error: &Error) -> bool {
    matches!(
        error,
        Error::CommandFailed { args, stderr, .. }
            if args.iter().any(|arg| arg == "push")
                && stderr.contains("cannot lock ref")
                && stderr.contains("reference already exists")
    )
}

fn parse_subtask_ref(subtask_ref: &str) -> Result<String, Error> {
    if let Some(number) = subtask_ref.strip_prefix('s') {
        if !number.is_empty() && number.chars().all(|character| character.is_ascii_digit()) {
            return Ok(subtask_ref.to_string());
        }
    }
    let index = subtask_ref
        .parse::<usize>()
        .map_err(|error| Error::Parse(format!("invalid_subtask_ref source={error}")))?;
    if index == 0 {
        return Err(Error::Parse("invalid_subtask_ref zero=true".to_string()));
    }
    Ok(format!("s{index}"))
}

fn parse_one_based_usize(input: &str, kind: &str) -> Result<usize, Error> {
    let index = input
        .parse::<usize>()
        .map_err(|error| Error::Parse(format!("invalid_{kind}_index source={error}")))?;
    if index == 0 {
        return Err(Error::Parse(format!("invalid_{kind}_index zero=true")));
    }
    Ok(index)
}

fn parse_nonempty_text<'a>(input: &'a str, kind: &str) -> Result<&'a str, Error> {
    let text = input.trim();
    if text.is_empty() {
        return Err(Error::Parse(format!("{kind}_empty=true")));
    }
    if text.chars().any(char::is_control) {
        return Err(Error::Parse(format!("{kind}_invalid=true")));
    }
    Ok(text)
}

fn parse_task_section_body(input: &str) -> Result<String, Error> {
    let text = input.trim();
    if text.is_empty() {
        return Err(Error::Parse("section_empty=true".into()));
    }
    if text
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
    {
        return Err(Error::Parse(
            "section_invalid=true recovery=\"remove control characters other than newline or tab\""
                .into(),
        ));
    }
    if text.lines().any(|line| {
        matches!(
            line,
            "## Summary"
                | "## Context / Why"
                | "## Acceptance criteria"
                | "## Subtasks"
                | "## Notes / Log"
        )
    }) {
        return Err(Error::Parse(
            "section_reserved_heading=true recovery=\"demote or rename the embedded heading\""
                .into(),
        ));
    }
    Ok(text.to_string())
}

fn current_date_string() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let days = (now.as_secs() / 86_400) as i64;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn parse_safe_relative_path(path_ref: &str, kind: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(path_ref);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(Error::Parse(format!("invalid_{kind}_ref ref={path_ref}")));
    }
    Ok(path)
}

fn parse_doc_ref(doc_ref: &str) -> Result<PathBuf, Error> {
    let path = parse_safe_relative_path(doc_ref, "doc")?;
    let mut components = path.components();
    if components
        .next()
        .and_then(|component| component.as_os_str().to_str())
        != Some("docs")
        || components.next().is_none()
    {
        return Err(Error::Parse(format!("invalid_doc_ref ref={doc_ref}")));
    }
    Ok(path)
}

fn collect_docs(root: &Path, directory: &Path, docs: &mut Vec<String>) -> Result<(), Error> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_docs(root, &entry.path(), docs)?;
        } else if file_type.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "md")
        {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .map_err(|error| Error::Parse(format!("doc_prefix source={error}")))?;
            docs.push(path_to_entry(relative)?);
        }
    }
    Ok(())
}

fn parse_status(status: &str) -> Result<&str, Error> {
    if !STATUS_DIRS.contains(&status) {
        return Err(Error::Parse(format!("invalid_status status={status}")));
    }
    Ok(status)
}

fn path_to_entry(path: &Path) -> Result<String, Error> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| Error::Parse("path_utf8=false".to_string()))
}

fn expand_home(path: &Path) -> Result<PathBuf, Error> {
    let path = path_to_entry(path)?;
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME")
            .map_err(|error| Error::Parse(format!("home_unavailable source={error}")))?;
        Ok(PathBuf::from(home).join(rest))
    } else {
        Ok(PathBuf::from(path))
    }
}

fn tiber_launcher_path() -> Result<PathBuf, Error> {
    if let Ok(path) = std::env::var("TIBER_LAUNCHER_PATH") {
        return Ok(PathBuf::from(path));
    }

    let current_exe = std::env::current_exe()?;
    if current_exe
        .components()
        .any(|component| component.as_os_str() == "dist")
    {
        if let Some(plugin_root) = current_exe
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
        {
            let launcher = plugin_root.join("bin").join("tiber");
            if launcher.exists() {
                return Ok(launcher);
            }
        }
    }

    let source_plugin_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or_else(|| Error::Parse("plugin_root_unavailable=true".to_string()))?;
    Ok(source_plugin_root.join("bin").join("tiber"))
}

fn install_launcher(launcher: &Path, installed: &Path) -> Result<(), Error> {
    static INSTALL_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    let launcher = fs::canonicalize(launcher)?;
    let launcher = path_to_entry(&launcher)?;
    let launcher = launcher.replace('\'', "'\"'\"'");
    let parent = installed
        .parent()
        .ok_or_else(|| Error::Parse("install_target_parent_missing=true".to_string()))?;
    let sequence = INSTALL_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let staged = parent.join(format!(".tiber-install-{}-{sequence}", std::process::id()));
    let result = (|| -> Result<(), Error> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)?;
        write!(file, "#!/usr/bin/env bash\nexec '{launcher}' \"$@\"\n")?;
        #[cfg(unix)]
        fs::set_permissions(&staged, fs::Permissions::from_mode(0o755))?;
        file.sync_all()?;
        fs::hard_link(&staged, installed)?;
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    let _ = fs::remove_file(&staged);
    result
}

#[cfg(unix)]
fn is_executable(path: &Path) -> Result<bool, Error> {
    Ok(fs::metadata(path)?.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> Result<bool, Error> {
    Ok(path.is_file())
}

fn git_status<I, S>(args: I, cwd: Option<&Path>) -> Result<(), Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let _ = git_output(args, cwd)?;
    Ok(())
}

fn git_output<I, S>(args: I, cwd: Option<&Path>) -> Result<String, Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect::<Vec<_>>();
    let mut command = Command::new("git");
    command.args(&args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command.env("GIT_TERMINAL_PROMPT", "0");
    command.env("LC_ALL", "C");
    command.env("LANGUAGE", "C");
    command_output("git", &args, command.output()?)
}

fn lock_metadata() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("pid={}\ntimestamp={timestamp}\n", std::process::id())
}

fn lock_retry_duration(env_name: &str, default: Duration) -> Duration {
    std::env::var(env_name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(default)
}

fn lock_retry_remaining(started_at: Instant, timeout: Duration) -> bool {
    started_at.elapsed() < timeout
}

fn is_tiber_lock_busy(error: &Error) -> bool {
    matches!(error, Error::Parse(message) if message.starts_with("tiber_lock_busy "))
}

fn stale_lock_contents(path: &Path) -> Result<Option<String>, Error> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(Error::Io(error)),
    };
    if lock_contents_are_stale(&contents) {
        Ok(Some(contents))
    } else {
        Ok(None)
    }
}

fn lock_contents_are_stale(contents: &str) -> bool {
    let pid = contents
        .lines()
        .find_map(|line| line.strip_prefix("pid="))
        .and_then(|pid| pid.parse::<u32>().ok());
    if pid.is_some_and(process_is_gone) {
        return true;
    }
    let timestamp = contents
        .lines()
        .find_map(|line| line.strip_prefix("timestamp="))
        .and_then(|timestamp| timestamp.parse::<u64>().ok());
    timestamp.is_some_and(|timestamp| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH + Duration::from_secs(timestamp))
            .unwrap_or_default()
            > Duration::from_secs(60 * 60)
    })
}

#[cfg(unix)]
fn process_is_gone(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|status| !status.success())
}

#[cfg(not(unix))]
fn process_is_gone(_pid: u32) -> bool {
    false
}

fn command_output(
    program: &str,
    args: &[std::ffi::OsString],
    output: Output,
) -> Result<String, Error> {
    if output.status.success() {
        return String::from_utf8(output.stdout)
            .map_err(|error| Error::Parse(format!("utf8=false source={error}")));
    }

    Err(Error::CommandFailed {
        program: program.to_string(),
        args: args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect(),
        status: output.status.to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

#[cfg(test)]
mod lock_tests {
    use super::*;

    #[test]
    fn checked_model_has_registered_mappings_for_current_and_legacy_events() {
        let report = check_tiber_model().expect("registered Tiber event mappings");

        assert_eq!(report.status, eventcore::model::CheckStatus::Verified);
        assert!(
            report.warnings.is_empty(),
            "complete Tiber model must have no unconsumed provenance or other warnings: {:#?}",
            report.warnings
        );
        assert!(backlog_admission_allowed(4, 5));
        assert!(!backlog_admission_allowed(5, 5));
    }

    #[test]
    fn ci_recovery_phase_rejects_unmodeled_persisted_values() {
        assert_eq!(
            serde_json::from_str::<CiRecoveryPhase>("\"waiting-ci\"").unwrap(),
            CiRecoveryPhase::WaitingCi
        );
        assert!(serde_json::from_str::<CiRecoveryPhase>("\"paused\"").is_err());
    }

    fn final_review_fixture(
        review_id: &str,
        reviewer_identity: &str,
        outcome: FinalReviewOutcome,
    ) -> FinalReviewRecord {
        FinalReviewRecord {
            review_id: review_id.into(),
            reviewer_identity: reviewer_identity.into(),
            reviewer_type: "independent-final-review".into(),
            requested_scope: vec!["src/**".into()],
            scope: vec!["src/lib.rs".into()],
            commit_range: "1111111..2222222".into(),
            outcome,
            evidence: "verification-receipt".into(),
            timestamp: "2026-08-28T12:00:00Z".into(),
            source_fingerprint: "source-fingerprint".into(),
            requested_verification_scope: vec!["tests/**".into()],
            verification_scope: vec!["tests/final_review.rs".into()],
            verification_fingerprint: "verification-fingerprint".into(),
        }
    }

    fn final_review_event(board: &TiberBoardStream, review: FinalReviewRecord) -> TiberEvent {
        TiberEvent::TaskFinalReviewRecorded(TaskFinalReviewRecordedEvent {
            stream_id: eventcore::model::StreamIdentity::as_stream_id(board).clone(),
            stem: "task-a".into(),
            review,
        })
    }

    #[test]
    fn lifecycle_commands_reject_a_finding_refreshed_after_the_filesystem_gate() {
        let board = TiberBoardStream(stream_id(BOARD_STREAM).expect("board stream"));
        let clean_reviews = vec![
            final_review_fixture("review-1", "reviewer-1", FinalReviewOutcome::Clean),
            final_review_fixture("review-2", "reviewer-2", FinalReviewOutcome::Clean),
            final_review_fixture("review-3", "reviewer-3", FinalReviewOutcome::Clean),
        ];
        let finding = final_review_event(
            &board,
            final_review_fixture("review-4", "reviewer-4", FinalReviewOutcome::Finding),
        );
        let created = TiberEvent::TaskCreated(TaskCreatedEvent {
            stream_id: eventcore::model::StreamIdentity::as_stream_id(&board).clone(),
            task: Box::new(Task::new("task-a".into(), "Task".into(), "now".into())),
        });
        let ordered = TiberEvent::BoardReordered(TaskOrderEvent {
            stream_id: eventcore::model::StreamIdentity::as_stream_id(&board).clone(),
            order: vec!["task-a".into()],
        });
        let review_events = clean_reviews
            .iter()
            .cloned()
            .map(|review| final_review_event(&board, review))
            .collect::<Vec<_>>();

        let transition_request = TransitionTaskRequest::model_builder()
            .board(board.clone())
            .intent(TransitionTaskIntent {
                stem: "task-a".into(),
                status: "done".into(),
                claim: None,
                max_queued: None,
                minimum_clean_reviews: 3,
                expected_clean_reviews: clean_reviews.clone(),
                completion_snapshot: None,
            })
            .build();
        let transition = TransitionTask::model_builder()
            .board(TransitionTaskRequestToBoard::apply(
                transition_request.as_ref(),
            ))
            .intent(TransitionTaskRequestToIntent::apply(
                transition_request.as_ref(),
            ))
            .build();
        let mut transition_state = eventcore::model::ModelCommandLogic::evolve(
            transition.as_ref(),
            Default::default(),
            &created,
        );
        transition_state = eventcore::model::ModelCommandLogic::evolve(
            transition.as_ref(),
            transition_state,
            &ordered,
        );
        for event in &review_events {
            transition_state = eventcore::model::ModelCommandLogic::evolve(
                transition.as_ref(),
                transition_state,
                event,
            );
        }
        transition_state = eventcore::model::ModelCommandLogic::evolve(
            transition.as_ref(),
            transition_state,
            &finding,
        );
        let transition_error = match eventcore::CommandLogic::handle(&transition, transition_state)
        {
            Ok(_) => panic!("refreshed finding must block transition"),
            Err(error) => error,
        };
        assert!(transition_error
            .to_string()
            .contains("tiber.final_review_evidence_changed_during_completion"));

        let close_request = CloseTasksFromCommitTrailersRequest::model_builder()
            .board(board.clone())
            .intent(CloseTasksFromCommitTrailersIntent {
                stems: vec!["task-a".into()],
                minimum_clean_reviews: 3,
                expected_clean_reviews: BTreeMap::from([("task-a".into(), clean_reviews)]),
                completion_snapshots: BTreeMap::new(),
            })
            .build();
        let close = CloseTasksFromCommitTrailers::model_builder()
            .board(CloseTasksFromCommitTrailersRequestToBoard::apply(
                close_request.as_ref(),
            ))
            .intent(CloseTasksFromCommitTrailersRequestToIntent::apply(
                close_request.as_ref(),
            ))
            .build();
        let mut close_state = eventcore::model::ModelCommandLogic::evolve(
            close.as_ref(),
            Default::default(),
            &created,
        );
        close_state =
            eventcore::model::ModelCommandLogic::evolve(close.as_ref(), close_state, &ordered);
        for event in &review_events {
            close_state =
                eventcore::model::ModelCommandLogic::evolve(close.as_ref(), close_state, event);
        }
        close_state =
            eventcore::model::ModelCommandLogic::evolve(close.as_ref(), close_state, &finding);
        let close_error = match eventcore::CommandLogic::handle(&close, close_state) {
            Ok(_) => panic!("refreshed finding must block trailer closure"),
            Err(error) => error,
        };
        assert!(close_error
            .to_string()
            .contains("tiber.final_review_evidence_changed_during_completion"));
    }

    #[test]
    fn trailer_closure_decides_from_task_statuses_not_full_task_documents() {
        let board = TiberBoardStream(stream_id(BOARD_STREAM).expect("board stream"));
        let request = CloseTasksFromCommitTrailersRequest::model_builder()
            .board(board.clone())
            .intent(CloseTasksFromCommitTrailersIntent {
                stems: vec!["task-a".into()],
                minimum_clean_reviews: 0,
                expected_clean_reviews: BTreeMap::new(),
                completion_snapshots: BTreeMap::new(),
            })
            .build();
        let command = CloseTasksFromCommitTrailers::model_builder()
            .board(CloseTasksFromCommitTrailersRequestToBoard::apply(
                request.as_ref(),
            ))
            .intent(CloseTasksFromCommitTrailersRequestToIntent::apply(
                request.as_ref(),
            ))
            .build();
        let created = TiberEvent::TaskCreated(TaskCreatedEvent {
            stream_id: eventcore::model::StreamIdentity::as_stream_id(&board).clone(),
            task: Box::new(Task::new(
                "task-a".into(),
                "Original title".into(),
                "now".into(),
            )),
        });
        let ordered = TiberEvent::BoardReordered(TaskOrderEvent {
            stream_id: eventcore::model::StreamIdentity::as_stream_id(&board).clone(),
            order: vec!["task-a".into()],
        });
        let state = eventcore::model::ModelCommandLogic::evolve(
            command.as_ref(),
            Default::default(),
            &created,
        );
        let state = eventcore::model::ModelCommandLogic::evolve(command.as_ref(), state, &ordered);
        let emitted: Vec<TiberEvent> = eventcore::CommandLogic::handle(&command, state)
            .expect("open trailer-referenced task may close")
            .into();

        assert!(matches!(
            emitted.as_slice(),
            [TiberEvent::TasksClosedFromCommitTrailers(
                TasksClosedFromCommitTrailersEvent { stems, order, .. }
            )] if stems == &["task-a"] && order.is_empty()
        ));
    }

    #[test]
    fn task_collection_states_fold_only_the_command_target() {
        let board_stream = stream_id(BOARD_STREAM).expect("board stream");
        let mut target = Task::new("task-a".into(), "Target".into(), "now".into());
        target.subtasks.push(Subtask {
            id: "s1".into(),
            checked: false,
            title: "target subtask".into(),
            after: vec![],
        });
        target.acceptance.push(ChecklistItem {
            checked: false,
            text: "target criterion".into(),
        });
        let mut unrelated = Task::new("task-b".into(), "Unrelated".into(), "now".into());
        unrelated.subtasks.push(Subtask {
            id: "s9".into(),
            checked: false,
            title: "unrelated subtask".into(),
            after: vec![],
        });
        unrelated.acceptance.push(ChecklistItem {
            checked: false,
            text: "unrelated criterion".into(),
        });
        let target_created = TiberEvent::TaskCreated(TaskCreatedEvent {
            stream_id: board_stream.clone(),
            task: Box::new(target),
        });
        let unrelated_created = TiberEvent::TaskCreated(TaskCreatedEvent {
            stream_id: board_stream,
            task: Box::new(unrelated),
        });

        let subtask_state = evolve_subtask_state(
            evolve_subtask_state(
                AddSubtaskState {
                    board_order: Vec::new(),
                    board_task_stems: BTreeSet::new(),
                    target_subtasks: None,
                },
                "task-a",
                &target_created,
            ),
            "task-a",
            &unrelated_created,
        );
        let acceptance_state = evolve_acceptance_state(
            evolve_acceptance_state(
                AddAcceptanceState {
                    board_order: Vec::new(),
                    board_task_stems: BTreeSet::new(),
                    target_acceptance: None,
                },
                "task-a",
                &target_created,
            ),
            "task-a",
            &unrelated_created,
        );

        assert_eq!(
            subtask_state
                .target_subtasks
                .expect("target subtasks")
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["s1"]
        );
        assert_eq!(
            acceptance_state
                .target_acceptance
                .expect("target acceptance")
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>(),
            vec!["target criterion"]
        );
    }

    #[test]
    fn board_validation_decides_from_status_and_link_facts() {
        let board = TiberBoardStream(stream_id(BOARD_STREAM).expect("board stream"));
        let statuses = BTreeMap::from([
            ("task-a".into(), "backlog".into()),
            ("task-b".into(), "backlog".into()),
        ]);
        let links = BTreeMap::from([
            ("task-a".into(), (vec!["task-b".into()], vec![])),
            ("task-b".into(), (vec![], vec![])),
        ]);

        let plan = task_board_repair_plan(
            &board,
            &["task-a".into(), "task-b".into()],
            &BTreeSet::from(["task-a".into(), "task-b".into()]),
            &statuses,
            &links,
        );

        assert!(plan.order_change.is_none());
        assert!(matches!(
            plan.link_changes.as_slice(),
            [TaskLinksChangedEvent {
                stem,
                blocked_by,
                ..
            }] if stem == "task-b" && blocked_by == &["task-a"]
        ));
        assert!(matches!(
            plan.repairs.as_slice(),
            [ValidationRepair::ReciprocalLinkAdded { task, field, target }]
                if task == "task-b" && field == "blocked_by" && target == "task-a"
        ));
    }

    fn recovery_state_for_event_test() -> CiRecoveryState {
        CiRecoveryState {
            schema_version: 1,
            incident_id: "ci-123".into(),
            state: CiRecoveryPhase::Diagnosing,
            epoch: 1,
            trigger: CiRecoveryTrigger {
                run_id: "123".into(),
                run_url: "https://example.invalid/runs/123".into(),
                failed_sha: "abcdef".into(),
                workflow: "CI".into(),
                git_ref: "refs/heads/main".into(),
            },
            triggers: vec![],
            owner: CiRecoveryParticipant {
                host: "owner".into(),
                session: "session-1".into(),
            },
            lease_expires_at: 60,
            participants: vec![],
            assignments: vec![],
            failure_record: None,
            diagnosis: None,
            next_action: None,
            replacement: None,
            release_proof: None,
        }
    }

    fn claimed_event(stream_id: StreamId, state: &CiRecoveryState) -> TiberEvent {
        TiberEvent::CiRecoveryClaimed(CiRecoveryClaimedEvent {
            stream_id,
            schema_version: state.schema_version,
            incident_id: state.incident_id.clone(),
            trigger: state.trigger.clone().into(),
            owner: state.owner.clone().into(),
            lease_expires_at: state.lease_expires_at,
        })
    }

    fn claim_command(state: &CiRecoveryState) -> eventcore::model::ModeledCommand<ClaimCiRecovery> {
        let request = ClaimCiRecoveryRequest::model_builder()
            .stream(CiRecoveryStream(
                stream_id(CI_RECOVERY_STREAM).expect("recovery stream"),
            ))
            .intent(ClaimCiRecoveryIntent {
                incident_id: state.incident_id.clone(),
                schema_version: state.schema_version,
                trigger: state.trigger.clone(),
                owner: state.owner.clone(),
                lease_expires_at: state.lease_expires_at,
            })
            .build();
        ClaimCiRecovery::model_builder()
            .stream(ClaimCiRecoveryRequestToStream::apply(request.as_ref()))
            .intent(ClaimCiRecoveryRequestToIntent::apply(request.as_ref()))
            .build()
    }

    fn transfer_command(
        state: &CiRecoveryState,
        recipient: CiRecoveryParticipant,
        observed_at: u64,
    ) -> eventcore::model::ModeledCommand<TransferCiRecovery> {
        let request = TransferCiRecoveryRequest::model_builder()
            .stream(CiRecoveryStream(
                stream_id(CI_RECOVERY_STREAM).expect("recovery stream"),
            ))
            .intent(TransferCiRecoveryIntent {
                incident_id: state.incident_id.clone(),
                expected_epoch: state.epoch,
                caller: state.owner.clone(),
                recipient,
                observed_at,
                lease_expires_at: observed_at + CI_RECOVERY_LEASE_SECONDS,
            })
            .build();
        TransferCiRecovery::model_builder()
            .stream(TransferCiRecoveryRequestToStream::apply(request.as_ref()))
            .intent(TransferCiRecoveryRequestToIntent::apply(request.as_ref()))
            .build()
    }

    fn takeover_command(
        state: &CiRecoveryState,
        successor: CiRecoveryParticipant,
        observed_at: u64,
    ) -> eventcore::model::ModeledCommand<TakeOverCiRecovery> {
        let request = TakeOverCiRecoveryRequest::model_builder()
            .stream(CiRecoveryStream(
                stream_id(CI_RECOVERY_STREAM).expect("recovery stream"),
            ))
            .intent(TakeOverCiRecoveryIntent {
                incident_id: state.incident_id.clone(),
                expected_epoch: state.epoch,
                successor,
                observed_at,
                lease_expires_at: observed_at + CI_RECOVERY_LEASE_SECONDS,
            })
            .build();
        TakeOverCiRecovery::model_builder()
            .stream(TakeOverCiRecoveryRequestToStream::apply(request.as_ref()))
            .intent(TakeOverCiRecoveryRequestToIntent::apply(request.as_ref()))
            .build()
    }

    #[test]
    fn claim_ci_recovery_decides_the_opening_fact_from_minimal_folded_state() {
        let state = recovery_state_for_event_test();
        let command = claim_command(&state);
        let emitted: Vec<TiberEvent> =
            eventcore::CommandLogic::handle(&command, Default::default())
                .expect("first claim is allowed")
                .into();
        assert!(matches!(
            emitted.as_slice(),
            [TiberEvent::CiRecoveryClaimed(CiRecoveryClaimedEvent { incident_id, .. })]
                if incident_id == &state.incident_id
        ));

        let folded = eventcore::model::ModelCommandLogic::evolve(
            command.as_ref(),
            Default::default(),
            &emitted[0],
        );
        let error = match eventcore::CommandLogic::handle(&command, folded) {
            Ok(_) => panic!("second active claim is rejected"),
            Err(error) => error,
        };
        assert!(error
            .to_string()
            .contains("ci_recovery_claim_already_active"));
    }

    #[test]
    fn transfer_ci_recovery_decides_its_ownership_fact_from_incident_facts() {
        let state = recovery_state_for_event_test();
        let successor = CiRecoveryParticipant {
            host: "successor".into(),
            session: "session-2".into(),
        };
        let command = transfer_command(&state, successor.clone(), 30);
        let claim = claimed_event(
            stream_id(CI_RECOVERY_STREAM).expect("recovery stream"),
            &state,
        );
        let folded = eventcore::model::ModelCommandLogic::evolve(
            command.as_ref(),
            Default::default(),
            &claim,
        );
        let emitted: Vec<TiberEvent> = eventcore::CommandLogic::handle(&command, folded)
            .expect("owner with an active lease can transfer")
            .into();
        assert!(matches!(
            emitted.as_slice(),
            [TiberEvent::CiRecoveryTransferred(CiRecoveryTransferredEvent {
                owner,
                epoch: 2,
                lease_expires_at,
                participant: Some(participant),
                ..
            })] if CiRecoveryParticipant::from(owner.clone()) == successor
                && CiRecoveryParticipant::from(participant.clone()) == successor
                && *lease_expires_at == 30 + CI_RECOVERY_LEASE_SECONDS
        ));

        let expired = transfer_command(&state, successor, state.lease_expires_at);
        let folded = eventcore::model::ModelCommandLogic::evolve(
            expired.as_ref(),
            Default::default(),
            &claim,
        );
        let error = match eventcore::CommandLogic::handle(&expired, folded) {
            Ok(_) => panic!("an expired owner cannot transfer"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("ci_recovery_lease_expired"));
    }

    #[test]
    fn takeover_ci_recovery_decides_its_ownership_fact_only_after_lease_expiry() {
        let state = recovery_state_for_event_test();
        let successor = CiRecoveryParticipant {
            host: "successor".into(),
            session: "session-2".into(),
        };
        let claim = claimed_event(
            stream_id(CI_RECOVERY_STREAM).expect("recovery stream"),
            &state,
        );
        let active = takeover_command(&state, successor.clone(), state.lease_expires_at - 1);
        let folded = eventcore::model::ModelCommandLogic::evolve(
            active.as_ref(),
            Default::default(),
            &claim,
        );
        let error = match eventcore::CommandLogic::handle(&active, folded) {
            Ok(_) => panic!("an active lease cannot be taken over"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("ci_recovery_lease_active"));

        let expired = takeover_command(&state, successor.clone(), state.lease_expires_at);
        let folded = eventcore::model::ModelCommandLogic::evolve(
            expired.as_ref(),
            Default::default(),
            &claim,
        );
        let emitted: Vec<TiberEvent> = eventcore::CommandLogic::handle(&expired, folded)
            .expect("a joined successor can take over after the lease expires")
            .into();
        assert!(matches!(
            emitted.as_slice(),
            [TiberEvent::CiRecoveryTakenOver(CiRecoveryTakenOverEvent {
                owner,
                epoch: 2,
                participant: Some(participant),
                ..
            })] if CiRecoveryParticipant::from(owner.clone()) == successor
                && CiRecoveryParticipant::from(participant.clone()) == successor
        ));
    }

    #[test]
    fn typed_claim_and_join_facts_rebuild_the_recovery_projection() {
        let state = recovery_state_for_event_test();
        let stream = stream_id(CI_RECOVERY_STREAM).expect("recovery stream");
        let claim = claimed_event(stream.clone(), &state);
        let helper = CiRecoveryParticipant {
            host: "helper".into(),
            session: "session-2".into(),
        };
        let mut joined_state = state.clone();
        joined_state.participants.push(helper.clone());
        let join = TiberEvent::CiRecoveryJoined(CiRecoveryJoinedEvent {
            stream_id: stream.clone(),
            trigger: None,
            participant: Some(helper.clone().into()),
        });
        let mut transferred_state = joined_state.clone();
        transferred_state.owner = helper.clone();
        transferred_state.epoch = 2;
        transferred_state.lease_expires_at = 120;
        let transfer = TiberEvent::CiRecoveryTransferred(CiRecoveryTransferredEvent {
            stream_id: stream.clone(),
            owner: helper.clone().into(),
            epoch: 2,
            lease_expires_at: 120,
            participant: None,
        });
        let successor = CiRecoveryParticipant {
            host: "successor".into(),
            session: "session-3".into(),
        };
        let mut takeover_state = transferred_state.clone();
        takeover_state.owner = successor.clone();
        takeover_state.epoch = 3;
        takeover_state.lease_expires_at = 180;
        takeover_state.participants.push(successor.clone());
        let takeover = TiberEvent::CiRecoveryTakenOver(CiRecoveryTakenOverEvent {
            stream_id: stream,
            owner: successor.clone().into(),
            epoch: 3,
            lease_expires_at: 180,
            participant: Some(successor.clone().into()),
        });

        let mut projection = TiberProjection::default();
        apply_tiber_event(&mut projection, &claim).expect("fold claim");
        apply_tiber_event(&mut projection, &join).expect("fold join");
        apply_tiber_event(&mut projection, &transfer).expect("fold transfer");
        apply_tiber_event(&mut projection, &takeover).expect("fold takeover");
        let recovery = projection.ci_recovery.expect("recovery projection");
        assert_eq!(recovery.incident_id, "ci-123");
        assert_eq!(recovery.owner, successor);
        assert_eq!(recovery.epoch, 3);
        assert_eq!(recovery.lease_expires_at, 180);
        assert_eq!(recovery.participants, vec![state.owner, helper, successor]);
        assert!(matches!(claim, TiberEvent::CiRecoveryClaimed(_)));
        assert!(matches!(join, TiberEvent::CiRecoveryJoined(_)));
        assert!(matches!(transfer, TiberEvent::CiRecoveryTransferred(_)));
        assert!(matches!(takeover, TiberEvent::CiRecoveryTakenOver(_)));
    }

    #[test]
    fn typed_ci_transition_facts_rebuild_a_completed_recovery_without_snapshots() {
        let stream = stream_id(CI_RECOVERY_STREAM).expect("recovery stream");
        let opening = recovery_state_for_event_test();
        let claim = claimed_event(stream.clone(), &opening);

        let mut assigned_state = opening.clone();
        assigned_state.assignments.push(CiRecoveryAssignment {
            id: "a1".into(),
            owner_epoch: 1,
            assignee: opening.owner.clone(),
            capabilities: vec!["inspect".into()],
            scope: "failed job".into(),
            report: None,
        });
        let assigned = TiberEvent::CiRecoveryAssigned(CiRecoveryAssignedEvent {
            stream_id: stream.clone(),
            assignment: assigned_state.assignments[0].clone().into(),
        });

        let mut reported_state = assigned_state.clone();
        reported_state.assignments[0].report = Some(CiRecoveryReport {
            summary: "reproduced".into(),
            evidence: "log line".into(),
        });
        let reported = TiberEvent::CiRecoveryReported(CiRecoveryReportedEvent {
            stream_id: stream.clone(),
            assignment_id: "a1".into(),
            assignee: opening.owner.clone().into(),
            report: reported_state.assignments[0]
                .report
                .clone()
                .expect("report present")
                .into(),
        });

        let mut diagnosed_state = reported_state.clone();
        diagnosed_state.failure_record = Some(CiRecoveryFailureRecord {
            job: "test".into(),
            step: "run".into(),
            log_evidence: "failure".into(),
        });
        diagnosed_state.diagnosis = Some(CiRecoveryDiagnosis {
            cause: "test regression".into(),
            classification: CiRecoveryClassification::Caused,
        });
        let diagnosed = TiberEvent::CiRecoveryDiagnosed(CiRecoveryDiagnosedEvent {
            stream_id: stream.clone(),
            epoch: 1,
            owner: opening.owner.clone().into(),
            failure_record: diagnosed_state
                .failure_record
                .clone()
                .expect("failure present")
                .into(),
            diagnosis: diagnosed_state
                .diagnosis
                .clone()
                .expect("diagnosis present")
                .into(),
        });

        let mut action_state = diagnosed_state.clone();
        action_state.next_action = Some(CiRecoveryAction {
            kind: CiRecoveryActionKind::Repair,
            description: "repair the regression".into(),
        });
        action_state.state = CiRecoveryPhase::ActionSelected;
        let action = TiberEvent::CiRecoveryActionChosen(CiRecoveryActionChosenEvent {
            stream_id: stream.clone(),
            epoch: 1,
            owner: opening.owner.clone().into(),
            action: action_state
                .next_action
                .clone()
                .expect("action present")
                .into(),
        });

        let mut replacement_state = action_state.clone();
        replacement_state.replacement = Some(CiRecoveryReplacement {
            run_id: "124".into(),
            run_url: "https://example.invalid/runs/124".into(),
            sha: "fedcba".into(),
            status: CiRecoveryReplacementStatus::Running,
        });
        replacement_state.state = CiRecoveryPhase::WaitingCi;
        let replacement =
            TiberEvent::CiRecoveryReplacementRecorded(CiRecoveryReplacementRecordedEvent {
                stream_id: stream.clone(),
                epoch: 1,
                owner: opening.owner.clone().into(),
                replacement: replacement_state
                    .replacement
                    .clone()
                    .expect("replacement present")
                    .into(),
            });

        let mut resolved_state = replacement_state.clone();
        resolved_state.release_proof = Some(CiRecoveryReleaseProof {
            replacement_run_id: "124".into(),
            replacement_run_url: "https://example.invalid/runs/124".into(),
            sha: "fedcba".into(),
            terminal_status: "success".into(),
        });
        resolved_state.state = CiRecoveryPhase::Resolved;
        let resolved = TiberEvent::CiRecoveryResolved(CiRecoveryResolvedEvent {
            stream_id: stream,
            participant: opening.owner.clone().into(),
            proof: resolved_state
                .release_proof
                .clone()
                .expect("proof present")
                .into(),
        });

        let events = [
            claim,
            assigned,
            reported,
            diagnosed,
            action,
            replacement,
            resolved,
        ];
        let mut projection = TiberProjection::default();
        for event in &events {
            assert!(
                !matches!(event, TiberEvent::LegacyRecoveryStatePublished(_)),
                "fresh transition must not write a snapshot"
            );
            apply_tiber_event(&mut projection, event).expect("fold typed fact");
        }
        let recovery = projection.ci_recovery.expect("recovery projection");
        assert_eq!(recovery.state, CiRecoveryPhase::Resolved);
        assert_eq!(
            recovery.assignments[0]
                .report
                .as_ref()
                .expect("report")
                .summary,
            "reproduced"
        );
        assert_eq!(
            recovery.release_proof.expect("proof").terminal_status,
            "success"
        );
    }

    #[test]
    fn legacy_ci_snapshot_events_remain_replayable() {
        let state = recovery_state_for_event_test();
        let event = TiberEvent::LegacyRecoveryStatePublished(CiRecoveryEvent {
            stream_id: stream_id(CI_RECOVERY_STREAM).expect("recovery stream"),
            state: Box::new(state.snapshot()),
        });
        let mut projection = TiberProjection::default();
        apply_tiber_event(&mut projection, &event).expect("fold legacy snapshot");
        assert_eq!(
            projection
                .ci_recovery
                .expect("recovery projection")
                .incident_id,
            "ci-123"
        );
    }

    fn temporary_repository(label: &str) -> PathBuf {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "tiber-git-{label}-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("create temporary repository");
        let output = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&path)
            .output()
            .expect("initialize temporary repository");
        assert!(output.status.success(), "git init should succeed");
        path
    }

    fn assert_scope_limit_condition(limits: FinalReviewScopeLimits, condition: &str) {
        let root = temporary_repository(condition);
        fs::write(root.join("one.txt"), "12").expect("write first scoped file");
        fs::write(root.join("two.txt"), "34").expect("write second scoped file");

        let error =
            source_scope_fingerprint_with_limits(&root, &[".".to_string()], None, "source", limits)
                .expect_err("bounded scope must fail closed");

        assert!(
            error.to_string().contains(condition),
            "unexpected diagnostic: {error}"
        );
        fs::remove_dir_all(root).expect("remove temporary repository");
    }

    #[test]
    fn immutable_tree_fingerprint_batches_processes_and_reuses_cached_scope() {
        let root = temporary_repository("tree-batch-cache");
        fs::write(root.join("one.txt"), "one\n").expect("write first blob");
        fs::write(root.join("two.txt"), "two\n").expect("write second blob");
        for args in [
            vec!["config", "user.name", "Tiber Test"],
            vec!["config", "user.email", "tiber@example.invalid"],
            vec!["config", "commit.gpgsign", "false"],
            vec!["add", "one.txt", "two.txt"],
            vec!["commit", "-q", "-m", "Add blobs"],
        ] {
            let output = Command::new("git")
                .args(args)
                .current_dir(&root)
                .output()
                .expect("run Git fixture command");
            assert!(output.status.success());
        }
        let (_, tree_oid) = canonical_commit_snapshot(&root, "HEAD").expect("resolve tree");
        let requested = vec!["*.txt".to_string()];
        let retained = vec![
            ":(literal)one.txt".to_string(),
            ":(literal)two.txt".to_string(),
        ];
        let mut cache = TreeFinalReviewFingerprintCache::default();
        let before = final_review_tree_git_process_count();
        let first = cached_tree_scope_fingerprint(
            &root, &tree_oid, &requested, &retained, "source", &mut cache,
        )
        .expect("fingerprint tree");
        let after_first = final_review_tree_git_process_count();
        let second = cached_tree_scope_fingerprint(
            &root, &tree_oid, &requested, &retained, "source", &mut cache,
        )
        .expect("reuse tree fingerprint");
        let after_second = final_review_tree_git_process_count();

        assert_eq!(first, second);
        assert_eq!(
            after_first - before,
            4,
            "one read-tree, one pathspec materialization, one ls-tree, and one blob batch"
        );
        assert_eq!(after_second, after_first, "cache must spawn no Git process");
        fs::remove_dir_all(root).expect("remove temporary repository");
    }

    #[test]
    fn immutable_tree_fingerprint_cache_evicts_the_least_recently_used_scope() {
        let root = temporary_repository("tree-cache-bound");
        for (path, content) in [
            ("one.txt", "one\n"),
            ("two.txt", "two\n"),
            ("three.txt", "three\n"),
        ] {
            fs::write(root.join(path), content).expect("write cached blob");
        }
        for args in [
            vec!["config", "user.name", "Tiber Test"],
            vec!["config", "user.email", "tiber@example.invalid"],
            vec!["config", "commit.gpgsign", "false"],
            vec!["add", "one.txt", "two.txt", "three.txt"],
            vec!["commit", "-q", "-m", "Add cached blobs"],
        ] {
            let output = Command::new("git")
                .args(args)
                .current_dir(&root)
                .output()
                .expect("run Git fixture command");
            assert!(output.status.success());
        }
        let (_, tree_oid) = canonical_commit_snapshot(&root, "HEAD").expect("resolve tree");
        let mut cache = TreeFinalReviewFingerprintCache::with_max_entries(2);
        let retained = |path: &str| vec![format!(":(literal){path}")];

        for path in ["one.txt", "two.txt", "one.txt", "three.txt"] {
            cached_tree_scope_fingerprint(
                &root,
                &tree_oid,
                &[path.to_string()],
                &retained(path),
                "source",
                &mut cache,
            )
            .expect("cache immutable tree scope");
        }

        let one_key = tree_final_review_fingerprint_cache_key(
            &tree_oid,
            "source",
            &["one.txt".to_string()],
            &retained("one.txt"),
        );
        let two_key = tree_final_review_fingerprint_cache_key(
            &tree_oid,
            "source",
            &["two.txt".to_string()],
            &retained("two.txt"),
        );
        let three_key = tree_final_review_fingerprint_cache_key(
            &tree_oid,
            "source",
            &["three.txt".to_string()],
            &retained("three.txt"),
        );
        assert_eq!(cache.entries.len(), 2);
        assert!(cache.entries.contains_key(&one_key));
        assert!(!cache.entries.contains_key(&two_key));
        assert!(cache.entries.contains_key(&three_key));
        fs::remove_dir_all(root).expect("remove temporary repository");
    }

    #[test]
    fn immutable_tree_fingerprint_uses_attributes_from_selected_tree() {
        let root = temporary_repository("tree-attribute-source");
        fs::write(root.join("one.txt"), "one\n").expect("write first blob");
        fs::write(root.join("two.txt"), "two\n").expect("write second blob");
        fs::write(root.join(".gitattributes"), "*.txt reviewed\n")
            .expect("write committed attributes");
        for args in [
            vec!["config", "user.name", "Tiber Test"],
            vec!["config", "user.email", "tiber@example.invalid"],
            vec!["config", "commit.gpgsign", "false"],
            vec!["add", "one.txt", "two.txt", ".gitattributes"],
            vec!["commit", "-q", "-m", "Add attributed blobs"],
        ] {
            let output = Command::new("git")
                .args(args)
                .current_dir(&root)
                .output()
                .expect("run Git fixture command");
            assert!(output.status.success());
        }
        let requested = vec![":(attr:reviewed)".to_string()];
        let retained = vec![":(literal)one.txt".to_string()];
        let expected = source_scope_fingerprint(&root, &requested, None, "source")
            .expect("fingerprint reviewed worktree");
        let (_, tree_oid) = canonical_commit_snapshot(&root, "HEAD").expect("resolve tree");
        fs::write(root.join(".gitattributes"), "one.txt reviewed\n")
            .expect("diverge worktree attributes");
        fs::write(root.join(".git/info/attributes"), "two.txt -reviewed\n")
            .expect("diverge repository-local attributes");

        let (missing, actual) =
            tree_scope_fingerprint(&root, &tree_oid, &requested, &retained, "source")
                .expect("fingerprint immutable tree");

        assert!(missing.is_empty());
        assert_eq!(actual, expected);
        fs::remove_dir_all(root).expect("remove temporary repository");
    }

    #[test]
    fn immutable_tree_attribute_scope_fails_closed_without_git_support() {
        assert!(final_review_pathspec_uses_attributes(":(attr:reviewed)"));
        assert!(final_review_pathspec_uses_attributes(
            ":(top,attr:reviewed)src"
        ));
        assert!(!final_review_pathspec_uses_attributes(
            ":(literal)attr:reviewed"
        ));

        let mut unsupported = Command::new("sh");
        unsupported.args(["-c", "exit 129"]);
        let error = final_review_attribute_source_support_from_command(&mut unsupported, "source")
            .expect_err("unsupported Git must fail closed");

        assert!(error
            .to_string()
            .contains("tiber.final_review_source_attribute_source_unsupported"));
        assert!(error.to_string().contains("upgrade Git to 2.41 or newer"));
    }

    #[test]
    fn final_review_scope_fingerprinting_bounds_git_output_paths_and_content() {
        assert_scope_limit_condition(
            FinalReviewScopeLimits {
                git_output_bytes: 1,
                paths: 10,
                content_bytes: 10,
            },
            "condition=git_output_bytes",
        );
        assert_scope_limit_condition(
            FinalReviewScopeLimits {
                git_output_bytes: 1024,
                paths: 1,
                content_bytes: 10,
            },
            "condition=path_count",
        );
        assert_scope_limit_condition(
            FinalReviewScopeLimits {
                git_output_bytes: 1024,
                paths: 10,
                content_bytes: 1,
            },
            "condition=content_bytes",
        );
    }

    #[test]
    fn final_review_gitlink_status_stops_after_the_first_dirty_byte() {
        let mut producer = Command::new("sh");
        producer.args(["-c", "printf x; sleep 3 >/dev/null 2>&1"]);
        let started = Instant::now();

        assert_eq!(
            final_review_gitlink_is_dirty_from_command(&mut producer, FINAL_REVIEW_SCOPE_LIMITS)
                .expect("inspect streaming dirty-status producer"),
            Some(true)
        );
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "dirty status must stop after its first byte instead of waiting for producer completion"
        );
    }

    #[test]
    fn current_task_lock_excludes_a_legacy_sentinel_client_for_its_lifetime() {
        let root = temporary_repository("legacy-exclusion");
        let repository = GitRepository::at(&root);
        let lock = repository
            .acquire_lock()
            .expect("acquire current task lock");
        let legacy_path = repository
            .git_common_dir()
            .expect("resolve common directory")
            .join("tiber/tiber.lock");

        let legacy_attempt = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&legacy_path);
        assert_eq!(
            legacy_attempt
                .expect_err("legacy client must remain excluded")
                .kind(),
            std::io::ErrorKind::AlreadyExists
        );

        drop(lock);
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&legacy_path)
            .expect("legacy client may acquire after current lock release");
        fs::remove_dir_all(root).expect("remove temporary repository");
    }

    #[test]
    fn unfinished_legacy_sentinel_is_removed_on_initialization_failure() {
        let root = temporary_repository("sentinel-rollback");
        let path = root.join("tiber.lock");
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .expect("create unfinished sentinel");

        drop(LegacySentinel {
            file,
            path: path.clone(),
            metadata: None,
        });

        assert!(!path.exists(), "unfinished sentinel must roll back");
        fs::remove_dir_all(root).expect("remove temporary repository");
    }
}
