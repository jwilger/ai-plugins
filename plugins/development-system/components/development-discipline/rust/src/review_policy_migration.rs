//! Explicit migration is an independently assessed policy change, not a waiver.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PriorAssignmentClosure {
    pub subagent_key: String,
    pub disposition: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Migration {
    pub assessment: PlanRiskAssessmentInput,
    pub expected_assignment: ExpectedPlanRiskAssignment,
    pub observed_source_tree: String,
    pub prior_source_tree: String,
    pub prior_assignment_closures: Vec<PriorAssignmentClosure>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct MigrationAudit {
    pub risk_assessment_id: String,
    pub prior_source_tree: String,
    pub observed_source_tree: String,
    pub prior_assignment_closures: Vec<PriorAssignmentClosure>,
    pub prior_policy_minimum: u64,
    pub new_policy_version: u64,
    pub reused_receipts: u64,
    pub retained_evidence_rationale: String,
}

pub(super) fn migration_schema() -> Value {
    let mut schema = review_reopen_schema();
    schema["properties"]["risk_assessment"] = risk_assessment_output_schema(false);
    schema["properties"]["prior_assignment_closures"] = json!({"type":"array","maxItems":MAX_REVIEW_LENSES,"items":{
        "type":"object","additionalProperties":false,"required":["subagent_key","disposition"],"properties":{
            "subagent_key":{"type":"string"},"disposition":{"type":"string","enum":["closed","not-started"]}
        }
    }});
    schema
}

fn tree(root: &str, snapshot: &str) -> Result<String, String> {
    if !valid_git_object_id(snapshot) {
        return Err("review_migration_snapshot_invalid=true".into());
    }
    let identity = snapshot_git_text(
        Path::new(root),
        &[
            "rev-parse".into(),
            "--verify".into(),
            format!("{snapshot}^{{tree}}"),
        ],
        None,
        None,
        "review_policy_migration_tree",
    )?;
    if !valid_git_object_id(&identity) {
        return Err("review_migration_tree_unavailable=true".into());
    }
    Ok(identity)
}

pub(super) fn prepare(
    state: &Value,
    operation_id: &str,
    evidence: &SharedTestEvidenceFacts,
) -> Result<(Value, ExpectedPlanRiskAssignment, String, String), String> {
    let material = ReviewSessionState::parse_legacy_wire(state)?;
    let mut arguments = json!({
        "session_id":format!("migration-{}",stable_storage_digest(&[&material.identity.session_id,operation_id])),
        "scope":material.scope.kind,"base":material.scope.base,
        "baseline_commit":material.scope.baseline_commit,"project_root":material.scope.project_root,
        "diff_hash":material.scope.diff_hash,"changed_files":material.scope.changed_files,
        "review_lifecycle":material.scope.review_lifecycle,"split_lineage":material.scope.split_lineage,
        "user_request":material.context.user_request,"acceptance_criteria":material.context.acceptance_criteria,
        "explicit_concerns":material.context.explicit_concerns,"shared_test_evidence":evidence
    });
    if material.scope.split_lineage.is_none() {
        arguments
            .as_object_mut()
            .expect("migration arguments object")
            .remove("split_lineage");
    }
    arguments["conditional_lenses"] = json!(material
        .model_routing
        .lens_objectives
        .iter()
        .filter(|(lens, _)| !LENSES.contains(&lens.as_str()) && lens.as_str() != SAFETY_LENS)
        .map(|(lens, description)| json!({"id":lens,"description":description}))
        .collect::<Vec<_>>());
    let response: Value = serde_json::from_str(&risk_assessment_result(&arguments)?)
        .map_err(|e| format!("review_migration_assignment_invalid source={e}"))?;
    let assignment = response["assignments"][0].clone();
    let expected = parse_expected_plan_risk_assignment(&assignment)?;
    let prior = tree(
        &material.scope.project_root,
        material
            .scope
            .snapshot_commit
            .as_deref()
            .ok_or("review_migration_snapshot_missing=true")?,
    )?;
    let current = tree(
        &material.scope.project_root,
        expected
            .scope
            .snapshot_commit
            .as_deref()
            .ok_or("review_migration_snapshot_missing=true")?,
    )?;
    if prior != current
        || expected.scope.baseline_commit != material.scope.baseline_commit
        || evidence.diff_hash != material.scope.diff_hash
        || !matches!(evidence.status, SharedTestStatus::Passed)
    {
        return Err("review_migration_source_or_verification_changed=true recovery=perform_bound_source_delta_review".into());
    }
    Ok((assignment, expected, prior, current))
}

pub(super) fn migrate(
    state: &SubmitReviewIterationMaterial,
    migration: &Migration,
    submission: &ReviewIterationSubmission,
) -> Result<SubmitReviewIterationMaterial, String> {
    state.contract.validate_current_protocol()?;
    if state.iteration_limit_hold {
        return Err("review_migration_iteration_limit_held=true".into());
    }
    if state
        .contract
        .risk_plan
        .as_ref()
        .is_some_and(|p| p.coverage.is_some())
    {
        return Err("review_policy_already_v3=true".into());
    }
    if migration.prior_source_tree != migration.observed_source_tree
        || submission.current_diff_hash != state.contract.scope.diff_hash
        || migration.expected_assignment.scope.baseline_commit
            != state.contract.scope.baseline_commit
    {
        return Err("review_migration_source_binding_invalid=true".into());
    }
    let expected_closures = state
        .contract
        .lenses
        .iter()
        .map(|lens| {
            format!(
                "{}:{}:{lens}",
                state.contract.session_id, state.iteration_index
            )
        })
        .collect::<HashSet<_>>();
    let actual_closures = migration
        .prior_assignment_closures
        .iter()
        .map(|c| c.subagent_key.clone())
        .collect::<HashSet<_>>();
    if actual_closures != expected_closures
        || actual_closures.len() != migration.prior_assignment_closures.len()
        || migration
            .prior_assignment_closures
            .iter()
            .any(|c| !matches!(c.disposition.as_str(), "closed" | "not-started"))
    {
        return Err("review_migration_prior_lifecycle_attestation_required=true".into());
    }
    if migration.assessment.coverage_policy.is_none() {
        return Err("review_migration_v3_policy_required=true".into());
    }
    let compiled = compile_risk_plan(
        Some(&migration.assessment),
        Some(&migration.expected_assignment),
        &state.contract.scope.changed_files,
        &PlanRiskCompileContext {
            user_request: &state.context.user_request,
            acceptance_criteria: &state.context.acceptance_criteria,
            explicit_concerns: &state.context.explicit_concerns,
            prior_defenses_by_lens: &state.current_defenses_by_lens,
            project_root: &state.contract.scope.project_root,
        },
    )?
    .ok_or("review_migration_assessment_required=true")?;
    if compiled.state.scope_split.as_ref().is_some_and(|s| s.hold) {
        return Err("review_migration_split_requires_separate_resolution=true".into());
    }
    let prior = state
        .contract
        .risk_plan
        .as_ref()
        .ok_or("review_migration_legacy_risk_plan_required=true")?;
    if prior.review_budget.hold
        || prior.review_budget.checkpoint_pending
        || prior.scope_split.as_ref().is_some_and(|s| s.hold)
    {
        return Err("review_migration_existing_hold_requires_resolution=true".into());
    }
    let mut next = state.clone();
    let mut plan = compiled.state;
    plan.review_budget = prior.review_budget.clone();
    plan.delta_history = prior.delta_history.clone();
    plan.resolved_blocking_findings = prior.resolved_blocking_findings.clone();
    let mut unresolved = next.unresolved_findings.clone().unwrap_or_default();
    for finding in compiled.blocking_findings {
        if !unresolved
            .iter()
            .any(|f| f.id == finding.id && f.lens == finding.lens)
        {
            unresolved.push(finding);
        }
    }
    if unresolved
        .iter()
        .any(|f| !plan.selected_lenses.contains(&f.lens))
    {
        return Err("review_migration_unresolved_lens_must_be_selected=true".into());
    }
    next.unresolved_findings = Some(unresolved);
    next.contract.required_clean_iterations = 1;
    next.contract.lenses = plan.selected_lenses.clone();
    next.contract.shared_test_evidence = submission.current_shared_test_evidence.clone();
    next.contract.risk_plan = Some(Box::new(plan));
    next.iteration_index = next_review_iteration(state.iteration_index)?;
    next.clean_streak = 0;
    next.verified_clean_iterations.clear();
    next.contract.review_contract_id = next.contract.computed_id();
    Ok(next)
}

pub(super) fn decision(
    state: &SubmitReviewIterationMaterial,
) -> Result<ReviewIterationDecision, String> {
    let assignments = build_typed_review_assignments(ReviewAssignmentMaterial {
        coverage: state
            .contract
            .risk_plan
            .as_ref()
            .and_then(|p| p.coverage.as_ref()),
        iteration: state.iteration_index,
        session_id: &state.contract.session_id,
        lenses: &state.contract.lenses,
        lens_objectives: &state.contract.lens_objectives,
        model_roles: &state.contract.model_roles,
        scope: &state.contract.scope,
        context: &state.context,
        defenses: &state.current_defenses_by_lens,
        deferred_findings: &state.deferred_findings,
        resolutions: &retained_resolutions(&state.finding_history),
        shared_test_evidence: state.contract.shared_test_evidence.as_ref(),
    })?;
    Ok(ReviewIterationDecision {
        changes: review_iteration_changes(
            state,
            ReviewIterationMutationSet {
                scope_changed: false,
                contract_changed: true,
                risk_changed: true,
                defenses_changed: false,
            },
        ),
        iteration_index: state.iteration_index,
        clean_streak: 0,
        checkpoint_minutes: MEDIUM_RISK_REVIEW_BUDGET_MINUTES,
        review_lifecycle: state.contract.scope.review_lifecycle.clone(),
        budget_requested: false,
        complete: false,
        filtered: json!({"policy_migrated":true,"review_credit_granted":false}),
        verification: json!({"status":"independent_risk_assessment_verified"}),
        reset_reason: "policy_migrated".into(),
        next_assignments: assignments,
        subagent_shutdown: Vec::new(),
        verifier_request: None,
        verifier_continuation: None,
        verifier_assignment: None,
        delta_risk_request: None,
        delta_risk_assignment: None,
    })
}
