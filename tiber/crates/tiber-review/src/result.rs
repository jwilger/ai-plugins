//! Result-acceptance intent with only the matching issued assignment and its
//! completion status folded for the decision.

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

use crate::types::FindingOccurrence;
use crate::{
    __eventcore_model_reviewevent, AssignmentResult, ReviewAssignment, ReviewEvent, ReviewFact,
    ReviewStream,
};

#[derive(ModelInput)]
struct AcceptAssignmentResultRequest {
    #[model(origin)]
    stream: ReviewStream,
    #[model(origin)]
    result: AssignmentResult,
}

/// Accepts one result with exact scheduler-issued provenance.
#[derive(ModelCommand)]
struct AcceptAssignmentResult {
    #[stream]
    stream: ReviewStream,
    result: AssignmentResult,
}

mapping! { AcceptResultRequestToStream: AcceptAssignmentResultRequest.stream => AcceptAssignmentResult.stream using clone; }
mapping! { AcceptResultRequestToResult: AcceptAssignmentResultRequest.result => AcceptAssignmentResult.result using clone; }

#[derive(ModelState)]
struct AcceptAssignmentResultState {
    #[model(default)]
    assignment: Option<ReviewAssignment>,
    #[model(default)]
    already_completed: bool,
    #[model(default)]
    superseded: bool,
    #[model(default)]
    invalidated_by_delta: bool,
}

#[derive(Clone)]
struct AcceptResultContext {
    assignment: Option<ReviewAssignment>,
    already_completed: bool,
    superseded: bool,
    invalidated_by_delta: bool,
}

#[derive(ModelOutput)]
struct AcceptResultDecision {
    context: AcceptResultContext,
}

fn result_context(
    assignment: &Option<ReviewAssignment>,
    completed: &bool,
    superseded: &bool,
    invalidated_by_delta: &bool,
) -> AcceptResultContext {
    AcceptResultContext {
        assignment: assignment.clone(),
        already_completed: *completed,
        superseded: *superseded,
        invalidated_by_delta: *invalidated_by_delta,
    }
}
mapping! { AcceptResultStateToDecision: (AcceptAssignmentResultState.assignment, AcceptAssignmentResultState.already_completed, AcceptAssignmentResultState.superseded, AcceptAssignmentResultState.invalidated_by_delta) => AcceptResultDecision.context using result_context; }

fn result_stream(stream: &ReviewStream) -> StreamId {
    stream.as_stream_id().clone()
}
mapping! { AcceptResultStreamToEvent: AcceptAssignmentResult.stream => ReviewEvent.stream using result_stream; }

fn accepted_result_fact(
    result: &AssignmentResult,
    context: &AcceptResultContext,
) -> Result<ReviewFact, CommandError> {
    if context.assignment.is_none()
        || context.already_completed
        || context.superseded
        || context.invalidated_by_delta
    {
        return Err(CommandError::ValidationError(
            "review_result_not_authorized".to_owned(),
        ));
    }
    Ok(ReviewFact::AssignmentResultAccepted {
        result: result.clone(),
    })
}
mapping! { AcceptResultToFact: (AcceptAssignmentResult.result, AcceptResultDecision.context) => ReviewEvent.fact using try accepted_result_fact, error = CommandError; }

impl ModelCommandLogic for AcceptAssignmentResult {
    type Event = ReviewEvent;
    type State = AcceptAssignmentResultState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut folded = state.into_inner();
        match &event.fact {
            ReviewFact::AssignmentIssued { assignment }
                if assignment.id() == self.result.assignment_id() =>
            {
                folded.assignment = Some(assignment.clone());
                folded.invalidated_by_delta = false;
            }
            ReviewFact::AssignmentResultAccepted { result }
                if result.assignment_id() == self.result.assignment_id() =>
            {
                folded.already_completed = true;
            }
            ReviewFact::AssignmentSuperseded { assignment_id, .. }
                if assignment_id == self.result.assignment_id() =>
            {
                folded.superseded = true;
            }
            ReviewFact::DeltaReassessed {
                affected_lenses, ..
            } if folded.assignment.is_some()
                && affected_lenses.contains(self.result.assignment_id().lens()) =>
            {
                folded.invalidated_by_delta = true;
            }
            ReviewFact::RiskAssessed { .. }
            | ReviewFact::AssignmentIssued { .. }
            | ReviewFact::AssignmentResultAccepted { .. }
            | ReviewFact::AssignmentSuperseded { .. }
            | ReviewFact::DeltaReassessed { .. }
            | ReviewFact::FindingResolutionVerified { .. }
            | ReviewFact::CleanReviewAccepted { .. } => {}
        }
        Modeled::from_built(folded)
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, CommandError> {
        let decision = AcceptResultDecision::model_builder()
            .context(AcceptResultStateToDecision::apply((
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
            )))
            .build();
        let context = &decision.as_ref().context;
        let assignment = context.assignment.as_ref().ok_or_else(|| {
            CommandError::ValidationError("review_assignment_not_issued".to_owned())
        })?;
        if context.already_completed {
            return Err(CommandError::ValidationError(
                "review_assignment_already_completed".to_owned(),
            ));
        }
        if context.superseded {
            return Err(CommandError::ValidationError(
                "review_assignment_superseded".to_owned(),
            ));
        }
        if context.invalidated_by_delta {
            return Err(CommandError::ValidationError(
                "review_assignment_invalidated_by_delta".to_owned(),
            ));
        }
        if assignment.snapshot() != self.result.snapshot()
            || assignment.agent_id() != self.result.agent_id()
            || assignment.model_role() != self.result.model_role()
            || assignment.context_receipt() != self.result.context_receipt()
            || assignment.lifecycle_receipt() != self.result.lifecycle_receipt()
        {
            return Err(CommandError::ValidationError(
                "review_assignment_provenance_mismatch".to_owned(),
            ));
        }
        let finding_ids = self
            .result
            .findings()
            .iter()
            .map(FindingOccurrence::id)
            .collect::<BTreeSet<_>>();
        if finding_ids.len() != self.result.findings().len()
            || self
                .result
                .findings()
                .iter()
                .any(|finding| finding.id().assignment_id() != self.result.assignment_id())
        {
            return Err(CommandError::ValidationError(
                "review_finding_identity_invalid".to_owned(),
            ));
        }
        Ok(ModeledEvents::one(
            ReviewEvent::model_builder()
                .stream(AcceptResultStreamToEvent::apply(self))
                .fact(AcceptResultToFact::apply((self, decision.as_ref()))?)
                .build(),
        ))
    }
}

/// Builds the checked result-acceptance command.
#[must_use]
pub fn accept_assignment_result(
    stream: ReviewStream,
    result: AssignmentResult,
) -> impl eventcore::CommandLogic<Event = ReviewEvent> {
    let request = AcceptAssignmentResultRequest::model_builder()
        .stream(stream)
        .result(result)
        .build();
    AcceptAssignmentResult::model_builder()
        .stream(AcceptResultRequestToStream::apply(request.as_ref()))
        .result(AcceptResultRequestToResult::apply(request.as_ref()))
        .build()
}
