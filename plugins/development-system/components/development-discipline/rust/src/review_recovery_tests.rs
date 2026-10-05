use super::*;

fn rpc(coordinator: &mut ReviewCoordinator, name: &str, arguments: Value) -> Value {
    coordinator
        .handle_json_rpc(&json!({"jsonrpc":"2.0", "id":1,
        "method":"tools/call", "params":{"name":name,"arguments":arguments}}))
        .unwrap()
}

fn completed_review(ship: bool) -> (PathBuf, Value) {
    let root = test_project_root(&format!(
        "reopen-{ship}-{}",
        SNAPSHOT_INDEX_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let mut coordinator = ReviewCoordinator::with_clock(|| 1000);
    let mut result = parsed_tool_text(&rpc(
        &mut coordinator,
        "final_review.plan",
        assessed_plan_arguments_for_diff_at_root(
            "reopen-session",
            "before",
            "medium",
            &[("correctness-behavior", "medium")],
            json!([]),
            Some(&root),
        ),
    ));
    for i in 0..3 {
        if ship && i == 2 {
            coordinator.now_epoch_seconds = Box::new(|| 5500);
        }
        result = parsed_tool_text(&rpc(
            &mut coordinator,
            "final_review.advance",
            json!({
            "state_ref":result["state_ref"], "current_diff_hash":"before",
            "lens_results":clean_lens_results_for(&result["state"])}),
        ));
    }
    if ship {
        result = parsed_tool_text(&rpc(
            &mut coordinator,
            "final_review.advance",
            json!({
            "state_ref":result["state_ref"], "current_diff_hash":"before", "lens_results":[],
            "review_budget_decision":{"decision":"ship","rationale":"All required review receipts are complete."}}),
        ));
    }
    assert_eq!(result["complete"], true, "{result}");
    (root, result)
}

#[test]
fn completed_sessions_reopen_with_durable_delta_request_and_fresh_lenses() {
    for ship in [false, true] {
        let (root, completed) = completed_review(ship);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn revised() {}\n").unwrap();
        let args = json!({"state_ref":completed["state_ref"], "operation_id":"reopen-1",
            "reason":"Source changed after review.", "current_diff_hash":"after",
            "current_changed_files":["src/lib.rs"],
            "current_shared_test_evidence":shared_test_evidence_for("after")});
        let mut restarted = ReviewCoordinator::with_clock(|| 5600);
        let reopened = parsed_tool_text(&rpc(&mut restarted, "final_review.reopen", args.clone()));
        assert_eq!(reopened["complete"], false);
        assert_eq!(reopened["state"]["clean_streak"], 0);
        assert_eq!(reopened["state"]["verified_clean_iterations"], json!([]));
        assert_eq!(
            reopened["state"]["session_id"],
            completed["state"]["session_id"]
        );
        assert_eq!(
            reopened["state"]["scope"]["baseline_commit"],
            completed["state"]["scope"]["baseline_commit"]
        );
        assert_eq!(
            reopened["state"]["finding_history"],
            completed["state"]["finding_history"]
        );
        assert_eq!(
            reopened["state"]["unresolved_findings"],
            completed["state"]["unresolved_findings"]
        );
        assert_eq!(
            reopened["transition_status"],
            "delta_risk_assessment_required"
        );
        let mut restarted = ReviewCoordinator::with_clock(|| 5601);
        let replay = parsed_tool_text(&rpc(&mut restarted, "final_review.reopen", args.clone()));
        assert_eq!(replay["state_ref"], reopened["state_ref"]);
        assert_eq!(
            replay["delta_risk_assignments"],
            reopened["delta_risk_assignments"]
        );
        let mut conflict = args.clone();
        conflict["reason"] = json!("different intent");
        assert!(
            rpc(&mut restarted, "final_review.reopen", conflict)["error"]["message"]
                .as_str()
                .unwrap()
                .contains("operation_id_conflict")
        );
        let mut stale = args.clone();
        stale["operation_id"] = json!("reopen-other");
        assert!(
            rpc(&mut restarted, "final_review.reopen", stale)["error"]["message"]
                .as_str()
                .unwrap()
                .contains("out_of_sync")
        );
        let assessment = delta_risk_assessment_for(
            &reopened["delta_risk_assignments"][0],
            "medium",
            &[("correctness-behavior", "medium")],
            &[],
            json!([]),
        );
        let advanced = parsed_tool_text(&rpc(
            &mut restarted,
            "final_review.advance",
            json!({
            "state_ref":reopened["state_ref"], "current_diff_hash":"after", "current_changed_files":["src/lib.rs"],
            "current_shared_test_evidence":shared_test_evidence_for("after"), "lens_results":[],
            "delta_risk_assessment":assessment}),
        ));
        assert_eq!(
            advanced["state"]["risk_plan"]["active_lenses"],
            advanced["state"]["risk_plan"]["selected_lenses"]
        );
        assert_eq!(advanced["complete"], false);
        assert_eq!(advanced["state"]["scope"]["diff_hash"], "after");
        let historic = parsed_tool_text(&rpc(&mut restarted, "final_review.reopen", args));
        assert_eq!(historic["response_historical"], true);
        assert_eq!(historic["next_tool"], "final_review.resume_latest");
        assert_eq!(historic["assignments_current"], false);
    }
}

#[test]
fn completed_ship_replay_retains_valid_contract_material() {
    let (_, completed) = completed_review(true);
    let state = completed["state"].clone();
    let session_id = state["session_id"].as_str().unwrap();
    let path = durable_report_database_path(state["scope"]["project_root"].as_str().unwrap(), None)
        .unwrap();
    let events = read_review_stream(
        &path,
        Some(Path::new(state["scope"]["project_root"].as_str().unwrap())),
        review_stream_id(session_id).unwrap(),
        ReviewPersistence::WorkflowAuthority,
    )
    .unwrap();
    let input = parse_advance_review_input(
        &json!({"state":state,"lens_results":[], "current_diff_hash":"after"}),
    )
    .unwrap();
    let command = SubmitReviewIteration {
        session_stream: FinalReviewStream(review_stream_id(session_id).unwrap()),
        catalog_stream: FinalReviewStream(catalog_stream_id().unwrap()),
        session_id: session_id.to_string(),
        intent: SubmitReviewIterationIntent {
            reopen: None,
            submission: review_iteration_submission(&input),
            expected_prior_revision: 0,
            now_epoch_seconds: 5600,
        },
    };
    let mut folded = Modeled::from_built(SubmitReviewIterationState {
        material: None,
        revision: None,
        catalog: ReviewCatalogRetention::default(),
    });
    for event in events {
        folded = command.evolve(folded, &event);
    }
    let material = folded.as_ref().material.as_ref().unwrap();
    assert!(
        material.contract.has_valid_id(),
        "ship replay must preserve the signed contract without rehashing"
    );
    assert!(material.contract.has_current_protocol());
}

#[test]
fn reopen_rejects_incomplete_test_receipts_before_resetting_completion() {
    let (_, completed) = completed_review(false);
    let mut coordinator = ReviewCoordinator::with_clock(|| 5600);
    let mut evidence = shared_test_evidence_for("after");
    evidence["commands"] = json!([]);
    let response = rpc(
        &mut coordinator,
        "final_review.reopen",
        json!({
        "state_ref":completed["state_ref"],"operation_id":"bad-tests", "reason":"Changed source",
        "current_diff_hash":"after","current_changed_files":["src/lib.rs"],
        "current_shared_test_evidence":evidence}),
    );
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("shared_test_evidence_commands_invalid"),
        "{response}"
    );
}
