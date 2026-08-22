//! Result-acceptance intent with only the matching issued assignment and its
//! completion status folded for the decision.

#![expect(
    clippy::missing_trait_methods,
    clippy::ref_option,
    clippy::single_call_fn,
    clippy::struct_excessive_bools,
    clippy::trivially_copy_pass_by_ref,
    reason = "EventCore mapping signatures and static stream discovery are defined by the checked-model API; independent durable closure causes remain explicit in the command projection"
)]

use alloc::collections::BTreeSet;

use eventcore::{
    CommandError, ModelCommand, ModelInput, ModelOutput, ModelState, StreamId, mapping,
    model::{ModelCommandLogic, Modeled, ModeledEvents, StreamIdentity as _},
};

use crate::types::{
    FindingOccurrence, MAX_ASSIGNMENT_RESULT_FINDINGS, MAX_REVIEW_LENSES, ReviewLens,
};
use crate::{
    __eventcore_model_reviewevent, AssignmentKind, AssignmentResult, AssignmentResultRejection,
    ReviewAssignment, ReviewEvent, ReviewFact, ReviewStream,
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
    #[model(default)]
    iteration_closed: bool,
    #[model(default)]
    assessed_lenses: BTreeSet<ReviewLens>,
}

#[derive(Clone)]
struct AcceptResultContext {
    assignment: Option<ReviewAssignment>,
    already_completed: bool,
    superseded: bool,
    invalidated_by_delta: bool,
    iteration_closed: bool,
    assessed_lenses: BTreeSet<ReviewLens>,
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
    iteration_closed: &bool,
    assessed_lenses: &BTreeSet<ReviewLens>,
) -> AcceptResultContext {
    AcceptResultContext {
        assignment: assignment.clone(),
        already_completed: *completed,
        superseded: *superseded,
        invalidated_by_delta: *invalidated_by_delta,
        iteration_closed: *iteration_closed,
        assessed_lenses: assessed_lenses.clone(),
    }
}
mapping! { AcceptResultStateToDecision: (AcceptAssignmentResultState.assignment, AcceptAssignmentResultState.already_completed, AcceptAssignmentResultState.superseded, AcceptAssignmentResultState.invalidated_by_delta, AcceptAssignmentResultState.iteration_closed, AcceptAssignmentResultState.assessed_lenses) => AcceptResultDecision.context using result_context; }

fn result_stream(stream: &ReviewStream) -> StreamId {
    stream.as_stream_id().clone()
}
mapping! { AcceptResultStreamToEvent: AcceptAssignmentResult.stream => ReviewEvent.stream using result_stream; }

fn accepted_result_fact(
    result: &AssignmentResult,
    context: &AcceptResultContext,
) -> Result<ReviewFact, CommandError> {
    let assignment = context
        .assignment
        .as_ref()
        .ok_or_else(|| CommandError::ValidationError("review_result_not_authorized".to_owned()))?;
    if context.already_completed
        || context.superseded
        || context.invalidated_by_delta
        || context.iteration_closed
    {
        return Err(CommandError::ValidationError(
            "review_result_not_authorized".to_owned(),
        ));
    }
    if assignment.snapshot() != result.snapshot()
        || assignment.agent_id() != result.agent_id()
        || assignment.model_role() != result.model_role()
        || assignment.context_receipt() != result.context_receipt()
        || assignment.lifecycle_receipt() != result.lifecycle_receipt()
    {
        return Ok(ReviewFact::AssignmentResultRejected {
            assignment_id: assignment.id().clone(),
            snapshot: assignment.snapshot().clone(),
            iteration: assignment.id().iteration(),
            reason: AssignmentResultRejection::ProvenanceMismatch,
        });
    }
    if result.findings().len() > MAX_ASSIGNMENT_RESULT_FINDINGS {
        return Ok(ReviewFact::AssignmentResultRejected {
            assignment_id: assignment.id().clone(),
            snapshot: assignment.snapshot().clone(),
            iteration: assignment.id().iteration(),
            reason: AssignmentResultRejection::FindingCountExceeded,
        });
    }
    let finding_ids = result
        .findings()
        .iter()
        .map(FindingOccurrence::id)
        .collect::<BTreeSet<_>>();
    if finding_ids.len() != result.findings().len()
        || result
            .findings()
            .iter()
            .any(|finding| finding.id().assignment_id() != result.assignment_id())
    {
        return Ok(ReviewFact::AssignmentResultRejected {
            assignment_id: assignment.id().clone(),
            snapshot: assignment.snapshot().clone(),
            iteration: assignment.id().iteration(),
            reason: AssignmentResultRejection::FindingIdentityInvalid,
        });
    }
    let classifications = result.delta_classifications();
    let classified_lenses = classifications
        .iter()
        .map(|classification| classification.lens().clone())
        .collect::<BTreeSet<_>>();
    let classifications_valid = if assignment.id().kind() == AssignmentKind::DeltaRisk {
        classifications.len() <= MAX_REVIEW_LENSES
            && classified_lenses.len() == classifications.len()
            && classified_lenses == context.assessed_lenses
    } else {
        classifications.is_empty()
    };
    if !classifications_valid {
        return Ok(ReviewFact::AssignmentResultRejected {
            assignment_id: assignment.id().clone(),
            snapshot: assignment.snapshot().clone(),
            iteration: assignment.id().iteration(),
            reason: AssignmentResultRejection::DeltaClassificationInvalid,
        });
    }
    Ok(ReviewFact::AssignmentResultAccepted {
        result: result.clone(),
    })
}
mapping! { AcceptResultToFact: (AcceptAssignmentResult.result, AcceptResultDecision.context) => ReviewEvent.fact using try accepted_result_fact, error = CommandError; }

