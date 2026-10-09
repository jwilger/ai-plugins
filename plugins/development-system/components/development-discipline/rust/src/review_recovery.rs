//! Same-session recovery after source changes invalidate a completed review.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct ReviewReopenIntent {
    pub(super) operation_id: String,
    pub(super) request_fingerprint: String,
    pub(super) reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) migration: Option<Box<review_policy_migration::Migration>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct ReviewReopenedFacts {
    pub(super) transition: AdvanceTransitionFacts,
    pub(super) operation_id: String,
    pub(super) request_fingerprint: String,
    pub(super) reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) policy_migration: Option<review_policy_migration::MigrationAudit>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct ReviewReopenedEvent {
    pub(super) stream: StreamId,
    pub(super) facts: ReviewReopenedFacts,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReopenReviewInput {
    state_ref: Value,
    operation_id: String,
    reason: String,
    current_diff_hash: String,
    current_changed_files: Vec<String>,
    current_shared_test_evidence: SharedTestEvidenceFacts,
    #[serde(default)]
    risk_assessment: Option<PlanRiskAssessmentInput>,
    #[serde(default)]
    prior_assignment_closures: Vec<review_policy_migration::PriorAssignmentClosure>,
}

pub(super) fn review_reopen_schema() -> Value {
    json!({"type":"object", "additionalProperties":false,
        "properties": {
            "state_ref":state_reference_schema(),
            "operation_id":{"type":"string","minLength":1,"maxLength":256},
            "reason":{"type":"string","minLength":1,"maxLength":4096},
            "current_diff_hash":{"type":"string","minLength":1},
            "current_changed_files":{"type":"array","minItems":1,"maxItems":MAX_CHANGED_FILES,"items":{"type":"string"}},
            "current_shared_test_evidence":shared_test_evidence_schema()
        },
        "required":["state_ref","operation_id","reason","current_diff_hash","current_changed_files","current_shared_test_evidence"]})
}

pub(super) fn reset_completed_review(
    state: &SubmitReviewIterationMaterial,
    intent: &ReviewReopenIntent,
    submission: &ReviewIterationSubmission,
    now: u64,
) -> Result<SubmitReviewIterationMaterial, String> {
    if let Some(migration) = &intent.migration {
        return review_policy_migration::migrate(state, migration, submission);
    }
    // This is authoritative command replay, never caller-carried state repair.
    state.contract.validate_current_protocol()?;
    if !typed_review_complete(state) {
        return Err(
            "review_reopen_requires_completed_session=true recovery=resume_latest_state"
                .to_string(),
        );
    }
    if submission.current_diff_hash == state.contract.scope.diff_hash {
        return Err("review_reopen_scope_unchanged=true complete=true".to_string());
    }
    let mut reset = state.clone();
    let plan = reset.contract.risk_plan.as_deref_mut().ok_or_else(|| {
        "review_reopen_risk_plan_required=true recovery=restart_final_review".to_string()
    })?;
    plan.review_budget.decision = None;
    plan.review_budget.checkpoint_pending = false;
    plan.review_budget.hold = false;
    plan.review_budget.next_checkpoint_at_epoch_seconds =
        Some(now.saturating_add(plan.review_budget.checkpoint_minutes.saturating_mul(60)));
    plan.discovery_saturation.confirmation_samples_by_lens = plan
        .selected_lenses
        .iter()
        .map(|lens| (lens.clone(), 0))
        .collect();
    plan.discovery_saturation.last_sample_added_new_by_lens = plan
        .selected_lenses
        .iter()
        .map(|lens| (lens.clone(), false))
        .collect();
    plan.active_lenses.clone_from(&plan.selected_lenses);
    plan.active_lens_passes.clone_from(&plan.lens_passes);
    reset.contract.lenses.clone_from(&plan.selected_lenses);
    if let Some(coverage) = plan.coverage.as_mut() {
        coverage.pending_reassessment = true;
        reset.contract.lenses = coverage.pending();
        plan.active_lenses = coverage.pending();
        plan.active_lens_passes.clear();
    }
    reset.clean_streak = 0;
    reset.verified_clean_iterations.clear();
    // Retain the iteration index so old assignment keys can never gain new credit.
    reset.contract.review_contract_id = reset.contract.computed_id();
    Ok(reset)
}

impl ReviewCoordinator {
    pub(super) fn reopen_review(&mut self, arguments: &Value) -> Result<Value, String> {
        self.reopen_review_with_policy(arguments, false)
    }

    pub(super) fn migrate_policy(&mut self, arguments: &Value) -> Result<Value, String> {
        self.reopen_review_with_policy(arguments, true)
    }

    fn reopen_review_with_policy(
        &mut self,
        arguments: &Value,
        migrating: bool,
    ) -> Result<Value, String> {
        let input: ReopenReviewInput = serde_json::from_value(arguments.clone())
            .map_err(|error| format!("review_reopen_input_invalid source={error}"))?;
        if input.operation_id.trim().is_empty() || input.operation_id.len() > 256 {
            return Err("review_reopen_operation_id_invalid=true".to_string());
        }
        if input.reason.trim().is_empty() || input.reason.len() > 4096 {
            return Err("review_reopen_reason_invalid=true".to_string());
        }
        let session_id = required_argument(&input.state_ref, "session_id")?;
        let root = required_argument(&input.state_ref, "project_root")?;
        let persistence = self.service_surface.review_persistence();
        let path = durable_report_database_path_for(
            root,
            input.state_ref.get("work_item_id").and_then(Value::as_str),
            persistence,
        )?;
        let stream = review_stream_id(session_id)?;
        if !migrating
            && (input.risk_assessment.is_some() || !input.prior_assignment_closures.is_empty())
        {
            return Err("review_reopen_migration_fields_forbidden=true".into());
        }
        let request_fingerprint = if migrating {
            format!("policy-migration:{}", arguments)
        } else {
            arguments.to_string()
        };
        // Search durable facts before rejecting the old reference: the caller may
        // have lost the successful response, including its replacement reference.
        let events = read_review_stream(&path, Some(Path::new(root)), stream.clone(), persistence)?;
        if let Some(reopened) = events.iter().find_map(|event| match event {
            FinalReviewEvent::ReviewReopened(event)
                if event.facts.operation_id == input.operation_id =>
            {
                Some(event)
            }
            _ => None,
        }) {
            if reopened.facts.request_fingerprint != request_fingerprint {
                return Err("review_reopen_operation_id_conflict=true recovery=retry_original_operation_or_use_new_operation_id".to_string());
            }
            let revision = reopened.facts.transition.metadata.revision;
            let mut projected = ReviewEventState::default();
            for event in events.iter().take_while(|event| {
                event
                    .metadata()
                    .is_none_or(|metadata| metadata.revision <= revision)
            }) {
                projected = apply_review_event(projected, &stream, event);
            }
            let session = projected
                .session
                .ok_or_else(|| "review_reopen_projection_missing=true".to_string())?;
            let state = session.state.to_wire();
            let response =
                load_committed_iteration_response(&state, session_id, revision, persistence)?;
            let historical = load_authoritative_session_read_only(&state, session_id, persistence)?
                .is_none_or(|latest| latest.revision != revision);

            return Ok(text_content(json!({
                "state":state,"complete":false,"transition_status":if migrating {"policy_migrated"} else {"delta_risk_assessment_required"},
                "delta_risk_assignments":if migrating {json!([])} else {json!([response.delta_risk_assignment])},"next_assignments":if migrating {json!(response.next_assignments)} else {json!([])},
                "migration_applied":migrating,
                "completion_blockers":unresolved_findings(&state), "subagent_shutdown":response.subagent_shutdown,
                "operation_id":input.operation_id,"operation_replayed":true,
                "response_historical":historical, "assignments_current":!historical,
                "next_tool":if historical {json!("final_review.resume_latest")} else {json!("final_review.pending_assignments")},
                "recovery":if historical {json!("Historical operation receipt: call final_review.resume_latest and final_review.pending_assignments before starting subagents; these old assignments are not current.")} else {Value::Null}
            }).to_string()));
        }
        let state = self.resolve_reference(&input.state_ref)?;
        self.validate_authoritative_state("final_review.reopen", &json!({"state":state}))?;
        if self.pending_verifiers.contains_key(session_id)
            || self.pending_delta_risks.contains_key(session_id)
        {
            return Err(
                "review_reopen_pending_assignment=true recovery=final_review.pending_assignments"
                    .to_string(),
            );
        }
        let migration = if migrating {
            if input.current_diff_hash != state["scope"]["diff_hash"].as_str().unwrap_or_default()
                || json!(input.current_changed_files) != state["scope"]["changed_files"]
            {
                return Err("review_migration_scope_changed=true recovery=perform_bound_source_delta_review".into());
            }
            if state
                .pointer("/risk_plan/coverage")
                .is_some_and(Value::is_object)
            {
                return Err("review_policy_already_v3=true".into());
            }
            let (assignment, expected, prior, current) = review_policy_migration::prepare(
                &state,
                &input.operation_id,
                &input.current_shared_test_evidence,
            )?;
            let Some(assessment) = input.risk_assessment.clone() else {
                return Ok(text_content(json!({"migration_applied":false,"state_ref":state_reference(&state)?,"assignments":[assignment],
                    "prior_assignment_closure_required":state["lenses"],"next_tool":"final_review.migrate_policy",
                    "guidance":"The recorded legacy policy remains authoritative. Run the returned genuine fresh independent risk scout, close or attest never-started prior assignments, then resubmit this operation with its actual assessment and prior_assignment_closures."}).to_string()));
            };
            Some(Box::new(review_policy_migration::Migration {
                assessment,
                expected_assignment: expected,
                prior_source_tree: prior,
                observed_source_tree: current,
                prior_assignment_closures: input.prior_assignment_closures.clone(),
            }))
        } else {
            None
        };
        let now = (self.now_epoch_seconds)();
        let intent = SubmitReviewIterationIntent {
            reopen: Some(ReviewReopenIntent {
                migration,
                operation_id: input.operation_id.clone(),
                request_fingerprint,
                reason: input.reason,
            }),
            expected_prior_revision: self
                .session_revisions
                .get(session_id)
                .copied()
                .unwrap_or_default(),
            now_epoch_seconds: now,
            submission: ReviewIterationSubmission {
                verification_requests: Vec::new(),
                observed_dependency_blobs: BTreeMap::new(),
                lens_results: Vec::new(),
                caller_decisions: Vec::new(),
                current_diff_hash: input.current_diff_hash,
                current_changed_files: Some(input.current_changed_files),
                current_shared_test_evidence: Some(input.current_shared_test_evidence),
                security_escalations: None,
                unrelated_follow_ups: None,
            },
        };
        self.submit_review_iteration(&json!({"state":state}), intent, now)
    }
}
