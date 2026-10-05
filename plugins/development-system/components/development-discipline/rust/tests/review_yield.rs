#[path = "../src/review_yield.rs"]
mod review_yield;

use serde_json::{json, Value};
fn round(complete: bool) -> Value {
    json!({"schema_version":1,"completed_lens_round":complete,
        "expected_lenses":["production-risk-footguns"],"completed_lenses":if complete {vec!["production-risk-footguns"]} else {vec![]},
        "scope":{"diff_hash":"abc","baseline_commit":"baseline"},"scope_changed":false,"source_changed":false,
        "raw_findings":[{"id":"one","lens":"production-risk-footguns","message":"allegation"}],
        "confirmed_findings":[],"duplicate_findings":[],"rejected_findings":[{"id":"one","lens":"production-risk-footguns"}],
        "reopened_findings":[],"repair_verified_findings":[],"verifier_evidence":{"assignment_id":"verifier-1","verdict":"rejected"}})
}
#[test]
fn distinguishes_completed_lens_rounds_from_resets_and_legacy_unknowns() {
    let state = json!({"session_id":"s","finding_history":[
        {"completed_iteration":1,"clean":false,"round_evidence":round(true)},
        {"completed_iteration":2,"clean":false,"round_evidence":round(false)},
        {"completed_iteration":3,"clean":false,"reset_reason":"scope-reset"}]});
    let report = review_yield::report(&state).unwrap();
    assert_eq!(report["observed_completed_lens_rounds"], 1);
    assert_eq!(report["legacy_rows_without_round_evidence"], 1);
    assert!(report["completed_lens_rounds"].is_null());
    assert!(report["totals"].is_null());
    assert_eq!(report["observed_totals"]["raw_allegations"], 2);
    assert_eq!(report["observed_totals"]["confirmed"], 0);
    assert_eq!(report["observed_totals"]["rejected"], 2);
}
#[test]
fn evidence_round_trip_is_stable_and_binds_scope_and_finding_identity() {
    let state = json!({"session_id":"s","finding_history":[{"completed_iteration":1,"round_evidence":round(true)}]});
    let first = review_yield::report(&state).unwrap();
    let reference = first["rounds"][0]["evidence_refs"]["raw_findings"][0]
        .as_str()
        .unwrap();
    let detail = review_yield::evidence(&state, reference).unwrap();
    assert_eq!(detail["finding"]["id"], "one");
    assert_eq!(detail["scope"]["diff_hash"], "abc");
    assert_eq!(review_yield::report(&state).unwrap(), first);
    let mut changed = state.clone();
    changed["finding_history"][0]["round_evidence"]["scope"]["diff_hash"] = json!("changed");
    assert!(review_yield::evidence(&changed, reference).is_err());
    assert!(review_yield::evidence(&state, "invented").is_err());
}
#[test]
fn complete_round_requires_every_expected_lens_once() {
    let mut evidence = round(true);
    evidence["expected_lenses"] = json!(["production-risk-footguns", "security-safety"]);
    let state = json!({"session_id":"s","finding_history":[{"round_evidence":evidence}]});
    assert!(review_yield::report(&state)
        .unwrap_err()
        .contains("completed_lenses"));
}
#[test]
fn clean_round_is_reported_without_claiming_it_was_useless() {
    let mut evidence = round(true);
    evidence["raw_findings"] = json!([]);
    evidence["rejected_findings"] = json!([]);
    evidence["verifier_evidence"] = Value::Null;
    let state = json!({"session_id":"s","finding_history":[{"completed_iteration":1,"clean":true,"round_evidence":evidence}]});
    let report = review_yield::report(&state).unwrap();
    assert_eq!(report["completed_lens_rounds"], 1);
    assert_eq!(report["totals"]["raw_allegations"], 0);
    assert_eq!(report["rounds"][0]["clean"], true);
    assert!(report.get("usefulness").is_none());
}