fn finding_iteration_completed_fact(result: &AssignmentResult) -> ReviewFact {
    ReviewFact::ReviewIterationCompleted {
        snapshot: result.snapshot().clone(),
        iteration: result.assignment_id().iteration(),
        clean: false,
        evidence_id: result.evidence_id().clone(),
    }
}
mapping! { FindingResultToIterationCompleted: AcceptAssignmentResult.result => ReviewEvent.fact using finding_iteration_completed_fact; }

impl ModelCommandLogic for AcceptAssignmentResult {
    type Event = ReviewEvent;
    type State = AcceptAssignmentResultState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut folded = state.into_inner();
        match &event.fact {
            ReviewFact::RiskAssessed { assessment, .. } => {
                folded.assessed_lenses = assessment
                    .routes()
                    .iter()
                    .map(|route| route.lens().clone())
                    .collect();
            }
            ReviewFact::AssignmentIssued { assignment }
                if assignment.id() == self.result.assignment_id() =>
            {
                folded.assignment = Some(assignment.clone());
                folded.invalidated_by_delta = false;
                folded.iteration_closed = false;
            }
            ReviewFact::AssignmentResultAccepted { result }
                if result.assignment_id() == self.result.assignment_id() =>
            {
                folded.already_completed = true;
            }
            ReviewFact::AssignmentResultRejected { assignment_id, .. }
                if assignment_id == self.result.assignment_id() =>
            {
                folded.already_completed = true;
            }
            ReviewFact::AssignmentResultRejected {
                snapshot,
                iteration,
                ..
            } if folded.assignment.as_ref().is_some_and(|assignment| {
                assignment.snapshot() == snapshot && assignment.id().iteration() == *iteration
            }) =>
            {
                folded.iteration_closed = true;
            }
            ReviewFact::AssignmentSuperseded { assignment_id, .. }
                if assignment_id == self.result.assignment_id() =>
            {
                folded.superseded = true;
            }
            ReviewFact::DeltaReassessed { from_snapshot, .. }
                if folded.assignment.as_ref().is_some_and(|assignment| {
                    assignment.snapshot() == from_snapshot
                        && assignment.id().iteration() == self.result.assignment_id().iteration()
                }) =>
            {
                folded.invalidated_by_delta = true;
            }
            ReviewFact::ReviewIterationCompleted {
                snapshot,
                iteration,
                ..
            } if folded.assignment.as_ref().is_some_and(|assignment| {
                assignment.snapshot() == snapshot && assignment.id().iteration() == *iteration
            }) =>
            {
                folded.iteration_closed = true;
            }
            ReviewFact::AssignmentIssued { .. }
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
        let decision = AcceptResultDecision::model_builder()
            .context(AcceptResultStateToDecision::apply((
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
            )))
            .build();
        let context = &decision.as_ref().context;
        if context.assignment.is_none() {
            return Err(CommandError::ValidationError(
                "review_assignment_not_issued".to_owned(),
            ));
        }
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
        if context.iteration_closed {
            return Err(CommandError::ValidationError(
                "review_assignment_iteration_closed".to_owned(),
            ));
        }
        let closes_iteration = matches!(
            AcceptResultToFact::apply((self, decision.as_ref()))?.into_inner(),
            ReviewFact::AssignmentResultAccepted { result } if !result.findings().is_empty()
        );
        let mut events = ModeledEvents::one(
            ReviewEvent::model_builder()
                .stream(AcceptResultStreamToEvent::apply(self))
                .fact(AcceptResultToFact::apply((self, decision.as_ref()))?)
                .build(),
        );
        if closes_iteration {
            events.push(
                ReviewEvent::model_builder()
                    .stream(AcceptResultStreamToEvent::apply(self))
                    .fact(FindingResultToIterationCompleted::apply(self))
                    .build(),
            );
        }
        Ok(events)
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
