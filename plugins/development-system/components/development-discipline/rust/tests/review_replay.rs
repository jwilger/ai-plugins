//! Real compiled-binary coverage of the one-operation recovery boundary.
use serde_json::{json, Value};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn invoke(root: &Path, tool: &str, arguments: &Value) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_development-discipline-mcp"))
        .args(["--replay-review-operation", root.to_str().unwrap(), tool])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(arguments.to_string().as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn rejects_non_review_tools_before_mutating_anything() {
    let root = tempfile::tempdir_in("/tmp").unwrap();
    let result = invoke(root.path(), "setup.apply", &json!({}));
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("replay_operation_not_allowed"),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .trim_end_matches('\n')
        .to_string()
}

fn proportional_plan(fixture: &Fixture) -> Value {
    let mut arguments = fixture.plan_args();
    arguments["risk_assessment"]["coverage_policy"] = json!({
        "artifact_kind":"code", "freshness_identity":"fixture-environment-and-inputs-v1",
        "requirements":[{"lens":"correctness-behavior", "scope_paths":["source.txt"],
            "required_samples":1, "escalation":null}]
    });
    fixture.call("final_review.plan", &arguments)
}

fn proportional_results(fixture: &Fixture, planned: &Value) -> Value {
    let results: Vec<_> = planned["assignments"].as_array().unwrap().iter().map(|assignment| {
        json!({"lens":assignment["lens"], "subagent_key":assignment["subagent_key"],
            "status":"clean", "findings":[], "additional_broad_test_run":false,
            "shared_test_evidence_id":assignment["shared_test_evidence"]["id"],
            "coverage_evidence":{"dependency_blobs":{"source.txt":format!("100644:{}",git(&fixture.root,&["hash-object","source.txt"]))}},
            "caller_attestation":{"model_role":assignment["model_role"],"fresh_context":true,"closed_after_result":true}})
    }).collect();
    json!(results)
}

#[test]
fn proportional_review_accepts_host_verified_deletion_and_rejects_forged_absence() {
    for forged in [true, false] {
        let fixture = Fixture::new();
        std::fs::remove_file(fixture.root.join("source.txt")).unwrap();
        let planned = proportional_plan(&fixture);
        let assignment = &planned["assignments"][0];
        let result = json!({"lens":assignment["lens"],"subagent_key":assignment["subagent_key"],"status":"clean","findings":[],"additional_broad_test_run":false,
            "shared_test_evidence_id":assignment["shared_test_evidence"]["id"],"coverage_evidence":{"dependency_blobs":{"source.txt":"absent"}},
            "caller_attestation":{"model_role":assignment["model_role"],"fresh_context":true,"closed_after_result":true}});
        if forged {
            std::fs::write(fixture.root.join("source.txt"), "restored source\n").unwrap();
        }
        let output = fixture.run("final_review.advance", &json!({"state_ref":planned["state_ref"],"current_diff_hash":"replay-fixture","lens_results":[result]}));
        if forged {
            assert_eq!(
                payload(output)["complete"],
                false,
                "forged absence earned credit"
            );
        } else {
            assert_eq!(payload(output)["complete"], true);
        }
    }
}

#[test]
fn proportional_review_completes_one_independent_round() {
    let fixture = Fixture::new();
    let planned = proportional_plan(&fixture);
    assert_eq!(planned["assignments"].as_array().unwrap().len(), 1);
    let finished = fixture.call(
        "final_review.advance",
        &json!({
        "state_ref":planned["state_ref"], "current_diff_hash":"replay-fixture",
        "lens_results":proportional_results(&fixture,&planned)}),
    );
    assert_eq!(finished["complete"], true, "{finished}");
    assert_eq!(finished["next_assignments"], json!([]));
}

#[test]
fn proportional_review_rejects_forged_source_receipt() {
    let fixture = Fixture::new();
    let planned = proportional_plan(&fixture);
    let mut results = proportional_results(&fixture, &planned);
    results[0]["coverage_evidence"]["dependency_blobs"]["source.txt"] =
        json!(format!("100644:{}", fixture.baseline));
    let result = fixture.run("final_review.advance", &json!({
        "state_ref":planned["state_ref"], "current_diff_hash":"replay-fixture", "lens_results":results}));
    assert_eq!(
        payload(result)["complete"],
        false,
        "forged evidence acquired clean credit"
    );
    let resumed = fixture.call(
        "final_review.resume_latest",
        &json!({"session_id":"replay-persistence","project_root":fixture.root}),
    );
    assert_ne!(resumed["complete"], true);
}

fn multi_lens_proportional_arguments(fixture: &Fixture, samples: u64) -> Value {
    let mut arguments = fixture.plan_args();
    arguments["risk_assessment"]["overall_risk"] =
        json!(if samples > 1 { "high" } else { "medium" });
    for dimension in arguments["risk_assessment"]["dimensions"]
        .as_array_mut()
        .unwrap()
    {
        if dimension["lens"] == "correctness-behavior" || dimension["lens"] == "tests-verification"
        {
            dimension["risk"] = json!(if samples > 1 { "high" } else { "medium" });
            dimension["evidence"] = json!("The changed input can invalidate required data.");
            dimension["plausible_failure"] = json!("An input loses an accepted record.");
            dimension["material_impact"] = json!("Required data integrity fails.");
        }
    }
    arguments["risk_assessment"]["coverage_policy"] = json!({"artifact_kind":"code","freshness_identity":"fixture-environment-and-inputs-v1",
        "requirements":[
            {"lens":"correctness-behavior","scope_paths":["source.txt"],"required_samples":samples,
                "escalation":if samples > 1 {json!({"consequence":"Irrecoverable accepted-record loss", "residual_uncertainty":"Interacting paths are not fully captured by regression fixtures", "sample_count_rationale":"Three independent discovery samples cover the interacting paths"})} else {Value::Null}},
            {"lens":"tests-verification","scope_paths":["source.txt"],"required_samples":1,"escalation":null}]});
    arguments
}

#[test]
fn proportional_review_rejects_unjustified_samples_and_missing_coverage() {
    for violation in [
        "no-escalation",
        "low-risk-extra",
        "empty-freshness",
        "missing-path",
        "uncertain-omission",
    ] {
        let fixture = Fixture::new();
        let mut args = multi_lens_proportional_arguments(&fixture, 3);
        match violation {
            "no-escalation" => {
                args["risk_assessment"]["coverage_policy"]["requirements"][0]["escalation"] =
                    Value::Null
            }
            "low-risk-extra" => args["risk_assessment"]["dimensions"][0]["risk"] = json!("low"),
            "empty-freshness" => {
                args["risk_assessment"]["coverage_policy"]["freshness_identity"] = json!("")
            }
            "missing-path" => {
                args["risk_assessment"]["coverage_policy"]["requirements"] = json!([{ "lens":"correctness-behavior", "scope_paths":["unrelated.txt"], "required_samples":1,"escalation":null}])
            }
            _ => {
                for dimension in args["risk_assessment"]["dimensions"]
                    .as_array_mut()
                    .unwrap()
                {
                    if dimension["lens"] == "security-safety" {
                        dimension["uncertain"] = json!(true);
                    }
                }
            }
        }
        assert!(
            !fixture.run("final_review.plan", &args).status.success(),
            "accepted {violation}"
        );
    }
}

#[test]
fn proportional_review_exceptional_requires_supported_trigger_and_two_samples() {
    for (samples, trigger, valid) in [
        (1, "authentication-or-authorization-boundary", false),
        (2, "invented-trigger", false),
        (2, "authentication-or-authorization-boundary", true),
    ] {
        let fixture = Fixture::new();
        let mut args = multi_lens_proportional_arguments(&fixture, 2);
        args["risk_assessment"]["overall_risk"] = json!("exceptional");
        args["risk_assessment"]["exceptional_triggers"] = json!([trigger]);
        args["risk_assessment"]["dimensions"][0]["risk"] = json!("exceptional");
        args["risk_assessment"]["coverage_policy"]["requirements"][0]["required_samples"] =
            json!(samples);
        let output = fixture.run("final_review.plan", &args);
        assert_eq!(
            output.status.success(),
            valid,
            "{samples} {trigger}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        if valid {
            let mut current = payload(output);
            for sample in 0..2 {
                let assignments = if sample == 0 {
                    current.clone()
                } else {
                    json!({"assignments":current["next_assignments"]})
                };
                current = fixture.call("final_review.advance", &json!({"state_ref":current["state_ref"],"current_diff_hash":"replay-fixture","lens_results":proportional_results(&fixture,&assignments)}));
                assert_eq!(current["complete"], sample == 1);
            }
        }
    }
}

#[test]
fn proportional_review_extra_samples_only_repeat_the_consequential_lens() {
    let fixture = Fixture::new();
    let planned = fixture.call(
        "final_review.plan",
        &multi_lens_proportional_arguments(&fixture, 3),
    );
    let mut response = fixture.call("final_review.advance", &json!({"state_ref":planned["state_ref"],"current_diff_hash":"replay-fixture","lens_results":proportional_results(&fixture,&planned)}));
    assert_eq!(response["complete"], false);
    assert_eq!(response["next_assignments"].as_array().unwrap().len(), 1);
    assert_eq!(
        response["next_assignments"][0]["lens"],
        "correctness-behavior"
    );
    for expected_complete in [false, true] {
        let pending = json!({"assignments":response["next_assignments"]});
        response = fixture.call("final_review.advance", &json!({"state_ref":response["state_ref"],"current_diff_hash":"replay-fixture","lens_results":proportional_results(&fixture,&pending)}));
        assert_eq!(response["complete"], expected_complete);
    }
    assert_eq!(
        response["state"]["risk_plan"]["coverage"]["receipts"]["tests-verification"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let report = fixture.call(
        "final_review.yield_report",
        &json!({"state_ref":response["state_ref"]}),
    );
    assert_eq!(report["observed_review_counts"]["accepted"], 4);
    assert_eq!(report["observed_batch_counts"]["complete-round"], 1);
    assert_eq!(report["observed_batch_counts"]["additional-risk-sample"], 2);
}

#[test]
fn proportional_review_invalid_lifecycle_preserves_valid_peer_receipts() {
    let fixture = Fixture::new();
    let planned = fixture.call(
        "final_review.plan",
        &multi_lens_proportional_arguments(&fixture, 1),
    );
    let mut results = proportional_results(&fixture, &planned);
    results[1]["caller_attestation"]["closed_after_result"] = json!(false);
    let response = fixture.call("final_review.advance", &json!({"state_ref":planned["state_ref"],"current_diff_hash":"replay-fixture","lens_results":results}));
    assert_eq!(response["complete"], false);
    assert_eq!(response["next_assignments"].as_array().unwrap().len(), 1);
    assert_eq!(
        response["next_assignments"][0]["lens"],
        "tests-verification"
    );
    let pending = json!({"assignments":response["next_assignments"]});
    let finished = fixture.call("final_review.advance", &json!({"state_ref":response["state_ref"],"current_diff_hash":"replay-fixture","lens_results":proportional_results(&fixture,&pending)}));
    assert_eq!(finished["complete"], true);
}

#[test]
fn proportional_review_malformed_coverage_replaces_only_invalid_assignment() {
    for invalid in ["missing", "empty", "wrong-blob"] {
        let fixture = Fixture::new();
        let planned = fixture.call(
            "final_review.plan",
            &multi_lens_proportional_arguments(&fixture, 1),
        );
        let mut results = proportional_results(&fixture, &planned);
        match invalid {
            "missing" => {
                results[1]
                    .as_object_mut()
                    .unwrap()
                    .remove("coverage_evidence");
            }
            "empty" => results[1]["coverage_evidence"]["dependency_blobs"] = json!({}),
            _ => {
                results[1]["coverage_evidence"]["dependency_blobs"]["source.txt"] =
                    json!(format!("100644:{}", fixture.baseline))
            }
        }
        let response = fixture.call("final_review.advance", &json!({"state_ref":planned["state_ref"],"current_diff_hash":"replay-fixture","lens_results":results}));
        assert_eq!(
            response["complete"], false,
            "invalid evidence earned completion: {invalid}"
        );
        assert_eq!(
            response["next_assignments"].as_array().unwrap().len(),
            1,
            "valid peer was discarded: {invalid}"
        );
        assert_eq!(
            response["next_assignments"][0]["lens"],
            "tests-verification"
        );
        assert_ne!(
            response["next_assignments"][0]["subagent_key"],
            planned["assignments"][1]["subagent_key"]
        );
        let peer =
            response["state"]["risk_plan"]["coverage"]["receipts"]["correctness-behavior"].clone();
        assert_eq!(peer.as_array().unwrap().len(), 1);
        assert_eq!(
            peer[0]["subagent_key"],
            planned["assignments"][0]["subagent_key"]
        );
        let report = fixture.call(
            "final_review.yield_report",
            &json!({"state_ref":response["state_ref"]}),
        );
        assert_eq!(report["observed_review_counts"]["accepted"], 1);
        assert_eq!(report["observed_review_counts"]["malformed"], 1);
        let pending = json!({"assignments":response["next_assignments"]});
        let finished = fixture.call("final_review.advance", &json!({"state_ref":response["state_ref"],"current_diff_hash":"replay-fixture","lens_results":proportional_results(&fixture,&pending)}));
        assert_eq!(finished["complete"], true);
        assert_eq!(
            finished["state"]["risk_plan"]["coverage"]["receipts"]["correctness-behavior"],
            peer
        );
    }
}

#[test]
fn proportional_review_cannot_omit_an_applicable_security_dimension() {
    let fixture = Fixture::new();
    let mut arguments = multi_lens_proportional_arguments(&fixture, 1);
    for dimension in arguments["risk_assessment"]["dimensions"]
        .as_array_mut()
        .unwrap()
    {
        if dimension["lens"] == "security-safety" {
            dimension["risk"] = json!("medium");
        }
    }
    let result = fixture.run("final_review.plan", &arguments);
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stdout).contains("review_coverage_applicable_lens_missing")
    );
}

fn check_proportional_source_delta(whole_scope: bool, variant: &str) {
    let mut fixture = Fixture::new();
    std::fs::write(fixture.root.join("peer.txt"), "peer baseline\n").unwrap();
    if variant == "reordered-peer" {
        std::fs::write(fixture.root.join("peer-two.txt"), "peer two baseline\n").unwrap();
    }
    git(&fixture.root, &["add", "peer.txt"]);
    if variant == "reordered-peer" {
        git(&fixture.root, &["add", "peer-two.txt"]);
    }
    git(&fixture.root, &["commit", "--quiet", "-m", "peer fixture"]);
    fixture.baseline = git(&fixture.root, &["rev-parse", "HEAD"]);
    std::fs::write(fixture.root.join("peer.txt"), "changed peer\n").unwrap();
    let mut arguments = multi_lens_proportional_arguments(&fixture, 3);
    arguments["changed_files"] = json!(["source.txt", "peer.txt"]);
    arguments["risk_assessment"]["coverage_policy"]["requirements"][1]["scope_paths"] =
        json!(["peer.txt"]);
    if variant == "reordered-peer" {
        std::fs::write(fixture.root.join("peer-two.txt"), "changed peer two\n").unwrap();
        arguments["changed_files"] = json!(["source.txt", "peer.txt", "peer-two.txt"]);
        arguments["risk_assessment"]["coverage_policy"]["requirements"][1]["scope_paths"] =
            json!(["peer.txt", "peer-two.txt"]);
    }
    if matches!(variant, "lower-peer" | "omitted-peer") {
        arguments["risk_assessment"]["coverage_policy"]["requirements"][1]["required_samples"] =
            json!(3);
        arguments["risk_assessment"]["coverage_policy"]["requirements"][1]["escalation"] =
            arguments["risk_assessment"]["coverage_policy"]["requirements"][0]["escalation"]
                .clone();
    }
    let scout = fixture.call("final_review.assess_risk", &arguments);
    for field in ["assignment_id", "subagent_key"] {
        arguments["risk_assessment"][field] = scout["assignments"][0][field].clone();
    }
    let planned = fixture.call("final_review.plan", &arguments);
    let mut results = proportional_results(&fixture, &planned);
    results[1]["coverage_evidence"]["dependency_blobs"] =
        json!({"peer.txt":format!("100644:{}",git(&fixture.root,&["hash-object","peer.txt"]))});
    if variant == "reordered-peer" {
        results[1]["coverage_evidence"]["dependency_blobs"]["peer-two.txt"] = json!(format!(
            "100644:{}",
            git(&fixture.root, &["hash-object", "peer-two.txt"])
        ));
    }
    let reviewed = fixture.call("final_review.advance", &json!({"state_ref":planned["state_ref"],"current_diff_hash":"replay-fixture","lens_results":results}));
    std::fs::write(fixture.root.join("source.txt"), "causal repair\n").unwrap();
    let evidence = json!({"id":"repair-evidence","diff_hash":"repair-snapshot","status":"passed","summary":"Fresh repaired behavior and regression evidence.","commands":["fixture:repair"]});
    let mut delta_arguments = json!({"state_ref":reviewed["state_ref"],"current_diff_hash":"repair-snapshot","current_changed_files":["source.txt","peer.txt"],"current_shared_test_evidence":evidence,"lens_results":[]});
    delta_arguments["current_changed_files"] = arguments["changed_files"].clone();
    let delta = fixture.call("final_review.advance", &delta_arguments);
    let assignment = &delta["delta_risk_assignments"][0];
    let mut assessment = arguments["risk_assessment"].clone();
    assessment["assignment_id"] = assignment["assignment_id"].clone();
    assessment["subagent_key"] = assignment["subagent_key"].clone();
    assessment["shared_test_evidence_id"] = json!("repair-evidence");
    assessment["caller_attestation"]["model_role"] = assignment["model_role"].clone();
    assessment["prior_diff_hash"] = json!("replay-fixture");
    assessment["current_diff_hash"] = json!("repair-snapshot");
    assessment["whole_scope_affected"] = json!(whole_scope);
    assessment["invalidation_rationale"] =
        json!("The repaired source has no dependency on the unchanged peer behavior.");
    for dimension in assessment["dimensions"].as_array_mut().unwrap() {
        dimension["affected"] = json!(dimension["lens"] == "correctness-behavior");
    }
    if variant == "lower-peer" {
        assessment["coverage_policy"]["requirements"][1]["required_samples"] = json!(1);
        assessment["coverage_policy"]["requirements"][1]["escalation"] = Value::Null;
    }
    if variant == "omitted-peer" {
        for dimension in assessment["dimensions"].as_array_mut().unwrap() {
            if dimension["lens"] == "tests-verification" {
                dimension["risk"] = json!("none");
                dimension["plausible_failure"] = json!("none");
                dimension["material_impact"] = json!("none");
            }
        }
        assessment["coverage_policy"]["requirements"]
            .as_array_mut()
            .unwrap()
            .remove(1);
        assessment["coverage_policy"]["requirements"][0]["scope_paths"] =
            json!(["source.txt", "peer.txt"]);
    }
    if variant == "split-hold" {
        assessment["split_required"] = json!(true);
        assessment["split_rationale"] =
            json!("The source and peer are independently buildable and shippable packages.");
        assessment["scope_growth_triggers"] = json!(["new-subsystem"]);
        assessment["split_candidates"] = json!([("source", "source.txt"), ("peer", "peer.txt")].iter().map(|(component, path)| json!({
            "id": component, "title":format!("Ship {component}"), "scope_paths":[path],
            "acceptance_criteria":[format!("{component} is independently usable")],
            "independently_shippable_reason":format!("{component} has its own tests and release artifact"),
            "delivery_boundaries":{
                "build":{"evidence_kind":"independent-build","command":format!("build {component}"),"artifact":format!("{component} package")},
                "test":{"evidence_kind":"independent-test","command":format!("test {component}")},
                "shipping":{"evidence_kind":"independent-shipping","artifact":format!("{component} package"),"mechanism":"package-publish"}
            }
        })).collect::<Vec<_>>());
    }
    if variant == "reordered-peer" {
        assessment["coverage_policy"]["requirements"][1]["scope_paths"] =
            json!(["peer-two.txt", "peer.txt"]);
    }
    delta_arguments["state_ref"] = delta["state_ref"].clone();
    delta_arguments["delta_risk_assessment"] = assessment;
    if matches!(variant, "lower-peer" | "omitted-peer") {
        let before = fixture.authority();
        let output = fixture.run("final_review.advance", &delta_arguments);
        assert!(
            !output.status.success(),
            "unchanged outstanding samples were silently dropped"
        );
        let expected = if variant == "omitted-peer" {
            "review_coverage_prior_requirement_missing"
        } else {
            "review_coverage_sample_requirement_weakened"
        };
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(expected),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert_eq!(
            fixture.authority(),
            before,
            "invalid reassessment must not mutate authoritative history"
        );
        return;
    }
    let renewed = fixture.call("final_review.advance", &delta_arguments);
    if variant == "split-hold" {
        assert_eq!(renewed["transition_status"], "split_confirmation_required");
        assert_eq!(renewed["complete"], false);
        assert_eq!(renewed["next_assignments"], json!([]));
        assert_eq!(renewed["state"]["lenses"], json!([]));
        assert_eq!(renewed["state"]["risk_plan"]["active_lenses"], json!([]));
        assert_eq!(
            renewed["state"]["risk_plan"]["active_lens_passes"],
            json!({})
        );
        let request = json!({"state_ref":renewed["state_ref"],"confirmation_id":renewed["scope_split"]["confirmation_id"],"explicit_user_confirmation":true,"tracker_representation":"delivery-tickets"});
        for extra in ["state", "unexpected"] {
            let mut invalid = request.clone();
            invalid[extra] = if extra == "state" {
                renewed["state"].clone()
            } else {
                json!("not part of the confirmation contract")
            };
            let before = fixture.authority();
            let rejected = fixture.run("final_review.confirm_split", &invalid);
            assert!(!rejected.status.success(), "accepted extra field {extra}");
            assert!(
                String::from_utf8_lossy(&rejected.stdout)
                    .contains("split_confirmation_explicit_user_confirmation_required"),
                "{}",
                String::from_utf8_lossy(&rejected.stdout)
            );
            assert_eq!(
                fixture.authority(),
                before,
                "rejected confirmation changed authoritative history"
            );
        }
        let confirmed = fixture.call("final_review.confirm_split", &request);
        assert_eq!(confirmed["tracker_mutation_authorized"], false);
        assert_eq!(confirmed["blocking_dependencies_authorized"], false);
        assert_eq!(confirmed["scope_split"]["confirmation_required"], false);
        assert_eq!(
            confirmed["scope_split"]["confirmed_representation"],
            "delivery-tickets"
        );
        let resumed = fixture.call(
            "final_review.resume_latest",
            &json!({"project_root":fixture.root,"session_id":"replay-persistence"}),
        );
        assert_eq!(resumed["state_ref"], confirmed["state_ref"]);
        assert_eq!(resumed["complete"], false);
        return;
    }
    assert_eq!(
        renewed["next_assignments"].as_array().unwrap().len(),
        if whole_scope { 2 } else { 1 }
    );
    assert_eq!(
        renewed["next_assignments"][0]["lens"],
        "correctness-behavior"
    );
    assert_eq!(
        renewed["state"]["risk_plan"]["coverage"]["receipts"]["tests-verification"]
            .as_array()
            .unwrap()
            .len(),
        if whole_scope { 0 } else { 1 }
    );
    assert_eq!(
        renewed["state"]["risk_plan"]["coverage"]["receipts"]["correctness-behavior"],
        json!([])
    );
    assert_eq!(
        renewed["state"]["risk_plan"]["coverage"]["invalidations"][0]["lens"],
        "correctness-behavior"
    );
    if !whole_scope {
        let mut current = renewed;
        for sample in 0..3 {
            let assignments = json!({"assignments":current["next_assignments"]});
            current = fixture.call("final_review.advance", &json!({"state_ref":current["state_ref"],"current_diff_hash":"repair-snapshot","lens_results":proportional_results(&fixture,&assignments)}));
            assert_eq!(current["complete"], sample == 2);
        }
        let report = fixture.call(
            "final_review.yield_report",
            &json!({"state_ref":current["state_ref"]}),
        );
        assert_eq!(report["observed_batch_counts"]["scoped-repair"], 1);
        assert_eq!(report["observed_batch_counts"]["additional-risk-sample"], 2);
    }
}

#[test]
fn proportional_review_source_repair_keeps_independent_peer_coverage() {
    check_proportional_source_delta(false, "normal");
}

#[test]
fn proportional_review_whole_scope_impact_invalidates_every_dependency() {
    check_proportional_source_delta(true, "normal");
}

#[test]
fn proportional_review_delta_rejects_lowering_unaffected_sample_floor() {
    check_proportional_source_delta(false, "lower-peer");
}
#[test]
fn proportional_review_delta_rejects_omitting_unaffected_sample_obligation() {
    check_proportional_source_delta(false, "omitted-peer");
}

#[test]
fn proportional_review_delta_reuses_permuted_unchanged_scope() {
    check_proportional_source_delta(false, "reordered-peer");
}

#[test]
fn proportional_review_delta_split_hold_is_confirmable() {
    check_proportional_source_delta(false, "split-hold");
}

#[test]
fn proportional_review_metadata_delta_returns_authoritative_completion() {
    let fixture = Fixture::new();
    let mut arguments = fixture.plan_args();
    arguments["risk_assessment"]["coverage_policy"] = json!({"artifact_kind":"code","freshness_identity":"fixture-environment-v1","requirements":[{"lens":"correctness-behavior","scope_paths":["source.txt"],"required_samples":1,"escalation":null}]});
    let planned = fixture.call("final_review.plan", &arguments);
    let finished = fixture.call("final_review.advance", &json!({"state_ref":planned["state_ref"],"current_diff_hash":"replay-fixture","lens_results":proportional_results(&fixture,&planned)}));
    assert_eq!(finished["complete"], true);
    git(&fixture.root, &["add", "source.txt"]);
    let evidence = json!({"id":"staged-evidence","diff_hash":"staged-identity","status":"passed","summary":"Staging preserves exact source bytes and behavior.","commands":["fixture:source-parity"]});
    let request = json!({"state_ref":finished["state_ref"],"operation_id":"staging-only-reopen","reason":"Bind completed review to staged identity without changing source.","current_diff_hash":"staged-identity","current_changed_files":["source.txt"],"current_shared_test_evidence":evidence});
    let pending = fixture.call("final_review.reopen", &request);
    let assignment = &pending["delta_risk_assignments"][0];
    let mut assessment = arguments["risk_assessment"].clone();
    assessment["coverage_policy"] = finished["state"]["risk_plan"]["coverage"]["policy"].clone();
    for field in ["assignment_id", "subagent_key"] {
        assessment[field] = assignment[field].clone();
    }
    assessment["caller_attestation"]["model_role"] = assignment["model_role"].clone();
    assessment["shared_test_evidence_id"] = json!("staged-evidence");
    assessment["prior_diff_hash"] = json!("replay-fixture");
    assessment["current_diff_hash"] = json!("staged-identity");
    assessment["whole_scope_affected"] = json!(false);
    assessment["invalidation_rationale"] = json!("Exact source bytes, scope, behavior and freshness are unchanged; only staging bookkeeping changed.");
    for dimension in assessment["dimensions"].as_array_mut().unwrap() {
        dimension["affected"] = json!(false);
    }
    let renewed = fixture.call("final_review.advance", &json!({"state_ref":pending["state_ref"],"current_diff_hash":"staged-identity","current_changed_files":["source.txt"],"current_shared_test_evidence":evidence,"lens_results":[],"delta_risk_assessment":assessment}));
    assert_eq!(renewed["complete"], true, "{renewed}");
    assert_eq!(renewed["next_assignments"], json!([]));
    assert_eq!(
        renewed["state"]["risk_plan"]["coverage"]["receipts"],
        finished["state"]["risk_plan"]["coverage"]["receipts"],
        "retained receipt identities must remain historical"
    );
    let resumed = fixture.call(
        "final_review.resume_latest",
        &json!({"project_root":fixture.root,"session_id":"replay-persistence"}),
    );
    assert_eq!(resumed["state_ref"], renewed["state_ref"]);
    assert_eq!(resumed["complete"], true);
}

#[test]
fn proportional_review_completed_session_reopens_for_bound_delta_assessment() {
    let fixture = Fixture::new();
    let planned = proportional_plan(&fixture);
    let finished = fixture.call("final_review.advance", &json!({"state_ref":planned["state_ref"],"current_diff_hash":"replay-fixture","lens_results":proportional_results(&fixture,&planned)}));
    assert_eq!(finished["complete"], true);
    std::fs::write(
        fixture.root.join("source.txt"),
        "post-completion causal repair\n",
    )
    .unwrap();
    let request = json!({"state_ref":finished["state_ref"],"operation_id":"source-repair-after-completion","reason":"A real source repair requires scoped independent reassessment.","current_diff_hash":"repaired-snapshot","current_changed_files":["source.txt"],"current_shared_test_evidence":{"id":"repaired-evidence","diff_hash":"repaired-snapshot","status":"passed","summary":"Fresh repair regression passed.","commands":["fixture:repaired"]}});
    let reopened = fixture.call("final_review.reopen", &request);
    assert_eq!(reopened["complete"], false);
    assert_eq!(
        reopened["delta_risk_assignments"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        reopened["state"]["risk_plan"]["coverage"]["pending_reassessment"],
        true
    );
    let resumed = fixture.call(
        "final_review.resume_latest",
        &json!({"project_root":fixture.root,"session_id":"replay-persistence"}),
    );
    assert_eq!(resumed["pending_phase"], "delta-risk");
    assert_eq!(
        fixture.call("final_review.reopen", &request)["operation_replayed"],
        true
    );
}

#[test]
fn proportional_review_migration_inventory_order_preserves_scope_and_rejects_mutations() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("peer.txt"), "unchanged peer\n").unwrap();
    let mut args = fixture.plan_args();
    args["changed_files"] = json!(["source.txt", "peer.txt"]);
    let scout = fixture.call("final_review.assess_risk", &args);
    for field in ["assignment_id", "subagent_key"] {
        args["risk_assessment"][field] = scout["assignments"][0][field].clone();
    }
    let legacy = fixture.call("final_review.plan", &args);
    let request = json!({"state_ref":legacy["state_ref"],"operation_id":"permuted-policy-preview","reason":"Migrate the same source inventory regardless of serialization order.","current_diff_hash":"replay-fixture","current_changed_files":["peer.txt","source.txt"],"current_shared_test_evidence":legacy["state"]["shared_test_evidence"]});
    let preview = fixture.call("final_review.migrate_policy", &request);
    assert_eq!(preview["migration_applied"], false);
    assert_eq!(preview["assignments"].as_array().unwrap().len(), 1);
    let authority = fixture.authority();
    for changed in [
        json!(["peer.txt", "source.txt", "source.txt"]),
        json!(["source.txt"]),
        json!(["peer.txt", "source.txt", "other.txt"]),
    ] {
        let mut invalid = request.clone();
        invalid["current_changed_files"] = changed;
        assert!(!fixture
            .run("final_review.migrate_policy", &invalid)
            .status
            .success());
        assert_eq!(fixture.authority(), authority);
    }
    let mut wrong_hash = request.clone();
    wrong_hash["current_diff_hash"] = json!("different-source");
    assert!(!fixture
        .run("final_review.migrate_policy", &wrong_hash)
        .status
        .success());
    std::fs::write(fixture.root.join("source.txt"), "unreviewed replacement\n").unwrap();
    assert!(!fixture
        .run("final_review.migrate_policy", &request)
        .status
        .success());
    std::fs::write(fixture.root.join("source.txt"), "changed\n").unwrap();
    std::fs::set_permissions(
        fixture.root.join("source.txt"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    assert!(!fixture
        .run("final_review.migrate_policy", &request)
        .status
        .success());
    std::fs::set_permissions(
        fixture.root.join("source.txt"),
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert_eq!(
        fixture.call("final_review.migrate_policy", &request)["migration_applied"],
        false
    );
    assert_eq!(fixture.authority(), authority);
    let assignment = &preview["assignments"][0];
    let mut assessment = args["risk_assessment"].clone();
    for field in ["assignment_id", "subagent_key"] {
        assessment[field] = assignment[field].clone();
    }
    assessment["shared_test_evidence_id"] = assignment["shared_test_evidence"]["id"].clone();
    assessment["caller_attestation"]["model_role"] = assignment["model_role"].clone();
    assessment["coverage_policy"] = json!({"artifact_kind":"code","freshness_identity":"fixture-environment-v1","requirements":[{"lens":"correctness-behavior","scope_paths":["peer.txt","source.txt"],"required_samples":1,"escalation":null}]});
    let mut apply = request.clone();
    apply["risk_assessment"] = assessment;
    apply["prior_assignment_closures"] = json!(legacy["assignments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| json!({"subagent_key":a["subagent_key"],"disposition":"not-started"}))
        .collect::<Vec<_>>());
    let migrated = fixture.call("final_review.migrate_policy", &apply);
    assert!(migrated["state"]["risk_plan"]["coverage"].is_object());
    assert_eq!(
        migrated["state"]["scope"]["baseline_commit"],
        legacy["state"]["scope"]["baseline_commit"]
    );
    assert_eq!(migrated["state"]["required_clean_iterations"], 1);
    let pending = json!({"assignments":migrated["next_assignments"]});
    let mut results = proportional_results(&fixture, &pending);
    results[0]["coverage_evidence"]["dependency_blobs"]["peer.txt"] = json!(format!(
        "100644:{}",
        git(&fixture.root, &["hash-object", "peer.txt"])
    ));
    assert_eq!(fixture.call("final_review.advance", &json!({"state_ref":migrated["state_ref"],"current_diff_hash":"replay-fixture","lens_results":results}))["complete"], true);
}

#[test]
fn proportional_review_migration_requires_independent_assessment_and_replays() {
    let fixture = Fixture::new();
    let legacy = fixture.call("final_review.plan", &fixture.plan_args());
    let reviewed = fixture.call("final_review.advance", &json!({"state_ref":legacy["state_ref"],"current_diff_hash":"replay-fixture","lens_results":proportional_results(&fixture,&legacy)}));
    assert_eq!(reviewed["complete"], false);
    assert_eq!(reviewed["state"]["required_clean_iterations"], 3);
    let mut request = json!({"state_ref":reviewed["state_ref"],"operation_id":"approved-policy-v3","reason":"Apply the reviewed proportional policy while preserving baseline and obligations.",
        "current_diff_hash":"replay-fixture","current_changed_files":["source.txt"],
        "current_shared_test_evidence":reviewed["state"]["shared_test_evidence"]});
    let preview = fixture.call("final_review.migrate_policy", &request);
    assert_eq!(preview["migration_applied"], false);
    assert_eq!(
        fixture.call(
            "final_review.resume_latest",
            &json!({"project_root":fixture.root,"session_id":"replay-persistence"})
        )["state_ref"],
        reviewed["state_ref"]
    );
    let assignment = &preview["assignments"][0];
    let mut assessment = fixture.plan_args()["risk_assessment"].clone();
    for field in ["assignment_id", "subagent_key"] {
        assessment[field] = assignment[field].clone();
    }
    assessment["shared_test_evidence_id"] = assignment["shared_test_evidence"]["id"].clone();
    assessment["caller_attestation"]["model_role"] = assignment["model_role"].clone();
    assessment["coverage_policy"] = json!({"artifact_kind":"code","freshness_identity":"fixture-environment-v1","requirements":[{"lens":"correctness-behavior","scope_paths":["source.txt"],"required_samples":1,"escalation":null}]});
    request["risk_assessment"] = assessment;
    request["prior_assignment_closures"] = json!(reviewed["next_assignments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| json!({"subagent_key":a["subagent_key"],"disposition":"not-started"}))
        .collect::<Vec<_>>());
    let migrated = fixture.call("final_review.migrate_policy", &request);
    assert_eq!(migrated["state"]["required_clean_iterations"], 1);
    assert_eq!(
        migrated["state"]["scope"]["baseline_commit"],
        reviewed["state"]["scope"]["baseline_commit"]
    );
    assert_eq!(
        migrated["state"]["finding_history"],
        reviewed["state"]["finding_history"]
    );
    assert_eq!(migrated["next_assignments"].as_array().unwrap().len(), 1);
    let replayed = fixture.call("final_review.migrate_policy", &request);
    assert_eq!(replayed["operation_replayed"], true);
    assert_eq!(replayed["state_ref"], migrated["state_ref"]);
    let pending = json!({"assignments":migrated["next_assignments"]});
    let finished = fixture.call("final_review.advance", &json!({"state_ref":migrated["state_ref"],"current_diff_hash":"replay-fixture","lens_results":proportional_results(&fixture,&pending)}));
    assert_eq!(finished["complete"], true);
}

#[test]
fn proportional_review_independent_rejection_completes_without_a_ritual_rerun() {
    let fixture = Fixture::new();
    let planned = proportional_plan(&fixture);
    let mut results = proportional_results(&fixture, &planned);
    results[0]["status"] = json!("findings");
    results[0]["findings"] = json!([{"id":"disputed-source-claim","severity":"MINOR","causality":"caused","causality_evidence":"A reviewer alleges a source failure.","likelihood":"possible","security_impact":"none","safety_impact":"none","path":"source.txt","line":1,"message":"The alleged source failure needs independent adjudication.","relevance":{"category":"diff_changed_file","explanation":"The source is in the requested change."}}]);
    let pending = fixture.call("final_review.advance", &json!({"state_ref":planned["state_ref"],"current_diff_hash":"replay-fixture","lens_results":results,
        "verification_requests":[{"finding_id":"disputed-source-claim","lens":"correctness-behavior"}],"unrelated_follow_ups":[{"finding_id":"disputed-source-claim","lens":"correctness-behavior","ticket_reference":"fixture-active-task"}]}));
    assert_eq!(pending["transition_status"], "verifier_required");
    let assignment = &pending["verifier_assignment"];
    let finished = fixture.call("final_review.advance", &json!({"state_ref":pending["state_ref"],"current_diff_hash":"replay-fixture","lens_results":[],
        "verifier_result":{"subagent_key":assignment["subagent_key"],"assignment_id":assignment["assignment_id"],"model_role":assignment["model_role"],"status":"verified",
            "caller_attestation":{"model_role":assignment["model_role"],"fresh_context":true,"closed_after_result":true},"verdicts":[{"finding_id":"disputed-source-claim","lens":"correctness-behavior","verdict":"rejected","severity":"MINOR","causality":"caused","causality_evidence":"Independent source and fixture inspection disproves the allegation.","security_impact":"none","safety_impact":"none","rationale":"The alleged failing branch does not exist.","dependency_blobs":{"source.txt":format!("100644:{}",git(&fixture.root,&["hash-object","source.txt"]))},"assumptions":["The inspected source is the only input boundary."]}]}}));
    assert_eq!(
        finished["complete"], true,
        "independent rejection did not complete coverage"
    );
    assert_eq!(finished["next_assignments"], json!([]));
    let resumed = fixture.call(
        "final_review.resume_latest",
        &json!({"session_id":"replay-persistence","project_root":fixture.root}),
    );
    assert_eq!(resumed["complete"], true);
}

#[test]
fn proportional_review_malformed_verifier_replaces_only_adjudication() {
    for samples in [1, 2] {
        let fixture = Fixture::new();
        let mut args = multi_lens_proportional_arguments(&fixture, samples);
        args["risk_assessment"]["coverage_policy"]["requirements"][0]["required_samples"] =
            json!(1);
        args["risk_assessment"]["coverage_policy"]["requirements"][0]["escalation"] = Value::Null;
        args["risk_assessment"]["coverage_policy"]["requirements"][1]["required_samples"] =
            json!(samples);
        if samples > 1 {
            args["risk_assessment"]["coverage_policy"]["requirements"][1]["escalation"] = json!({"consequence":"Accepted data corruption", "residual_uncertainty":"Independent fixture interpretation may miss interacting cases", "sample_count_rationale":"Two independent interpretations reduce this consequential uncertainty"});
        }
        let planned = fixture.call("final_review.plan", &args);
        let mut results = proportional_results(&fixture, &planned);
        results[0]["status"] = json!("findings");
        results[0]["findings"] = json!([{"id":"disputed-claim","severity":"MINOR","causality":"caused","causality_evidence":"The reviewer alleges a changed source failure.","likelihood":"possible","security_impact":"none","safety_impact":"none","path":"source.txt","line":1,"message":"An ordinary allegation needs independent adjudication.","relevance":{"category":"diff_changed_file","explanation":"source.txt is reviewed."}}]);
        let pending = fixture.call("final_review.advance", &json!({"state_ref":planned["state_ref"],"current_diff_hash":"replay-fixture","lens_results":results,
        "verification_requests":[{"finding_id":"disputed-claim","lens":"correctness-behavior"}],"unrelated_follow_ups":[{"finding_id":"disputed-claim","lens":"correctness-behavior","ticket_reference":"fixture-active-task"}]}));
        let assignment = &pending["verifier_assignment"];
        let rejected = fixture.call("final_review.advance", &json!({"state_ref":pending["state_ref"],"current_diff_hash":"replay-fixture","lens_results":[],
        "verifier_result":{"subagent_key":assignment["subagent_key"],"assignment_id":assignment["assignment_id"],"model_role":assignment["model_role"],"status":"verified",
            "caller_attestation":{"model_role":assignment["model_role"],"fresh_context":false,"closed_after_result":true},"verdicts":[]}}));
        assert_eq!(rejected["complete"], false);
        assert_eq!(rejected["transition_status"], "verifier_required");
        assert_eq!(rejected["next_assignments"], json!([]));
        assert_ne!(
            rejected["verifier_assignment"]["subagent_key"],
            assignment["subagent_key"]
        );
        assert_eq!(
            rejected["state"]["risk_plan"]["coverage"]["receipts"]["tests-verification"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            fixture.call(
                "final_review.resume_latest",
                &json!({"session_id":"replay-persistence","project_root":fixture.root})
            )["pending_phase"],
            "verifier"
        );
        let replacement = &rejected["verifier_assignment"];
        let finished = fixture.call("final_review.advance", &json!({"state_ref":rejected["state_ref"],"current_diff_hash":"replay-fixture","lens_results":[],
        "verifier_result":{"subagent_key":replacement["subagent_key"],"assignment_id":replacement["assignment_id"],"model_role":replacement["model_role"],"status":"verified",
            "caller_attestation":{"model_role":replacement["model_role"],"fresh_context":true,"closed_after_result":true},"verdicts":[{"finding_id":"disputed-claim","lens":"correctness-behavior","verdict":"rejected","severity":"MINOR","causality":"caused","causality_evidence":"The current source has no alleged failing branch.","security_impact":"none","safety_impact":"none","rationale":"Independent source inspection disproved the allegation.","dependency_blobs":{"source.txt":format!("100644:{}",git(&fixture.root,&["hash-object","source.txt"]))},"assumptions":[]}]}}));
        assert_eq!(finished["complete"], samples == 1);
        if samples == 2 {
            assert_eq!(finished["next_assignments"].as_array().unwrap().len(), 1);
            assert_eq!(
                finished["next_assignments"][0]["lens"],
                "tests-verification"
            );
            assert_eq!(
                finished["state"]["risk_plan"]["coverage"]["receipts"]["tests-verification"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
        }
        let report = fixture.call(
            "final_review.yield_report",
            &json!({"state_ref":finished["state_ref"]}),
        );
        assert_eq!(
            report["observed_review_counts"]["accepted"], 2,
            "reused lens results were counted as fresh reviewers"
        );
        assert_eq!(
            report["observed_batch_counts"]["verifier-replacement"], 1,
            "{report}"
        );
    }
}

fn check_rendered_review_continuity(proof: Option<&str>, changed_output: bool, reordered: bool) {
    let mut fixture = Fixture::new();
    let render = |color: &str| {
        format!("<svg xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"10\" height=\"10\" fill=\"{color}\"/></svg>\n")
    };
    std::fs::write(fixture.root.join("diagram.svg"), render("blue")).unwrap();
    git(&fixture.root, &["add", "diagram.svg"]);
    if reordered {
        std::fs::write(fixture.root.join("second.svg"), render("blue")).unwrap();
        git(&fixture.root, &["add", "second.svg"]);
    }
    git(
        &fixture.root,
        &["commit", "--quiet", "-m", "render fixture"],
    );
    fixture.baseline = git(&fixture.root, &["rev-parse", "HEAD"]);
    std::fs::write(fixture.root.join("diagram.svg"), render("green")).unwrap();
    let mut arguments = multi_lens_proportional_arguments(&fixture, 3);
    arguments["changed_files"] = json!(["source.txt", "diagram.svg"]);
    arguments["risk_assessment"]["coverage_policy"]["requirements"][1]["scope_paths"] =
        json!(["diagram.svg"]);
    arguments["risk_assessment"]["coverage_policy"]["requirements"][1]["artifact_kind"] =
        json!("rendered-artifact");
    if reordered {
        std::fs::write(fixture.root.join("second.svg"), render("green")).unwrap();
        arguments["changed_files"] = json!(["source.txt", "diagram.svg", "second.svg"]);
        arguments["risk_assessment"]["coverage_policy"]["requirements"][1]["scope_paths"] =
            json!(["diagram.svg", "second.svg"]);
    }
    let scout = fixture.call("final_review.assess_risk", &arguments);
    for field in ["assignment_id", "subagent_key"] {
        arguments["risk_assessment"][field] = scout["assignments"][0][field].clone();
    }
    let planned = fixture.call("final_review.plan", &arguments);
    let mut results = proportional_results(&fixture, &planned);
    results[1]["coverage_evidence"]["dependency_blobs"] = json!({"diagram.svg":format!("100644:{}",git(&fixture.root,&["hash-object","diagram.svg"])),"source.txt":format!("100644:{}",git(&fixture.root,&["hash-object","source.txt"]))});
    if reordered {
        results[1]["coverage_evidence"]["dependency_blobs"]["second.svg"] = json!(format!(
            "100644:{}",
            git(&fixture.root, &["hash-object", "second.svg"])
        ));
    }
    let reviewed = fixture.call("final_review.advance", &json!({"state_ref":planned["state_ref"],"current_diff_hash":"replay-fixture","lens_results":results}));
    let prior_output = std::fs::read(fixture.root.join("diagram.svg")).unwrap();
    std::fs::write(fixture.root.join("source.txt"), "validator-only repair\n").unwrap();
    // A fresh fixture render explicitly checks the complete declared output.
    let current_output = render(if changed_output { "red" } else { "green" });
    std::fs::write(fixture.root.join("diagram.svg"), &current_output).unwrap();
    assert_eq!(prior_output == current_output.as_bytes(), !changed_output);
    let evidence = json!({"id":"render-parity","diff_hash":"validator-repair","status":"passed","summary":"Fresh current-source rendering compared for every declared fixture and rendering condition.","commands":["fixture:validate-and-render-parity"],"artifact_reference":"fixture-current-render-parity"});
    let mut request = json!({"state_ref":reviewed["state_ref"],"current_diff_hash":"validator-repair","current_changed_files":["source.txt","diagram.svg"],"current_shared_test_evidence":evidence,"lens_results":[]});
    request["current_changed_files"] = arguments["changed_files"].clone();
    let pending = fixture.call("final_review.advance", &request);
    let assigned = &pending["delta_risk_assignments"][0];
    let mut assessment = arguments["risk_assessment"].clone();
    for field in ["assignment_id", "subagent_key"] {
        assessment[field] = assigned[field].clone();
    }
    assessment["shared_test_evidence_id"] = json!("render-parity");
    assessment["caller_attestation"]["model_role"] = assigned["model_role"].clone();
    assessment["prior_diff_hash"] = json!("replay-fixture");
    assessment["current_diff_hash"] = json!("validator-repair");
    assessment["whole_scope_affected"] = json!(false);
    assessment["invalidation_rationale"] = json!("Current-source fixture rendering and unchanged coverage/conditions establish which visual output changed; source hash changes alone are not visual changes.");
    for dimension in assessment["dimensions"].as_array_mut().unwrap() {
        dimension["affected"] = json!(dimension["lens"] == "correctness-behavior");
    }
    if let Some(kind) = proof {
        assessment["render_continuity"] = json!([{"lens":"tests-verification","kind":kind,"current_diff_hash":"validator-repair","artifact_paths":["diagram.svg"],"evidence_reference":"fixture-current-render-parity"}]);
    }
    if reordered {
        assessment["coverage_policy"]["requirements"][1]["scope_paths"] =
            json!(["second.svg", "diagram.svg"]);
        assessment["render_continuity"][0]["artifact_paths"] = json!(["diagram.svg", "second.svg"]);
    }
    request["state_ref"] = pending["state_ref"].clone();
    request["delta_risk_assessment"] = assessment;
    let renewed = fixture.call("final_review.advance", &request);
    if reordered {
        assert_eq!(
            renewed["state"]["risk_plan"]["coverage"]["receipts"]["tests-verification"],
            reviewed["state"]["risk_plan"]["coverage"]["receipts"]["tests-verification"]
        );
    }
    let keep_visual = proof == Some("current-output-equivalence") && !changed_output;
    assert_eq!(
        renewed["next_assignments"].as_array().unwrap().len(),
        if keep_visual { 1 } else { 2 }
    );
    assert_eq!(
        renewed["state"]["risk_plan"]["coverage"]["receipts"]["tests-verification"]
            .as_array()
            .unwrap()
            .len(),
        if keep_visual { 1 } else { 0 }
    );
}

#[test]
fn proportional_review_validator_only_repair_reuses_proven_unchanged_visuals() {
    check_rendered_review_continuity(Some("current-output-equivalence"), false, false);
    // Changed generator inputs require current-output-equivalence, not an unchanged-input claim.
    check_rendered_review_continuity(Some("unchanged-render-inputs"), false, false);
}

#[test]
fn proportional_review_changed_or_unproven_visuals_require_fresh_review() {
    check_rendered_review_continuity(Some("current-output-equivalence"), true, false);
    check_rendered_review_continuity(None, false, false);
}

#[test]
fn proportional_review_delta_reuses_permuted_render_scope_and_proof() {
    check_rendered_review_continuity(Some("current-output-equivalence"), false, true);
}

struct Fixture {
    directory: tempfile::TempDir,
    root: std::path::PathBuf,
    state: std::path::PathBuf,
    baseline: String,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::Builder::new()
            .prefix("review replay ")
            .tempdir_in("/tmp")
            .unwrap();
        let root = directory.path().join("repository with spaces");
        std::fs::create_dir(&root).unwrap();
        git(&root, &["init", "--quiet"]);
        for (key, value) in [
            ("user.name", "Replay Fixture"),
            ("user.email", "replay@example.invalid"),
            ("commit.gpgsign", "false"),
            ("core.logAllRefUpdates", "always"),
        ] {
            git(&root, &["config", key, value]);
        }
        std::fs::write(root.join("source.txt"), "baseline\n").unwrap();
        git(&root, &["add", "source.txt"]);
        git(&root, &["commit", "--quiet", "-m", "fixture"]);
        let baseline = git(&root, &["rev-parse", "HEAD"]);
        std::fs::write(root.join("source.txt"), "changed\n").unwrap();
        let state = directory.path().join("state");
        Self {
            directory,
            root,
            state,
            baseline,
        }
    }
    fn command(&self, tool: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_development-discipline-mcp"));
        command.args([
            "--replay-review-operation",
            self.root.to_str().unwrap(),
            tool,
        ]);
        self.configure(&mut command);
        command
    }
    fn configure(&self, command: &mut Command) {
        command
            .env("XDG_STATE_HOME", &self.state)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    }
    fn run(&self, tool: &str, args: &Value) -> Output {
        run(self.command(tool), args)
    }
    fn call(&self, tool: &str, args: &Value) -> Value {
        payload(self.run(tool, args))
    }
    fn plan_args(&self) -> Value {
        let mut common = json!({"session_id":"replay-persistence", "baseline_commit":self.baseline,
            "scope":"uncommitted", "project_root":self.root, "changed_files":["source.txt"],
            "diff_hash":"replay-fixture", "shared_test_evidence":{"id":"replay-evidence", "diff_hash":"replay-fixture", "status":"passed", "summary":"The fixture checks isolated replay.", "commands":["fixture:replay"]}});
        let assessed = self.call("final_review.assess_risk", &common);
        let assignment = &assessed["assignments"][0];
        let dimensions: Vec<_> = assignment["review_dimensions"].as_array().unwrap().iter().enumerate().map(|(i,lens)| json!({"lens":lens,"risk":if i==0 {"low"} else {"none"},"evidence":"Bounded persistence fixture.","plausible_failure":if i==0 {"Lost review state"} else {"none"},"material_impact":if i==0 {"Cannot resume"} else {"none"},"uncertain":false})).collect();
        common["risk_assessment"] = json!({"assignment_id":assignment["assignment_id"],"subagent_key":assignment["subagent_key"],"shared_test_evidence_id":assignment["shared_test_evidence"]["id"],"overall_risk":"low","dimensions":dimensions,"exceptional_triggers":[],"split_required":false,"plan_assumptions":[],"findings":[],"caller_attestation":{"model_role":assignment["model_role"],"fresh_context":true,"closed_after_result":true}});
        common
    }
    fn common(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(git(
            &self.root,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        ))
    }
    fn authority(&self) -> Option<String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args([
                "rev-parse",
                "--verify",
                "refs/tiber/plugin-advisory-final-review",
            ])
            .output()
            .unwrap();
        output
            .status
            .success()
            .then(|| String::from_utf8(output.stdout).unwrap())
    }
    fn sign(&self) {
        let key = self.directory.path().join("signing key");
        let output = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&key)
            .output()
            .unwrap();
        assert!(output.status.success());
        git(&self.root, &["config", "gpg.format", "ssh"]);
        git(
            &self.root,
            &["config", "user.signingkey", key.to_str().unwrap()],
        );
        git(&self.root, &["config", "commit.gpgsign", "true"]);
    }
}
fn run(mut command: Command, args: &Value) -> Output {
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(args.to_string().as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}
fn payload(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[test]
fn real_review_persists_with_scoped_writes_and_preserves_source() {
    let fixture = Fixture::new();
    fixture.sign();
    let root = &fixture.root;
    let index = std::fs::read(root.join(".git/index")).unwrap();
    let config = std::fs::read(root.join(".git/config")).unwrap();
    let args = fixture.plan_args();
    let planned = fixture.call("final_review.plan", &args);
    let resumed = fixture.call(
        "final_review.resume_latest",
        &json!({"session_id":"replay-persistence","project_root":root}),
    );
    assert_eq!(planned["state_ref"], resumed["state_ref"]);
    let commit = fixture.authority().unwrap();
    assert!(git(root, &["cat-file", "-p", commit.trim()])
        .contains("gpgsig -----BEGIN SSH SIGNATURE-----"));
    assert_eq!(git(root, &["rev-parse", "HEAD"]), fixture.baseline);
    assert_eq!(std::fs::read(root.join(".git/index")).unwrap(), index);
    assert_eq!(std::fs::read(root.join(".git/config")).unwrap(), config);
    assert_eq!(
        std::fs::read_to_string(root.join("source.txt")).unwrap(),
        "changed\n"
    );
    // Delete only the disposable projection; Git reconstructs the same revision.
    std::fs::remove_dir_all(
        fixture
            .state
            .join("development-discipline/final-review-reports"),
    )
    .unwrap();
    let rebuilt = fixture.call(
        "final_review.resume_latest",
        &json!({"session_id":"replay-persistence","project_root":root}),
    );
    assert_eq!(planned["state_ref"], rebuilt["state_ref"]);
    assert_eq!(fixture.authority().unwrap(), commit);
}

#[test]
fn each_required_git_path_denial_is_diagnosed_and_recoverable() {
    for denied in [
        "objects",
        "refs/tiber",
        "logs/refs/tiber",
        "plugin-advisory-final-review",
    ] {
        let fixture = Fixture::new();
        let args = fixture.plan_args();
        let common = fixture.common();
        let scratch = fixture.directory.path().join("scratch");
        std::fs::create_dir(&scratch).unwrap();
        let mut command = Command::new("bwrap");
        command.args([
            "--ro-bind",
            "/",
            "/",
            "--unshare-net",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
        ]);
        for relative in [
            "objects",
            "refs/tiber",
            "logs/refs/tiber",
            "plugin-advisory-final-review",
        ] {
            if relative != denied {
                let p = common.join(relative);
                command.arg("--bind").arg(&p).arg(&p);
            }
        }
        for p in [&fixture.state, &scratch] {
            command.arg("--bind").arg(p).arg(p);
        }
        command
            .arg("--")
            .arg(env!("CARGO_BIN_EXE_development-discipline-mcp"))
            .arg("--replay-review-operation-child")
            .arg(&fixture.root)
            .arg("final_review.plan")
            .env("TMPDIR", &scratch)
            .env("TIBER_EVENT_STORE_DIAGNOSTICS", "1");
        fixture.configure(&mut command);
        let failed = run(command, &args);
        assert!(!failed.status.success(), "denied {denied} must fail");
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&failed.stdout),
            String::from_utf8_lossy(&failed.stderr)
        );
        assert!(
            diagnostics.contains(denied),
            "missing denied path {denied}: {diagnostics}"
        );
        assert!(
            fixture.authority().is_none(),
            "denied {denied} must not publish"
        );
        fixture.call("final_review.plan", &args);
        assert!(fixture.authority().is_some());
    }
}

#[test]
fn signing_failure_is_not_bypassed_and_retry_keeps_one_review() {
    let fixture = Fixture::new();
    let args = fixture.plan_args();
    fixture.sign();
    git(&fixture.root, &["config", "gpg.ssh.program", "/bin/false"]);
    let failed = fixture.run("final_review.plan", &args);
    assert!(!failed.status.success());
    assert!(fixture.authority().is_none());
    assert!(
        String::from_utf8_lossy(&failed.stderr).contains("phase=signing"),
        "{}",
        String::from_utf8_lossy(&failed.stderr)
    );
    git(&fixture.root, &["config", "--unset", "gpg.ssh.program"]);
    let planned = fixture.call("final_review.plan", &args);
    let commit = fixture.authority();
    // A repeated submission must not append a second planned review.
    let _ = fixture.run("final_review.plan", &args);
    assert_eq!(fixture.authority(), commit);
    let resumed = fixture.call(
        "final_review.resume_latest",
        &json!({"session_id":"replay-persistence","project_root":fixture.root}),
    );
    assert_eq!(planned["state_ref"], resumed["state_ref"]);
}

#[test]
fn linked_worktree_recovers_using_common_directory() {
    let mut fixture = Fixture::new();
    let linked = fixture
        .directory
        .path()
        .join("linked worktree\nwith newline");
    git(
        &fixture.root,
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "linked",
            linked.to_str().unwrap(),
        ],
    );
    fixture.root = linked;
    std::fs::write(fixture.root.join("source.txt"), "linked change\n").unwrap();
    let args = fixture.plan_args();
    fixture.call("final_review.plan", &args);
    assert!(fixture.authority().is_some());
}

#[test]
fn sandbox_blocks_signer_source_index_config_other_refs_and_network_writes() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let args = fixture.plan_args();
    fixture.sign();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let script = fixture.directory.path().join("probe-signer");
    let paths = [
        fixture.root.join("source.txt"),
        fixture.common().join("index"),
        fixture.common().join("config"),
        fixture.common().join("refs/heads/forbidden"),
        fixture.directory.path().join("unrelated-state"),
    ];
    let mut body = String::from("#!/usr/bin/env python3\nimport os, socket, sys\n");
    for path in paths {
        body.push_str(&format!("try:\n f=os.open({:?}, os.O_WRONLY|os.O_CREAT|os.O_APPEND, 0o600); os.close(f); print('BOUNDARY_BROKEN', file=sys.stderr)\nexcept OSError: print('WRITE_DENIED', file=sys.stderr)\n",path.to_string_lossy()));
    }
    body.push_str(&format!("s=socket.socket(); s.settimeout(0.2)\ntry:\n s.connect(('127.0.0.1',{port})); print('BOUNDARY_BROKEN',file=sys.stderr)\nexcept OSError: print('NETWORK_DENIED',file=sys.stderr)\nsys.exit(1)\n"));
    std::fs::write(&script, body).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    git(
        &fixture.root,
        &["config", "gpg.ssh.program", script.to_str().unwrap()],
    );
    let failed = fixture.run("final_review.plan", &args);
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(!failed.status.success());
    assert!(!stderr.contains("BOUNDARY_BROKEN"), "{stderr}");
    assert_eq!(stderr.matches("WRITE_DENIED").count(), 5, "{stderr}");
    assert!(stderr.contains("NETWORK_DENIED"), "{stderr}");
    assert!(fixture.authority().is_none());
}

#[test]
fn interrupted_signing_can_be_retried_without_forging_completion() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let args = fixture.plan_args();
    fixture.sign();
    let entered = fixture
        .common()
        .join("plugin-advisory-final-review/signing-entered");
    let script = fixture.directory.path().join("paused-signer");
    std::fs::write(&script,format!("#!/usr/bin/env python3\nimport pathlib,time\npathlib.Path({:?}).write_text('entered')\ntime.sleep(60)\n",entered.to_string_lossy())).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    git(
        &fixture.root,
        &["config", "gpg.ssh.program", script.to_str().unwrap()],
    );
    let mut child = fixture.command("final_review.plan").spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(args.to_string().as_bytes())
        .unwrap();
    for _ in 0..100 {
        if entered.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(
        entered.exists(),
        "signer must reach the deterministic interruption point"
    );
    child.kill().unwrap();
    let _ = child.wait_with_output().unwrap();
    assert!(fixture.authority().is_none());
    git(&fixture.root, &["config", "--unset", "gpg.ssh.program"]);
    let planned = fixture.call("final_review.plan", &args);
    assert_eq!(planned["state"]["clean_streak"], 0);
    assert!(fixture.authority().is_some());
}

#[test]
fn state_inside_source_is_rejected_before_creating_any_directories() {
    for relative in ["forbidden-state", "", "alias"] {
        let fixture = Fixture::new();
        let state = if relative == "alias" {
            let outside = fixture.directory.path().join("outside");
            std::fs::create_dir(&outside).unwrap();
            outside.join("../repository with spaces/forbidden-state")
        } else {
            fixture.root.join(relative)
        };
        let forbidden = if relative.is_empty() {
            fixture.root.join("development-discipline")
        } else {
            fixture.root.join("forbidden-state")
        };
        let common = fixture.common();
        let mut command = fixture.command("final_review.assess_risk");
        command.env("XDG_STATE_HOME", &state);
        let failed = run(command, &json!({"project_root":fixture.root}));
        assert!(!failed.status.success());
        assert!(String::from_utf8_lossy(&failed.stderr)
            .contains("replay_state_inside_source_forbidden"));
        assert!(
            !forbidden.exists(),
            "rejected recovery must not create directories in source"
        );
        for relative in [
            "refs/tiber",
            "logs/refs/tiber",
            "plugin-advisory-final-review",
        ] {
            assert!(
                !common.join(relative).exists(),
                "rejected recovery must not prepare writable mount {relative}"
            );
        }
    }
}

#[test]
fn report_path_symlinks_are_rejected_without_creating_state_outside_scope() {
    let fixture = Fixture::new();
    let outside = fixture.directory.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, &fixture.state).unwrap();
    let failed = fixture.run(
        "final_review.assess_risk",
        &json!({"project_root":fixture.root}),
    );
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("symlink_or_file_forbidden"));
    assert_eq!(std::fs::read_dir(outside).unwrap().count(), 0);
}

#[test]
fn old_flat_legacy_row_imports_once_without_modifying_the_legacy_database() {
    let fixture = Fixture::new();
    let planned = fixture.call("final_review.plan", &fixture.plan_args());
    let reports = fixture
        .state
        .join("development-discipline/final-review-reports");
    let binding = std::fs::read_dir(&reports)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let flat = reports
        .join(binding.file_name().unwrap())
        .with_extension("sqlite");
    std::fs::copy(binding.join("report.sqlite"), &flat).unwrap();
    let mut insert = Command::new("python3");
    insert.args(["-c","import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute('INSERT INTO final_review_session(session_id,state_json,updated_at,revision) VALUES(?,?,?,?)',('replay-persistence',sys.stdin.read(),1,1)); c.commit(); c.close()"]).arg(&flat).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = run(insert, &planned["state"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::write(format!("{}.eventcore.lock", flat.display()), b"").unwrap();
    let legacy_bytes = std::fs::read(&flat).unwrap();
    git(
        &fixture.root,
        &[
            "update-ref",
            "-d",
            "refs/tiber/plugin-advisory-final-review",
        ],
    );
    std::fs::remove_dir_all(&binding).unwrap();
    let resumed = fixture.call(
        "final_review.resume_latest",
        &json!({"session_id":"replay-persistence","project_root":fixture.root}),
    );
    // Read-only recovery uses the compatibility row without appending import.
    assert_eq!(resumed["state_ref"]["session_id"], "replay-persistence");
    assert!(fixture.authority().is_none());
    assert_eq!(std::fs::read(&flat).unwrap(), legacy_bytes);
}

fn clean_results(state: &Value) -> Value {
    Value::Array(state["lenses"].as_array().unwrap().iter().map(|lens| {
        let lens=lens.as_str().unwrap();
        let strong=["architecture-maintainability","security-safety","safety-human-harm"].contains(&lens);
        json!({"lens":lens,"subagent_key":format!("{}:{}:{lens}",state["session_id"].as_str().unwrap(),state["iteration_index"].as_u64().unwrap()),"status":"clean","shared_test_evidence_id":state["shared_test_evidence"]["id"],"additional_broad_test_run":false,
            "caller_attestation":{"model_role":state["model_roles"][if strong {"verifier"} else {"lens_review"}],"fresh_context":true,"closed_after_result":true}})
    }).collect())
}

#[test]
fn delta_artifact_limits_hold_without_losing_prior_authority() {
    for variable in [
        "DEVELOPMENT_SYSTEM_DELTA_ARTIFACT_MAX_BYTES",
        "DEVELOPMENT_SYSTEM_DELTA_CACHE_MAX_BYTES",
    ] {
        let fixture = Fixture::new();
        std::fs::write(fixture.root.join("source.txt"), "old line\n".repeat(20_000)).unwrap();
        let planned = fixture.call("final_review.plan", &fixture.plan_args());
        let authority = fixture.authority();
        std::fs::write(fixture.root.join("source.txt"), "new line\n".repeat(20_000)).unwrap();
        let mut command = if let Some(binary) = std::env::var_os("DEVELOPMENT_SYSTEM_REPRO_BINARY")
        {
            let mut command = Command::new(binary);
            command.args([
                "--replay-review-operation",
                fixture.root.to_str().unwrap(),
                "final_review.advance",
            ]);
            fixture.configure(&mut command);
            command
        } else {
            fixture.command("final_review.advance")
        };
        command.env(variable, "1");
        let output = run(
            command,
            &json!({"state_ref":planned["state_ref"],"lens_results":[],"current_diff_hash":"budget-delta","current_changed_files":["source.txt"],"current_shared_test_evidence":{"id":"budget-tests","diff_hash":"budget-delta","status":"passed","summary":"isolated limit fixture","commands":["fixture:budget"]}}),
        );
        assert!(
            !output.status.success(),
            "a tiny explicit artifact budget must hold rather than persist unbounded bytes"
        );
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(text.contains("budget_exceeded"), "{text}");
        assert_eq!(
            fixture.authority(),
            authority,
            "budget failure changes no review credit or authority"
        );
    }
}

#[test]
fn explicit_ordinary_finding_verification_persists_independent_rejection() {
    let fixture = Fixture::new();
    let planned = fixture.call("final_review.plan", &fixture.plan_args());
    let mut lenses = clean_results(&planned["state"]);
    let lens = lenses[0]["lens"].as_str().unwrap().to_owned();
    lenses[0]["status"] = json!("findings");
    lenses[0]["findings"] = json!([{"id":"ordinary-disputed-claim","severity":"MINOR","causality":"caused","causality_evidence":"The reviewer alleges a changed source failure.","likelihood":"possible","security_impact":"none","safety_impact":"none","path":"source.txt","line":1,"message":"An ordinary allegation needs independent adjudication.","relevance":{"category":"diff_changed_file","explanation":"source.txt is reviewed."}}]);
    let pending = fixture.call("final_review.advance", &json!({
        "state_ref":planned["state_ref"],"lens_results":lenses,"current_diff_hash":"replay-fixture",
        "verification_requests":[{"finding_id":"ordinary-disputed-claim","lens":lens}],
        "unrelated_follow_ups":[{"finding_id":"ordinary-disputed-claim","lens":lens,"ticket_reference":"fixture-active-task"}]
    }));
    assert_eq!(pending["transition_status"], "verifier_required");
    let resumed = fixture.call(
        "final_review.resume_latest",
        &json!({"project_root":fixture.root,"session_id":"replay-persistence"}),
    );
    assert_eq!(resumed["state_ref"], pending["state_ref"]);
    let assignment = &pending["verifier_assignment"];
    let changed_targets = fixture.run("final_review.advance", &json!({
        "state_ref":resumed["state_ref"],"lens_results":[],"current_diff_hash":"replay-fixture",
        "verification_requests":[{"finding_id":"different-claim","lens":lens}],
        "verifier_result":{"subagent_key":assignment["subagent_key"],"assignment_id":assignment["assignment_id"],"model_role":assignment["model_role"],"status":"verified",
            "caller_attestation":{"model_role":assignment["model_role"],"fresh_context":true,"closed_after_result":true},
            "verdicts":[{"finding_id":"ordinary-disputed-claim","lens":lens,"verdict":"rejected","severity":"MINOR","causality":"caused","causality_evidence":"Independent verification inspected source.txt.","security_impact":"none","safety_impact":"none","rationale":"No alleged failing branch."}]}
    }));
    assert!(!changed_targets.status.success());
    assert!(String::from_utf8_lossy(&changed_targets.stdout)
        .contains("pending_verifier_verification_requests_mismatch"));
    let resolved = fixture.call("final_review.advance", &json!({
        "state_ref":resumed["state_ref"],"lens_results":[],"current_diff_hash":"replay-fixture",
        "verifier_result":{"subagent_key":assignment["subagent_key"],"assignment_id":assignment["assignment_id"],"model_role":assignment["model_role"],"status":"verified",
            "caller_attestation":{"model_role":assignment["model_role"],"fresh_context":true,"closed_after_result":true},
            "verdicts":[{"finding_id":"ordinary-disputed-claim","lens":lens,"verdict":"rejected","severity":"MINOR","causality":"caused","causality_evidence":"Independent fixture verification inspected source.txt.","security_impact":"none","safety_impact":"none","rationale":"The fixture source does not contain the alleged failing branch."}]}
    }));
    assert_eq!(resolved["state"]["clean_streak"], 0);
    assert_eq!(resolved["state"]["required_clean_iterations"], 3);
    let report = fixture.call(
        "final_review.yield_report",
        &json!({"state_ref":resolved["state_ref"]}),
    );
    assert_eq!(report["totals"]["raw_allegations"], 1);
    assert_eq!(report["totals"]["rejected"], 1);
    assert_eq!(report["rounds"][0]["clean"], false);
    let reference = report["rounds"][0]["evidence_refs"]["rejected_findings"][0]
        .as_str()
        .unwrap();
    let proof = fixture.call(
        "final_review.evidence",
        &json!({"state_ref":resolved["state_ref"],"evidence_ref":reference}),
    );
    assert_eq!(proof["finding"]["id"], "ordinary-disputed-claim");
    assert_eq!(
        proof["verifier_evidence"]["assignment_id"],
        assignment["assignment_id"]
    );
    let resolution = &resolved["state"]["finding_history"][0]["resolution_history"][0];
    assert_eq!(resolution["finding_id"], "ordinary-disputed-claim");
    assert_eq!(
        resolution["verifier"]["assignment_id"],
        assignment["assignment_id"]
    );
    assert_eq!(resolution["scope"]["diff_hash"], "replay-fixture");
}

#[test]
fn explicit_verification_rejects_unknown_duplicate_and_changed_scope_targets() {
    for case in ["unknown", "duplicate", "changed-scope"] {
        let fixture = Fixture::new();
        let planned = fixture.call("final_review.plan", &fixture.plan_args());
        let mut lenses = clean_results(&planned["state"]);
        let lens = lenses[0]["lens"].as_str().unwrap().to_owned();
        lenses[0]["status"] = json!("findings");
        lenses[0]["findings"] = json!([{"id":"ordinary-claim","severity":"MINOR","causality":"caused","causality_evidence":"The allegation concerns source.txt.","likelihood":"possible","security_impact":"none","safety_impact":"none","path":"source.txt","message":"A relevant ordinary claim.","relevance":{"category":"diff_changed_file","explanation":"source.txt is reviewed."}}]);
        let target = json!({"finding_id":if case == "unknown" {"invented"} else {"ordinary-claim"},"lens":lens});
        let targets = if case == "duplicate" {
            json!([target, target])
        } else {
            json!([target])
        };
        let before = fixture.authority();
        let result = fixture.run("final_review.advance", &json!({"state_ref":planned["state_ref"],"lens_results":lenses,"current_diff_hash":if case == "changed-scope" {"different"} else {"replay-fixture"},"verification_requests":targets}));
        assert!(!result.status.success());
        let output = String::from_utf8_lossy(&result.stdout);
        assert!(
            output.contains(match case {
                "unknown" => "verification_request_finding_not_eligible",
                "duplicate" => "verification_request_duplicate",
                _ => "verification_requests_require_unchanged_ordinary_review_scope",
            }),
            "{output}"
        );
        assert_eq!(
            fixture.authority(),
            before,
            "invalid targets append no review event"
        );
    }
}
#[test]
fn public_yield_counts_actual_rounds_and_delta_request_does_not_add_one() {
    exercise_public_yield_delta(false);
}

#[test]
fn public_yield_records_real_captured_source_changes() {
    exercise_public_yield_delta(true);
}

fn exercise_public_yield_delta(change_source: bool) {
    let fixture = Fixture::new();
    let planned = fixture.call("final_review.plan", &fixture.plan_args());
    let initial = fixture.call(
        "final_review.yield_report",
        &json!({"state_ref":planned["state_ref"]}),
    );
    assert_eq!(initial["completed_lens_rounds"], 0);
    let mut malformed = clean_results(&planned["state"]);
    malformed[0]["subagent_key"] = json!("wrong-assignment");
    malformed[0]["findings"] =
        json!([{"id":"raw-malformed","message":"Unparsed allegation survives."}]);
    let reset=fixture.call("final_review.advance",&json!({"state_ref":planned["state_ref"],"lens_results":malformed,"current_diff_hash":"replay-fixture"}));
    let reset_report = fixture.call(
        "final_review.yield_report",
        &json!({"state_ref":reset["state_ref"]}),
    );
    assert_eq!(reset_report["completed_lens_rounds"], 0);
    assert_eq!(reset_report["totals"]["raw_allegations"], 1);
    let evidence_ref = reset_report["rounds"][0]["evidence_refs"]["raw_findings"][0]
        .as_str()
        .unwrap();
    let detail = fixture.call(
        "final_review.evidence",
        &json!({"state_ref":reset["state_ref"],"evidence_ref":evidence_ref}),
    );
    assert_eq!(detail["finding"]["id"], "raw-malformed");
    assert_eq!(
        detail["finding"]["reviewer_subagent_key"],
        "wrong-assignment"
    );
    let advanced=fixture.call("final_review.advance",&json!({"state_ref":reset["state_ref"],"lens_results":clean_results(&reset["state"]),"current_diff_hash":"replay-fixture"}));
    let before = fixture.authority();
    let report = fixture.call(
        "final_review.yield_report",
        &json!({"state_ref":advanced["state_ref"]}),
    );
    assert_eq!(report["completed_lens_rounds"], 1);
    assert_eq!(report["totals"]["confirmed"], 0);
    assert_eq!(fixture.authority(), before, "report reads append no event");
    if change_source {
        std::fs::write(fixture.root.join("source.txt"), "actual changed source\n").unwrap();
    }
    let delta=fixture.call("final_review.advance",&json!({"state_ref":advanced["state_ref"],"lens_results":[],"current_diff_hash":"changed-scope","current_changed_files":["source.txt"],"current_shared_test_evidence":{"id":"changed-evidence","diff_hash":"changed-scope","status":"passed","summary":"changed evidence","commands":["fixture:delta"]}}));
    let delta_report = fixture.call(
        "final_review.yield_report",
        &json!({"state_ref":delta["state_ref"]}),
    );
    assert_eq!(delta_report["completed_lens_rounds"], 1);
    let assignment = &delta["delta_risk_assignments"][0];
    let dimensions = assignment["review_dimensions"].as_array().unwrap().iter().enumerate().map(|(index,lens)| json!({
        "lens":lens,"risk":if index==0 {"low"} else {"none"},
        "evidence":"Inspected the changed fixture source.",
        "plausible_failure":if index==0 {"The changed source can alter the output."} else {"none"},
        "material_impact":if index==0 {"The result could be incorrect."} else {"none"},
        "uncertain":false,"affected":index==0
    })).collect::<Vec<_>>();
    let resolved=fixture.call("final_review.advance",&json!({
        "state_ref":delta["state_ref"],"lens_results":[],"current_diff_hash":"changed-scope",
        "current_changed_files":["source.txt"],
        "current_shared_test_evidence":{"id":"changed-evidence","diff_hash":"changed-scope","status":"passed","summary":"changed evidence","commands":["fixture:delta"]},
        "delta_risk_assessment":{
            "assignment_id":assignment["assignment_id"],"subagent_key":assignment["subagent_key"],
            "shared_test_evidence_id":assignment["shared_test_evidence"]["id"],
            "prior_diff_hash":assignment["prior_diff_hash"],"current_diff_hash":assignment["current_diff_hash"],
            "overall_risk":"low","dimensions":dimensions,"exceptional_triggers":[],"split_required":false,"plan_assumptions":[],"findings":[],
            "caller_attestation":{"model_role":assignment["model_role"],"fresh_context":true,"closed_after_result":true}
        }
    }));
    assert_eq!(resolved["state"]["scope"]["diff_hash"], "changed-scope");
    let after_delta=fixture.call("final_review.advance",&json!({"state_ref":resolved["state_ref"],"lens_results":clean_results(&resolved["state"]),"current_diff_hash":"changed-scope"}));
    let report = fixture.call(
        "final_review.yield_report",
        &json!({"state_ref":after_delta["state_ref"]}),
    );
    assert_eq!(report["completed_lens_rounds"], 2);
    let round = report["rounds"].as_array().unwrap().last().unwrap();
    assert_eq!(
        round["source_changed"], change_source,
        "source change requires actual captured tree changes"
    );
    assert_eq!(round["prior_scope"]["diff_hash"], "replay-fixture");
    assert_eq!(round["scope"]["diff_hash"], "changed-scope");
    let detail = fixture.call(
        "final_review.evidence",
        &json!({"state_ref":after_delta["state_ref"],"evidence_ref":round["evidence_ref"]}),
    );
    assert_eq!(
        detail["round_evidence"]["prior_scope"]["diff_hash"],
        "replay-fixture"
    );
    let proof = &detail["round_evidence"]["source_change_evidence"];
    assert_eq!(
        proof["prior_snapshot_commit"],
        detail["round_evidence"]["prior_scope"]["snapshot_commit"]
    );
    assert_eq!(
        proof["current_snapshot_commit"],
        detail["round_evidence"]["scope"]["snapshot_commit"]
    );
    assert_eq!(proof["prior_tree"] != proof["current_tree"], change_source);
    assert_eq!(
        detail["round_evidence"]["scope"]["diff_hash"],
        "changed-scope"
    );
}

#[test]
fn public_yield_preserves_raw_round_across_verifier_restart_and_counts_actual_verdicts() {
    for verdict in ["confirmed", "rejected", "malformed"] {
        let fixture = Fixture::new();
        let planned = fixture.call("final_review.plan", &fixture.plan_args());
        let mut lenses = clean_results(&planned["state"]);
        let lens = lenses[0]["lens"].as_str().unwrap().to_owned();
        lenses[0]["status"] = json!("findings");
        lenses[0]["findings"] = json!([{"id":"real-candidate","severity":"CRITICAL","causality":"caused","causality_evidence":"The changed source is the trigger.","likelihood":"likely","security_impact":"major","safety_impact":"none","path":"source.txt","line":1,"message":"A concrete bounded fixture allegation.","relevance":{"category":"diff_changed_file","explanation":"source.txt is in the declared change."}}]);
        let pending=fixture.call("final_review.advance",&json!({"state_ref":planned["state_ref"],"lens_results":lenses,"current_diff_hash":"replay-fixture"}));
        assert_eq!(pending["transition_status"], "verifier_required");
        let pending_report = fixture.call(
            "final_review.yield_report",
            &json!({"state_ref":pending["state_ref"]}),
        );
        assert_eq!(pending_report["completed_lens_rounds"], 0);
        let assignment = &pending["verifier_assignment"];
        let verified=fixture.call("final_review.advance",&json!({"state_ref":pending["state_ref"],"lens_results":[],"current_diff_hash":"replay-fixture","unrelated_follow_ups":if verdict == "confirmed" {json!([{"finding_id":"real-candidate","lens":lens,"ticket_reference":"fixture-follow-up"}])} else {json!([])},"verifier_result":{"subagent_key":assignment["subagent_key"],"assignment_id":assignment["assignment_id"],"model_role":assignment["model_role"],"status":"verified","caller_attestation":{"model_role":assignment["model_role"],"fresh_context":verdict!="malformed","closed_after_result":true},"verdicts":[{"finding_id":"real-candidate","lens":lens,"verdict":if verdict=="malformed" {"rejected"} else {verdict},"severity":"CRITICAL","causality":"caused","causality_evidence":"Independent verifier inspected the scenario.","security_impact":"none","safety_impact":"none","rationale":"The isolated verifier fixture decided this outcome."}]}}));
        let report = fixture.call(
            "final_review.yield_report",
            &json!({"state_ref":verified["state_ref"]}),
        );
        assert_eq!(
            report["completed_lens_rounds"],
            if verdict == "malformed" { 0 } else { 1 }
        );
        assert_eq!(report["totals"]["raw_allegations"], 1);
        if verdict != "malformed" {
            assert_eq!(report["totals"][verdict], 1);
        } else {
            assert_eq!(report["totals"]["confirmed"], 0);
            assert_eq!(report["totals"]["rejected"], 0);
        }
        assert_eq!(report["rounds"][0]["clean"], false);
        assert_eq!(
            report["totals"]["repair_verified"], 0,
            "an initial confirmed finding is not a repair"
        );
        let evidence_ref = report["rounds"][0]["evidence_refs"]["raw_findings"][0]
            .as_str()
            .unwrap();
        let detail = fixture.call(
            "final_review.evidence",
            &json!({"state_ref":verified["state_ref"],"evidence_ref":evidence_ref}),
        );
        assert_eq!(detail["finding"]["id"], "real-candidate");
        if verdict != "malformed" {
            assert_eq!(
                detail["verifier_evidence"]["assignment_id"],
                assignment["assignment_id"]
            );
        }
    }
}

fn reassess_changed_scope(fixture: &Fixture, state_ref: &Value) -> Value {
    let changed_files = if fixture.root.join("README.md").exists() {
        json!(["source.txt", "README.md"])
    } else {
        json!(["source.txt"])
    };
    let delta=fixture.call("final_review.advance",&json!({"state_ref":state_ref,"lens_results":[],"current_diff_hash":"changed-scope","current_changed_files":changed_files,"current_shared_test_evidence":{"id":"changed-evidence","diff_hash":"changed-scope","status":"passed","summary":"changed evidence","commands":["fixture:delta"]}}));
    let assignment = &delta["delta_risk_assignments"][0];
    let dimensions = assignment["review_dimensions"].as_array().unwrap().iter().enumerate().map(|(index,lens)| json!({
        "lens":lens,"risk":if index==0 {"low"} else {"none"},
        "evidence":"Inspected the changed fixture source.",
        "plausible_failure":if index==0 {"The changed source can alter the output."} else {"none"},
        "material_impact":if index==0 {"The result could be incorrect."} else {"none"},
        "uncertain":false,"affected":index==0
    })).collect::<Vec<_>>();
    fixture.call("final_review.advance",&json!({
        "state_ref":delta["state_ref"],"lens_results":[],"current_diff_hash":"changed-scope",
        "current_changed_files":changed_files,
        "current_shared_test_evidence":{"id":"changed-evidence","diff_hash":"changed-scope","status":"passed","summary":"changed evidence","commands":["fixture:delta"]},
        "delta_risk_assessment":{
            "assignment_id":assignment["assignment_id"],"subagent_key":assignment["subagent_key"],
            "shared_test_evidence_id":assignment["shared_test_evidence"]["id"],
            "prior_diff_hash":assignment["prior_diff_hash"],"current_diff_hash":assignment["current_diff_hash"],
            "overall_risk":"low","dimensions":dimensions,"exceptional_triggers":[],"split_required":false,"plan_assumptions":[],"findings":[],
            "caller_attestation":{"model_role":assignment["model_role"],"fresh_context":true,"closed_after_result":true}
        }
    }))
}

#[test]
fn public_yield_distinguishes_verified_repair_from_rejected_initial_allegation() {
    for (current_verdict, source_changed, unrelated_changed) in [
        ("confirmed", true, false),
        ("rejected", true, false),
        ("rejected", false, false),
        ("rejected", false, true),
    ] {
        let fixture = Fixture::new();
        let planned = fixture.call("final_review.plan", &fixture.plan_args());
        let allegation = json!({"id":"repair-candidate","severity":"CRITICAL","causality":"caused","causality_evidence":"The changed source is the trigger.","likelihood":"likely","security_impact":"major","safety_impact":"none","path":"source.txt","line":1,"message":"The changed source permits the original failure.","relevance":{"category":"diff_changed_file","explanation":"source.txt is in the declared change."}});
        let mut lenses = clean_results(&planned["state"]);
        let lens = lenses[0]["lens"].clone();
        lenses[0]["status"] = json!("findings");
        lenses[0]["findings"] = json!([allegation]);
        let pending=fixture.call("final_review.advance",&json!({"state_ref":planned["state_ref"],"lens_results":lenses,"current_diff_hash":"replay-fixture"}));
        let submit_verdict = |pending: &Value, verdict: &str, diff: &str| {
            let assignment = &pending["verifier_assignment"];
            fixture.call("final_review.advance",&json!({"state_ref":pending["state_ref"],"lens_results":[],"current_diff_hash":diff,
                "verifier_result":{"subagent_key":assignment["subagent_key"],"assignment_id":assignment["assignment_id"],"model_role":assignment["model_role"],"status":"verified",
                    "caller_attestation":{"model_role":assignment["model_role"],"fresh_context":true,"closed_after_result":true},
                    "verdicts":[{"finding_id":"repair-candidate","lens":lens,"verdict":verdict,"severity":"CRITICAL","causality":"caused","causality_evidence":"Inspected source.txt and replayed the exact original failure input.","security_impact":"major","safety_impact":"none","rationale":if verdict=="rejected" {"The repaired guard rejects the original failure input before access."} else {"The original failure input still bypasses the guard."}}]}}))
        };
        let confirmed = submit_verdict(&pending, "confirmed", "replay-fixture");
        assert_eq!(
            confirmed["state"]["unresolved_findings"][0]["verification"]["verdict"],
            "confirmed"
        );
        if source_changed {
            std::fs::write(
                fixture.root.join("source.txt"),
                "repaired guard rejects the original input\n",
            )
            .unwrap();
        }
        if unrelated_changed {
            std::fs::write(
                fixture.root.join("README.md"),
                "An unrelated documentation change.\n",
            )
            .unwrap();
        }
        let changed = reassess_changed_scope(&fixture, &confirmed["state_ref"]);
        if unrelated_changed {
            assert_ne!(
                confirmed["state"]["scope"]["snapshot_commit"],
                changed["state"]["scope"]["snapshot_commit"]
            );
        }
        let mut lenses = clean_results(&changed["state"]);
        lenses[0]["status"] = json!("findings");
        lenses[0]["findings"] = json!([allegation]);
        let pending=fixture.call("final_review.advance",&json!({"state_ref":changed["state_ref"],"lens_results":lenses,"current_diff_hash":"changed-scope","caller_decisions":[{"finding_id":"repair-candidate","lens":lens,"decision":"fixed","remediation_path":"source.txt"}]}));
        assert_eq!(pending["transition_status"], "verifier_required");
        let completed = submit_verdict(&pending, current_verdict, "changed-scope");
        let report = fixture.call(
            "final_review.yield_report",
            &json!({"state_ref":completed["state_ref"]}),
        );
        let last = report["rounds"].as_array().unwrap().last().unwrap();
        if current_verdict == "rejected" && !source_changed {
            assert_eq!(
                last["counts"]["repair_verified"], 0,
                "caller hash or unrelated README changes must not prove a source.txt repair"
            );
            assert_eq!(last["counts"]["rejected"], 1);
        } else if current_verdict == "rejected" {
            assert!(
                completed["state"]["unresolved_findings"]
                    .as_array()
                    .is_none_or(Vec::is_empty),
                "independently verified repair must clear the original unresolved defect"
            );
            assert_eq!(last["counts"]["repair_verified"], 1);
            assert_eq!(
                last["counts"]["rejected"], 0,
                "a repaired real defect is not an initial false allegation"
            );
            let detail=fixture.call("final_review.evidence",&json!({"state_ref":completed["state_ref"],"evidence_ref":last["evidence_refs"]["repair_verified_findings"][0]}));
            assert_eq!(detail["finding"]["verdict"], "rejected");
        } else {
            assert_eq!(last["counts"]["repair_verified"], 0);
            assert_eq!(last["counts"]["confirmed"], 1);
        }
    }
}

#[test]
fn native_pruning_persists_retained_window_coverage_across_restart() {
    let fixture = Fixture::new();
    let planned = fixture.call("final_review.plan", &fixture.plan_args());
    let advanced=fixture.call("final_review.advance",&json!({"state_ref":planned["state_ref"],"lens_results":clean_results(&planned["state"]),"current_diff_hash":"replay-fixture"}));
    let mut state = advanced["state"].clone();
    let template = state["finding_history"][0].clone();
    state["finding_history"] = json!((100..164)
        .map(|iteration| {
            let mut row = template.clone();
            row["completed_iteration"] = json!(iteration);
            row
        })
        .collect::<Vec<_>>());
    let reports = fixture
        .state
        .join("development-discipline/final-review-reports");
    let binding = std::fs::read_dir(&reports)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let flat = reports
        .join(binding.file_name().unwrap())
        .with_extension("sqlite");
    std::fs::copy(binding.join("report.sqlite"), &flat).unwrap();
    let mut insert = Command::new("python3");
    insert.args(["-c","import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute('INSERT INTO final_review_session(session_id,state_json,updated_at,revision) VALUES(?,?,?,?)',('replay-persistence',sys.stdin.read(),1,1)); c.commit(); c.close()"]).arg(&flat).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let inserted = run(insert, &state);
    assert!(
        inserted.status.success(),
        "{}",
        String::from_utf8_lossy(&inserted.stderr)
    );
    std::fs::write(format!("{}.eventcore.lock", flat.display()), b"").unwrap();
    git(
        &fixture.root,
        &[
            "update-ref",
            "-d",
            "refs/tiber/plugin-advisory-final-review",
        ],
    );
    std::fs::remove_dir_all(&binding).unwrap();
    let mut current = fixture.call(
        "final_review.resume_latest",
        &json!({"session_id":"replay-persistence","project_root":fixture.root}),
    );
    current["state"] = state;
    for omitted in 1..=2 {
        current=fixture.call("final_review.advance",&json!({"state_ref":current["state_ref"],"lens_results":clean_results(&current["state"]),"current_diff_hash":"replay-fixture"}));
        assert_eq!(
            current["state"]["finding_history"]
                .as_array()
                .unwrap()
                .len(),
            64
        );
        assert_eq!(
            current["state"]["finding_history"][0]["omitted_prior_history_rows"],
            omitted
        );
        let report = fixture.call(
            "final_review.yield_report",
            &json!({"state_ref":current["state_ref"]}),
        );
        assert_eq!(report["counts_available_for_full_history"], false);
        assert_eq!(report["coverage"]["omitted_prior_history_rows"], omitted);
        assert!(report["completed_lens_rounds"].is_null());
        assert!(report["totals"].is_null());
        assert_eq!(report["observed_completed_lens_rounds"], 64);
        assert!(report["observed_totals"]["new_confirmed"].is_null());
        let resumed = fixture.call(
            "final_review.resume_latest",
            &json!({"session_id":"replay-persistence","project_root":fixture.root}),
        );
        assert_eq!(resumed["state_ref"], current["state_ref"]);
        let restarted_report = fixture.call(
            "final_review.yield_report",
            &json!({"state_ref":resumed["state_ref"]}),
        );
        assert_eq!(
            restarted_report["coverage"]["omitted_prior_history_rows"],
            omitted
        );
    }
}

#[test]
fn delta_artifact_native_subdirectory_preserves_project_scope_and_recovers() {
    let fixture = Fixture::new();
    let project = fixture.root.join(" package with spaces [literal]");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(
        project.join("source.txt"),
        "old project line\n".repeat(20_000),
    )
    .unwrap();
    let native = |name: &str, arguments: &Value| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_development-discipline-mcp"));
        fixture.configure(&mut command);
        command
            .current_dir(&project)
            .env("DEVELOPMENT_SYSTEM_SERVICE", "plugin-advisory");
        payload(run(
            command,
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":arguments}}),
        ))
    };
    let mut args = json!({"session_id":"native-subdirectory", "baseline_commit":fixture.baseline,
        "scope":"uncommitted", "project_root":project, "changed_files":["source.txt"],
        "diff_hash":"before", "shared_test_evidence":{"id":"before-tests", "diff_hash":"before", "status":"passed", "summary":"fixture tests", "commands":["fixture:before"]}});
    let assessed = native("final_review.assess_risk", &args);
    let risk = &assessed["assignments"][0];
    let dimensions = risk["review_dimensions"].as_array().unwrap().iter().enumerate().map(|(index,lens)| json!({"lens":lens,"risk":if index==0 {"low"} else {"none"},"evidence":"Bounded subdirectory fixture.","plausible_failure":if index==0 {"Evidence omits source"} else {"none"},"material_impact":if index==0 {"Review receives incomplete evidence"} else {"none"},"uncertain":false})).collect::<Vec<_>>();
    args["risk_assessment"] = json!({"assignment_id":risk["assignment_id"],"subagent_key":risk["subagent_key"],"shared_test_evidence_id":risk["shared_test_evidence"]["id"],"overall_risk":"low","dimensions":dimensions,"exceptional_triggers":[],"split_required":false,"plan_assumptions":[],"findings":[],"caller_attestation":{"model_role":risk["model_role"],"fresh_context":true,"closed_after_result":true}});
    let planned = native("final_review.plan", &args);
    std::fs::write(
        project.join("source.txt"),
        "new project line\n".repeat(20_000),
    )
    .unwrap();
    std::fs::write(
        fixture.root.join("source.txt"),
        "UNRELATED ROOT CONTENT MUST STAY OUT\n",
    )
    .unwrap();
    let pending = native(
        "final_review.advance",
        &json!({"state_ref":planned["state_ref"],"lens_results":[],"current_diff_hash":"subdirectory-delta","current_changed_files":["source.txt"],"current_shared_test_evidence":{"id":"subdirectory-tests","diff_hash":"subdirectory-delta","status":"passed","summary":"fixture tests","commands":["fixture:subdirectory"]}}),
    );
    let assignment = &pending["delta_risk_assignments"][0];
    assert_eq!(
        assignment["delta_evidence"]["changed_paths"],
        json!(["source.txt"]),
        "native subdirectory evidence must not silently omit the actual project delta"
    );
    let artifact = assignment["delta_evidence"]["artifact_reference"]
        .as_str()
        .unwrap();
    let bytes = std::fs::read(artifact).unwrap();
    let patch = String::from_utf8_lossy(&bytes);
    assert!(patch.contains(" package with spaces [literal]/source.txt"));
    assert!(patch.contains("-old project line"));
    assert!(patch.contains("+new project line"));
    assert!(!patch.contains("UNRELATED ROOT CONTENT"));
    std::fs::remove_file(artifact).unwrap();
    std::fs::write(project.join("source.txt"), "newer worktree content\n").unwrap();
    std::fs::write(project.join(".gitattributes"), "source.txt -diff\n").unwrap();
    std::fs::write(fixture.common().join("info/attributes"), "* -diff\n").unwrap();
    let recovered = native(
        "final_review.pending_assignments",
        &json!({"state_ref":pending["state_ref"],"subagent_key":assignment["subagent_key"]}),
    );
    assert_eq!(recovered["assignment"], *assignment);
    assert_eq!(std::fs::read(artifact).unwrap(), bytes);
}

#[test]
fn delta_artifact_recovers_gitlink_ignored_by_source_configuration() {
    let mut fixture = Fixture::new();
    let dependency = fixture.root.join("dependency");
    std::fs::create_dir(&dependency).unwrap();
    git(&dependency, &["init", "--quiet"]);
    git(&dependency, &["config", "user.name", "Replay Fixture"]);
    git(
        &dependency,
        &["config", "user.email", "replay@example.invalid"],
    );
    git(&dependency, &["config", "commit.gpgsign", "false"]);
    std::fs::write(dependency.join("lib.txt"), "old dependency\n").unwrap();
    git(&dependency, &["add", "lib.txt"]);
    git(&dependency, &["commit", "--quiet", "-m", "old dependency"]);
    let old_commit = git(&dependency, &["rev-parse", "HEAD"]);
    git(
        &fixture.root,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{old_commit},dependency"),
        ],
    );
    git(
        &fixture.root,
        &["commit", "--quiet", "-m", "include gitlink"],
    );
    fixture.baseline = git(&fixture.root, &["rev-parse", "HEAD"]);
    git(&fixture.root, &["config", "diff.ignoreSubmodules", "all"]);
    std::fs::write(fixture.root.join("source.txt"), "old line\n".repeat(20_000)).unwrap();
    let planned = fixture.call("final_review.plan", &fixture.plan_args());
    std::fs::write(dependency.join("lib.txt"), "new dependency\n").unwrap();
    git(&dependency, &["add", "lib.txt"]);
    git(&dependency, &["commit", "--quiet", "-m", "new dependency"]);
    let new_commit = git(&dependency, &["rev-parse", "HEAD"]);
    std::fs::write(fixture.root.join("source.txt"), "new line\n".repeat(20_000)).unwrap();
    let pending = fixture.call("final_review.advance", &json!({"state_ref":planned["state_ref"],"lens_results":[],"current_diff_hash":"gitlink-delta", "current_changed_files":["source.txt","dependency"],"current_shared_test_evidence":{"id":"gitlink-tests","diff_hash":"gitlink-delta","status":"passed","summary":"fixture tests","commands":["fixture:gitlink"]}}));
    let assignment = &pending["delta_risk_assignments"][0];
    let artifact = assignment["delta_evidence"]["artifact_reference"]
        .as_str()
        .unwrap();
    let bytes = std::fs::read(artifact).unwrap();
    let patch = String::from_utf8_lossy(&bytes);
    assert!(patch.contains(&format!("-Subproject commit {old_commit}")));
    assert!(patch.contains(&format!("+Subproject commit {new_commit}")));
    std::fs::remove_file(artifact).unwrap();
    let rebuilt = fixture.call(
        "final_review.pending_assignments",
        &json!({"state_ref":pending["state_ref"],"subagent_key":assignment["subagent_key"]}),
    );
    assert_eq!(rebuilt["assignment"], *assignment);
    assert_eq!(std::fs::read(artifact).unwrap(), bytes);
    assert_eq!(
        assignment["delta_evidence"]["changed_paths"],
        json!(["dependency", "source.txt"])
    );
}

#[test]
fn delta_artifact_intact_cache_survives_git_presentation_changes() {
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("source.txt"), "old line\n".repeat(20_000)).unwrap();
    let planned = fixture.call("final_review.plan", &fixture.plan_args());
    std::fs::write(fixture.root.join("source.txt"), "new line\n".repeat(20_000)).unwrap();
    let pending = fixture.call("final_review.advance", &json!({"state_ref":planned["state_ref"],"lens_results":[],"current_diff_hash":"render-delta", "current_changed_files":["source.txt"],"current_shared_test_evidence":{"id":"render-tests","diff_hash":"render-delta","status":"passed","summary":"fixture tests","commands":["fixture:render"]}}));
    let assignment = &pending["delta_risk_assignments"][0];
    let artifact = assignment["delta_evidence"]["artifact_reference"]
        .as_str()
        .unwrap();
    let bytes = std::fs::read(artifact).unwrap();
    git(&fixture.root, &["config", "diff.noprefix", "true"]);
    git(&fixture.root, &["config", "diff.context", "19"]);
    let retrieved = fixture.call(
        "final_review.pending_assignments",
        &json!({"state_ref":pending["state_ref"],"subagent_key":assignment["subagent_key"]}),
    );
    assert_eq!(retrieved["assignment"], *assignment);
    assert_eq!(std::fs::read(artifact).unwrap(), bytes);
    // Missing cache must rebuild from pinned inputs, not current worktree
    // attributes, local custom drivers, or changed presentation configuration.
    std::fs::remove_file(artifact).unwrap();
    std::fs::write(fixture.root.join(".gitattributes"), "source.txt -diff\n").unwrap();
    std::fs::write(
        fixture.common().join("info/attributes"),
        "source.txt -diff\n",
    )
    .unwrap();
    git(&fixture.root, &["config", "diff.algorithm", "histogram"]);
    git(&fixture.root, &["config", "diff.mnemonicPrefix", "true"]);
    git(&fixture.root, &["config", "diff.sourcePrefix", "before/"]);
    git(&fixture.root, &["config", "diff.dstPrefix", "after/"]);
    let rebuilt = fixture.call(
        "final_review.pending_assignments",
        &json!({"state_ref":pending["state_ref"],"subagent_key":assignment["subagent_key"]}),
    );
    assert_eq!(rebuilt["assignment"], *assignment);
    assert_eq!(std::fs::read(artifact).unwrap(), bytes);
    assert_eq!(
        assignment["delta_evidence"]["patch_rendering"],
        "git-snapshot-v2"
    );
}

