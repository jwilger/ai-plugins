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
fn public_yield_counts_actual_rounds_and_delta_request_does_not_add_one() {
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
        round["source_changed"], true,
        "compare actual lens rounds across the delta reassessment"
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
        "git-snapshot-v1"
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
