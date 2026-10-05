mod budget_continuation_tests {
    use super::*;
    use std::sync::Arc;

    fn call(coordinator: &mut ReviewCoordinator, name: &str, arguments: Value) -> Value {
        let response = coordinator
            .handle_json_rpc(&json!({
                "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": {"name": name, "arguments": arguments}
            }))
            .expect("JSON-RPC response");
        assert!(response.get("error").is_none(), "{response}");
        serde_json::from_str(
            response["result"]["content"][0]["text"]
                .as_str()
                .expect("result text"),
        )
        .expect("result JSON")
    }

    #[test]
    fn timed_checkpoint_can_continue_without_waiving_three_clean_rounds() {
        let clock = Arc::new(AtomicU64::new(1_000));
        let worker_clock = Arc::clone(&clock);
        let mut coordinator =
            ReviewCoordinator::with_clock(move || worker_clock.load(Ordering::SeqCst));
        let planned = call(
            &mut coordinator,
            "final_review.plan",
            assessed_plan_arguments(
                "continuable-budget",
                "medium",
                &[("correctness-behavior", "medium")],
                json!([]),
            ),
        );
        clock.store(5_500, Ordering::SeqCst);
        let checkpoint = call(
            &mut coordinator,
            "final_review.advance",
            json!({
                "state_ref": planned["state_ref"],
                "current_diff_hash": "continuable-budget-diff",
                "lens_results": clean_lens_results_for(&planned["state"])
            }),
        );
        assert_eq!(checkpoint["advance_kind"], "review_budget_checkpoint");
        assert_eq!(checkpoint["state"]["clean_streak"], 1);
        let continued = call(
            &mut coordinator,
            "final_review.continue_review",
            json!({
                "state_ref": checkpoint["state_ref"],
                "operation_id": "continue-first-window",
                "rationale": "The approved work still requires two complete clean rounds; no human decision is needed."
            }),
        );
        assert_eq!(continued["complete"], false);
        assert_eq!(continued["state"]["clean_streak"], 1);
        assert_eq!(
            continued["state"]["risk_plan"]["review_budget"]["started_at_epoch_seconds"],
            1_000
        );
        assert!(!continued["next_assignments"]
            .as_array()
            .expect("fresh assignments")
            .is_empty());
        let second = call(
            &mut coordinator,
            "final_review.advance",
            json!({
                "state_ref": continued["state_ref"],
                "current_diff_hash": "continuable-budget-diff",
                "lens_results": clean_lens_results_for(&continued["state"])
            }),
        );
        assert_eq!(second["complete"], false);
        assert_eq!(second["state"]["clean_streak"], 2);
        let third = call(
            &mut coordinator,
            "final_review.advance",
            json!({
                "state_ref": second["state_ref"],
                "current_diff_hash": "continuable-budget-diff",
                "lens_results": clean_lens_results_for(&second["state"])
            }),
        );
        assert_eq!(third["complete"], true);
        assert_eq!(third["state"]["clean_streak"], 3);
    }

    fn checkpoint_fixture(label: &str) -> Value {
        let session = format!(
            "{label}-{}",
            SNAPSHOT_INDEX_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let mut coordinator = ReviewCoordinator::with_clock(|| 1000);
        let plan = call(
            &mut coordinator,
            "final_review.plan",
            assessed_plan_arguments(
                &session,
                "medium",
                &[("correctness-behavior", "medium")],
                json!([]),
            ),
        );
        coordinator.now_epoch_seconds = Box::new(|| 5500);
        call(
            &mut coordinator,
            "final_review.advance",
            json!({
            "state_ref":plan["state_ref"], "current_diff_hash":plan["state"]["scope"]["diff_hash"],
            "lens_results":clean_lens_results_for(&plan["state"])}),
        )
    }

    fn continuation_arguments(checkpoint: &Value, operation_id: &str) -> Value {
        json!({"state_ref":checkpoint["state_ref"],"operation_id":operation_id,
            "rationale":"Required reviews remain and no human decision is necessary."})
    }

    fn raw_call(coordinator: &mut ReviewCoordinator, name: &str, arguments: Value) -> Value {
        coordinator
            .handle_json_rpc(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":name,"arguments":arguments}}))
            .unwrap()
    }

    #[test]
    fn budget_continuation_retry_survives_response_loss_and_restart() {
        let checkpoint = checkpoint_fixture("budget-retry");
        let args = continuation_arguments(&checkpoint, "window-1");
        let continued = call(
            &mut ReviewCoordinator::with_clock(|| 5500),
            "final_review.continue_review",
            args.clone(),
        );
        let mut restarted = ReviewCoordinator::with_clock(|| 7000);
        let replay = call(&mut restarted, "final_review.continue_review", args.clone());
        assert_eq!(replay["state_ref"], continued["state_ref"]);
        assert_eq!(
            replay["state"]["risk_plan"]["review_budget"]["assessment_history"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            replay["state"]["risk_plan"]["review_budget"]["started_at_epoch_seconds"],
            1000
        );
        assert_eq!(
            replay["state"]["risk_plan"]["review_budget"]["next_checkpoint_at_epoch_seconds"],
            10000
        );
        let mut conflict = args.clone();
        conflict["rationale"] = json!("Changed request");
        assert!(
            raw_call(&mut restarted, "final_review.continue_review", conflict)["error"]["message"]
                .as_str()
                .unwrap()
                .contains("operation_conflict")
        );
        let mut stale = args;
        stale["operation_id"] = json!("window-other");
        assert!(
            raw_call(&mut restarted, "final_review.continue_review", stale)["error"]["message"]
                .as_str()
                .unwrap()
                .contains("out_of_sync")
        );
    }

    #[test]
    fn budget_continuation_repeated_windows_preserve_clean_receipts_and_history() {
        let checkpoint = checkpoint_fixture("budget-windows");
        let first_args = continuation_arguments(&checkpoint, "window-1");
        let mut coordinator = ReviewCoordinator::with_clock(|| 5500);
        let continued = call(
            &mut coordinator,
            "final_review.continue_review",
            first_args.clone(),
        );
        coordinator.now_epoch_seconds = Box::new(|| 10000);
        let second_checkpoint = call(
            &mut coordinator,
            "final_review.advance",
            json!({
            "state_ref":continued["state_ref"],"current_diff_hash":continued["state"]["scope"]["diff_hash"],
            "lens_results":clean_lens_results_for(&continued["state"])}),
        );
        assert_eq!(
            second_checkpoint["advance_kind"],
            "review_budget_checkpoint"
        );
        let replay_at_checkpoint = call(
            &mut coordinator,
            "final_review.continue_review",
            first_args.clone(),
        );
        assert_eq!(replay_at_checkpoint["pending_phase"], "review-budget");
        assert_eq!(replay_at_checkpoint["pending_assignments"], json!([]));
        let continued = call(
            &mut coordinator,
            "final_review.continue_review",
            continuation_arguments(&second_checkpoint, "window-2"),
        );
        assert_eq!(continued["state"]["clean_streak"], 2);
        assert_eq!(
            continued["state"]["verified_clean_iterations"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            continued["state"]["risk_plan"]["review_budget"]["assessment_history"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            continued["state"]["risk_plan"]["review_budget"]["started_at_epoch_seconds"],
            1000
        );
        assert_eq!(
            continued["state"]["risk_plan"]["review_budget"]["next_checkpoint_at_epoch_seconds"],
            14500
        );
        let complete = call(
            &mut coordinator,
            "final_review.advance",
            json!({
            "state_ref":continued["state_ref"],"current_diff_hash":continued["state"]["scope"]["diff_hash"],
            "lens_results":clean_lens_results_for(&continued["state"])}),
        );
        assert_eq!(complete["complete"], true);
        let replay = call(
            &mut ReviewCoordinator::with_clock(|| 11000),
            "final_review.continue_review",
            first_args,
        );
        assert_eq!(replay["complete"], true);
        assert_eq!(replay["state_ref"], complete["state_ref"]);
        assert_eq!(replay["next_assignments"], json!([]));
    }

    #[test]
    fn budget_continuation_concurrent_retry_records_only_one_assessment() {
        let checkpoint = checkpoint_fixture("budget-concurrent");
        let args = continuation_arguments(&checkpoint, "window-1");
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let workers = (0..2)
            .map(|_| {
                let args = args.clone();
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    raw_call(
                        &mut ReviewCoordinator::with_clock(|| 5500),
                        "final_review.continue_review",
                        args,
                    )
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        let responses = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert!(
            responses
                .iter()
                .any(|response| response.get("result").is_some()),
            "{responses:?}"
        );
        for response in responses
            .iter()
            .filter(|response| response.get("error").is_some())
        {
            assert!(
                response["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("out_of_sync"),
                "{response}"
            );
        }
        let replay = call(
            &mut ReviewCoordinator::with_clock(|| 5700),
            "final_review.continue_review",
            args,
        );
        assert_eq!(
            replay["state"]["risk_plan"]["review_budget"]["assessment_history"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(replay["state"]["clean_streak"], 1);
    }

    #[test]
    fn budget_continuation_recovers_legacy_escalation_with_recorded_resolution() {
        let checkpoint = checkpoint_fixture("budget-legacy-escalation");
        let mut coordinator = ReviewCoordinator::with_clock(|| 5500);
        let held = call(
            &mut coordinator,
            "final_review.advance",
            json!({
            "state_ref":checkpoint["state_ref"],"current_diff_hash":checkpoint["state"]["scope"]["diff_hash"],"lens_results":[],
            "review_budget_decision":{"decision":"escalate","rationale":"Elapsed review budget interrupted unfinished reviews.","escalation_reference":"legacy-budget-checkpoint"}}),
        );
        let args = continuation_arguments(&held, "recover-legacy");
        let mut restarted = ReviewCoordinator::with_clock(|| 5600);
        assert!(
            raw_call(&mut restarted, "final_review.continue_review", args.clone())["error"]
                ["message"]
                .as_str()
                .unwrap()
                .contains("recovery_evidence_required")
        );
        let mut args = args;
        args["recovery_reference"]=json!("The interruption was elapsed time only; reviews remain within the approved work and no human input is necessary.");
        let continued = call(&mut restarted, "final_review.continue_review", args);
        assert_eq!(continued["complete"], false);
        assert_eq!(continued["state"]["clean_streak"], 1);
        assert_eq!(
            continued["state"]["risk_plan"]["review_budget"]["hold"],
            false
        );
        assert_eq!(
            continued["state"]["risk_plan"]["review_budget"]["assessment_history"][0]
                ["prior_decision"]["decision"],
            "escalate"
        );
        assert!(!continued["next_assignments"].as_array().unwrap().is_empty());
    }

    #[test]
    fn budget_continuation_replay_keeps_pending_delta_assignment_authoritative() {
        let checkpoint = checkpoint_fixture("budget-pending-delta");
        let args = continuation_arguments(&checkpoint, "window-1");
        let mut coordinator = ReviewCoordinator::with_clock(|| 5500);
        let continued = call(
            &mut coordinator,
            "final_review.continue_review",
            args.clone(),
        );
        let pending = call(
            &mut coordinator,
            "final_review.advance",
            json!({
            "state_ref":continued["state_ref"],"current_diff_hash":"revised", "current_changed_files":["src/lib.rs"],
            "current_shared_test_evidence":shared_test_evidence_for("revised"),"lens_results":[]}),
        );
        assert_eq!(
            pending["transition_status"],
            "delta_risk_assessment_required"
        );
        let replay = call(
            &mut ReviewCoordinator::with_clock(|| 5600),
            "final_review.continue_review",
            args,
        );
        assert_eq!(
            replay["next_assignments"],
            json!([]),
            "A retry cannot issue old lens assignments while delta assessment is pending."
        );
        assert_eq!(replay["pending_phase"], "delta-risk");
    }

    #[test]
    fn budget_continuation_replay_keeps_pending_verifier_assignment_authoritative() {
        let checkpoint = checkpoint_fixture("budget-pending-verifier");
        let args = continuation_arguments(&checkpoint, "window-1");
        let mut coordinator = ReviewCoordinator::with_clock(|| 5500);
        let continued = call(
            &mut coordinator,
            "final_review.continue_review",
            args.clone(),
        );
        let mut results = clean_lens_results_for(&continued["state"]);
        let mut finding = risk_finding_lens_result(
            &continued["state"],
            "correctness-behavior",
            "new-security-defect",
            "CRITICAL",
        );
        finding["findings"][0]["security_impact"] = json!("critical");
        for result in results.as_array_mut().unwrap() {
            if result["lens"] == "correctness-behavior" {
                *result = finding.clone();
            }
        }
        let pending = call(
            &mut coordinator,
            "final_review.advance",
            json!({
            "state_ref":continued["state_ref"],"current_diff_hash":continued["state"]["scope"]["diff_hash"],"lens_results":results}),
        );
        assert_eq!(pending["transition_status"], "verifier_required");
        let replay = call(
            &mut ReviewCoordinator::with_clock(|| 5600),
            "final_review.continue_review",
            args,
        );
        assert_eq!(replay["next_assignments"], json!([]));
        assert_eq!(replay["pending_phase"], "verifier");
        assert_eq!(
            replay["pending_assignments"][0]["assignment_id"],
            pending["verifier_assignment"]["assignment_id"]
        );
    }

    #[test]
    fn budget_continuation_replay_keeps_iteration_limit_hold_terminal() {
        let checkpoint = checkpoint_fixture("budget-iteration-limit");
        let args = continuation_arguments(&checkpoint, "window-1");
        let mut coordinator = ReviewCoordinator::with_clock(|| 5500);
        let mut result = call(
            &mut coordinator,
            "final_review.continue_review",
            args.clone(),
        );
        for _ in 0..MAX_REVIEW_ITERATIONS {
            if review_iteration_limit_hold_active(&result["state"]) {
                break;
            }
            result = call(
                &mut coordinator,
                "final_review.advance",
                json!({
                "state_ref":result["state_ref"],"current_diff_hash":result["state"]["scope"]["diff_hash"],"lens_results":[]}),
            );
        }
        assert_eq!(result["state"]["iteration_limit_hold"], true);
        let replay = call(
            &mut ReviewCoordinator::with_clock(|| 5600),
            "final_review.continue_review",
            args,
        );
        assert_eq!(replay["complete"], false);
        assert_eq!(replay["next_assignments"], json!([]));
        assert_eq!(replay["pending_phase"], "iteration-limit-hold");
        assert_eq!(replay["pending_assignments"], json!([]));
    }
}
