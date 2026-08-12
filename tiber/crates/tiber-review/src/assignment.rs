//! Assignment-issuance intent with only routing, current-assignment, and
//! scheduler-receipt state folded for its decision.

#![expect(
    clippy::missing_trait_methods,
    clippy::ref_option,
    clippy::single_call_fn,
    clippy::too_many_arguments,
    clippy::trivially_copy_pass_by_ref,
    reason = "EventCore mapping signatures and static stream discovery are defined by the checked-model API"
)]

use alloc::collections::{BTreeMap, BTreeSet};
use eventcore::{
    CommandError, ModelCommand, ModelInput, ModelOutput, ModelState, StreamId, mapping,
    model::{ModelCommandLogic, Modeled, ModeledEvents, StreamIdentity as _},
};

use crate::types::{AgentId, FindingOccurrenceId, FindingSeverity};
use crate::types::{LifecycleReceiptId, ModelRole, ReviewIteration};
use crate::{
    __eventcore_model_reviewevent, AssignmentAttempt, AssignmentKind, ContextReceiptId,
    ReviewAssignment, ReviewEvent, ReviewFact, ReviewSnapshotId, ReviewStream, VerifierRoute,
};

#[derive(Clone, Copy, Default)]
enum IterationStatus {
    #[default]
    Current,
    Advanced(ReviewIteration),
    Exhausted,
}

impl IterationStatus {
    const fn expected(self) -> Option<ReviewIteration> {
        match self {
            Self::Current => Some(ReviewIteration::FIRST),
            Self::Advanced(iteration) => Some(iteration),
            Self::Exhausted => None,
        }
    }

    fn advance(self) -> Self {
        self.expected()
            .and_then(|iteration| iteration.next().ok())
            .map_or(Self::Exhausted, Self::Advanced)
    }
}

#[derive(ModelInput)]
struct IssueAssignmentRequest {
    #[model(origin)]
    stream: ReviewStream,
    #[model(origin)]
    assignment: ReviewAssignment,
}

/// Issues one scheduler-owned review assignment.
#[derive(ModelCommand)]
struct IssueAssignment {
    #[stream]
    stream: ReviewStream,
    assignment: ReviewAssignment,
}

mapping! { IssueAssignmentRequestToStream: IssueAssignmentRequest.stream => IssueAssignment.stream using clone; }
mapping! { IssueAssignmentRequestToAssignment: IssueAssignmentRequest.assignment => IssueAssignment.assignment using clone; }

#[derive(ModelState)]
struct IssueAssignmentState {
    #[model(default)]
    risk_assessed: bool,
    #[model(default)]
    expected_role: Option<ModelRole>,
    #[model(default)]
    current_snapshot: Option<ReviewSnapshotId>,
    #[model(default)]
    assignment_active: bool,
    #[model(default)]
    used_context_receipts: BTreeSet<ContextReceiptId>,
    #[model(default)]
    used_lifecycle_receipts: BTreeSet<LifecycleReceiptId>,
    #[model(default)]
    lens_result_accepted: bool,
    #[model(default)]
    lens_reviewer_agents: BTreeMap<crate::ReviewLens, AgentId>,
    #[model(default)]
    completed_assignments: BTreeSet<crate::AssignmentId>,
    #[model(default)]
    risk_agent: Option<AgentId>,
    #[model(default)]
    blocking_findings: BTreeSet<FindingOccurrenceId>,
    #[model(default)]
    expected_attempt: AssignmentAttempt,
    #[model(default)]
    iteration_status: IterationStatus,
}

#[derive(Clone)]
struct IssueAssignmentContext {
    risk_assessed: bool,
    expected_role: Option<ModelRole>,
    current_snapshot: Option<ReviewSnapshotId>,
    assignment_active: bool,
    used_context_receipts: BTreeSet<ContextReceiptId>,
    used_lifecycle_receipts: BTreeSet<LifecycleReceiptId>,
    lens_result_accepted: bool,
    lens_reviewer_agents: BTreeMap<crate::ReviewLens, AgentId>,
    completed_assignments: BTreeSet<crate::AssignmentId>,
    risk_agent: Option<AgentId>,
    blocking_findings: BTreeSet<FindingOccurrenceId>,
    expected_attempt: AssignmentAttempt,
    iteration_status: IterationStatus,
}

#[derive(ModelOutput)]
struct IssueAssignmentDecision {
    context: IssueAssignmentContext,
}