#[test]
fn delta_artifact_survives_one_operation_replay_and_recovers_from_recorded_snapshots() {
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("source.txt"), "old line\n".repeat(20_000)).unwrap();
    let planned = fixture.call("final_review.plan", &fixture.plan_args());
    std::fs::write(fixture.root.join("source.txt"), "new line\n".repeat(20_000)).unwrap();
    let arguments = json!({"state_ref":planned["state_ref"],"lens_results":[],"current_diff_hash":"large-delta", "current_changed_files":["source.txt"],"current_shared_test_evidence":{"id":"large-delta-tests","diff_hash":"large-delta","status":"passed","summary":"fixture tests","commands":["fixture:delta"]}});
    let pending = fixture.call("final_review.advance", &arguments);
    let assignment = &pending["delta_risk_assignments"][0];
    let evidence = &assignment["delta_evidence"];
    let artifact = std::path::PathBuf::from(evidence["artifact_reference"].as_str().unwrap());
    assert!(
        artifact.is_file(),
        "successful replay returned deleted artifact: {}",
        artifact.display()
    );
    assert!(
        artifact.starts_with(&fixture.state),
        "artifact must be in durable per-project state"
    );
    let bytes = std::fs::read(&artifact).unwrap();
    let digest = git(&fixture.root, &["hash-object", artifact.to_str().unwrap()]);
    assert_eq!(digest, evidence["artifact_digest"].as_str().unwrap());
    let reference = pending["state_ref"].clone();
    let retrieve = json!({"state_ref":reference,"subagent_key":assignment["subagent_key"]});
    let restarted = fixture.call("final_review.pending_assignments", &retrieve);
    assert_eq!(restarted["assignment"], *assignment);
    let authority_before = fixture.authority();
    let mut unchanged_source = arguments.clone();
    unchanged_source["state_ref"] = reference.clone();
    unchanged_source["current_diff_hash"] = json!("caller-only-change");
    unchanged_source["current_shared_test_evidence"]["diff_hash"] = json!("caller-only-change");
    unchanged_source["current_shared_test_evidence"]["id"] = json!("caller-only-evidence");
    assert!(
        !fixture
            .run("final_review.advance", &unchanged_source)
            .status
            .success(),
        "caller hash changes cannot supersede unchanged captured source"
    );
    assert_eq!(fixture.authority(), authority_before);
    // A later working-tree edit cannot alter or replace the pending assignment.
    std::fs::write(
        fixture.root.join("source.txt"),
        "third unrelated working state\n",
    )
    .unwrap();
    let mut stale_resolution = arguments.clone();
    stale_resolution["state_ref"] = reference.clone();
    let dimensions=assignment["review_dimensions"].as_array().unwrap().iter().enumerate().map(|(index,lens)|json!({"lens":lens,"risk":if index==0 {"low"} else {"none"},"evidence":"Inspected the original pinned snapshots.","plausible_failure":if index==0 {"The source can alter the result."} else {"none"},"material_impact":if index==0 {"The result can be incorrect."} else {"none"},"uncertain":false,"affected":index==0})).collect::<Vec<_>>();
    stale_resolution["delta_risk_assessment"] = json!({"assignment_id":assignment["assignment_id"],"subagent_key":assignment["subagent_key"],"shared_test_evidence_id":assignment["shared_test_evidence"]["id"],"prior_diff_hash":assignment["prior_diff_hash"],"current_diff_hash":assignment["current_diff_hash"],"overall_risk":"low","dimensions":dimensions,"exceptional_triggers":[],"split_required":false,"plan_assumptions":[],"findings":[],"caller_attestation":{"model_role":assignment["model_role"],"fresh_context":true,"closed_after_result":true}});
    let stale = fixture.run("final_review.advance", &stale_resolution);
    assert!(
        !stale.status.success(),
        "old assessment must not authorize newly changed source"
    );

    std::fs::remove_file(&artifact).unwrap();
    let rebuilt = fixture.call("final_review.pending_assignments", &retrieve);
    assert_eq!(rebuilt["assignment"], *assignment);
    assert_eq!(std::fs::read(&artifact).unwrap(), bytes);
    let resumed = fixture.call(
        "final_review.resume_latest",
        &json!({"project_root":fixture.root,"session_id":"replay-persistence"}),
    );
    assert_eq!(resumed["state_ref"], reference);
    assert_eq!(std::fs::read(&artifact).unwrap(), bytes);
    // Concurrent recovery converges without replacing another complete artifact.
    std::fs::remove_file(&artifact).unwrap();
    std::thread::scope(|scope| {
        let calls = (0..4)
            .map(|_| scope.spawn(|| fixture.call("final_review.pending_assignments", &retrieve)))
            .collect::<Vec<_>>();
        for call in calls {
            assert_eq!(call.join().unwrap()["assignment"], *assignment);
        }
    });
    assert_eq!(std::fs::read(&artifact).unwrap(), bytes);
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&artifact, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::write(&artifact, b"tampered cached evidence").unwrap();
    let tampered = fixture.run("final_review.pending_assignments", &retrieve);
    assert!(!tampered.status.success());
    assert!(
        format!(
            "{}{}",
            String::from_utf8_lossy(&tampered.stderr),
            String::from_utf8_lossy(&tampered.stdout)
        )
        .contains("existing_digest_mismatch"),
        "{}{}",
        String::from_utf8_lossy(&tampered.stderr),
        String::from_utf8_lossy(&tampered.stdout)
    );
    assert_eq!(
        std::fs::read(&artifact).unwrap(),
        b"tampered cached evidence"
    );
    std::fs::remove_file(&artifact).unwrap();
    let unrelated = fixture.directory.path().join("unrelated.patch");
    std::fs::write(&unrelated, &bytes).unwrap();
    std::os::unix::fs::symlink(&unrelated, &artifact).unwrap();
    assert!(!fixture
        .run("final_review.pending_assignments", &retrieve)
        .status
        .success());
    assert_eq!(std::fs::read(&unrelated).unwrap(), bytes);
    std::fs::remove_file(&artifact).unwrap();
    assert!(Command::new("mkfifo")
        .arg(&artifact)
        .status()
        .unwrap()
        .success());
    let mut bounded = Command::new("timeout");
    bounded.args([
        "2s",
        env!("CARGO_BIN_EXE_development-discipline-mcp"),
        "--replay-review-operation",
        fixture.root.to_str().unwrap(),
        "final_review.pending_assignments",
    ]);
    fixture.configure(&mut bounded);
    let fifo = run(bounded, &retrieve);
    assert_ne!(
        fifo.status.code(),
        Some(124),
        "FIFO cache tamper must fail immediately rather than block reading"
    );
    assert!(!fifo.status.success());
    std::fs::remove_file(&artifact).unwrap();
    std::fs::create_dir(&artifact).unwrap();
    assert!(!fixture
        .run("final_review.pending_assignments", &retrieve)
        .status
        .success());
    std::fs::remove_dir(&artifact).unwrap();
    std::os::unix::fs::symlink(fixture.directory.path().join("missing-evidence"), &artifact)
        .unwrap();
    assert!(!fixture
        .run("final_review.pending_assignments", &retrieve)
        .status
        .success());
    std::fs::remove_file(&artifact).unwrap();
    std::fs::hard_link(&unrelated, &artifact).unwrap();
    assert!(!fixture
        .run("final_review.pending_assignments", &retrieve)
        .status
        .success());
    std::fs::remove_file(&artifact).unwrap();
    let repaired = fixture.call("final_review.pending_assignments", &retrieve);
    assert_eq!(repaired["assignment"], *assignment);
    // A genuinely new scope requests a fresh bound scout, without claiming the
    // pending old scope matches current source or inventing an assessment.
    let mut superseding = arguments.clone();
    superseding["state_ref"] = reference;
    superseding["current_diff_hash"] = json!("third-scope");
    superseding["current_shared_test_evidence"]["diff_hash"] = json!("third-scope");
    superseding["current_shared_test_evidence"]["id"] = json!("third-tests");
    let replacement = fixture.call("final_review.advance", &superseding);
    assert_eq!(
        replacement["transition_status"],
        "delta_risk_assessment_required"
    );
    assert_ne!(
        replacement["delta_risk_assignments"][0]["assignment_id"],
        assignment["assignment_id"]
    );
    assert_eq!(
        replacement["state"]["clean_streak"],
        planned["state"]["clean_streak"]
    );
    assert_eq!(
        std::fs::read(&artifact).unwrap(),
        bytes,
        "old immutable evidence survives supersession"
    );
    assert!(replacement["subagent_shutdown"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["subagent_key"] == assignment["subagent_key"]));
    let latest=fixture.call("final_review.pending_assignments",&json!({"state_ref":replacement["state_ref"],"subagent_key":replacement["delta_risk_assignments"][0]["subagent_key"]}));
    assert_eq!(
        latest["assignment"],
        replacement["delta_risk_assignments"][0]
    );
    let authority_after = fixture.authority();
    stale_resolution["state_ref"] = replacement["state_ref"].clone();
    assert!(
        !fixture
            .run("final_review.advance", &stale_resolution)
            .status
            .success(),
        "superseded scout results remain invalid"
    );
    assert_eq!(fixture.authority(), authority_after);
}

