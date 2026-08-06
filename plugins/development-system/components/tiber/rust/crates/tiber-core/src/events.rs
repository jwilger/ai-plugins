//! Tiber's authoritative domain-event vocabulary.
//!
//! The model has four semantic stream families:
//! `tiber:repository` records format initialization, `tiber:board` owns
//! lifecycle membership and strict backlog order, `tiber:task:<id>` owns one
//! task's details and history, and `tiber:ci-recovery` owns the repository-wide
//! CI incident. Commands that affect more than one family emit one atomic
//! multi-stream append. Task and CI events are intentionally in the same enum
//! because they share one EventCore store and the single `tiber` Git branch.
//!
//! Every mutating behavior has a named event below. Projections fold these
//! events into typed task, board, and recovery state; no Markdown snapshot is
//! authoritative. Adding a mutator therefore requires adding or deliberately
//! reusing a semantic event and extending the corresponding fold.

use crate::task::{ChecklistItem, Claim, Note, Subtask, Task, ValidationRepair};
use eventcore_types::{Event, StreamId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum TiberEvent {
    RepositoryInitialized {
        stream_id: StreamId,
    },
    TaskCreated {
        stream_id: StreamId,
        task: Box<Task>,
    },
    TaskTransitioned {
        stream_id: StreamId,
        stem: String,
        status: String,
        claim: Option<Claim>,
    },
    TaskPriorityChanged {
        stream_id: StreamId,
        order: Vec<String>,
    },
    TaskLinksChanged {
        stream_id: StreamId,
        stem: String,
        blocks: Vec<String>,
        blocked_by: Vec<String>,
    },
    TaskSubtaskAdded {
        stream_id: StreamId,
        stem: String,
        subtask: Subtask,
    },
    TaskSubtaskChecked {
        stream_id: StreamId,
        stem: String,
        subtask_id: String,
        checked: bool,
    },
    TaskDetailsUpdated {
        stream_id: StreamId,
        stem: String,
        title: String,
        tags: Vec<String>,
        summary: String,
        context: String,
    },
    TaskClaimChanged {
        stream_id: StreamId,
        stem: String,
        claim: Option<Claim>,
    },
    TaskPullRequestChanged {
        stream_id: StreamId,
        stem: String,
        url: Option<String>,
        status: Option<String>,
    },
    TaskAcceptanceAdded {
        stream_id: StreamId,
        stem: String,
        item: ChecklistItem,
    },
    TaskAcceptanceChecked {
        stream_id: StreamId,
        stem: String,
        index: usize,
        checked: bool,
    },
    TaskAcceptanceRemoved {
        stream_id: StreamId,
        stem: String,
        index: usize,
    },
    TaskNoteAdded {
        stream_id: StreamId,
        stem: String,
        note: Note,
    },
    TaskValidationRepaired {
        stream_id: StreamId,
        repairs: Vec<ValidationRepair>,
    },
    TaskClosedFromTrailer {
        stream_id: StreamId,
        stem: String,
    },
    TaskRemoved {
        stream_id: StreamId,
        stem: String,
    },
    BoardReordered {
        stream_id: StreamId,
        order: Vec<String>,
    },
    TaskStatePublished {
        stream_id: StreamId,
    },
    CiRecoveryClaimed {
        stream_id: StreamId,
        state: Box<serde_json::Value>,
    },
    CiRecoveryJoined {
        stream_id: StreamId,
        state: Box<serde_json::Value>,
    },
    CiRecoveryTransferred {
        stream_id: StreamId,
        state: Box<serde_json::Value>,
    },
    CiRecoveryTakenOver {
        stream_id: StreamId,
        state: Box<serde_json::Value>,
    },
    CiRecoveryAssigned {
        stream_id: StreamId,
        state: Box<serde_json::Value>,
    },
    CiRecoveryReported {
        stream_id: StreamId,
        state: Box<serde_json::Value>,
    },
    CiRecoveryHeartbeatRecorded {
        stream_id: StreamId,
        state: Box<serde_json::Value>,
    },
    CiRecoveryDiagnosed {
        stream_id: StreamId,
        state: Box<serde_json::Value>,
    },
    CiRecoveryActionChosen {
        stream_id: StreamId,
        state: Box<serde_json::Value>,
    },
    CiRecoveryReplacementRecorded {
        stream_id: StreamId,
        state: Box<serde_json::Value>,
    },
    CiRecoveryResolved {
        stream_id: StreamId,
        state: Box<serde_json::Value>,
    },
    RecoveryStatePublished {
        stream_id: StreamId,
        state: Box<serde_json::Value>,
    },
}

impl TiberEvent {
    pub fn stream_id_value(&self) -> &StreamId {
        match self {
            Self::RepositoryInitialized { stream_id }
            | Self::TaskCreated { stream_id, .. }
            | Self::TaskTransitioned { stream_id, .. }
            | Self::TaskPriorityChanged { stream_id, .. }
            | Self::TaskLinksChanged { stream_id, .. }
            | Self::TaskSubtaskAdded { stream_id, .. }
            | Self::TaskSubtaskChecked { stream_id, .. }
            | Self::TaskDetailsUpdated { stream_id, .. }
            | Self::TaskClaimChanged { stream_id, .. }
            | Self::TaskPullRequestChanged { stream_id, .. }
            | Self::TaskAcceptanceAdded { stream_id, .. }
            | Self::TaskAcceptanceChecked { stream_id, .. }
            | Self::TaskAcceptanceRemoved { stream_id, .. }
            | Self::TaskNoteAdded { stream_id, .. }
            | Self::TaskValidationRepaired { stream_id, .. }
            | Self::TaskClosedFromTrailer { stream_id, .. }
            | Self::TaskRemoved { stream_id, .. }
            | Self::BoardReordered { stream_id, .. }
            | Self::TaskStatePublished { stream_id, .. }
            | Self::CiRecoveryClaimed { stream_id, .. }
            | Self::CiRecoveryJoined { stream_id, .. }
            | Self::CiRecoveryTransferred { stream_id, .. }
            | Self::CiRecoveryTakenOver { stream_id, .. }
            | Self::CiRecoveryAssigned { stream_id, .. }
            | Self::CiRecoveryReported { stream_id, .. }
            | Self::CiRecoveryHeartbeatRecorded { stream_id, .. }
            | Self::CiRecoveryDiagnosed { stream_id, .. }
            | Self::CiRecoveryActionChosen { stream_id, .. }
            | Self::CiRecoveryReplacementRecorded { stream_id, .. }
            | Self::CiRecoveryResolved { stream_id, .. }
            | Self::RecoveryStatePublished { stream_id, .. } => stream_id,
        }
    }
}

impl Event for TiberEvent {
    fn stream_id(&self) -> &StreamId {
        self.stream_id_value()
    }
    fn event_type_name() -> &'static str {
        "tiber.domain_event"
    }
}