fn assignment_context(
    risk_assessed: &bool,
    expected_role: &Option<ModelRole>,
    snapshot: &Option<ReviewSnapshotId>,
    assignment_active: &bool,
    receipts: &BTreeSet<ContextReceiptId>,
    lifecycle_receipts: &BTreeSet<LifecycleReceiptId>,
    lens_result: &bool,
    lens_agents: &BTreeMap<crate::ReviewLens, AgentId>,
    completed_assignments: &BTreeSet<crate::AssignmentId>,
    risk_agent: &Option<AgentId>,
    blockers: &BTreeSet<FindingOccurrenceId>,
    expected_attempt: &AssignmentAttempt,
    iteration_status: &IterationStatus,
) -> IssueAssignmentContext {
    IssueAssignmentContext {
        risk_assessed: *risk_assessed,
        expected_role: expected_role.clone(),
        current_snapshot: snapshot.clone(),
        assignment_active: *assignment_active,
        used_context_receipts: receipts.clone(),
        used_lifecycle_receipts: lifecycle_receipts.clone(),
        lens_result_accepted: *lens_result,
        lens_reviewer_agents: lens_agents.clone(),
        completed_assignments: completed_assignments.clone(),
        risk_agent: risk_agent.clone(),
        blocking_findings: blockers.clone(),
        expected_attempt: *expected_attempt,
        iteration_status: *iteration_status,
    }
}

mapping! {
    IssueAssignmentStateToDecision:
        (IssueAssignmentState.risk_assessed, IssueAssignmentState.expected_role, IssueAssignmentState.current_snapshot, IssueAssignmentState.assignment_active, IssueAssignmentState.used_context_receipts, IssueAssignmentState.used_lifecycle_receipts, IssueAssignmentState.lens_result_accepted, IssueAssignmentState.lens_reviewer_agents, IssueAssignmentState.completed_assignments, IssueAssignmentState.risk_agent, IssueAssignmentState.blocking_findings, IssueAssignmentState.expected_attempt, IssueAssignmentState.iteration_status) => IssueAssignmentDecision.context
        using assignment_context;
}

fn assignment_stream(stream: &ReviewStream) -> StreamId {
    stream.as_stream_id().clone()
}

fn same_assignment_work(left: &crate::AssignmentId, right: &crate::AssignmentId) -> bool {
    left.lens() == right.lens()
        && left.kind() == right.kind()
        && left.iteration() == right.iteration()
        && left.occurrence_key() == right.occurrence_key()
}
mapping! { IssueAssignmentStreamToEvent: IssueAssignment.stream => ReviewEvent.stream using assignment_stream; }

fn assignment_fact(
    assignment: &ReviewAssignment,
    context: &IssueAssignmentContext,
) -> Result<ReviewFact, CommandError> {
    if !context.risk_assessed || context.assignment_active {
        return Err(CommandError::ValidationError(
            "review_assignment_not_authorized".to_owned(),
        ));
    }
    Ok(ReviewFact::AssignmentIssued {
        assignment: assignment.clone(),
    })
}
mapping! { IssueAssignmentToFact: (IssueAssignment.assignment, IssueAssignmentDecision.context) => ReviewEvent.fact using try assignment_fact, error = CommandError; }

impl ModelCommandLogic for IssueAssignment {
    type Event = ReviewEvent;
    type State = IssueAssignmentState;

