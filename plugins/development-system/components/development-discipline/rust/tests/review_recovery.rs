use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
use tempfile::TempDir;

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn call(root: &Path, state: &Path, name: &str, arguments: Value) -> Value {
    // Every request starts a fresh native process, exercising Git-backed replay.
    let mut process = Command::new(env!("CARGO_BIN_EXE_development-discipline-mcp"))
        .current_dir(root)
        .env("XDG_STATE_HOME", state)
        .env("DEVELOPMENT_SYSTEM_SERVICE", "plugin-advisory")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(
        process.stdin.take().unwrap(),
        "{}",
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":name,"arguments":arguments}})
    )
    .unwrap();
    let output = process.wait_with_output().unwrap();
    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(response.get("error").is_none(), "{response}");
    serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

fn evidence(hash: &str) -> Value {
    json!({"id":format!("tests-{hash}"),"diff_hash":hash,"status":"passed",
        "summary":"Fixture verification passed.","commands":["fixture:verification"]})
}

#[test]
fn native_completed_review_reopens_and_replays_after_projection_loss() {
    let root = TempDir::new().unwrap();
    let state = TempDir::new().unwrap();
    git(root.path(), &["init", "--quiet"]);
    fs::create_dir(root.path().join("src")).unwrap();
    fs::write(root.path().join("src/review.rs"), "const VALUE:u8=1;\n").unwrap();
    fs::write(
        root.path().join(".development-system.toml"),
        r#"schema_version = 3
[features]
tiber = true
[final_review.models.codex]
pre_filter = "gpt-6-astra"
lens_review = "gpt-6-sol"
post_filter = "gpt-6-luna"
verifier = "gpt-6-astra"
[scopes.source]
category = "source"
include = ["src/**"]
"#,
    )
    .unwrap();
    git(root.path(), &["add", "."]);
    git(
        root.path(),
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ],
    );
    let baseline = git(root.path(), &["rev-parse", "HEAD"]);
    fs::write(root.path().join("src/review.rs"), "const VALUE:u8=2;\n").unwrap();
    let mut args = json!({"session_id":"native-reopen","base":"HEAD","baseline_commit":baseline,
        "scope":"uncommitted","project_root":root.path(),"changed_files":["src/review.rs"],
        "diff_hash":"before","shared_test_evidence":evidence("before"),"pre_filter_model_role":"gpt-6-astra"});
    let risk = call(
        root.path(),
        state.path(),
        "final_review.assess_risk",
        args.clone(),
    );
    let assignment = &risk["assignments"][0];
    let dimensions=assignment["review_dimensions"].as_array().unwrap().iter().map(|lens| {
        let selected=lens=="correctness-behavior";
        json!({"lens":lens,"risk":if selected {"medium"} else {"none"},
            "evidence":if selected {"Changed recovery logic"} else {"No concrete risk"},
            "plausible_failure":if selected {"Recovery loses review state"} else {"none"},
            "material_impact":if selected {"Required review cannot finish"} else {"none"},"uncertain":false})
    }).collect::<Vec<_>>();
    args["risk_assessment"] = json!({"assignment_id":assignment["assignment_id"],"subagent_key":assignment["subagent_key"],
        "shared_test_evidence_id":assignment["shared_test_evidence"]["id"],"overall_risk":"medium","dimensions":dimensions,
        "exceptional_triggers":[],"split_required":false,"plan_assumptions":[],"findings":[],
        "caller_attestation":{"model_role":assignment["model_role"],"fresh_context":true,"closed_after_result":true}});
    args["unrelated_finding_policy"] = json!({"default":"report"});
    let mut result = call(root.path(), state.path(), "final_review.plan", args);
    for _ in 0..3 {
        let assignments = result
            .get("assignments")
            .or_else(|| result.get("next_assignments"))
            .unwrap()
            .as_array()
            .unwrap();
        let results=assignments.iter().map(|assignment|json!({
            "lens":assignment["lens"],"subagent_key":assignment["subagent_key"],"status":"clean","findings":[],
            "shared_test_evidence_id":assignment["shared_test_evidence"]["id"],"additional_broad_test_run":false,
            "caller_attestation":{"model_role":assignment["model_role"],"fresh_context":true,"closed_after_result":true}
        })).collect::<Vec<_>>();
        result = call(
            root.path(),
            state.path(),
            "final_review.advance",
            json!({"state_ref":result["state_ref"],"current_diff_hash":"before","lens_results":results}),
        );
    }
    assert_eq!(result["complete"], true);
    fs::write(root.path().join("src/review.rs"), "const VALUE:u8=3;\n").unwrap();
    let args = json!({"state_ref":result["state_ref"],"operation_id":"native-reopen-1","reason":"Changed source after completion",
        "current_diff_hash":"after","current_changed_files":["src/review.rs"],"current_shared_test_evidence":evidence("after")});
    let reopened = call(
        root.path(),
        state.path(),
        "final_review.reopen",
        args.clone(),
    );
    assert_eq!(reopened["state"]["scope"]["baseline_commit"], baseline);
    assert_eq!(reopened["state"]["clean_streak"], 0);
    assert_eq!(reopened["state"]["verified_clean_iterations"], json!([]));
    assert_eq!(
        reopened["transition_status"],
        "delta_risk_assessment_required"
    );
    let mut directories = vec![state.path().to_path_buf()];
    let mut removed = 0;
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                directories.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "sqlite")
            {
                fs::remove_file(&path).unwrap();
                removed += 1;
                for suffix in ["-wal", "-shm"] {
                    let sidecar = std::path::PathBuf::from(format!("{}{suffix}", path.display()));
                    if sidecar.exists() {
                        fs::remove_file(sidecar).unwrap();
                    }
                }
            }
        }
    }
    assert_eq!(removed, 1);
    let replay = call(root.path(), state.path(), "final_review.reopen", args);
    assert_eq!(replay["state_ref"], reopened["state_ref"]);
    assert_eq!(
        replay["delta_risk_assignments"],
        reopened["delta_risk_assignments"]
    );
    assert_eq!(replay["response_historical"], false);
    let pending = call(
        root.path(),
        state.path(),
        "final_review.pending_assignments",
        json!({"state_ref":replay["state_ref"]}),
    );
    assert_eq!(pending["pending_phase"], "delta-risk");
}
