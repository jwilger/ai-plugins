//! Assignment-supersession intent with only the named assignment's issued,
//! completion, and supersession facts folded for its decision.

#![expect(
    clippy::missing_trait_methods,
    clippy::ref_option,
    clippy::single_call_fn,
    clippy::trivially_copy_pass_by_ref,
    reason = "EventCore mapping signatures and static stream discovery are defined by the checked-model API"
)]

use eventcore::{
    CommandError, ModelCommand, ModelInput, ModelOutput, ModelState, StreamId, mapping,
    model::{ModelCommandLogic, Modeled, ModeledEvents, StreamIdentity as _},
};

use crate::{
    __eventcore_model_reviewevent, AssignmentId, EvidenceId, ReviewEvent, ReviewFact,
    ReviewSnapshotId, ReviewStream,
};

#[derive(ModelInput)]
struct SupersedeAssignmentRequest {
    #[model(origin)]
    stream: ReviewStream,
    #[model(origin)]
    assignment_id: AssignmentId,
    #[model(origin)]
    reason: EvidenceId,
}

/// Supersedes one incomplete assignment after failure, cancellation, or staleness.
#[derive(ModelCommand)]
struct SupersedeAssignment {
    #[stream]
    stream: ReviewStream,
    assignment_id: AssignmentId,
    reason: EvidenceId,
}

mapping! { SupersedeRequestToStream: SupersedeAssignmentRequest.stream => SupersedeAssignment.stream using clone; }
mapping! { SupersedeRequestToAssignment: SupersedeAssignmentRequest.assignment_id => SupersedeAssignment.assignment_id using clone; }
mapping! { SupersedeRequestToReason: SupersedeAssignmentRequest.reason => SupersedeAssignment.reason using clone; }

#[derive(ModelState)]
struct SupersedeAssignmentState {
    #[model(default)]
    issued: bool,
    #[model(default)]
    assignment_snapshot: Option<ReviewSnapshotId>,
    #[model(default)]
    completed: bool,
    #[model(default)]
    superseded: bool,
}

#[derive(Clone)]
struct SupersedeContext {
    issued: bool,
    assignment_snapshot: Option<ReviewSnapshotId>,
    completed: bool,
    superseded: bool,
}

#[derive(ModelOutput)]
struct SupersedeDecision {
    context: SupersedeContext,
}

fn supersede_context(
    issued: &bool,
    assignment_snapshot: &Option<ReviewSnapshotId>,
    completed: &bool,
    superseded: &bool,
) -> SupersedeContext {
    SupersedeContext {
        issued: *issued,
        assignment_snapshot: assignment_snapshot.clone(),
        completed: *completed,
        superseded: *superseded,
    }
}
mapping! { SupersedeStateToDecision: (SupersedeAssignmentState.issued, SupersedeAssignmentState.assignment_snapshot, SupersedeAssignmentState.completed, SupersedeAssignmentState.superseded) => SupersedeDecision.context using supersede_context; }

fn supersede_stream(stream: &ReviewStream) -> StreamId {
    stream.as_stream_id().clone()
}
mapping! { SupersedeStreamToEvent: SupersedeAssignment.stream => ReviewEvent.stream using supersede_stream; }

fn superseded_fact(
    assignment_id: &AssignmentId,
    reason: &EvidenceId,
    context: &SupersedeContext,
) -> Result<ReviewFact, CommandError> {
    if !context.issued
        || context.assignment_snapshot.is_none()
        || context.completed
        || context.superseded
    {
        return Err(CommandError::ValidationError(
            "review_supersession_not_authorized".to_owned(),
        ));
    }
    let replacement_attempt = assignment_id
        .attempt()
        .next()
        .map_err(|error| CommandError::ValidationError(error.code().to_owned()))?;
    Ok(ReviewFact::AssignmentSuperseded {
        assignment_id: assignment_id.clone(),
        replacement_attempt,
        reason: reason.clone(),
    })
}
mapping! { SupersedeToFact: (SupersedeAssignment.assignment_id, SupersedeAssignment.reason, SupersedeDecision.context) => ReviewEvent.fact using try superseded_fact, error = CommandError; }