    #[expect(
        clippy::too_many_lines,
        reason = "the command-specific fold keeps assignment authority and delta invalidation visible together"
    )]
    fn evolve(&self, state: Modeled<Self::State>, event: &Self::Event) -> Modeled<Self::State> {
        let mut folded = state.into_inner();
        match &event.fact {
            ReviewFact::RiskAssessed {
                snapshot,
                assessment,
            } => {
                folded.risk_assessed = true;
                folded.risk_agent = Some(assessment.agent_id().clone());
                folded
                    .used_context_receipts
                    .insert(assessment.context_receipt().clone());
                folded
                    .used_lifecycle_receipts
                    .insert(assessment.lifecycle_receipt().clone());
                folded.current_snapshot = Some(snapshot.clone());
                folded.expected_role =
                    if matches!(self.assignment.id().kind(), AssignmentKind::DeltaRisk) {
                        Some(assessment.delta_model_role().clone())
                    } else {
                        assessment.routes().iter().find_map(|route| {
                            if route.lens() != self.assignment.id().lens() {
                                return None;
                            }
                            match self.assignment.id().kind() {
                                AssignmentKind::Lens => Some(route.reviewer_model_role().clone()),
                                AssignmentKind::RemediationVerifier => {
                                    Some(route.remediation_model_role().clone())
                                }
                                AssignmentKind::Verifier => match route.verifier() {
                                    VerifierRoute::Required { model_role } => {
                                        Some(model_role.clone())
                                    }
                                    VerifierRoute::NotRequired => None,
                                },
                                AssignmentKind::DeltaRisk => None,
                            }
                        })
                    };
            }
            ReviewFact::AssignmentIssued { assignment } => {
                folded
                    .used_context_receipts
                    .insert(assignment.context_receipt().clone());
                folded
                    .used_lifecycle_receipts
                    .insert(assignment.lifecycle_receipt().clone());
                if assignment.id().kind() == AssignmentKind::Lens {
                    folded.lens_reviewer_agents.insert(
                        assignment.id().lens().clone(),
                        assignment.agent_id().clone(),
                    );
                }
                if same_assignment_work(assignment.id(), self.assignment.id()) {
                    folded.assignment_active = true;
                }
            }
            ReviewFact::AssignmentResultAccepted { result }
                if result.assignment_id().lens() == self.assignment.id().lens()
                    && result.assignment_id().kind() == AssignmentKind::Lens =>
            {
                folded
                    .completed_assignments
                    .insert(result.assignment_id().clone());
                folded.lens_result_accepted = true;
                folded.lens_reviewer_agents.insert(
                    result.assignment_id().lens().clone(),
                    result.agent_id().clone(),
                );
                folded.blocking_findings.extend(
                    result
                        .findings()
                        .iter()
                        .filter(|finding| finding.severity() == FindingSeverity::Blocking)
                        .map(|finding| finding.id().clone()),
                );
                folded.assignment_active = false;
            }
            ReviewFact::AssignmentResultAccepted { result }
                if same_assignment_work(result.assignment_id(), self.assignment.id()) =>
            {
                folded
                    .completed_assignments
                    .insert(result.assignment_id().clone());
                folded.assignment_active = false;
            }
            ReviewFact::AssignmentSuperseded {
                assignment_id,
                replacement_attempt,
                ..
            } if same_assignment_work(assignment_id, self.assignment.id()) => {
                folded.assignment_active = false;
                folded.expected_attempt = *replacement_attempt;
            }
            ReviewFact::DeltaReassessed {
                to_snapshot,
                affected_lenses,
                ..
            } if affected_lenses.contains(self.assignment.id().lens()) => {
                folded.current_snapshot = Some(to_snapshot.clone());
                folded.assignment_active = false;
                folded.lens_result_accepted = false;
                folded
                    .lens_reviewer_agents
                    .retain(|lens, _| !affected_lenses.contains(lens));
                folded
                    .blocking_findings
                    .retain(|finding| !affected_lenses.contains(finding.assignment_id().lens()));
                folded.expected_attempt = AssignmentAttempt::FIRST;
                folded.iteration_status = folded.iteration_status.advance();
            }
            ReviewFact::DeltaReassessed { to_snapshot, .. } => {
                folded.current_snapshot = Some(to_snapshot.clone());
                if self.assignment.id().kind() == AssignmentKind::DeltaRisk {
                    folded.assignment_active = false;
                    folded.expected_attempt = AssignmentAttempt::FIRST;
                    folded.iteration_status = folded.iteration_status.advance();
                }
            }
            ReviewFact::AssignmentResultAccepted { .. }
            | ReviewFact::AssignmentSuperseded { .. }
            | ReviewFact::FindingResolutionVerified { .. }
            | ReviewFact::CleanReviewAccepted { .. } => {}
        }
        Modeled::from_built(folded)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the business command keeps all assignment-authority invariants visible in one decision without delegating or sharing write state"
    )]
    fn decide(
        &self,
        state: Modeled<Self::State>,
    ) -> Result<ModeledEvents<Self::Event>, CommandError> {
        let decision = IssueAssignmentDecision::model_builder()
            .context(IssueAssignmentStateToDecision::apply((
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
                state.as_ref(),
                state.as_ref(),
                state.as_ref(),
            )))
            .build();
        let context = &decision.as_ref().context;
        if !context.risk_assessed {
            return Err(CommandError::ValidationError(
                "review_risk_assessment_required".to_owned(),
            ));
        }
        let expected_iteration = context.iteration_status.expected().ok_or_else(|| {
            CommandError::ValidationError("review_iteration_limit_reached".to_owned())
        })?;
        let expected_role = context.expected_role.as_ref().ok_or_else(|| {
            CommandError::ValidationError("review_risk_assessment_required".to_owned())
        })?;
        if context.assignment_active {
            return Err(CommandError::ValidationError(
                "review_assignment_active".to_owned(),
            ));
        }
        if context.completed_assignments.contains(self.assignment.id()) {
            return Err(CommandError::ValidationError(
                "review_assignment_already_completed".to_owned(),
            ));
        }
        let stream_session = self
            .stream
            .session()
            .map_err(|error| CommandError::ValidationError(error.code().to_owned()))?;
        if self.assignment.id().session() != &stream_session
            || context.current_snapshot.as_ref() != Some(self.assignment.snapshot())
            || self.assignment.id().iteration() != expected_iteration
        {
            return Err(CommandError::ValidationError(
                "review_assignment_authority_mismatch".to_owned(),
            ));
        }
        if self.assignment.id().attempt() != context.expected_attempt {
            return Err(CommandError::ValidationError(
                "review_assignment_attempt_mismatch".to_owned(),
            ));
        }
        if context
            .used_context_receipts
            .contains(self.assignment.context_receipt())
        {
            return Err(CommandError::ValidationError(
                "review_context_receipt_reused".to_owned(),
            ));
        }
        if context
            .used_lifecycle_receipts
            .contains(self.assignment.lifecycle_receipt())
        {
            return Err(CommandError::ValidationError(
                "review_lifecycle_receipt_reused".to_owned(),
            ));
        }
        if expected_role != self.assignment.model_role() {
            return Err(CommandError::ValidationError(
                "review_assignment_model_role_mismatch".to_owned(),
            ));
        }
        if context.risk_agent.as_ref() == Some(self.assignment.agent_id()) {
            return Err(CommandError::ValidationError(
                "review_assignment_agent_must_differ_from_risk_assessor".to_owned(),
            ));
        }
        match self.assignment.id().kind() {
            AssignmentKind::DeltaRisk
                if self.assignment.target_snapshot().is_none()
                    || self.assignment.target_snapshot() == Some(self.assignment.snapshot()) =>
            {
                return Err(CommandError::ValidationError(
                    "review_delta_target_snapshot_required".to_owned(),
                ));
            }
            AssignmentKind::Lens
            | AssignmentKind::Verifier
            | AssignmentKind::RemediationVerifier
                if self.assignment.target_snapshot().is_some() =>
            {
                return Err(CommandError::ValidationError(
                    "review_assignment_target_snapshot_unexpected".to_owned(),
                ));
            }
            AssignmentKind::Lens
            | AssignmentKind::Verifier
            | AssignmentKind::DeltaRisk
            | AssignmentKind::RemediationVerifier => {}
        }
        if self.assignment.id().kind() != AssignmentKind::RemediationVerifier
            && self.assignment.id().occurrence_key().is_some()
        {
            return Err(CommandError::ValidationError(
                "review_assignment_occurrence_key_unexpected".to_owned(),
            ));
        }
        if matches!(
            self.assignment.id().kind(),
            AssignmentKind::Verifier | AssignmentKind::RemediationVerifier
        ) && !context.lens_result_accepted
        {
            return Err(CommandError::ValidationError(
                "review_lens_result_required".to_owned(),
            ));
        }
        if self.assignment.id().kind() == AssignmentKind::RemediationVerifier
            && self
                .assignment
                .finding_target()
                .is_none_or(|finding| !context.blocking_findings.contains(finding))
        {
            return Err(CommandError::ValidationError(
                "review_remediation_finding_target_required".to_owned(),
            ));
        }
        if self.assignment.id().kind() == AssignmentKind::RemediationVerifier
            && self.assignment.id().occurrence_key()
                != self
                    .assignment
                    .finding_target()
                    .map(FindingOccurrenceId::evidence_id)
        {
            return Err(CommandError::ValidationError(
                "review_remediation_assignment_identity_mismatch".to_owned(),
            ));
        }
        if matches!(
            self.assignment.id().kind(),
            AssignmentKind::Verifier | AssignmentKind::RemediationVerifier
        ) && context
            .lens_reviewer_agents
            .get(self.assignment.id().lens())
            == Some(self.assignment.agent_id())
        {
            return Err(CommandError::ValidationError(
                "review_verifier_agent_must_be_distinct".to_owned(),
            ));
        }
        if self.assignment.id().kind() == AssignmentKind::Lens
            && context.lens_reviewer_agents.iter().any(|(lens, agent)| {
                lens != self.assignment.id().lens() && agent == self.assignment.agent_id()
            })
        {
            return Err(CommandError::ValidationError(
                "review_lens_agent_must_be_independent".to_owned(),
            ));
        }
        Ok(ModeledEvents::one(
            ReviewEvent::model_builder()
                .stream(IssueAssignmentStreamToEvent::apply(self))
                .fact(IssueAssignmentToFact::apply((self, decision.as_ref()))?)
                .build(),
        ))
    }
}

/// Builds the checked assignment-issuance command.
#[must_use]
pub fn issue_assignment(
    stream: ReviewStream,
    assignment: ReviewAssignment,
) -> impl eventcore::CommandLogic<Event = ReviewEvent> {
    let request = IssueAssignmentRequest::model_builder()
        .stream(stream)
        .assignment(assignment)
        .build();
    IssueAssignment::model_builder()
        .stream(IssueAssignmentRequestToStream::apply(request.as_ref()))
        .assignment(IssueAssignmentRequestToAssignment::apply(request.as_ref()))
        .build()
}
