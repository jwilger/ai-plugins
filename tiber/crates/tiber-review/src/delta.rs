//! Delta-reassessment intent with only the assessed lenses and current source
//! snapshot folded for its decision.

#![expect(
    clippy::missing_trait_methods,
    clippy::ref_option,
    clippy::single_call_fn,
    clippy::trivially_copy_pass_by_ref,
    reason = "EventCore mapping signatures and static stream discovery are defined by the checked-model API"
)]

use alloc::collections::BTreeSet;

use eventcore::{
    CommandError, ModelCommand, ModelInput, ModelOutput, ModelState, StreamId, mapping,
    model::{ModelCommandLogic, Modeled, ModeledEvents, StreamIdentity as _},
};

use crate::{
    __eventcore_model_reviewevent, AssignmentId, AssignmentKind, AssignmentResult,
    ReviewAssignment, ReviewEvent, ReviewFact, ReviewLens, ReviewSnapshotId, ReviewStream,
};

#[derive(ModelInput)]
struct ReassessDeltaRequest {
    #[model(origin)]
    stream: ReviewStream,
    #[model(origin)]
    from_snapshot: ReviewSnapshotId,
    #[model(origin)]
    to_snapshot: ReviewSnapshotId,
    #[model(origin)]
    delta_assignment_id: AssignmentId,
}

/// Records how an exact source-content delta affects assessed review lenses.
#[derive(ModelCommand)]
struct ReassessDelta {
    #[stream]
    stream: ReviewStream,
    from_snapshot: ReviewSnapshotId,
    to_snapshot: ReviewSnapshotId,
    delta_assignment_id: AssignmentId,
}

mapping! { ReassessDeltaRequestToStream: ReassessDeltaRequest.stream => ReassessDelta.stream using clone; }
mapping! { ReassessDeltaRequestToFromSnapshot: ReassessDeltaRequest.from_snapshot => ReassessDelta.from_snapshot using clone; }
mapping! { ReassessDeltaRequestToToSnapshot: ReassessDeltaRequest.to_snapshot => ReassessDelta.to_snapshot using clone; }
mapping! { ReassessDeltaRequestToAssignment: ReassessDeltaRequest.delta_assignment_id => ReassessDelta.delta_assignment_id using clone; }

#[derive(ModelState)]
struct ReassessDeltaState {
    #[model(default)]
    current_snapshot: Option<ReviewSnapshotId>,
    #[model(default)]
    assessed_lenses: BTreeSet<ReviewLens>,
    #[model(default)]
    accepted_delta_result: Option<AssignmentResult>,
    #[model(default)]
    delta_assignment: Option<ReviewAssignment>,
    #[model(default)]
    iteration_closed: bool,
}

#[derive(Clone)]
struct ReassessDeltaContext {
    current_snapshot: Option<ReviewSnapshotId>,
    assessed_lenses: BTreeSet<ReviewLens>,
    accepted_delta_result: Option<AssignmentResult>,
    delta_assignment: Option<ReviewAssignment>,
    iteration_closed: bool,
}

#[derive(ModelOutput)]
struct ReassessDeltaDecision {
    context: ReassessDeltaContext,
}

fn delta_context(
    current_snapshot: &Option<ReviewSnapshotId>,
    assessed_lenses: &BTreeSet<ReviewLens>,
    result: &Option<AssignmentResult>,
    assignment: &Option<ReviewAssignment>,
    iteration_closed: &bool,
) -> ReassessDeltaContext {
    ReassessDeltaContext {
        current_snapshot: current_snapshot.clone(),
        assessed_lenses: assessed_lenses.clone(),
        accepted_delta_result: result.clone(),
        delta_assignment: assignment.clone(),
        iteration_closed: *iteration_closed,
    }
}

mapping! {
    ReassessDeltaStateToDecision:
        (ReassessDeltaState.current_snapshot, ReassessDeltaState.assessed_lenses, ReassessDeltaState.accepted_delta_result, ReassessDeltaState.delta_assignment, ReassessDeltaState.iteration_closed) => ReassessDeltaDecision.context
        using delta_context;
}

fn delta_stream(stream: &ReviewStream) -> StreamId {
    stream.as_stream_id().clone()
}

mapping! { ReassessDeltaStreamToEvent: ReassessDelta.stream => ReviewEvent.stream using delta_stream; }

fn delta_reassessed_fact(
    from_snapshot: &ReviewSnapshotId,
    to_snapshot: &ReviewSnapshotId,
    context: &ReassessDeltaContext,
) -> Result<ReviewFact, CommandError> {
    let result = context
        .accepted_delta_result
        .as_ref()
        .ok_or_else(|| CommandError::ValidationError("review_delta_result_required".to_owned()))?;
    let affected_lenses = result
        .delta_classifications()
        .iter()
        .filter(|classification| classification.affected())
        .map(|classification| classification.lens().clone())
        .collect();
    Ok(ReviewFact::DeltaReassessed {
        from_snapshot: from_snapshot.clone(),
        to_snapshot: to_snapshot.clone(),
        affected_lenses,
        evidence_id: result.evidence_id().clone(),
    })
}

mapping! {
    ReassessDeltaToFact:
        (ReassessDelta.from_snapshot, ReassessDelta.to_snapshot, ReassessDeltaDecision.context) => ReviewEvent.fact
        using try delta_reassessed_fact, error = CommandError;
}

