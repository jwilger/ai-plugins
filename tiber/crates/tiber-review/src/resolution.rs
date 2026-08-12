//! Finding-resolution intent with only the named blocking occurrence's
//! acceptance and resolution status folded for the decision.

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

use crate::types::{
    AssignmentId, AssignmentKind, AssignmentResult, FindingOccurrenceId, FindingSeverity,
    ReviewAssignment,
};
use crate::{__eventcore_model_reviewevent, ReviewEvent, ReviewFact, ReviewStream};

#[derive(ModelInput)]
struct VerifyFindingResolutionRequest {
    #[model(origin)]
    stream: ReviewStream,
    #[model(origin)]
    finding_id: FindingOccurrenceId,
    #[model(origin)]
    remediation_assignment_id: AssignmentId,
}

/// Verifies the resolution of one previously accepted blocking occurrence.
#[derive(ModelCommand)]
struct VerifyFindingResolution {
    #[stream]
    stream: ReviewStream,
    finding_id: FindingOccurrenceId,
    remediation_assignment_id: AssignmentId,
}

mapping! { VerifyResolutionRequestToStream: VerifyFindingResolutionRequest.stream => VerifyFindingResolution.stream using clone; }
mapping! { VerifyResolutionRequestToFinding: VerifyFindingResolutionRequest.finding_id => VerifyFindingResolution.finding_id using clone; }
mapping! { VerifyResolutionRequestToAssignment: VerifyFindingResolutionRequest.remediation_assignment_id => VerifyFindingResolution.remediation_assignment_id using clone; }

#[derive(ModelState)]
struct VerifyFindingResolutionState {
    #[model(default)]
    blocking_occurrence_accepted: bool,
    #[model(default)]
    already_resolved: bool,
    #[model(default)]
    remediation_assignment: Option<ReviewAssignment>,
    #[model(default)]
    remediation_result: Option<AssignmentResult>,
}

#[derive(Clone)]
struct ResolutionContext {
    blocking_occurrence_accepted: bool,
    already_resolved: bool,
    remediation_assignment: Option<ReviewAssignment>,
    remediation_result: Option<AssignmentResult>,
}

#[derive(ModelOutput)]
struct ResolutionDecision {
    context: ResolutionContext,
}

fn resolution_context(
    accepted: &bool,
    resolved: &bool,
    assignment: &Option<ReviewAssignment>,
    result: &Option<AssignmentResult>,
) -> ResolutionContext {
    ResolutionContext {
        blocking_occurrence_accepted: *accepted,
        already_resolved: *resolved,
        remediation_assignment: assignment.clone(),
        remediation_result: result.clone(),
    }
}
mapping! { ResolutionStateToDecision: (VerifyFindingResolutionState.blocking_occurrence_accepted, VerifyFindingResolutionState.already_resolved, VerifyFindingResolutionState.remediation_assignment, VerifyFindingResolutionState.remediation_result) => ResolutionDecision.context using resolution_context; }

fn resolution_stream(stream: &ReviewStream) -> StreamId {
    stream.as_stream_id().clone()
}
mapping! { ResolutionStreamToEvent: VerifyFindingResolution.stream => ReviewEvent.stream using resolution_stream; }

fn resolution_fact(
    finding_id: &FindingOccurrenceId,
    context: &ResolutionContext,
) -> Result<ReviewFact, CommandError> {
    let result = context.remediation_result.as_ref().ok_or_else(|| {
        CommandError::ValidationError("review_remediation_result_required".to_owned())
    })?;
    Ok(ReviewFact::FindingResolutionVerified {
        finding_id: finding_id.clone(),
        evidence_id: result.evidence_id().clone(),
    })
}
mapping! { ResolutionToFact: (VerifyFindingResolution.finding_id, ResolutionDecision.context) => ReviewEvent.fact using try resolution_fact, error = CommandError; }

