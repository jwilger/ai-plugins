use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use tempfile::TempDir;

#[test]
fn workflow_guard_blocks_unrelated_work_for_a_shared_ci_recovery_hold() {
    let repository = TempDir::new().expect("temporary repository");
    assert!(Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(repository.path())
        .status()
        .expect("initialize repository")
        .success());
    let hold_directory = repository.path().join(".git/development-discipline");
    fs::create_dir_all(&hold_directory).expect("create hold directory");
    fs::write(
        hold_directory.join("workflow-blocker.json"),
        r#"{"schema_version":1,"kind":"ci_claim_failed","error_code":"development_workflow.ci_recovery_claim_failed","required_action":"retry the shared CI-recovery claim","created_at":1}"#,
    )
    .expect("write hold");

    let output = guard(
        repository.path(),
        r#"{"cwd":".","tool_name":"Bash","tool_input":{"command":"cargo test"}}"#,
    );

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("development_workflow.blocked"));
    assert!(stderr.contains("development_workflow.ci_recovery_claim_failed"));
}

#[test]
fn workflow_guard_requires_red_before_production_edits_but_allows_test_edits() {
    let repository = TempDir::new().expect("temporary repository");
    assert!(Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(repository.path())
        .status()
        .expect("initialize repository")
        .success());
    let state_directory = repository.path().join(".git/development-discipline");
    fs::create_dir_all(&state_directory).expect("create state directory");
    fs::write(
        state_directory.join("workflow-state.json"),
        r#"{"phase":"AwaitingRed","change_kind":"Production","red_observed":false,"green_observed":false,"clean_review_observed":false}"#,
    )
    .expect("write lifecycle state");

    let production_edit = guard(
        repository.path(),
        r#"{"cwd":".","tool_name":"Write","tool_input":{"path":"src/lib.rs","content":"implementation"}}"#,
    );
    assert_eq!(production_edit.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&production_edit.stderr)
        .contains("development_workflow.red_evidence_required"));

    let test_edit = guard(
        repository.path(),
        r#"{"cwd":".","tool_name":"Write","tool_input":{"path":"tests/behavior.rs","content":"failing test"}}"#,
    );
    assert!(test_edit.status.success());

    let mixed_patch = guard(
        repository.path(),
        r#"{"cwd":".","tool_name":"ApplyPatch","tool_input":{"patch":"*** Update File: tests/behavior.rs\n*** Update File: src/lib.rs\n"}}"#,
    );
    assert_eq!(mixed_patch.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&mixed_patch.stderr)
        .contains("development_workflow.red_evidence_required"));

    let focused_patch = guard(
        repository.path(),
        r#"{"cwd":".","tool_name":"ApplyPatch","tool_input":{"patch":"*** Update File: tests/behavior.rs\n"}}"#,
    );
    assert!(focused_patch.status.success());
}

#[test]
fn workflow_guard_requires_a_lifecycle_before_any_repository_mutation() {
    let repository = TempDir::new().expect("temporary repository");
    assert!(Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(repository.path())
        .status()
        .expect("initialize repository")
        .success());

    let production_edit = guard(
        repository.path(),
        r#"{"cwd":".","tool_name":"Write","tool_input":{"path":"src/lib.rs","content":"implementation"}}"#,
    );
    assert_eq!(production_edit.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&production_edit.stderr)
        .contains("development_workflow.lifecycle_required"));

    let test_edit = guard(
        repository.path(),
        r#"{"cwd":".","tool_name":"Write","tool_input":{"path":"tests/behavior.rs","content":"failing test"}}"#,
    );
    assert_eq!(test_edit.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&test_edit.stderr)
        .contains("development_workflow.lifecycle_required"));
}

#[test]
fn workflow_guard_allows_harness_named_lifecycle_start_and_read_only_git_before_start() {
    let repository = TempDir::new().expect("temporary repository");
    assert!(Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(repository.path())
        .status()
        .expect("initialize repository")
        .success());

    let lifecycle_start = guard(
        repository.path(),
        r#"{"cwd":".","tool_name":"mcp__development_discipline__workflow_start","tool_input":{"change_kind":"production"}}"#,
    );
    assert!(lifecycle_start.status.success());

    let git_status = guard(
        repository.path(),
        r#"{"cwd":".","tool_name":"Bash","tool_input":{"command":"git status --short --branch"}}"#,
    );
    assert!(git_status.status.success());
}

#[test]
fn workflow_guard_allows_non_mutating_research_at_every_lifecycle_phase() {
    let repository = TempDir::new().expect("temporary repository");
    git(repository.path(), &["init", "--quiet"]);
    let state_directory = repository.path().join(".git/development-discipline");
    fs::create_dir_all(&state_directory).expect("create state directory");

    for phase in [
        "AwaitingRed",
        "AwaitingImplementation",
        "AwaitingReview",
        "AwaitingDelivery",
    ] {
        fs::write(
            state_directory.join("workflow-state.json"),
            format!(
                r#"{{"phase":"{phase}","change_kind":"Production","red_observed":false,"green_observed":false,"clean_review_observed":false}}"#
            ),
        )
        .expect("write lifecycle state");

        let output = guard(
            repository.path(),
            r#"{"cwd":".","tool_name":"Bash","tool_input":{"command":"sed -n '1,20p' README.md; git status --short --branch; gh run list --limit 1"}}"#,
        );
        assert!(output.status.success(), "phase={phase} output={output:?}");
    }

    let lookup = guard(
        repository.path(),
        r#"{"cwd":".","tool_name":"webrun","tool_input":{"query":"EventCore model checking"}}"#,
    );
    assert!(lookup.status.success(), "{lookup:?}");
}

