//! Durable budget assessments are progress checkpoints, never review waivers.
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Assessment {
    pub operation_id: String,
    pub request_fingerprint: String,
    pub recorded_at_epoch_seconds: u64,
    pub rationale: String,
    pub prior_decision: Option<ReviewBudgetDecisionFacts>,
}

pub(super) fn validate_operation_id(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err("review_continuation_operation_id_invalid=true".to_string());
    }
    Ok(())
}

impl ReviewCoordinator {
    pub(super) fn continue_review(&mut self, arguments: &Value, now: u64) -> Result<Value, String> {
        let object = arguments
            .as_object()
            .ok_or_else(|| "review_continuation_arguments_invalid=true".to_string())?;
        if object.keys().any(|key| {
            !matches!(
                key.as_str(),
                "state_ref" | "operation_id" | "rationale" | "recovery_reference"
            )
        }) {
            return Err("review_continuation_unknown_field=true".to_string());
        }
        let state_ref = arguments
            .get("state_ref")
            .ok_or_else(|| "review_continuation_state_ref_required=true".to_string())?;
        let operation_id = required_argument(arguments, "operation_id")?;
        validate_operation_id(operation_id)?;
        let rationale = required_argument(arguments, "rationale")?;
        let recovery_reference = arguments
            .get("recovery_reference")
            .map(|value| {
                value.as_str().map(str::to_string).ok_or_else(|| {
                    "review_continuation_recovery_reference_invalid=true".to_string()
                })
            })
            .transpose()?;
        let request_fingerprint =
            stable_storage_digest(&["review-continuation-v1", &arguments.to_string()]);
        let session_id = required_argument(state_ref, "session_id")?;
        let project_root = required_argument(state_ref, "project_root")?;
        let mut lookup = json!({"scope": {"project_root": project_root}});
        if let Some(work_item) = state_ref.get("work_item_id") {
            lookup["work_item_id"] = work_item.clone();
        }
        let restored = load_authoritative_session_read_only(
            &lookup,
            session_id,
            self.service_surface.review_persistence(),
        )?
        .ok_or_else(|| "review_session_not_found=true".to_string())?;
        validate_restored_review_protocol_state(&restored.state)?;
        let canonical_ref = state_reference(&restored.state)?;
        for field in ["project_root", "work_item_id"] {
            if canonical_ref.get(field) != state_ref.get(field) {
                return Err(format!("review_state_binding_mismatch=true field={field}"));
            }
        }
        if let Some(history) = restored
            .state
            .pointer("/risk_plan/review_budget/assessment_history")
            .and_then(Value::as_array)
        {
            if let Some(previous) = history
                .iter()
                .find(|entry| entry["operation_id"] == operation_id)
            {
                if previous["request_fingerprint"] != request_fingerprint {
                    return Err("review_continuation_operation_conflict=true recovery=resume_latest_and_use_a_new_operation_id".to_string());
                }
                let current = self.resolve_reference_read_only(&canonical_ref)?;
                return continuation_response(self, &current, true);
            }
        }
        let state = self.resolve_reference_read_only(state_ref)?;
        let intent = SubmitReviewBudgetDecisionIntent {
            expected_prior_revision: self
                .session_revisions
                .get(session_id)
                .copied()
                .ok_or_else(|| "review_session_revision_missing=true".to_string())?,
            current_diff_hash: required_argument(
                state
                    .get("scope")
                    .ok_or_else(|| "review_scope_required=true".to_string())?,
                "diff_hash",
            )?
            .to_string(),
            lens_result_count: 0,
            decision: ReviewBudgetDecisionFacts::Continue {
                operation_id: operation_id.to_string(),
                request_fingerprint,
                rationale: rationale.to_string(),
                recovery_reference,
            },
            now_epoch_seconds: now,
        };
        self.submit_review_budget_decision(&json!({"state": state}), intent)?;
        let persisted = self
            .sessions
            .get(session_id)
            .ok_or_else(|| "review_session_projection_missing_after_execution=true".to_string())?;
        continuation_response(self, persisted, false)
    }
}

fn continuation_response(
    coordinator: &ReviewCoordinator,
    state: &Value,
    replayed: bool,
) -> Result<Value, String> {
    let typed = ReviewSessionState::parse_legacy_wire(state)?;
    let material = SubmitReviewIterationMaterial::from(typed);
    let complete = review_state_complete(state);
    let state_ref = state_reference(state)?;
    let pending = coordinator.pending_assignment_summary(state, &state_ref, None)?;
    let assignments = if pending["pending_phase"] != "lens-review"
        || complete
        || review_budget_checkpoint_pending(state)
        || review_budget_hold_active(state)
        || scope_split_hold_active(state)
    {
        Vec::new()
    } else {
        build_typed_review_assignments(ReviewAssignmentMaterial {
            coverage: material
                .contract
                .risk_plan
                .as_ref()
                .and_then(|p| p.coverage.as_ref()),
            resolutions: &retained_resolutions(&material.finding_history),
            iteration: material.iteration_index,
            session_id: &material.contract.session_id,
            lenses: &material.contract.lenses,
            lens_objectives: &material.contract.lens_objectives,
            model_roles: &material.contract.model_roles,
            scope: &material.contract.scope,
            context: &material.context,
            defenses: &material.current_defenses_by_lens,
            deferred_findings: &material.deferred_findings,
            shared_test_evidence: material.contract.shared_test_evidence.as_ref(),
        })?
    };
    Ok(text_content(json!({"state": state, "state_ref": state_reference(state)?,
        "transition_status": if replayed {"replayed"} else {"advanced"}, "advance_kind": "review_budget_continuation",
        "complete": complete, "completion_blockers": unresolved_findings(state), "next_assignments": assignments,
        "subagent_shutdown": [], "pending_phase": pending["pending_phase"],
        "pending_assignments":pending["assignments"], "pending_assignment_summary":pending,
        "prompt_retrieval_tool":"final_review.pending_assignments"}).to_string()))
}