impl ModelCommandLogic for VerifyFindingResolution {
    type Event = ReviewEvent;
    type State = VerifyFindingResolutionState;

    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut folded = state.into_inner();
        match &event.fact {
            ReviewFact::AssignmentResultAccepted { result } => {
                folded.blocking_occurrence_accepted |= result.findings().iter().any(|finding| {
                    finding.id() == &self.finding_id
                        && finding.severity() == FindingSeverity::Blocking
                });
                if result.assignment_id() == &self.remediation_assignment_id {
                    folded.remediation_result = Some(result.clone());
                }
            }
            ReviewFact::AssignmentIssued { assignment }
                if assignment.id() == &self.remediation_assignment_id
                    && assignment.id().kind() == AssignmentKind::RemediationVerifier
                    && assignment.finding_target() == Some(&self.finding_id) =>
            {
                folded.remediation_assignment = Some(assignment.clone());
            }
            ReviewFact::DeltaReassessed {
                affected_lenses, ..
            } if affected_lenses.contains(self.finding_id.assignment_id().lens()) => {
                folded.blocking_occurrence_accepted = false;
                folded.remediation_assignment = None;
                folded.remediation_result = None;
            }
            ReviewFact::FindingResolutionVerified { finding_id, .. }
                if finding_id == &self.finding_id =>
            {
                folded.already_resolved = true;
            }
            ReviewFact::RiskAssessed { .. }
            | ReviewFact::AssignmentIssued { .. }
            | ReviewFact::AssignmentSuperseded { .. }
            | ReviewFact::FindingResolutionVerified { .. }
            | ReviewFact::DeltaReassessed { .. }
            | ReviewFact::CleanReviewAccepted { .. } => {}
        }
        Modeled::from_built(folded)
    }

    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, CommandError> {
        let decision = ResolutionDecision::model_builder()
            .context(ResolutionStateToDecision::apply((
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
            )))
            .build();
        let context = &decision.as_ref().context;
        if !context.blocking_occurrence_accepted {
            return Err(CommandError::ValidationError(
                "review_blocking_finding_not_accepted".to_owned(),
            ));
        }
        if context.already_resolved {
            return Err(CommandError::ValidationError(
                "review_finding_already_resolved".to_owned(),
            ));
        }
        let assignment = context.remediation_assignment.as_ref().ok_or_else(|| {
            CommandError::ValidationError("review_remediation_assignment_required".to_owned())
        })?;
        let result = context.remediation_result.as_ref().ok_or_else(|| {
            CommandError::ValidationError("review_remediation_result_required".to_owned())
        })?;
        if assignment.id() != result.assignment_id()
            || assignment.finding_target() != Some(&self.finding_id)
        {
            return Err(CommandError::ValidationError(
                "review_remediation_provenance_mismatch".to_owned(),
            ));
        }
        Ok(ModeledEvents::one(
            ReviewEvent::model_builder()
                .stream(ResolutionStreamToEvent::apply(self))
                .fact(ResolutionToFact::apply((self, decision.as_ref()))?)
                .build(),
        ))
    }
}

/// Builds the checked blocking-finding resolution command.
#[must_use]
pub fn verify_finding_resolution(
    stream: ReviewStream,
    finding_id: FindingOccurrenceId,
    remediation_assignment_id: AssignmentId,
) -> impl eventcore::CommandLogic<Event = ReviewEvent> {
    let request = VerifyFindingResolutionRequest::model_builder()
        .stream(stream)
        .finding_id(finding_id)
        .remediation_assignment_id(remediation_assignment_id)
        .build();
    VerifyFindingResolution::model_builder()
        .stream(VerifyResolutionRequestToStream::apply(request.as_ref()))
        .finding_id(VerifyResolutionRequestToFinding::apply(request.as_ref()))
        .remediation_assignment_id(VerifyResolutionRequestToAssignment::apply(request.as_ref()))
        .build()
}