#[test]
fn workflow_guard_accepts_raw_test_patch_payload_before_red_but_blocks_raw_production_patch() {
    let repository = TempDir::new().expect("temporary repository");
    git(repository.path(), &["init", "--quiet"]);
    let state_directory = repository.path().join(".git/development-discipline");
    fs::create_dir_all(&state_directory).expect("create state directory");
    fs::write(
        state_directory.join("workflow-state.json"),
        r#"{"phase":"AwaitingRed","change_kind":"Production","red_observed":false,"green_observed":false,"clean_review_observed":false}"#,
    )
    .expect("write lifecycle state");

    let test_patch = guard(
        repository.path(),
        r#"{"cwd":".","tool_name":"apply_patch","tool_input":"*** Begin Patch\n*** Update File: tests/workflow_guard.rs\n*** End Patch"}"#,
    );
    assert!(test_patch.status.success(), "{test_patch:?}");

    let production_patch = guard(
        repository.path(),
        r#"{"cwd":".","tool_name":"apply_patch","tool_input":"*** Begin Patch\n*** Update File: src/workflow.rs\n*** End Patch"}"#,
    );
    assert_eq!(production_patch.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&production_patch.stderr)
        .contains("development_workflow.red_evidence_required"));
}

#[test]
fn workflow_guard_blocks_recognized_terminal_mutations_without_a_lifecycle() {
    let repository = TempDir::new().expect("temporary repository");
    git(repository.path(), &["init", "--quiet"]);

    for command in ["sed -i 's/old/new/' src/workflow.rs", "rm src/workflow.rs"] {
        let output = guard(
            repository.path(),
            &format!(r#"{{"cwd":".","tool_name":"Bash","tool_input":{{"command":"{command}"}}}}"#),
        );
        assert_eq!(output.status.code(), Some(2), "command={command}");
        assert!(String::from_utf8_lossy(&output.stderr)
            .contains("development_workflow.lifecycle_required"));
    }
}

#[test]
fn workflow_guard_blocks_post_review_mutations_until_delivery_is_authorized() {
    let repository = TempDir::new().expect("temporary repository");
    assert!(Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(repository.path())
        .status()
        .expect("initialize repository")
        .success());
    let state_directory = repository.path().join(".git/development-discipline");
    fs::create_dir_all(&state_directory).expect("create state directory");
    fs::write(
        state_directory.join("workflow-state.json"),
        r#"{"phase":"AwaitingDelivery","change_kind":"Production","red_observed":true,"green_observed":true,"clean_review_observed":true}"#,
    )
    .expect("write lifecycle state");

    let mutation = guard(
        repository.path(),
        r#"{"cwd":".","tool_name":"Write","tool_input":{"path":"src/lib.rs","content":"late change"}}"#,
    );
    assert_eq!(mutation.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&mutation.stderr)
        .contains("development_workflow.delivery_authorization_required"));
}

#[test]
fn workflow_guard_observes_an_active_hold_published_by_another_clone() {
    let remote = TempDir::new().expect("bare remote");
    git(remote.path(), &["init", "--bare", "--quiet"]);
    let first = TempDir::new().expect("first clone");
    git(first.path(), &["init", "--quiet"]);
    git(first.path(), &["config", "user.name", "Workflow test"]);
    git(
        first.path(),
        &["config", "user.email", "workflow@example.test"],
    );
    git(
        first.path(),
        &[
            "remote",
            "add",
            "origin",
            remote.path().to_str().expect("remote path"),
        ],
    );
    fs::write(
        first.path().join("ci-recovery.json"),
        r#"{"schema_version":1,"incident_id":"ci-remote","state":"diagnosing","epoch":1,"trigger":{"run_id":"1","run_url":"https://example.test/1","failed_sha":"abc","workflow":"CI","git_ref":"refs/heads/main"},"owner":{"host":"first","session":"one"},"lease_expires_at":9999999999}"#,
    )
    .expect("write remote state");
    git(first.path(), &["add", "ci-recovery.json"]);
    git(
        first.path(),
        &["commit", "--quiet", "-m", "active workflow recovery"],
    );
    git(first.path(), &["branch", "-M", "development-workflow"]);
    git(
        first.path(),
        &["push", "--quiet", "origin", "development-workflow"],
    );

    let second = TempDir::new().expect("second clone");
    git(
        second.path(),
        &[
            "clone",
            "--quiet",
            remote.path().to_str().expect("remote path"),
            ".",
        ],
    );
    let output = guard(
        second.path(),
        r#"{"cwd":".","tool_name":"Bash","tool_input":{"command":"cargo test"}}"#,
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("development_workflow.ci_recovery_active")
    );
}

fn git(root: &std::path::Path, arguments: &[&str]) {
    assert!(Command::new("git")
        .args(arguments)
        .current_dir(root)
        .status()
        .expect("git should run")
        .success());
}

fn guard(repository: &std::path::Path, input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_development-discipline-mcp"))
        .arg("workflow-guard")
        .current_dir(repository)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start workflow guard");
    child
        .stdin
        .take()
        .expect("guard standard input")
        .write_all(input.as_bytes())
        .expect("write hook input");
    child.wait_with_output().expect("finish workflow guard")
}