#[test]
fn delta_artifact_interrupted_generation_retries_without_publishing_partial_bytes() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("source.txt"), "old line\n".repeat(20_000)).unwrap();
    let planned = fixture.call("final_review.plan", &fixture.plan_args());
    std::fs::write(fixture.root.join("source.txt"), "new line\n".repeat(20_000)).unwrap();
    let arguments = json!({"state_ref":planned["state_ref"],"lens_results":[],"current_diff_hash":"large-delta", "current_changed_files":["source.txt"],"current_shared_test_evidence":{"id":"large-delta-tests","diff_hash":"large-delta","status":"passed","summary":"fixture tests","commands":["fixture:delta"]}});
    let bin = fixture.directory.path().join("paused-bin");
    std::fs::create_dir(&bin).unwrap();
    let git_location = Command::new("sh")
        .args(["-c", "command -v git"])
        .output()
        .unwrap();
    let real_git = String::from_utf8(git_location.stdout)
        .unwrap()
        .trim()
        .to_string();
    let wrapper = bin.join("git");
    std::fs::write(&wrapper,format!("#!/usr/bin/env python3\nimport os,sys,time\nif '--binary' in sys.argv:\n sys.stdout.write('partial interrupted patch');sys.stdout.flush();time.sleep(60)\nos.execv({real_git:?},[{real_git:?}]+sys.argv[1:])\n")).unwrap();
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let cache_parent = fixture
        .state
        .join("development-discipline/final-review-delta-evidence");
    let cache = std::fs::read_dir(&cache_parent)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let candidate = cache.join(".candidate.patch");
    let mut command = fixture.command("final_review.advance");
    command.env(
        "PATH",
        format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
    );
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(arguments.to_string().as_bytes())
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let mut partial_bytes_observed = false;
    while std::time::Instant::now() < deadline {
        if std::fs::metadata(&candidate).is_ok_and(|metadata| metadata.len() > 0) {
            partial_bytes_observed = true;
            break;
        }
        if child.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    // Reap even when readiness fails: an assertion must not leak the sleeping
    // renderer or its namespace. The test still requires actual partial bytes.
    let interrupted = child.kill();
    let output = child.wait_with_output().unwrap();
    assert!(
        partial_bytes_observed,
        "partial patch was not observed before interruption: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    interrupted.expect("interrupt the live partial-patch renderer");
    let resumed = fixture.call("final_review.advance", &arguments);
    let artifact = resumed["delta_risk_assignments"][0]["delta_evidence"]["artifact_reference"]
        .as_str()
        .unwrap();
    let bytes = std::fs::read(artifact).unwrap();
    assert!(!bytes.starts_with(b"partial interrupted"));
    assert_eq!(
        git(&fixture.root, &["hash-object", artifact]),
        resumed["delta_risk_assignments"][0]["delta_evidence"]["artifact_digest"]
    );
    assert!(!candidate.exists());
}