impl ModelCommandLogic for ReassessDelta {
    type Event = ReviewEvent;
    type State = ReassessDeltaState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut folded = state.into_inner();
        if let ReviewFact::RiskAssessed {
            snapshot,
            assessment,
        } = &event.fact
        {
            folded.current_snapshot = Some(snapshot.clone());
            folded.assessed_lenses = assessment
                .routes()
                .iter()
                .map(|route| route.lens().clone())
                .collect();
        }
        if let ReviewFact::DeltaReassessed { to_snapshot, .. } = &event.fact {
            folded.current_snapshot = Some(to_snapshot.clone());
        }
        if let ReviewFact::AssignmentResultAccepted { result } = &event.fact
            && result.assignment_id() == &self.delta_assignment_id
            && result.assignment_id().kind() == AssignmentKind::DeltaRisk
        {
            folded.accepted_delta_result = Some(result.clone());
        }
        if let ReviewFact::AssignmentIssued { assignment } = &event.fact
            && assignment.id() == &self.delta_assignment_id
            && assignment.id().kind() == AssignmentKind::DeltaRisk
        {
            folded.delta_assignment = Some(assignment.clone());
            folded.iteration_closed = false;
        }
        if let ReviewFact::AssignmentResultRejected {
            snapshot,
            iteration,
            ..
        } = &event.fact
            && folded.delta_assignment.as_ref().is_some_and(|assignment| {
                assignment.snapshot() == snapshot && assignment.id().iteration() == *iteration
            })
        {
            folded.iteration_closed = true;
        }
        if let ReviewFact::ReviewIterationCompleted {
            snapshot,
            iteration,
            ..
        } = &event.fact
            && folded.delta_assignment.as_ref().is_some_and(|assignment| {
                assignment.snapshot() == snapshot && assignment.id().iteration() == *iteration
            })
        {
            folded.iteration_closed = true;
        }
        if let ReviewFact::CleanReviewAccepted { snapshot, .. } = &event.fact
            && folded
                .delta_assignment
                .as_ref()
                .is_some_and(|assignment| assignment.snapshot() == snapshot)
        {
            folded.iteration_closed = true;
        }
        Modeled::from_built(folded)
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, CommandError> {
        let decision = ReassessDeltaDecision::model_builder()
            .context(ReassessDeltaStateToDecision::apply((
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
            )))
            .build();
        let context = &decision.as_ref().context;
        if context.iteration_closed {
            return Err(CommandError::ValidationError(
                "review_delta_iteration_closed".to_owned(),
            ));
        }
        let current_snapshot = context.current_snapshot.as_ref().ok_or_else(|| {
            CommandError::ValidationError("review_risk_assessment_required".to_owned())
        })?;
        if current_snapshot != &self.from_snapshot {
            return Err(CommandError::ValidationError(
                "review_delta_from_snapshot_mismatch".to_owned(),
            ));
        }
        let assignment = context.delta_assignment.as_ref().ok_or_else(|| {
            CommandError::ValidationError("review_delta_assignment_required".to_owned())
        })?;
        if assignment.snapshot() != &self.from_snapshot
            || assignment.target_snapshot() != Some(&self.to_snapshot)
        {
            return Err(CommandError::ValidationError(
                "review_delta_snapshot_authority_mismatch".to_owned(),
            ));
        }
        let result = context.accepted_delta_result.as_ref().ok_or_else(|| {
            CommandError::ValidationError("review_delta_result_required".to_owned())
        })?;
        let classified = result
            .delta_classifications()
            .iter()
            .map(|classification| classification.lens().clone())
            .collect::<BTreeSet<_>>();
        if classified.len() != result.delta_classifications().len()
            || classified != context.assessed_lenses
        {
            return Err(CommandError::ValidationError(
                "review_delta_classification_incomplete".to_owned(),
            ));
        }
        Ok(ModeledEvents::one(
            ReviewEvent::model_builder()
                .stream(ReassessDeltaStreamToEvent::apply(self))
                .fact(ReassessDeltaToFact::apply((self, self, decision.as_ref()))?)
                .build(),
        ))
    }
}

/// Builds the checked delta-reassessment command from parsed boundary values.
#[must_use]
pub fn reassess_delta(
    stream: ReviewStream,
    from_snapshot: ReviewSnapshotId,
    to_snapshot: ReviewSnapshotId,
    delta_assignment_id: AssignmentId,
) -> impl eventcore::CommandLogic<Event = ReviewEvent> {
    let request = ReassessDeltaRequest::model_builder()
        .stream(stream)
        .from_snapshot(from_snapshot)
        .to_snapshot(to_snapshot)
        .delta_assignment_id(delta_assignment_id)
        .build();
    ReassessDelta::model_builder()
        .stream(ReassessDeltaRequestToStream::apply(request.as_ref()))
        .from_snapshot(ReassessDeltaRequestToFromSnapshot::apply(request.as_ref()))
        .to_snapshot(ReassessDeltaRequestToToSnapshot::apply(request.as_ref()))
        .delta_assignment_id(ReassessDeltaRequestToAssignment::apply(request.as_ref()))
        .build()
}