#[test]
fn repeated_confirmations_and_reopened_findings_are_not_new_findings() {
    let mut first = round(true);
    first["confirmed_findings"] = json!([{"id":"one","lens":"production-risk-footguns"}]);
    first["rejected_findings"] = json!([]);
    let mut later = first.clone();
    later["scope_changed"] = json!(true);
    later["source_changed"] = json!(true);
    later["reopened_findings"] = later["confirmed_findings"].clone();
    later["repair_verified_findings"] = later["confirmed_findings"].clone();
    let state = json!({"session_id":"s","finding_history":[{"round_evidence":first},{"round_evidence":later}]});
    let report = review_yield::report(&state).unwrap();
    assert_eq!(report["totals"]["confirmed"], 2);
    assert_eq!(report["totals"]["new_confirmed"], 1);
    assert_eq!(report["totals"]["reopened"], 1);
    assert_eq!(report["totals"]["repair_verified"], 1);
    assert_eq!(report["rounds"][1]["scope_changed"], true);
    assert_eq!(report["rounds"][1]["source_changed"], true);
}

#[test]
fn verdict_counts_require_persisted_adjudication_evidence() {
    let mut raw = round(true);
    raw["verifier_evidence"] = Value::Null;
    let state = json!({"session_id":"s","finding_history":[{"round_evidence":raw}]});
    assert!(review_yield::report(&state)
        .unwrap_err()
        .contains("adjudication_evidence_required"));
}

#[test]
fn evidence_reference_survives_unrelated_history_retention() {
    let state = json!({"session_id":"s","finding_history":[{"completed_iteration":1}, {"completed_iteration":2,"round_evidence":round(true)}]});
    let report = review_yield::report(&state).unwrap();
    let reference = report["rounds"][1]["evidence_refs"]["raw_findings"][0]
        .as_str()
        .unwrap();
    let retained =
        json!({"session_id":"s","finding_history":[state["finding_history"][1].clone()]});
    assert!(review_yield::evidence(&retained, reference).is_ok());
}

#[test]
fn rejected_raw_allegation_never_reports_a_clean_round() {
    let state = json!({"session_id":"s","finding_history":[{"completed_iteration":1,"clean":true,"reset_reason":"findings_or_malformed_results","round_evidence":round(true)}]});
    let report = review_yield::report(&state).unwrap();
    assert_eq!(report["rounds"][0]["clean"], false);
    assert_eq!(report["rounds"][0]["finding_free"], false);
}

#[test]
fn truncated_history_reports_observed_counts_without_full_totals_or_false_novelty() {
    let mut observed = round(true);
    observed["confirmed_findings"] =
        json!([{"id":"seen-before-pruning","lens":"production-risk-footguns"}]);
    observed["rejected_findings"] = json!([]);
    let state = json!({"session_id":"s","iteration_index":2,"finding_history":[{"completed_iteration":1,"omitted_prior_history_rows":65,"round_evidence":observed}]});
    let report = review_yield::report(&state).unwrap();
    assert_eq!(report["counts_available_for_full_history"], false);
    assert_eq!(report["coverage"]["status"], "retained_window_only");
    assert_eq!(report["coverage"]["omitted_prior_history_rows"], 65);
    assert!(report["totals"].is_null());
    assert!(report["completed_lens_rounds"].is_null());
    assert_eq!(report["observed_completed_lens_rounds"], 1);
    assert_eq!(report["observed_totals"]["confirmed"], 1);
    assert!(report["observed_totals"]["new_confirmed"].is_null());
    assert_eq!(
        report["observed_totals"]["first_confirmed_in_observed_history"],
        1
    );
    assert!(report["rounds"][0]["counts"]["new_confirmed"].is_null());
    let reference = report["rounds"][0]["evidence_ref"].as_str().unwrap();
    assert!(review_yield::evidence(&state, reference).is_ok());
}