impl ModelCommandLogic for SupersedeAssignment {
    type Event = ReviewEvent;
    type State = SupersedeAssignmentState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut folded = state.into_inner();
        match &event.fact {
            ReviewFact::AssignmentIssued { assignment }
                if assignment.id() == &self.assignment_id =>
            {
                folded.issued = true;
                folded.assignment_snapshot = Some(assignment.snapshot().clone());
            }
            ReviewFact::AssignmentResultAccepted { result }
                if result.assignment_id() == &self.assignment_id =>
            {
                folded.completed = true;
            }
            ReviewFact::AssignmentResultRejected { assignment_id, .. }
                if assignment_id == &self.assignment_id =>
            {
                folded.completed = true;
            }
            ReviewFact::AssignmentResultRejected {
                snapshot,
                iteration,
                ..
            } if folded.assignment_snapshot.as_ref() == Some(snapshot)
                && self.assignment_id.iteration() == *iteration =>
            {
                folded.completed = true;
            }
            ReviewFact::DeltaReassessed { from_snapshot, .. }
                if folded.assignment_snapshot.as_ref() == Some(from_snapshot) =>
            {
                folded.completed = true;
            }
            ReviewFact::ReviewIterationCompleted {
                snapshot,
                iteration,
                ..
            } if folded.assignment_snapshot.as_ref() == Some(snapshot)
                && self.assignment_id.iteration() == *iteration =>
            {
                folded.completed = true;
            }
            ReviewFact::CleanReviewAccepted { snapshot, .. }
                if folded.assignment_snapshot.as_ref() == Some(snapshot) =>
            {
                folded.completed = true;
            }
            ReviewFact::AssignmentSuperseded { assignment_id, .. }
                if assignment_id == &self.assignment_id =>
            {
                folded.superseded = true;
            }
            ReviewFact::RiskAssessed { .. }
            | ReviewFact::AssignmentIssued { .. }
            | ReviewFact::AssignmentResultAccepted { .. }
            | ReviewFact::AssignmentResultRejected { .. }
            | ReviewFact::AssignmentSuperseded { .. }
            | ReviewFact::DeltaReassessed { .. }
            | ReviewFact::FindingResolutionVerified { .. }
            | ReviewFact::ReviewIterationCompleted { .. }
            | ReviewFact::CleanReviewAccepted { .. } => {}
        }
        Modeled::from_built(folded)
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, CommandError> {
        let decision = SupersedeDecision::model_builder()
            .context(SupersedeStateToDecision::apply((
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
            )))
            .build();
        let context = &decision.as_ref().context;
        if !context.issued || context.assignment_snapshot.is_none() {
            return Err(CommandError::ValidationError(
                "review_assignment_not_issued".to_owned(),
            ));
        }
        if context.completed {
            return Err(CommandError::ValidationError(
                "review_completed_assignment_cannot_be_superseded".to_owned(),
            ));
        }
        if context.superseded {
            return Err(CommandError::ValidationError(
                "review_assignment_already_superseded".to_owned(),
            ));
        }
        Ok(ModeledEvents::one(
            ReviewEvent::model_builder()
                .stream(SupersedeStreamToEvent::apply(self))
                .fact(SupersedeToFact::apply((self, self, decision.as_ref()))?)
                .build(),
        ))
    }
}

/// Builds the checked assignment-supersession command.
#[must_use]
pub fn supersede_assignment(
    stream: ReviewStream,
    assignment_id: AssignmentId,
    reason: EvidenceId,
) -> impl eventcore::CommandLogic<Event = ReviewEvent> {
    let request = SupersedeAssignmentRequest::model_builder()
        .stream(stream)
        .assignment_id(assignment_id)
        .reason(reason)
        .build();
    SupersedeAssignment::model_builder()
        .stream(SupersedeRequestToStream::apply(request.as_ref()))
        .assignment_id(SupersedeRequestToAssignment::apply(request.as_ref()))
        .reason(SupersedeRequestToReason::apply(request.as_ref()))
        .build()
}
