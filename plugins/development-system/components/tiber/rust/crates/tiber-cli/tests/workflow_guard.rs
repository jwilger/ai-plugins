mod support;

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use support::{assert_success, TempRepo};

#[test]
fn workflow_guard_denies_work_and_allows_only_structured_recovery() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    let blocker_dir = repo.path().join(".git/tiber");
    fs::create_dir_all(&blocker_dir).expect("create blocker directory");
    fs::write(
        blocker_dir.join("workflow-blocker.json"),
        r#"{"schema_version":1,"kind":"ci_claim_failed","error_code":"tiber.ci_recovery_claim_failed","required_action":"retry the shared Tiber CI-recovery claim","created_at":1}"#,
    )
    .expect("write blocker");

    let denied = guard(
        &repo,
        r#"{"cwd":".","tool_name":"Bash","tool_input":{"command":"gh run view 123 --log-failed"}}"#,
    );
    assert_eq!(denied.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&denied.stderr);
    assert!(stderr.contains("workflow_blocked=true"));
    assert!(stderr.contains("tiber.ci_recovery_claim_failed"));
    assert!(stderr.contains("Do not diagnose, edit, test, rerun, push, or perform unrelated work"));

    for tool_name in [
        "mcp__tiber__tiber.ci_recovery.claim",
        "mcp__tiber__tiber.ci_recovery.status",
        "mcp__tiber__tiber.sync",
    ] {
        let allowed = guard(
            &repo,
            &format!(r#"{{"cwd":".","tool_name":"{tool_name}","tool_input":{{}}}}"#),
        );
        assert_success(allowed);
    }

    let other_tiber = guard(
        &repo,
        r#"{"cwd":".","tool_name":"mcp__tiber__tiber.create","tool_input":{"title":"keep going"}}"#,
    );
    assert_eq!(other_tiber.status.code(), Some(2));

    for command in [
        "tiber sync",
        "/opt/development-system/bin/tiber ci-recovery status",
        "tiber ci-recovery claim --run-id 123 --run-url https://example.invalid/runs/123 --failed-sha abcdef --workflow CI --ref refs/heads/main",
        "tiber ci-recovery claim --run-id=123 --run-url=https://example.invalid/runs/123 --failed-sha=abcdef --workflow=CI --ref=refs/heads/main",
    ] {
        let allowed = guard(
            &repo,
            &format!(
                r#"{{"cwd":".","tool_name":"Bash","tool_input":{{"command":{}}}}}"#,
                serde_json::to_string(command).expect("command JSON")
            ),
        );
        assert_success(allowed);
    }

    for command in [
        "tiber sync && gh run view 123 --log-failed",
        "tiber sync > /tmp/result",
        "sh -c 'tiber sync'",
        "env tiber sync",
        "tiber ci-recovery status; cargo test",
        "tiber ci-recovery claim --run-id 123 --run-url x --failed-sha abc --workflow CI --ref refs/heads/main | tee claim",
        "echo tiber sync",
        "tiber ci-recovery claim --run-id $(steal-session) --run-url x --failed-sha abc --workflow CI --ref refs/heads/main",
        "tiber ci-recovery claim --run-id `steal-session` --run-url x --failed-sha abc --workflow CI --ref refs/heads/main",
    ] {
        let denied = guard(
            &repo,
            &format!(
                r#"{{"cwd":".","tool_name":"Bash","tool_input":{{"command":{}}}}}"#,
                serde_json::to_string(command).expect("command JSON")
            ),
        );
        assert_eq!(denied.status.code(), Some(2), "must deny: {command}");
    }
}

#[test]
fn workflow_guard_allows_tools_without_a_blocker_and_fails_closed_on_corruption() {
    let repo = TempRepo::initialized();
    let allowed = guard(
        &repo,
        r#"{"cwd":".","tool_name":"Bash","tool_input":{"command":"cargo test"}}"#,
    );
    assert_success(allowed);

    fs::create_dir_all(repo.path().join(".git/tiber")).expect("create blocker directory");
    fs::write(
        repo.path().join(".git/tiber/workflow-blocker.json"),
        "not json",
    )
    .expect("write corrupt blocker");
    let denied = guard(&repo, r#"{"cwd":".","tool_name":"Edit","tool_input":{}}"#);
    assert_eq!(denied.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&denied.stderr).contains("workflow_blocker_invalid"));
}

#[test]
fn workflow_blocker_is_shared_through_the_git_common_directory() {
    let repo = TempRepo::initialized();
    let linked = TempRepo::new();
    assert_success(
        Command::new("git")
            .args(["worktree", "add", "-b", "linked-guard"])
            .arg(linked.path())
            .current_dir(repo.path())
            .output()
            .expect("create linked worktree"),
    );
    fs::create_dir_all(repo.path().join(".git/tiber")).expect("create blocker directory");
    fs::write(
        repo.path().join(".git/tiber/workflow-blocker.json"),
        r#"{"schema_version":1,"kind":"ci_claim_failed","error_code":"tiber.ci_recovery_claim_failed","required_action":"retry claim","created_at":1}"#,
    )
    .expect("write blocker");

    let denied = guard_at(
        linked.path(),
        r#"{"cwd":".","tool_name":"Write","tool_input":{}}"#,
    );
    assert_eq!(denied.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&denied.stderr).contains("workflow_blocked=true"));
}

fn guard(repo: &TempRepo, input: &str) -> std::process::Output {
    guard_at(repo.path(), input)
}

fn guard_at(directory: &Path, input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tiber"))
        .arg("workflow-guard")
        .current_dir(directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start workflow guard");
    child
        .stdin
        .take()
        .expect("guard stdin")
        .write_all(input.as_bytes())
        .expect("write hook input");
    child.wait_with_output().expect("finish workflow guard")
}