fn with_attempts(mut evidence: Value) -> Value {
    evidence["review_attempts"] = json!([{
        "lens":"production-risk-footguns", "subagent_key":"s:1:production-risk-footguns",
        "assigned_subagent_key":"s:1:production-risk-footguns", "submitted_status":"findings",
        "caller_attestation":{"model_role":"reviewer","fresh_context":true,"closed_after_result":true},
        "actual_model":null,"scope_bound":true,"disposition":"accepted","malformed_reasons":[]
    }]);
    evidence
}

#[test]
fn review_attempt_counts_stay_unknown_for_legacy_and_pruned_rows() {
    for history in [
        json!([{"completed_iteration":1,"round_evidence":round(true)},{"completed_iteration":2,"round_evidence":with_attempts(round(true))}]),
        json!([{"omitted_prior_history_rows":65,"round_evidence":with_attempts(round(true))}]),
        json!([{"completed_iteration":1},{"round_evidence":with_attempts(round(true))}]),
    ] {
        let report =
            review_yield::report(&json!({"session_id":"s","finding_history":history})).unwrap();
        assert!(report["review_counts"].is_null());
        assert!(report["round_attempts"].is_null());
        assert_eq!(report["observed_round_attempts"], 1);
        assert_eq!(
            report["observed_review_counts"],
            json!({"submitted":1,"accepted":1,"malformed":0})
        );
        assert_eq!(report["review_counts_available_for_full_history"], false);
    }
}

#[test]
fn review_attempt_evidence_preserves_references_after_retention() {
    let state = json!({"session_id":"s","finding_history":[{"completed_iteration":1}, {"completed_iteration":2,"round_evidence":with_attempts(round(true))}]});
    let report = review_yield::report(&state).unwrap();
    let reference = report["rounds"][1]["evidence_ref"].as_str().unwrap();
    let retained =
        json!({"session_id":"s","finding_history":[state["finding_history"][1].clone()]});
    let detail = review_yield::evidence(&retained, reference).unwrap();
    assert_eq!(
        detail["round_evidence"]["review_attempts"][0]["caller_attestation"]["model_role"],
        "reviewer"
    );
    assert!(detail["round_evidence"]["review_attempts"][0]["actual_model"].is_null());
}

#[test]
fn accepted_attempts_require_bound_assignments_and_no_malformed_reasons() {
    for (key, value) in [
        ("scope_bound", json!(false)),
        ("malformed_reasons", json!(["missing pii classification"])),
    ] {
        let mut evidence = with_attempts(round(true));
        evidence["review_attempts"][0][key] = value;
        assert!(review_yield::report(
            &json!({"session_id":"s","finding_history":[{"round_evidence":evidence}]})
        )
        .is_err());
    }
}

#[test]
fn incomplete_raw_containers_never_become_zero_allegations_or_clean_evidence() {
    let mut evidence = with_attempts(round(false));
    evidence["raw_findings"] = json!([]);
    evidence["raw_findings_complete"] = json!(false);
    evidence["rejected_findings"] = json!([]);
    let state = json!({"session_id":"s","finding_history":[{"completed_iteration":1,"clean":true,"round_evidence":evidence}]});
    let report = review_yield::report(&state).unwrap();
    assert_eq!(report["completed_lens_rounds"], 0);
    assert_eq!(report["review_counts"]["submitted"], 1);
    assert_eq!(report["counts_available_for_full_history"], false);
    assert_eq!(
        report["raw_allegation_counts_available_for_full_history"],
        false
    );
    assert!(report["totals"]["raw_allegations"].is_null());
    assert!(report["observed_totals"]["raw_allegations"].is_null());
    assert_eq!(report["observed_totals"]["retained_raw_allegations"], 0);
    assert!(report["rounds"][0]["finding_free"].is_null());
    assert_eq!(report["rounds"][0]["clean"], false);
    let mut invalid = state;
    invalid["finding_history"][0]["round_evidence"]["completed_lens_round"] = json!(true);
    assert!(review_yield::report(&invalid)
        .unwrap_err()
        .contains("complete_round_raw_evidence_required"));
}
