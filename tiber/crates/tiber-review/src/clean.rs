//! Clean-review acceptance as a business-domain intent with only the evidence
//! needed to decide that intent folded from durable review facts.

#![expect(
    clippy::missing_trait_methods,
    clippy::ref_option,
    clippy::single_call_fn,
    clippy::too_many_arguments,
    clippy::trivially_copy_pass_by_ref,
    reason = "EventCore mapping signatures and static stream discovery are defined by the checked-model API; clean decisions map every independent folded fact explicitly"
)]

use alloc::collections::BTreeSet;

use eventcore::{
    CommandError, ModelCommand, ModelInput, ModelOutput, ModelState, StreamId, mapping,
    model::{ModelCommandLogic, Modeled, ModeledEvents, StreamIdentity as _},
};

use crate::types::{FindingSeverity, RequiredCleanIterations, ReviewIteration};
use crate::{
    __eventcore_model_reviewevent, AssignmentKind, EvidenceId, FindingOccurrenceId, ReviewEvent,
    ReviewFact, ReviewLens, ReviewSnapshotId, ReviewStream, RiskAssessment, VerifierRoute,
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct RequiredWork {
    lens: ReviewLens,
    kind: AssignmentKind,
}

impl RequiredWork {
    const fn new(lens: ReviewLens, kind: AssignmentKind) -> Self {
        Self { lens, kind }
    }
}

#[derive(ModelInput)]
struct AcceptCleanReviewRequest {
    #[model(origin)]
    stream: ReviewStream,
    #[model(origin)]
    snapshot: ReviewSnapshotId,
    #[model(origin)]
    evidence_id: EvidenceId,
}

/// Accepts an exact source snapshot only after all routed work is current and
/// every blocking finding occurrence has separately verified resolution.
#[derive(ModelCommand)]
struct AcceptCleanReview {
    #[stream]
    stream: ReviewStream,
    snapshot: ReviewSnapshotId,
    evidence_id: EvidenceId,
}

mapping! { AcceptCleanRequestToStream: AcceptCleanReviewRequest.stream => AcceptCleanReview.stream using clone; }
mapping! { AcceptCleanRequestToSnapshot: AcceptCleanReviewRequest.snapshot => AcceptCleanReview.snapshot using clone; }
mapping! { AcceptCleanRequestToEvidence: AcceptCleanReviewRequest.evidence_id => AcceptCleanReview.evidence_id using clone; }

#[derive(ModelState)]
struct AcceptCleanReviewState {
    #[model(default)]
    current_snapshot: Option<ReviewSnapshotId>,
    #[model(default)]
    required_work: BTreeSet<RequiredWork>,
    #[model(default)]
    accepted_work: BTreeSet<RequiredWork>,
    #[model(default)]
    unresolved_blockers: BTreeSet<FindingOccurrenceId>,
    #[model(default)]
    current_findings: BTreeSet<FindingOccurrenceId>,
    #[model(default)]
    delta_reassessment_pending: bool,
    #[model(default)]
    current_iteration: ReviewIteration,
    #[model(default)]
    consecutive_clean_iterations: u32,
    #[model(default)]
    required_clean_iterations: RequiredCleanIterations,
    #[model(default)]
    already_clean: bool,
}

#[derive(Clone)]
struct CleanReviewContext {
    current_snapshot: Option<ReviewSnapshotId>,
    required_work: BTreeSet<RequiredWork>,
    accepted_work: BTreeSet<RequiredWork>,
    unresolved_blockers: BTreeSet<FindingOccurrenceId>,
    current_findings: BTreeSet<FindingOccurrenceId>,
    delta_reassessment_pending: bool,
    current_iteration: ReviewIteration,
    consecutive_clean_iterations: u32,
    required_clean_iterations: RequiredCleanIterations,
    already_clean: bool,
}

#[derive(ModelOutput)]
struct AcceptCleanReviewDecision {
    context: CleanReviewContext,
}

fn clean_review_context(
    snapshot: &Option<ReviewSnapshotId>,
    required: &BTreeSet<RequiredWork>,
    accepted: &BTreeSet<RequiredWork>,
    blockers: &BTreeSet<FindingOccurrenceId>,
    findings: &BTreeSet<FindingOccurrenceId>,
    delta_reassessment_pending: &bool,
    iteration: &ReviewIteration,
    consecutive_clean_iterations: &u32,
    required_clean_iterations: &RequiredCleanIterations,
    already_clean: &bool,
) -> CleanReviewContext {
    CleanReviewContext {
        current_snapshot: snapshot.clone(),
        required_work: required.clone(),
        accepted_work: accepted.clone(),
        unresolved_blockers: blockers.clone(),
        current_findings: findings.clone(),
        delta_reassessment_pending: *delta_reassessment_pending,
        current_iteration: *iteration,
        consecutive_clean_iterations: *consecutive_clean_iterations,
        required_clean_iterations: *required_clean_iterations,
        already_clean: *already_clean,
    }
}

mapping! {
    AcceptCleanStateToDecision:
        (AcceptCleanReviewState.current_snapshot, AcceptCleanReviewState.required_work, AcceptCleanReviewState.accepted_work, AcceptCleanReviewState.unresolved_blockers, AcceptCleanReviewState.current_findings, AcceptCleanReviewState.delta_reassessment_pending, AcceptCleanReviewState.current_iteration, AcceptCleanReviewState.consecutive_clean_iterations, AcceptCleanReviewState.required_clean_iterations, AcceptCleanReviewState.already_clean) => AcceptCleanReviewDecision.context
        using clean_review_context;
}

fn clean_review_stream(stream: &ReviewStream) -> StreamId {
    stream.as_stream_id().clone()
}

mapping! { AcceptCleanStreamToEvent: AcceptCleanReview.stream => ReviewEvent.stream using clean_review_stream; }

fn review_iteration_fact(
    snapshot: &ReviewSnapshotId,
    evidence_id: &EvidenceId,
    context: &CleanReviewContext,
) -> Result<ReviewFact, CommandError> {
    if context.current_snapshot.as_ref() != Some(snapshot)
        || context.already_clean
        || context.delta_reassessment_pending
        || !context.required_work.is_subset(&context.accepted_work)
        || !context.unresolved_blockers.is_empty()
    {
        return Err(CommandError::ValidationError(
            "review_clean_not_authorized".to_owned(),
        ));
    }
    Ok(ReviewFact::ReviewIterationCompleted {
        snapshot: snapshot.clone(),
        iteration: context.current_iteration,
        clean: context.current_findings.is_empty(),
        evidence_id: evidence_id.clone(),
    })
}

fn clean_review_fact(
    snapshot: &ReviewSnapshotId,
    evidence_id: &EvidenceId,
    context: &CleanReviewContext,
) -> Result<ReviewFact, CommandError> {
    let next_clean_streak = context.consecutive_clean_iterations.saturating_add(1);
    if context.current_snapshot.as_ref() != Some(snapshot)
        || context.already_clean
        || context.delta_reassessment_pending
        || !context.required_work.is_subset(&context.accepted_work)
        || !context.unresolved_blockers.is_empty()
        || !context.current_findings.is_empty()
        || next_clean_streak < context.required_clean_iterations.get()
    {
        return Err(CommandError::ValidationError(
            "review_clean_not_authorized".to_owned(),
        ));
    }
    Ok(ReviewFact::CleanReviewAccepted {
        snapshot: snapshot.clone(),
        evidence_id: evidence_id.clone(),
    })
}

mapping! {
    AcceptCleanToIterationFact:
        (AcceptCleanReview.snapshot, AcceptCleanReview.evidence_id, AcceptCleanReviewDecision.context) => ReviewEvent.fact
        using try review_iteration_fact, error = CommandError;
}

mapping! {
    AcceptCleanToFact:
        (AcceptCleanReview.snapshot, AcceptCleanReview.evidence_id, AcceptCleanReviewDecision.context) => ReviewEvent.fact
        using try clean_review_fact, error = CommandError;
}

fn advance_iteration(state: &mut AcceptCleanReviewState, clean: bool) {
    state.accepted_work.clear();
    state.current_findings.clear();
    state.consecutive_clean_iterations = if clean {
        state.consecutive_clean_iterations.saturating_add(1)
    } else {
        0
    };
    if let Ok(next_iteration) = state.current_iteration.next() {
        state.current_iteration = next_iteration;
    }
}

fn begin_review(
    state: &mut AcceptCleanReviewState,
    snapshot: &ReviewSnapshotId,
    assessment: &RiskAssessment,
) {
    state.current_snapshot = Some(snapshot.clone());
    state.required_work.clear();
    state.accepted_work.clear();
    state.unresolved_blockers.clear();
    state.current_findings.clear();
    state.delta_reassessment_pending = false;
    state.current_iteration = ReviewIteration::FIRST;
    state.consecutive_clean_iterations = 0;
    state.required_clean_iterations = assessment.required_clean_iterations();
    state.already_clean = false;
    for route in assessment.routes() {
        state.required_work.insert(RequiredWork::new(
            route.lens().clone(),
            AssignmentKind::Lens,
        ));
        if matches!(route.verifier(), VerifierRoute::Required { .. }) {
            state.required_work.insert(RequiredWork::new(
                route.lens().clone(),
                AssignmentKind::Verifier,
            ));
        }
    }
}

impl ModelCommandLogic for AcceptCleanReview {
    type Event = ReviewEvent;
    type State = AcceptCleanReviewState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut folded = state.into_inner();
        match &event.fact {
            ReviewFact::RiskAssessed {
                snapshot,
                assessment,
            } => {
                begin_review(&mut folded, snapshot, assessment);
            }
            ReviewFact::AssignmentIssued { assignment }
                if assignment.id().kind() == AssignmentKind::DeltaRisk
                    && folded.current_snapshot.as_ref() == Some(assignment.snapshot())
                    && folded.current_iteration == assignment.id().iteration() =>
            {
                folded.delta_reassessment_pending = true;
            }
            ReviewFact::AssignmentResultAccepted { result }
                if folded.current_snapshot.as_ref() == Some(result.snapshot())
                    && folded.current_iteration == result.assignment_id().iteration() =>
            {
                folded.accepted_work.insert(RequiredWork::new(
                    result.assignment_id().lens().clone(),
                    result.assignment_id().kind(),
                ));
                for finding in result.findings() {
                    folded.current_findings.insert(finding.id().clone());
                    if finding.severity() == FindingSeverity::Blocking {
                        folded.unresolved_blockers.insert(finding.id().clone());
                    }
                }
            }
            ReviewFact::AssignmentResultRejected {
                snapshot,
                iteration,
                ..
            } if folded.current_snapshot.as_ref() == Some(snapshot)
                && folded.current_iteration == *iteration =>
            {
                advance_iteration(&mut folded, false);
            }
            ReviewFact::DeltaReassessed {
                from_snapshot,
                to_snapshot,
                ..
            } if folded.current_snapshot.as_ref() == Some(from_snapshot) => {
                folded.current_snapshot = Some(to_snapshot.clone());
                folded.delta_reassessment_pending = false;
                folded.accepted_work.clear();
                folded.current_findings.clear();
                folded.consecutive_clean_iterations = 0;
                if let Ok(next_iteration) = folded.current_iteration.next() {
                    folded.current_iteration = next_iteration;
                }
                folded.already_clean = false;
            }
            ReviewFact::FindingResolutionVerified { finding_id, .. } => {
                folded.unresolved_blockers.remove(finding_id);
            }
            ReviewFact::ReviewIterationCompleted {
                snapshot,
                iteration,
                clean,
                ..
            } if folded.current_snapshot.as_ref() == Some(snapshot)
                && folded.current_iteration == *iteration =>
            {
                advance_iteration(&mut folded, *clean);
            }
            ReviewFact::CleanReviewAccepted { snapshot, .. }
                if folded.current_snapshot.as_ref() == Some(snapshot) =>
            {
                folded.already_clean = true;
            }
            ReviewFact::AssignmentIssued { .. }
            | ReviewFact::AssignmentResultAccepted { .. }
            | ReviewFact::AssignmentResultRejected { .. }
            | ReviewFact::AssignmentSuperseded { .. }
            | ReviewFact::DeltaReassessed { .. }
            | ReviewFact::ReviewIterationCompleted { .. }
            | ReviewFact::CleanReviewAccepted { .. } => {}
        }
        Modeled::from_built(folded)
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, CommandError> {
        let decision = AcceptCleanReviewDecision::model_builder()
            .context(AcceptCleanStateToDecision::apply((
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
            )))
            .build();
        let context = &decision.as_ref().context;
        if context.current_snapshot.as_ref() != Some(&self.snapshot) {
            return Err(CommandError::ValidationError(
                "review_clean_snapshot_mismatch".to_owned(),
            ));
        }
        if context.already_clean {
            return Err(CommandError::ValidationError(
                "review_snapshot_already_clean".to_owned(),
            ));
        }
        if context.delta_reassessment_pending {
            return Err(CommandError::ValidationError(
                "review_delta_reassessment_pending".to_owned(),
            ));
        }
        if !context.required_work.is_subset(&context.accepted_work) {
            return Err(CommandError::ValidationError(
                "review_required_work_incomplete".to_owned(),
            ));
        }
        if !context.unresolved_blockers.is_empty() {
            return Err(CommandError::ValidationError(
                "review_blocking_findings_unresolved".to_owned(),
            ));
        }
        let mut events = ModeledEvents::one(
            ReviewEvent::model_builder()
                .stream(AcceptCleanStreamToEvent::apply(self))
                .fact(AcceptCleanToIterationFact::apply((
                    self,
                    self,
                    decision.as_ref(),
                ))?)
                .build(),
        );
        let next_clean_streak = context.consecutive_clean_iterations.saturating_add(1);
        if context.current_findings.is_empty()
            && next_clean_streak >= context.required_clean_iterations.get()
        {
            events.push(
                ReviewEvent::model_builder()
                    .stream(AcceptCleanStreamToEvent::apply(self))
                    .fact(AcceptCleanToFact::apply((self, self, decision.as_ref()))?)
                    .build(),
            );
        }
        Ok(events)
    }
}

/// Records one complete review iteration and accepts the snapshot only after
/// the configured consecutive finding-free streak is satisfied.
#[must_use]
pub fn accept_clean_review(
    stream: ReviewStream,
    snapshot: ReviewSnapshotId,
    evidence_id: EvidenceId,
) -> impl eventcore::CommandLogic<Event = ReviewEvent> {
    let request = AcceptCleanReviewRequest::model_builder()
        .stream(stream)
        .snapshot(snapshot)
        .evidence_id(evidence_id)
        .build();
    AcceptCleanReview::model_builder()
        .stream(AcceptCleanRequestToStream::apply(request.as_ref()))
        .snapshot(AcceptCleanRequestToSnapshot::apply(request.as_ref()))
        .evidence_id(AcceptCleanRequestToEvidence::apply(request.as_ref()))
        .build()
}
