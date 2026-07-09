mod support;

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use support::{assert_success, assert_success_ref, task_stem, TempRepo};

#[test]
fn sync_commits_local_tasks_state_to_tasks_branch() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Sync durable task"]));
    let stem = task_stem(&repo, "backlog", "sync-durable-task");

    let sync = repo.tiber(["sync"]);

    assert_success(sync);
    let tree = repo.git_output([
        "ls-tree",
        "-r",
        "--name-only",
        "tasks",
        &format!("backlog/{stem}.md"),
    ]);
    assert_success_ref(&tree);
    assert_eq!(
        String::from_utf8(tree.stdout).expect("tree output should be utf8"),
        format!("backlog/{stem}.md\n")
    );
}

#[test]
fn sync_pushes_tasks_branch_to_origin_when_remote_exists() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let clone = TempRepo::new();
    assert_success(
        Command::new("git")
            .args(["clone", origin.path().to_str().expect("origin path utf8")])
            .arg(clone.path())
            .output()
            .expect("clone repository"),
    );
    clone.git(["config", "user.email", "tiber@example.test"]);
    clone.git(["config", "user.name", "Tiber Test"]);
    clone.git(["config", "commit.gpgsign", "false"]);
    assert_success(clone.tiber(["init"]));
    assert_success(clone.tiber(["create", "Remote sync task"]));
    let stem = task_stem(&clone, "backlog", "remote-sync-task");

    assert_success(clone.tiber(["sync"]));

    let remote_tasks = clone.git_output(["ls-remote", "--heads", "origin", "tasks"]);
    assert_success_ref(&remote_tasks);
    assert!(
        !remote_tasks.stdout.is_empty(),
        "sync should push refs/heads/tasks to origin"
    );

    let second_clone = TempRepo::new();
    assert_success(
        Command::new("git")
            .args(["clone", origin.path().to_str().expect("origin path utf8")])
            .arg(second_clone.path())
            .output()
            .expect("clone repository"),
    );
    let tree = second_clone.git_output([
        "ls-tree",
        "-r",
        "--name-only",
        "origin/tasks",
        &format!("backlog/{stem}.md"),
    ]);
    assert_success_ref(&tree);
    assert_eq!(
        String::from_utf8(tree.stdout).expect("tree output should be utf8"),
        format!("backlog/{stem}.md\n")
    );
}

#[test]
fn sync_accepts_remote_only_edit_to_existing_task_when_local_changed_elsewhere() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Shared merge task"]));
    let shared_stem = task_stem(&remote, "backlog", "shared-merge-task");
    assert_success(remote.tiber(["sync"]));

    let local = clone_repo(&origin);
    local.git(["fetch", "origin", "tasks:tasks"]);

    remote.insert_task_file(
        "backlog",
        &shared_stem,
        &task_document("Remote edited shared merge task"),
    );
    assert_success(remote.tiber(["sync"]));

    local.insert_task_file(
        "backlog",
        "20260708-abcd-local-only",
        &task_document("Local only merge task"),
    );
    let sync = local.tiber(["sync"]);

    assert_success(sync);
    let shared = origin.git_output(["show", &format!("tasks:backlog/{shared_stem}.md")]);
    assert_success_ref(&shared);
    assert!(
        String::from_utf8(shared.stdout)
            .expect("shared task utf8")
            .contains("title: Remote edited shared merge task"),
        "remote-only task edit should be accepted"
    );
    assert_success_ref(&origin.git_output([
        "cat-file",
        "-e",
        "tasks:backlog/20260708-abcd-local-only.md",
    ]));
}

#[test]
fn sync_keeps_local_only_edit_to_existing_task_when_remote_changed_elsewhere() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Shared local merge task"]));
    let shared_stem = task_stem(&remote, "backlog", "shared-local-merge-task");
    assert_success(remote.tiber(["sync"]));

    let local = clone_repo(&origin);
    local.git(["fetch", "origin", "tasks:tasks"]);

    remote.insert_task_file(
        "backlog",
        "20260708-wxyz-remote-only",
        &task_document("Remote only merge task"),
    );
    assert_success(remote.tiber(["sync"]));

    local.insert_task_file(
        "backlog",
        &shared_stem,
        &task_document("Local edited shared local merge task"),
    );
    let sync = local.tiber(["sync"]);

    assert_success(sync);
    let shared = origin.git_output(["show", &format!("tasks:backlog/{shared_stem}.md")]);
    assert_success_ref(&shared);
    assert!(
        String::from_utf8(shared.stdout)
            .expect("shared task utf8")
            .contains("title: Local edited shared local merge task"),
        "local-only task edit should be preserved"
    );
    assert_success_ref(&origin.git_output([
        "cat-file",
        "-e",
        "tasks:backlog/20260708-wxyz-remote-only.md",
    ]));
}

#[test]
fn write_commands_merge_divergent_remote_tasks_before_resolving_refs() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Remote divergent task"]));
    let remote_stem = task_stem(&remote, "backlog", "remote-divergent-task");
    assert_success(remote.tiber(["sync"]));

    let local = clone_repo(&origin);
    local.git(["fetch", "origin", "tasks:tasks"]);
    local.insert_task_file(
        "backlog",
        "20260708-locl-local-divergent-task",
        &task_document("Local divergent task"),
    );

    assert_success(remote.tiber(["create", "Later remote task"]));
    let later_remote_stem = task_stem(&remote, "backlog", "later-remote-task");
    assert_success(remote.tiber(["sync"]));

    let transition = local.tiber(["transition", &later_remote_stem, "in-progress"]);

    assert_success_ref(&transition);
    let verification = clone_repo(&origin);
    assert_success_ref(&verification.git_output([
        "cat-file",
        "-e",
        &format!("origin/tasks:in-progress/{later_remote_stem}.md"),
    ]));
    assert_success_ref(&verification.git_output([
        "cat-file",
        "-e",
        "origin/tasks:backlog/20260708-locl-local-divergent-task.md",
    ]));
    assert_success_ref(&verification.git_output([
        "cat-file",
        "-e",
        &format!("origin/tasks:backlog/{remote_stem}.md"),
    ]));
}

#[test]
fn task_branch_push_ignores_source_checkout_pre_push_hooks() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let clone = clone_repo(&origin);
    install_failing_pre_push_hook(&clone);

    assert_success(clone.tiber(["init"]));
    assert_success(clone.tiber(["create", "Hook isolated task"]));
    let stem = task_stem(&clone, "backlog", "hook-isolated-task");

    let verification = clone_repo(&origin);
    let tree = verification.git_output([
        "ls-tree",
        "-r",
        "--name-only",
        "origin/tasks",
        &format!("backlog/{stem}.md"),
    ]);
    assert_success_ref(&tree);
    assert_eq!(
        String::from_utf8(tree.stdout).expect("tree output should be utf8"),
        format!("backlog/{stem}.md\n")
    );
}

#[test]
fn task_commits_honor_enabled_git_commit_signing_and_surface_failures() {
    let repo = TempRepo::initialized();
    repo.git(["config", "commit.gpgsign", "true"]);
    repo.git(["config", "gpg.format", "ssh"]);
    repo.git(["config", "user.signingkey", "/missing/tiber-signing-key"]);

    let init = repo.tiber(["init"]);

    assert!(
        !init.status.success(),
        "tiber init should fail when configured commit signing cannot sign"
    );
    let stderr = String::from_utf8(init.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("tiber.command_failed program=git"));
    assert!(stderr.contains("args_redacted=true"));
    assert!(stderr.contains("stderr="));
    assert!(stderr.contains("sign"));
    assert!(!stderr.contains("commit-tree"));
    assert!(!stderr.contains("-S"));
}

#[test]
fn write_commands_sync_to_origin_without_explicit_sync() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let clone = clone_repo(&origin);
    assert_success(clone.tiber(["init"]));
    assert_success(clone.tiber(["create", "Implicit write sync task"]));
    let stem = task_stem(&clone, "backlog", "implicit-write-sync-task");

    let verification = clone_repo(&origin);
    let tree = verification.git_output([
        "ls-tree",
        "-r",
        "--name-only",
        "origin/tasks",
        &format!("backlog/{stem}.md"),
    ]);
    assert_success_ref(&tree);
    assert_eq!(
        String::from_utf8(tree.stdout).expect("tree output should be utf8"),
        format!("backlog/{stem}.md\n")
    );
}

#[test]
fn write_commands_fast_forward_remote_tasks_before_resolving_refs() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let writer = clone_repo(&origin);
    assert_success(writer.tiber(["init"]));
    assert_success(writer.tiber(["create", "Fresh clone mutable task"]));
    let stem = task_stem(&writer, "backlog", "fresh-clone-mutable-task");

    let fresh = clone_repo(&origin);
    let transitioned = fresh.tiber(["transition", &stem, "in-progress"]);

    assert_success_ref(&transitioned);
    let verification = clone_repo(&origin);
    assert_success_ref(&verification.git_output([
        "cat-file",
        "-e",
        &format!("origin/tasks:in-progress/{stem}.md"),
    ]));
}

#[test]
fn write_commands_fast_forward_populated_remote_tasks_before_resolving_refs() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let first = clone_repo(&origin);
    assert_success(first.tiber(["init"]));
    assert_success(first.tiber(["create", "Existing populated task"]));

    let second = clone_repo(&origin);
    assert_success(second.tiber(["create", "Later remote task"]));
    let later_stem = task_stem(&second, "backlog", "later-remote-task");

    let transitioned = first.tiber(["transition", &later_stem, "in-progress"]);

    assert_success_ref(&transitioned);
    let verification = clone_repo(&origin);
    assert_success_ref(&verification.git_output([
        "cat-file",
        "-e",
        &format!("origin/tasks:in-progress/{later_stem}.md"),
    ]));
}

#[test]
fn write_commands_report_fetch_failure_before_missing_ref_retry() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let first = clone_repo(&origin);
    assert_success(first.tiber(["init"]));
    assert_success(first.tiber(["create", "Existing local task"]));

    let second = clone_repo(&origin);
    assert_success(second.tiber(["create", "Unreachable remote task"]));
    let remote_stem = task_stem(&second, "backlog", "unreachable-remote-task");

    let missing_origin = first
        .path()
        .join("private")
        .join("user-secret@example.invalid")
        .join("missing-origin.git");
    first.git([
        "remote",
        "set-url",
        "origin",
        missing_origin
            .to_str()
            .expect("missing origin path should be utf8"),
    ]);

    let transition = first.tiber(["transition", &remote_stem, "in-progress"]);

    assert!(
        !transition.status.success(),
        "transition should surface remote refresh failure"
    );
    let stderr = String::from_utf8(transition.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("tasks_remote_refresh_failed"),
        "stderr should report the failed remote refresh: {stderr}"
    );
    assert!(
        stderr.contains("args_redacted=true"),
        "stderr should redact fetch command arguments: {stderr}"
    );
    assert!(
        stderr.contains("stderr_redacted=true"),
        "stderr should redact fetch stderr: {stderr}"
    );
    assert!(
        !stderr.contains("task_ref_missing"),
        "stderr should not hide the remote failure behind missing local task ref: {stderr}"
    );
    assert!(
        !stderr.contains("secret@example.invalid"),
        "stderr should not leak token-bearing remote details: {stderr}"
    );
}

#[test]
fn transition_sync_does_not_resurrect_stale_remote_status_path() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let clone = clone_repo(&origin);
    assert_success(clone.tiber(["init"]));
    assert_success(clone.tiber(["create", "Move synced task"]));
    let stem = task_stem(&clone, "backlog", "move-synced-task");

    assert_success(clone.tiber(["transition", &stem, "in-progress"]));

    let verification = clone_repo(&origin);
    assert!(
        !verification
            .git_output(["cat-file", "-e", &format!("origin/tasks:backlog/{stem}.md")])
            .status
            .success(),
        "sync should not resurrect the stale backlog copy after transition"
    );
    assert_success_ref(&verification.git_output([
        "cat-file",
        "-e",
        &format!("origin/tasks:in-progress/{stem}.md"),
    ]));
}

#[test]
fn sync_conflicts_when_remote_and_local_move_same_task_to_different_statuses() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote_mover = clone_repo(&origin);
    let local_mover = clone_repo(&origin);
    assert_success(remote_mover.tiber(["init"]));
    assert_success(remote_mover.tiber(["create", "Divergent move task"]));
    let stem = task_stem(&remote_mover, "backlog", "divergent-move-task");
    local_mover.git(["fetch", "origin", "tasks:tasks"]);

    assert_success(remote_mover.tiber(["transition", &stem, "in-progress"]));
    let local_contents = local_mover.task_file("backlog", &stem);
    local_mover.insert_task_file("done", &stem, &local_contents);
    local_mover.remove_tasks_tree_file(&format!("backlog/{stem}.md"));

    let sync = local_mover.tiber(["sync"]);

    assert!(
        !sync.status.success(),
        "sync should fail instead of silently choosing one divergent status move"
    );
    let stderr = String::from_utf8(sync.stderr).expect("stderr should be utf8");
    assert!(stderr.contains(&format!(
        "sync_conflict path={:?}",
        format!("in-progress/{stem}.md")
    )));
    assert!(stderr.contains("run tiber conflict show <path>"));
    assert!(stderr.contains("mcp_tool=tiber.conflict_show"));
    assert!(stderr.contains("mcp_resolve_tool=tiber.conflict_resolve"));

    let conflict = local_mover.tiber(["conflict", "show", &format!("in-progress/{stem}.md")]);
    assert_success_ref(&conflict);
    let conflict: serde_json::Value =
        serde_json::from_slice(&conflict.stdout).expect("conflict output should be json");
    assert_eq!(conflict["path"], format!("in-progress/{stem}.md"));
    assert_eq!(conflict["local_path"], format!("done/{stem}.md"));
    assert_eq!(conflict["remote_path"], format!("in-progress/{stem}.md"));
    assert!(conflict["local"]
        .as_str()
        .expect("local conflict side")
        .contains("title: Divergent move task"));
    assert!(conflict["remote"]
        .as_str()
        .expect("remote conflict side")
        .contains("title: Divergent move task"));

    let conflict_from_local_path =
        local_mover.tiber(["conflict", "show", &format!("done/{stem}.md")]);
    assert_success_ref(&conflict_from_local_path);
    let conflict_from_local_path: serde_json::Value =
        serde_json::from_slice(&conflict_from_local_path.stdout)
            .expect("conflict output should be json");
    assert_eq!(conflict_from_local_path["path"], format!("done/{stem}.md"));
    assert_eq!(
        conflict_from_local_path["local_path"],
        format!("done/{stem}.md")
    );
    assert_eq!(
        conflict_from_local_path["remote_path"],
        format!("in-progress/{stem}.md")
    );
}

#[test]
fn conflict_resolve_local_chooses_local_status_move_and_removes_remote_counterpart() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote_mover = clone_repo(&origin);
    let local_mover = clone_repo(&origin);
    assert_success(remote_mover.tiber(["init"]));
    assert_success(remote_mover.tiber(["create", "Local status move resolution"]));
    let stem = task_stem(&remote_mover, "backlog", "local-status-move-resolution");
    local_mover.git(["fetch", "origin", "tasks:tasks"]);

    assert_success(remote_mover.tiber(["transition", &stem, "in-progress"]));
    let local_contents = local_mover.task_file("backlog", &stem);
    local_mover.insert_task_file("done", &stem, &local_contents);
    local_mover.remove_tasks_tree_file(&format!("backlog/{stem}.md"));
    let sync = local_mover.tiber(["sync"]);
    assert!(!sync.status.success(), "setup should produce a conflict");

    let resolved =
        local_mover.tiber(["conflict", "resolve", &format!("done/{stem}.md"), "--local"]);

    assert_success_ref(&resolved);
    let verification = clone_repo(&origin);
    assert_success_ref(&verification.git_output([
        "cat-file",
        "-e",
        &format!("origin/tasks:done/{stem}.md"),
    ]));
    assert!(
        !verification
            .git_output([
                "cat-file",
                "-e",
                &format!("origin/tasks:in-progress/{stem}.md"),
            ])
            .status
            .success(),
        "local status-move resolution should remove remote status counterpart"
    );
}

#[test]
fn conflict_resolve_local_accepts_remote_status_move_path() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote_mover = clone_repo(&origin);
    let local_mover = clone_repo(&origin);
    assert_success(remote_mover.tiber(["init"]));
    assert_success(remote_mover.tiber(["create", "Local remote-path resolution"]));
    let stem = task_stem(&remote_mover, "backlog", "local-remote-path-resolution");
    local_mover.git(["fetch", "origin", "tasks:tasks"]);

    assert_success(remote_mover.tiber(["transition", &stem, "in-progress"]));
    let local_contents = local_mover.task_file("backlog", &stem);
    local_mover.insert_task_file("done", &stem, &local_contents);
    local_mover.remove_tasks_tree_file(&format!("backlog/{stem}.md"));
    let sync = local_mover.tiber(["sync"]);
    assert!(!sync.status.success(), "setup should produce a conflict");

    let resolved = local_mover.tiber([
        "conflict",
        "resolve",
        &format!("in-progress/{stem}.md"),
        "--local",
    ]);

    assert_success_ref(&resolved);
    let verification = clone_repo(&origin);
    assert_success_ref(&verification.git_output([
        "cat-file",
        "-e",
        &format!("origin/tasks:done/{stem}.md"),
    ]));
    assert!(
        !verification
            .git_output([
                "cat-file",
                "-e",
                &format!("origin/tasks:in-progress/{stem}.md"),
            ])
            .status
            .success(),
        "local status-move resolution should remove remote status counterpart"
    );
}

#[test]
fn conflict_resolve_remote_chooses_remote_status_move_and_removes_local_counterpart() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote_mover = clone_repo(&origin);
    let local_mover = clone_repo(&origin);
    assert_success(remote_mover.tiber(["init"]));
    assert_success(remote_mover.tiber(["create", "Remote status move resolution"]));
    let stem = task_stem(&remote_mover, "backlog", "remote-status-move-resolution");
    local_mover.git(["fetch", "origin", "tasks:tasks"]);

    assert_success(remote_mover.tiber(["transition", &stem, "in-progress"]));
    let local_contents = local_mover.task_file("backlog", &stem);
    local_mover.insert_task_file("done", &stem, &local_contents);
    local_mover.remove_tasks_tree_file(&format!("backlog/{stem}.md"));
    let sync = local_mover.tiber(["sync"]);
    assert!(!sync.status.success(), "setup should produce a conflict");

    let resolved = local_mover.tiber([
        "conflict",
        "resolve",
        &format!("done/{stem}.md"),
        "--remote",
    ]);

    assert_success_ref(&resolved);
    let verification = clone_repo(&origin);
    assert_success_ref(&verification.git_output([
        "cat-file",
        "-e",
        &format!("origin/tasks:in-progress/{stem}.md"),
    ]));
    assert!(
        !verification
            .git_output(["cat-file", "-e", &format!("origin/tasks:done/{stem}.md")])
            .status
            .success(),
        "remote status-move resolution should remove local status counterpart"
    );
}

#[test]
fn conflict_resolve_remote_accepts_remote_status_move_path() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote_mover = clone_repo(&origin);
    let local_mover = clone_repo(&origin);
    assert_success(remote_mover.tiber(["init"]));
    assert_success(remote_mover.tiber(["create", "Remote remote-path resolution"]));
    let stem = task_stem(&remote_mover, "backlog", "remote-remote-path-resolution");
    local_mover.git(["fetch", "origin", "tasks:tasks"]);

    assert_success(remote_mover.tiber(["transition", &stem, "in-progress"]));
    let local_contents = local_mover.task_file("backlog", &stem);
    local_mover.insert_task_file("done", &stem, &local_contents);
    local_mover.remove_tasks_tree_file(&format!("backlog/{stem}.md"));
    let sync = local_mover.tiber(["sync"]);
    assert!(!sync.status.success(), "setup should produce a conflict");

    let resolved = local_mover.tiber([
        "conflict",
        "resolve",
        &format!("in-progress/{stem}.md"),
        "--remote",
    ]);

    assert_success_ref(&resolved);
    let verification = clone_repo(&origin);
    assert_success_ref(&verification.git_output([
        "cat-file",
        "-e",
        &format!("origin/tasks:in-progress/{stem}.md"),
    ]));
    assert!(
        !verification
            .git_output(["cat-file", "-e", &format!("origin/tasks:done/{stem}.md")])
            .status
            .success(),
        "remote status-move resolution should remove local status counterpart"
    );
}

#[test]
fn conflict_resolve_local_accepts_same_content_path_only_move() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote_mover = clone_repo(&origin);
    let local_mover = clone_repo(&origin);
    assert_success(remote_mover.tiber(["init"]));
    assert_success(remote_mover.tiber(["create", "Same content path move"]));
    let stem = task_stem(&remote_mover, "backlog", "same-content-path-move");
    local_mover.git(["fetch", "origin", "tasks:tasks"]);
    let contents = local_mover.task_file("backlog", &stem);

    remote_mover.insert_task_file("abandoned", &stem, &contents);
    remote_mover.remove_tasks_tree_file(&format!("backlog/{stem}.md"));
    remote_mover.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);
    local_mover.insert_task_file("done", &stem, &contents);
    local_mover.remove_tasks_tree_file(&format!("backlog/{stem}.md"));
    let sync = local_mover.tiber(["sync"]);
    assert!(!sync.status.success(), "setup should produce a conflict");

    let resolved =
        local_mover.tiber(["conflict", "resolve", &format!("done/{stem}.md"), "--local"]);

    assert_success_ref(&resolved);
    let verification = clone_repo(&origin);
    assert_success_ref(&verification.git_output([
        "cat-file",
        "-e",
        &format!("origin/tasks:done/{stem}.md"),
    ]));
    assert!(
        !verification
            .git_output([
                "cat-file",
                "-e",
                &format!("origin/tasks:abandoned/{stem}.md"),
            ])
            .status
            .success(),
        "local path-only resolution should remove remote status counterpart"
    );
}

#[test]
fn structured_update_syncs_existing_remote_task_without_self_conflict() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let clone = clone_repo(&origin);
    assert_success(clone.tiber(["init"]));
    assert_success(clone.tiber(["create", "Update synced task"]));
    let stem = task_stem(&clone, "backlog", "update-synced-task");

    let update = clone.tiber([
        "update",
        &stem,
        "--summary",
        "Updated summary after the task was already synced.",
        "--tags",
        "tiber,sync",
    ]);

    assert_success_ref(&update);
    let verification = clone_repo(&origin);
    let task = verification.git_output(["show", &format!("origin/tasks:backlog/{stem}.md")]);
    assert_success_ref(&task);
    let task = String::from_utf8(task.stdout).expect("task should be utf8");
    assert!(task.contains("tags: [tiber, sync]"));
    assert!(task.contains("Updated summary after the task was already synced."));
}

#[test]
fn sync_rejects_oversized_local_task_before_publish() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let writer = clone_repo(&origin);
    assert_success(writer.tiber(["init"]));
    let near_limit_body = "x".repeat(1024 * 1024 - 512);
    writer.insert_task_file(
        "backlog",
        "9999-oversized-local-task",
        &format!(
            "---\ntitle: Oversized local task\nblocked_by: []\nblocks: []\ntags: []\n---\n\n## Notes / Log\n\n{near_limit_body}\n"
        ),
    );

    let sync = writer.tiber([
        "note",
        "add",
        "9999-oversized-local-task",
        &"y".repeat(1024),
    ]);

    assert!(
        !sync.status.success(),
        "oversized local task should fail before publish"
    );
    let stderr = String::from_utf8(sync.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("task_blob_too_large"));
    assert!(stderr.contains("path=\"backlog/9999-oversized-local-task.md\""));
    assert!(!stderr.contains("/tmp/"));
    assert!(stderr.contains("max_bytes=1048576"));
    assert!(
        stderr.contains("reduce this local Tiber task below the size limit"),
        "stderr should describe local recovery:\n{stderr}"
    );
    assert!(!stderr.contains("conflict show"));
    assert!(!stderr.contains("origin/tasks"));
}

#[test]
fn write_commands_fail_when_implicit_sync_cannot_merge_conflict() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let writer = clone_repo(&origin);
    let conflicted = clone_repo(&origin);
    assert_success(writer.tiber(["init"]));
    assert_success(conflicted.tiber(["init"]));

    assert_success(writer.tiber(["create", "Shared implicit conflict"]));
    let remote_stem = task_stem(&writer, "backlog", "shared-implicit-conflict");
    conflicted.insert_task_file(
        "backlog",
        &remote_stem,
        "---\ntitle: Local divergent copy\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );

    let create = conflicted.tiber(["create", "Trigger implicit sync"]);

    assert!(
        !create.status.success(),
        "write command should fail when implicit sync cannot merge"
    );
    let stderr = String::from_utf8(create.stderr).expect("stderr should be utf8");
    assert!(stderr.contains(&format!(
        "sync_conflict path={:?}",
        format!("backlog/{remote_stem}.md")
    )));
    assert!(stderr.contains("run tiber conflict show <path>"));
    assert!(stderr.contains("mcp_tool=tiber.conflict_show"));
    assert!(stderr.contains("mcp_resolve_tool=tiber.conflict_resolve"));
}

#[test]
fn implicit_sync_conflict_preserves_write_command_edit_for_resolution() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Shared update conflict"]));
    let stem = task_stem(&remote, "backlog", "shared-update-conflict");
    assert_success(remote.tiber(["sync"]));
    local.git(["fetch", "origin", "tasks:refs/heads/tasks"]);

    assert_success(remote.tiber([
        "update",
        &stem,
        "--summary",
        "Remote summary that should conflict.",
    ]));
    let local_update = local.tiber([
        "update",
        &stem,
        "--summary",
        "Local summary that must survive the failed implicit sync.",
    ]);

    assert!(
        !local_update.status.success(),
        "local update should hit an implicit sync conflict"
    );
    let show = local.tiber(["conflict", "show", &format!("backlog/{stem}.md")]);
    assert_success_ref(&show);
    let stdout = String::from_utf8(show.stdout).expect("conflict show output should be utf8");
    assert!(
        stdout.contains("Local summary that must survive the failed implicit sync."),
        "conflict show should expose the actual local write-command edit:\n{stdout}"
    );
}

#[test]
fn sync_fetches_and_preserves_remote_tasks_before_pushing() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let first = clone_repo(&origin);
    let second = clone_repo(&origin);
    assert_success(first.tiber(["init"]));
    assert_success(second.tiber(["init"]));
    let second_stem = "20260708-bcde-second-raced-task";
    second.insert_task_file(
        "backlog",
        second_stem,
        "---\ntitle: Second raced task\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );

    assert_success(first.tiber(["create", "First remote task"]));
    let first_stem = task_stem(&first, "backlog", "first-remote-task");
    assert_success(first.tiber(["sync"]));

    assert_success(second.tiber(["create", "Second remote task"]));
    let second_stem = task_stem(&second, "backlog", "second-remote-task");
    assert_success(second.tiber(["sync"]));

    let verification = clone_repo(&origin);
    let tree = verification.git_output(["ls-tree", "-r", "--name-only", "origin/tasks", "backlog"]);
    assert_success_ref(&tree);
    let tree = String::from_utf8(tree.stdout).expect("tree output should be utf8");
    assert!(tree.contains(&format!("backlog/{first_stem}.md")));
    assert!(tree.contains(&format!("backlog/{second_stem}.md")));
}

#[test]
fn sync_does_not_resurrect_fast_forward_remote_task_deletion() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Remote deleted task"]));
    let deleted_stem = task_stem(&remote, "backlog", "remote-deleted-task");
    assert_success(local.tiber(["list"]));

    remote.remove_tasks_tree_file(&format!("backlog/{deleted_stem}.md"));
    remote.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);

    assert_success(local.tiber(["create", "Local surviving task"]));
    let local_stem = task_stem(&local, "backlog", "local-surviving-task");

    let verification = clone_repo(&origin);
    assert!(
        !verification
            .git_output([
                "cat-file",
                "-e",
                &format!("origin/tasks:backlog/{deleted_stem}.md")
            ])
            .status
            .success(),
        "sync must not resurrect a task deleted by the advanced remote tasks ref"
    );
    assert_success_ref(&verification.git_output([
        "cat-file",
        "-e",
        &format!("origin/tasks:backlog/{local_stem}.md"),
    ]));
}

#[test]
fn sync_preserves_local_task_deletion_when_remote_task_is_unchanged() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Locally deleted unchanged remote"]));
    let deleted_stem = task_stem(&remote, "backlog", "locally-deleted-unchanged-remote");
    assert_success(remote.tiber(["sync"]));
    assert_success(local.tiber(["list"]));
    local.git(["fetch", "origin", "tasks:refs/heads/tasks"]);

    local.remove_tasks_tree_file(&format!("backlog/{deleted_stem}.md"));
    assert_success(remote.tiber(["create", "Remote new task"]));
    assert_success(remote.tiber(["sync"]));

    assert_success(local.tiber(["sync"]));

    let verification = clone_repo(&origin);
    assert!(
        !verification
            .git_output([
                "cat-file",
                "-e",
                &format!("origin/tasks:backlog/{deleted_stem}.md")
            ])
            .status
            .success(),
        "sync should publish the local deletion when remote kept the base contents"
    );
    let remote_listing =
        verification.git_output(["ls-tree", "-r", "--name-only", "origin/tasks", "backlog"]);
    assert_success_ref(&remote_listing);
    assert!(
        String::from_utf8(remote_listing.stdout)
            .expect("remote listing should be utf8")
            .contains("-remote-new-task.md"),
        "sync should preserve unrelated remote additions"
    );
}

#[test]
fn sync_conflicts_when_local_deletes_task_that_remote_changed() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Locally deleted remote edit"]));
    let stem = task_stem(&remote, "backlog", "locally-deleted-remote-edit");
    assert_success(remote.tiber(["sync"]));
    assert_success(local.tiber(["list"]));
    local.git(["fetch", "origin", "tasks:refs/heads/tasks"]);

    local.remove_tasks_tree_file(&format!("backlog/{stem}.md"));
    remote.insert_task_file(
        "backlog",
        &stem,
        "---\ntitle: Remote changed deleted task\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    remote.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);

    let sync = local.tiber(["sync"]);

    assert!(
        !sync.status.success(),
        "local deletion should conflict with a changed remote task"
    );
    let stderr = String::from_utf8(sync.stderr).expect("stderr utf8");
    assert!(
        stderr.contains("sync_conflict"),
        "sync should report a conflict instead of resurrecting the task: {stderr}"
    );
}

#[test]
fn conflict_resolve_local_keeps_local_edit_when_remote_deleted_task() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Deleted local resolution"]));
    let stem = task_stem(&remote, "backlog", "deleted-local-resolution");
    assert_success(local.tiber(["list"]));

    remote.remove_tasks_tree_file(&format!("backlog/{stem}.md"));
    remote.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);
    local.insert_task_file(
        "backlog",
        &stem,
        "---\ntitle: Locally kept after delete\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    let sync = local.tiber(["sync"]);
    assert!(
        !sync.status.success(),
        "remote deletion with local edit should conflict"
    );

    let conflict = local.tiber(["conflict", "show", &format!("backlog/{stem}.md")]);
    assert_success_ref(&conflict);
    let conflict: serde_json::Value =
        serde_json::from_slice(&conflict.stdout).expect("conflict output should be json");
    assert_eq!(conflict["remote"], serde_json::Value::Null);

    let resolved = local.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{stem}.md"),
        "--local",
    ]);
    assert_success_ref(&resolved);

    let verification = clone_repo(&origin);
    let task = verification.git_output(["show", &format!("origin/tasks:backlog/{stem}.md")]);
    assert_success_ref(&task);
    assert!(
        String::from_utf8(task.stdout)
            .expect("task output should be utf8")
            .contains("title: Locally kept after delete"),
        "local resolution should restore the selected local task"
    );
}

#[test]
fn conflict_resolve_remote_accepts_remote_deletion_of_local_edit() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Deleted remote resolution"]));
    let stem = task_stem(&remote, "backlog", "deleted-remote-resolution");
    assert_success(local.tiber(["list"]));

    remote.remove_tasks_tree_file(&format!("backlog/{stem}.md"));
    remote.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);
    local.insert_task_file(
        "backlog",
        &stem,
        "---\ntitle: Locally discarded after delete\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    let sync = local.tiber(["sync"]);
    assert!(
        !sync.status.success(),
        "remote deletion with local edit should conflict"
    );

    let resolved = local.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{stem}.md"),
        "--remote",
    ]);
    assert_success_ref(&resolved);

    let verification = clone_repo(&origin);
    assert!(
        !verification
            .git_output(["cat-file", "-e", &format!("origin/tasks:backlog/{stem}.md")])
            .status
            .success(),
        "remote resolution should publish the selected deletion"
    );
}

#[test]
fn sync_hard_fails_when_remote_and_local_task_contents_conflict() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let first = clone_repo(&origin);
    let second = clone_repo(&origin);
    assert_success(first.tiber(["init"]));
    assert_success(second.tiber(["init"]));

    assert_success(first.tiber(["create", "Shared remote task"]));
    let remote_stem = task_stem(&first, "backlog", "shared-remote-task");
    assert_success(first.tiber(["sync"]));

    second.insert_task_file(
        "backlog",
        &remote_stem,
        "---\ntitle: Locally divergent task\nblocked_by: []\nblocks: []\ntags: []\n---\n\n## Notes / Log\n\n--- remote\nthis marker is task content\n",
    );

    let sync = second.tiber(["sync"]);

    assert!(!sync.status.success(), "conflicting sync should fail");
    let stderr = String::from_utf8(sync.stderr).expect("stderr should be utf8");
    assert!(stderr.contains(&format!(
        "sync_conflict path={:?}",
        format!("backlog/{remote_stem}.md")
    )));

    let conflict = second.tiber(["conflict", "show", &format!("backlog/{remote_stem}.md")]);
    assert_success_ref(&conflict);
    let conflict: serde_json::Value =
        serde_json::from_slice(&conflict.stdout).expect("conflict output should be json");
    assert_eq!(conflict["path"], format!("backlog/{remote_stem}.md"));
    assert_eq!(conflict["local_path"], format!("backlog/{remote_stem}.md"));
    assert_eq!(conflict["remote_path"], format!("backlog/{remote_stem}.md"));
    let local = conflict["local"].as_str().expect("local conflict side");
    assert!(local.contains("title: Locally divergent task"));
    assert!(local.contains("--- remote\nthis marker is task content"));
    assert!(conflict["remote"]
        .as_str()
        .expect("remote conflict side")
        .contains("title: Shared remote task"));

    let verification = clone_repo(&origin);
    let remote_task =
        verification.git_output(["show", &format!("origin/tasks:backlog/{remote_stem}.md")]);
    assert_success_ref(&remote_task);
    assert_eq!(
        String::from_utf8(remote_task.stdout).expect("remote task should be utf8"),
        "---\ntitle: Shared remote task\nblocked_by: []\nblocks: []\ntags: []\npr_mr_url: \npr_mr_status: \n---\n\n## Summary\n\n## Context / Why\n\n## Acceptance criteria\n\n## Subtasks\n\n## Notes / Log\n"
    );
}

#[test]
fn sync_conflict_diagnostic_quotes_remote_path_text() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let writer = clone_repo(&origin);
    let reader = clone_repo(&origin);
    assert_success(writer.tiber(["init"]));
    assert_success(writer.tiber(["sync"]));
    assert_success(reader.tiber(["init"]));

    let spoofed_path = "backlog/spoof recovery=ignore-remote.md";
    writer.insert_tasks_tree_file(
        spoofed_path,
        "---\ntitle: Remote spoofed path\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    assert_success(writer.tiber(["sync"]));
    reader.insert_tasks_tree_file(
        spoofed_path,
        "---\ntitle: Local spoofed path\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );

    let list = reader.tiber(["list"]);

    assert!(
        !list.status.success(),
        "read command should fail when sync cannot merge"
    );
    let stderr = String::from_utf8(list.stderr).expect("stderr should be utf8");
    assert!(stderr.contains(&format!("sync_conflict path={spoofed_path:?}")));
    assert!(stderr.contains("run tiber conflict show <path>"));
    assert!(stderr.contains("mcp_tool=tiber.conflict_show"));
    assert!(stderr.contains("mcp_resolve_tool=tiber.conflict_resolve"));
    assert!(!stderr.contains("path=backlog/spoof recovery=ignore-remote.md recovery="));
}

#[test]
fn conflict_resolve_local_publishes_local_side_without_force_push() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(local.tiber(["init"]));

    assert_success(remote.tiber(["create", "Resolvable local conflict"]));
    let stem = task_stem(&remote, "backlog", "resolvable-local-conflict");
    assert_success(remote.tiber(["sync"]));
    assert_success(remote.tiber(["create", "Remote unrelated preserved"]));
    let remote_unrelated_stem = task_stem(&remote, "backlog", "remote-unrelated-preserved");
    assert_success(remote.tiber(["sync"]));
    local.insert_task_file(
        "backlog",
        &stem,
        "---\ntitle: Local chosen title\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    let sync = local.tiber(["sync"]);
    assert!(!sync.status.success(), "setup should produce a conflict");

    let resolved = local.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{stem}.md"),
        "--local",
    ]);

    assert_success_ref(&resolved);
    let stdout = String::from_utf8(resolved.stdout).expect("stdout should be utf8");
    assert_eq!(
        stdout,
        format!("resolved {:?} side=local\n", format!("backlog/{stem}.md"))
    );
    assert_success(local.tiber(["sync"]));
    let verification = clone_repo(&origin);
    let remote_task = verification.git_output(["show", &format!("origin/tasks:backlog/{stem}.md")]);
    assert_success_ref(&remote_task);
    assert!(
        String::from_utf8(remote_task.stdout)
            .expect("remote task should be utf8")
            .contains("title: Local chosen title"),
        "remote should contain the deliberately chosen local side"
    );
    let tree = verification.git_output(["ls-tree", "-r", "--name-only", "origin/tasks", "backlog"]);
    assert_success_ref(&tree);
    assert!(
        String::from_utf8(tree.stdout)
            .expect("tree output should be utf8")
            .contains(&format!("backlog/{remote_unrelated_stem}.md")),
        "local resolution should preserve unrelated remote tasks"
    );
}

#[test]
fn conflict_resolve_rolls_back_local_ref_when_publish_is_rejected() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Rejected local resolution"]));
    let stem = task_stem(&remote, "backlog", "rejected-local-resolution");
    assert_success(remote.tiber(["sync"]));
    assert_success(local.tiber(["list"]));
    remote.insert_task_file(
        "backlog",
        &stem,
        "---\ntitle: Remote rejected resolution\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    remote.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);
    local.insert_task_file(
        "backlog",
        &stem,
        "---\ntitle: Local rejected resolution\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    let setup_conflict = local.tiber(["sync"]);
    assert!(
        !setup_conflict.status.success(),
        "setup should produce a conflict"
    );
    let before = local.git_output(["rev-parse", "tasks"]);
    assert_success_ref(&before);
    let before = String::from_utf8(before.stdout).expect("before ref utf8");
    let hook = install_rejecting_origin_hook(&origin);

    let rejected = local.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{stem}.md"),
        "--local",
    ]);

    assert!(
        !rejected.status.success(),
        "resolution publish should fail while origin rejects pushes"
    );
    let after = local.git_output(["rev-parse", "tasks"]);
    assert_success_ref(&after);
    assert_eq!(
        String::from_utf8(after.stdout).expect("after ref utf8"),
        before,
        "failed conflict resolution should roll local tasks ref back"
    );

    fs::remove_file(hook).expect("remove rejecting hook");
    let resolved = local.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{stem}.md"),
        "--local",
    ]);
    assert_success_ref(&resolved);
    let verification = clone_repo(&origin);
    let remote_task = verification.git_output(["show", &format!("origin/tasks:backlog/{stem}.md")]);
    assert_success_ref(&remote_task);
    assert!(
        String::from_utf8(remote_task.stdout)
            .expect("remote task utf8")
            .contains("title: Local rejected resolution"),
        "retry after failed publish should publish the chosen local side"
    );
}

#[test]
fn conflict_resolve_remote_applies_remote_side_and_preserves_unrelated_local_tasks() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(local.tiber(["init"]));

    assert_success(remote.tiber(["create", "Resolvable remote conflict"]));
    let remote_stem = task_stem(&remote, "backlog", "resolvable-remote-conflict");
    assert_success(remote.tiber(["sync"]));
    local.git(["fetch", "origin", "tasks:tasks"]);
    remote.insert_task_file(
        "backlog",
        &remote_stem,
        "---\ntitle: Remote kept title\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    assert_success(remote.tiber(["sync"]));
    let local_stem = "20260708-efgh-local-unrelated-task";
    local.insert_task_file(
        "backlog",
        local_stem,
        "---\ntitle: Local unrelated task\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    local.insert_task_file(
        "backlog",
        &remote_stem,
        "---\ntitle: Local discarded title\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    let sync = local.tiber(["sync"]);
    assert!(!sync.status.success(), "setup should produce a conflict");

    let resolved = local.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{remote_stem}.md"),
        "--remote",
    ]);

    assert_success_ref(&resolved);
    assert_success(local.tiber(["sync"]));
    let verification = clone_repo(&origin);
    let tree = verification.git_output(["ls-tree", "-r", "--name-only", "origin/tasks", "backlog"]);
    assert_success_ref(&tree);
    let tree = String::from_utf8(tree.stdout).expect("tree output should be utf8");
    assert!(tree.contains(&format!("backlog/{remote_stem}.md")));
    assert!(tree.contains(&format!("backlog/{local_stem}.md")));
    let remote_task =
        verification.git_output(["show", &format!("origin/tasks:backlog/{remote_stem}.md")]);
    assert_success_ref(&remote_task);
    assert!(String::from_utf8(remote_task.stdout)
        .expect("remote task should be utf8")
        .contains("title: Remote kept title"));
}

#[test]
fn conflict_resolve_remote_retries_when_remote_advances_during_push() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    let racer = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Resolver raced conflict"]));
    let conflict_stem = task_stem(&remote, "backlog", "resolver-raced-conflict");
    assert_success(remote.tiber(["sync"]));
    assert_success(local.tiber(["list"]));

    remote.insert_task_file(
        "backlog",
        &conflict_stem,
        "---\ntitle: Remote chosen raced conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    remote.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);
    racer.git(["fetch", "origin", "+tasks:refs/heads/tasks"]);
    local.insert_task_file(
        "backlog",
        &conflict_stem,
        "---\ntitle: Local discarded raced conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    let sync = local.tiber(["sync"]);
    assert!(!sync.status.success(), "setup should produce a conflict");

    let racer_stem = "20260708-cdef-resolver-racer-task";
    racer.insert_task_file(
        "backlog",
        racer_stem,
        "---\ntitle: Resolver racer task\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    let racer_commit = racer.git_output(["rev-parse", "tasks"]);
    assert_success_ref(&racer_commit);
    let racer_commit = String::from_utf8(racer_commit.stdout)
        .expect("racer commit should be utf8")
        .trim()
        .to_string();
    racer.git(["push", "origin", "refs/heads/tasks:refs/tiber-race/tasks"]);
    origin.git(["update-ref", "-d", "refs/tiber-race/tasks"]);
    let marker = install_one_shot_origin_push_race(&origin, &racer_commit);

    let resolved = local.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{conflict_stem}.md"),
        "--remote",
    ]);

    assert_success_ref(&resolved);
    assert!(marker.exists(), "origin hook should trigger resolver race");
    let verification = clone_repo(&origin);
    let tree = verification.git_output(["ls-tree", "-r", "--name-only", "origin/tasks", "backlog"]);
    assert_success_ref(&tree);
    let tree = String::from_utf8(tree.stdout).expect("tree output should be utf8");
    assert!(
        tree.contains(&format!("backlog/{conflict_stem}.md")),
        "remote tree:\n{tree}"
    );
    assert!(
        tree.contains(&format!("backlog/{racer_stem}.md")),
        "remote tree:\n{tree}"
    );
    let remote_task =
        verification.git_output(["show", &format!("origin/tasks:backlog/{conflict_stem}.md")]);
    assert_success_ref(&remote_task);
    assert!(
        String::from_utf8(remote_task.stdout)
            .expect("remote task should be utf8")
            .contains("title: Remote chosen raced conflict"),
        "resolver retry should preserve the selected remote side"
    );
}

#[test]
fn conflict_resolve_rolls_back_when_selected_remote_side_changes_during_retry() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    let racer = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Selected race conflict"]));
    let conflict_stem = task_stem(&remote, "backlog", "selected-race-conflict");
    assert_success(remote.tiber(["sync"]));
    assert_success(local.tiber(["list"]));

    remote.insert_task_file(
        "backlog",
        &conflict_stem,
        "---\ntitle: Remote original selected race\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    remote.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);
    racer.git(["fetch", "origin", "+tasks:refs/heads/tasks"]);
    local.insert_task_file(
        "backlog",
        &conflict_stem,
        "---\ntitle: Local selected race preserved\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    let sync = local.tiber(["sync"]);
    assert!(!sync.status.success(), "setup should produce a conflict");

    racer.insert_task_file(
        "backlog",
        &conflict_stem,
        "---\ntitle: Remote changed during resolver retry\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    let racer_commit = racer.git_output(["rev-parse", "tasks"]);
    assert_success_ref(&racer_commit);
    let racer_commit = String::from_utf8(racer_commit.stdout)
        .expect("racer commit should be utf8")
        .trim()
        .to_string();
    racer.git(["push", "origin", "refs/heads/tasks:refs/tiber-race/tasks"]);
    origin.git(["update-ref", "-d", "refs/tiber-race/tasks"]);
    let marker = install_one_shot_origin_push_race(&origin, &racer_commit);

    let resolved = local.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{conflict_stem}.md"),
        "--remote",
    ]);

    assert!(
        !resolved.status.success(),
        "selected remote side changed during retry should fail"
    );
    assert!(marker.exists(), "origin hook should trigger resolver race");
    let stderr = String::from_utf8(resolved.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("sync_conflict"));
    assert!(stderr.contains(&format!(
        "sync_conflict path={:?}",
        format!("backlog/{conflict_stem}.md")
    )));
    assert!(stderr.contains("run tiber conflict show <path>"));
    assert!(stderr.contains("mcp_tool=tiber.conflict_show"));
    assert!(stderr.contains("mcp_resolve_tool=tiber.conflict_resolve"));
    let local_task = local.git_output(["show", &format!("tasks:backlog/{conflict_stem}.md")]);
    assert_success_ref(&local_task);
    assert!(
        String::from_utf8(local_task.stdout)
            .expect("local task should be utf8")
            .contains("title: Local selected race preserved"),
        "failed resolver retry should preserve the original local conflict side"
    );
}

#[test]
fn conflict_resolve_rolls_back_when_selected_remote_side_disappears_during_retry() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    let racer = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Selected disappearing conflict"]));
    let conflict_stem = task_stem(&remote, "backlog", "selected-disappearing-conflict");
    assert_success(remote.tiber(["sync"]));
    assert_success(local.tiber(["list"]));

    remote.insert_task_file(
        "backlog",
        &conflict_stem,
        "---\ntitle: Remote disappearing selected race\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    remote.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);
    racer.git(["fetch", "origin", "+tasks:refs/heads/tasks"]);
    local.insert_task_file(
        "backlog",
        &conflict_stem,
        "---\ntitle: Local disappearing selected race\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    let sync = local.tiber(["sync"]);
    assert!(!sync.status.success(), "setup should produce a conflict");

    racer.remove_tasks_tree_file(&format!("backlog/{conflict_stem}.md"));
    let racer_commit = racer.git_output(["rev-parse", "tasks"]);
    assert_success_ref(&racer_commit);
    let racer_commit = String::from_utf8(racer_commit.stdout)
        .expect("racer commit should be utf8")
        .trim()
        .to_string();
    racer.git(["push", "origin", "refs/heads/tasks:refs/tiber-race/tasks"]);
    origin.git(["update-ref", "-d", "refs/tiber-race/tasks"]);
    let marker = install_one_shot_origin_push_race(&origin, &racer_commit);

    let resolved = local.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{conflict_stem}.md"),
        "--remote",
    ]);

    assert!(
        !resolved.status.success(),
        "selected remote side disappearing during retry should fail"
    );
    assert!(marker.exists(), "origin hook should trigger resolver race");
    let stderr = String::from_utf8(resolved.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("sync_conflict"));
    assert!(stderr.contains(&format!(
        "sync_conflict path={:?}",
        format!("backlog/{conflict_stem}.md")
    )));
    assert!(stderr.contains("run tiber conflict show <path>"));
    assert!(stderr.contains("mcp_tool=tiber.conflict_show"));
    assert!(stderr.contains("mcp_resolve_tool=tiber.conflict_resolve"));
    let local_task = local.git_output(["show", &format!("tasks:backlog/{conflict_stem}.md")]);
    assert_success_ref(&local_task);
    assert!(
        String::from_utf8(local_task.stdout)
            .expect("local task should be utf8")
            .contains("title: Local disappearing selected race"),
        "failed resolver retry should preserve the original local conflict side"
    );
}

#[test]
fn conflict_resolve_rolls_back_when_selected_remote_side_moves_during_retry() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    let racer = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Selected moving conflict"]));
    let conflict_stem = task_stem(&remote, "backlog", "selected-moving-conflict");
    assert_success(remote.tiber(["sync"]));
    assert_success(local.tiber(["list"]));

    let remote_contents =
        "---\ntitle: Remote moving selected race\nblocked_by: []\nblocks: []\ntags: []\n---\n";
    remote.insert_task_file("backlog", &conflict_stem, remote_contents);
    remote.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);
    racer.git(["fetch", "origin", "+tasks:refs/heads/tasks"]);
    local.insert_task_file(
        "backlog",
        &conflict_stem,
        "---\ntitle: Local moving selected race\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    let sync = local.tiber(["sync"]);
    assert!(!sync.status.success(), "setup should produce a conflict");

    racer.remove_tasks_tree_file(&format!("backlog/{conflict_stem}.md"));
    racer.insert_task_file("done", &conflict_stem, remote_contents);
    let racer_commit = racer.git_output(["rev-parse", "tasks"]);
    assert_success_ref(&racer_commit);
    let racer_commit = String::from_utf8(racer_commit.stdout)
        .expect("racer commit should be utf8")
        .trim()
        .to_string();
    racer.git(["push", "origin", "refs/heads/tasks:refs/tiber-race/tasks"]);
    origin.git(["update-ref", "-d", "refs/tiber-race/tasks"]);
    let marker = install_one_shot_origin_push_race(&origin, &racer_commit);

    let resolved = local.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{conflict_stem}.md"),
        "--remote",
    ]);

    assert!(
        !resolved.status.success(),
        "selected remote side moving during retry should fail"
    );
    assert!(marker.exists(), "origin hook should trigger resolver race");
    let stderr = String::from_utf8(resolved.stderr).expect("stderr should be utf8");
    assert!(stderr.contains(&format!(
        "sync_conflict path={:?}",
        format!("backlog/{conflict_stem}.md")
    )));
    assert!(stderr.contains("run tiber conflict show <path>"));
    let local_task = local.git_output(["show", &format!("tasks:backlog/{conflict_stem}.md")]);
    assert_success_ref(&local_task);
    assert!(
        String::from_utf8(local_task.stdout)
            .expect("local task should be utf8")
            .contains("title: Local moving selected race"),
        "failed resolver retry should preserve the original local conflict side"
    );
}

#[test]
fn conflict_resolve_makes_progress_when_multiple_conflicts_exist() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(local.tiber(["init"]));

    assert_success(remote.tiber(["create", "First multi conflict"]));
    let first = task_stem(&remote, "backlog", "first-multi-conflict");
    assert_success(remote.tiber(["create", "Second multi conflict"]));
    let second = task_stem(&remote, "backlog", "second-multi-conflict");
    assert_success(remote.tiber(["sync"]));
    assert_success(local.tiber(["list"]));

    remote.insert_task_file(
        "backlog",
        &first,
        "---\ntitle: Remote first multi conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    remote.insert_task_file(
        "backlog",
        &second,
        "---\ntitle: Remote second multi conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    remote.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);
    local.insert_task_file(
        "backlog",
        &first,
        "---\ntitle: Local first multi conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    local.insert_task_file(
        "backlog",
        &second,
        "---\ntitle: Local second multi conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    let sync = local.tiber(["sync"]);
    assert!(!sync.status.success(), "setup should produce conflicts");

    let single = local.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{first}.md"),
        "--local",
    ]);
    assert!(
        !single.status.success(),
        "single-path resolution should refuse an unselected second conflict"
    );
    let stderr = String::from_utf8(single.stderr).expect("stderr utf8");
    assert!(stderr.contains(&format!(
        "sync_conflict path={:?}",
        format!("backlog/{second}.md")
    )));

    let duplicate_same_path = local.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{first}.md"),
        "--remote",
        &format!("backlog/{first}.md"),
        "--local",
    ]);
    assert!(
        !duplicate_same_path.status.success(),
        "duplicate same-path resolution should be rejected"
    );
    let stderr = String::from_utf8(duplicate_same_path.stderr).expect("stderr utf8");
    assert!(stderr.contains("duplicate_conflict_resolution"));
    assert!(stderr.contains(&format!("backlog/{first}.md")));

    assert_success(local.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{first}.md"),
        "--local",
        &format!("backlog/{second}.md"),
        "--remote",
    ]));
    assert_success(local.tiber(["sync"]));
    let verification = clone_repo(&origin);
    let first_task = verification.git_output(["show", &format!("origin/tasks:backlog/{first}.md")]);
    assert_success_ref(&first_task);
    assert!(String::from_utf8(first_task.stdout)
        .expect("first task utf8")
        .contains("title: Local first multi conflict"));
    let second_task =
        verification.git_output(["show", &format!("origin/tasks:backlog/{second}.md")]);
    assert_success_ref(&second_task);
    assert!(String::from_utf8(second_task.stdout)
        .expect("second task utf8")
        .contains("title: Remote second multi conflict"));
}

#[test]
fn conflict_resolve_rejects_non_conflicted_paths() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Not conflicted"]));
    let stem = task_stem(&remote, "backlog", "not-conflicted");
    assert_success(remote.tiber(["sync"]));
    assert_success(local.tiber(["list"]));
    assert_success(local.tiber(["sync"]));

    let resolve = local.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{stem}.md"),
        "--local",
    ]);

    assert!(
        !resolve.status.success(),
        "non-conflicted resolution should be rejected"
    );
    let stderr = String::from_utf8(resolve.stderr).expect("stderr utf8");
    assert!(
        stderr.contains("conflict_side_not_in_conflict"),
        "unexpected stderr: {stderr}"
    );
    assert!(stderr.contains(&format!("backlog/{stem}.md")));
}

#[test]
fn conflict_resolve_rejects_duplicate_same_stem_batch_entries() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Moved duplicate conflict"]));
    let stem = task_stem(&remote, "backlog", "moved-duplicate-conflict");
    assert_success(remote.tiber(["sync"]));
    assert_success(local.tiber(["list"]));

    remote.insert_task_file(
        "backlog",
        &stem,
        "---\ntitle: Remote moved duplicate conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    remote.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);
    local.remove_tasks_tree_file(&format!("backlog/{stem}.md"));
    local.insert_task_file(
        "in-progress",
        &stem,
        "---\ntitle: Local moved duplicate conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );

    let duplicate_same_stem = local.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{stem}.md"),
        "--remote",
        &format!("in-progress/{stem}.md"),
        "--local",
    ]);

    assert!(
        !duplicate_same_stem.status.success(),
        "duplicate same-stem resolution should be rejected"
    );
    let stderr = String::from_utf8(duplicate_same_stem.stderr).expect("stderr utf8");
    assert!(stderr.contains("duplicate_conflict_resolution"));
    assert!(stderr.contains(&format!("in-progress/{stem}.md")));
}

#[test]
fn conflict_resolve_local_order_appends_unrelated_remote_tasks() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Common first"]));
    let first = task_stem(&remote, "backlog", "common-first");
    assert_success(remote.tiber(["create", "Common second"]));
    let second = task_stem(&remote, "backlog", "common-second");
    assert_success(remote.tiber(["sync"]));
    assert_success(local.tiber(["list"]));

    local.insert_tasks_tree_file("order.md", &format!("{second}\n{first}\n"));
    assert_success(remote.tiber(["create", "Remote order addition"]));
    let remote_addition = task_stem(&remote, "backlog", "remote-order-addition");
    assert_success(remote.tiber(["sync"]));

    let sync = local.tiber(["sync"]);
    assert!(
        !sync.status.success(),
        "setup should produce order conflict"
    );
    let resolved = local.tiber(["conflict", "resolve", "order.md", "--local"]);

    assert_success_ref(&resolved);
    assert_success(local.tiber(["sync"]));
    let verification = clone_repo(&origin);
    let order = verification.git_output(["show", "origin/tasks:order.md"]);
    assert_success_ref(&order);
    assert_eq!(
        String::from_utf8(order.stdout).expect("order should be utf8"),
        format!("{second}\n{first}\n{remote_addition}\n")
    );
}

#[test]
fn conflict_resolve_remote_order_appends_unrelated_local_tasks() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let remote = clone_repo(&origin);
    let local = clone_repo(&origin);
    assert_success(remote.tiber(["init"]));
    assert_success(remote.tiber(["create", "Remote first"]));
    let first = task_stem(&remote, "backlog", "remote-first");
    assert_success(remote.tiber(["create", "Remote second"]));
    let second = task_stem(&remote, "backlog", "remote-second");
    assert_success(remote.tiber(["sync"]));
    assert_success(local.tiber(["list"]));

    let local_addition = "20260708-2abc-local-order-addition";
    local.insert_task_file(
        "backlog",
        local_addition,
        "---\ntitle: Local order addition\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    local.insert_tasks_tree_file(
        "order.md",
        &format!("{first}\n{second}\n{local_addition}\n"),
    );
    remote.insert_tasks_tree_file("order.md", &format!("{second}\n{first}\n"));
    remote.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);

    let sync = local.tiber(["sync"]);
    assert!(
        !sync.status.success(),
        "setup should produce order conflict"
    );
    let resolved = local.tiber(["conflict", "resolve", "order.md", "--remote"]);

    assert_success_ref(&resolved);
    assert_success(local.tiber(["sync"]));
    let verification = clone_repo(&origin);
    let order = verification.git_output(["show", "origin/tasks:order.md"]);
    assert_success_ref(&order);
    assert_eq!(
        String::from_utf8(order.stdout).expect("order should be utf8"),
        format!("{second}\n{first}\n{local_addition}\n")
    );
}

#[test]
fn sync_retries_once_when_remote_advances_during_push() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let first = clone_repo(&origin);
    let second = clone_repo(&origin);
    let racer = clone_repo(&origin);
    assert_success(first.tiber(["init"]));
    assert_success(first.tiber(["create", "Base remote task"]));
    let base_stem = task_stem(&first, "backlog", "base-remote-task");
    assert_success(first.tiber(["sync"]));

    assert_success(second.tiber(["init"]));
    let second_stem = "20260708-bcde-second-raced-task";
    second.insert_task_file(
        "backlog",
        second_stem,
        "---\ntitle: Second raced task\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    assert_success(racer.tiber(["init"]));
    racer.git(["fetch", "origin", "tasks:refs/remotes/origin/tasks"]);
    racer.git([
        "update-ref",
        "refs/heads/tasks",
        "refs/remotes/origin/tasks",
    ]);
    let racer_stem = "20260708-abcd-racer-remote-task";
    racer.insert_task_file(
        "backlog",
        racer_stem,
        "---\ntitle: Racer remote task\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    let racer_commit = racer.git_output(["rev-parse", "tasks"]);
    assert_success_ref(&racer_commit);
    let racer_commit = String::from_utf8(racer_commit.stdout)
        .expect("racer commit should be utf8")
        .trim()
        .to_string();
    racer.git(["push", "origin", "refs/heads/tasks:refs/tiber-race/tasks"]);
    origin.git(["update-ref", "-d", "refs/tiber-race/tasks"]);
    let marker = install_one_shot_origin_push_race(&origin, &racer_commit);

    assert_success(second.tiber(["sync"]));
    assert!(marker.exists(), "origin hook should trigger the push race");

    let verification = clone_repo(&origin);
    let tree = verification.git_output(["ls-tree", "-r", "--name-only", "origin/tasks", "backlog"]);
    assert_success_ref(&tree);
    let tree = String::from_utf8(tree.stdout).expect("tree output should be utf8");
    assert!(
        tree.contains(&format!("backlog/{base_stem}.md")),
        "remote tree:\n{tree}"
    );
    assert!(
        tree.contains(&format!("backlog/{racer_stem}.md")),
        "remote tree:\n{tree}"
    );
    assert!(
        tree.contains(&format!("backlog/{second_stem}.md")),
        "remote tree:\n{tree}"
    );
}

#[test]
fn read_commands_sync_remote_tasks_without_explicit_sync() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let writer = clone_repo(&origin);
    let reader = clone_repo(&origin);
    assert_success(writer.tiber(["init"]));
    assert_success(reader.tiber(["init"]));

    assert_success(writer.tiber(["create", "Remote read task"]));
    let stem = task_stem(&writer, "backlog", "remote-read-task");
    assert_success(writer.tiber(["sync"]));

    let list = reader.tiber(["list"]);

    assert_success_ref(&list);
    assert_eq!(
        String::from_utf8(list.stdout).expect("list output should be utf8"),
        format!("{stem}\tRemote read task\n")
    );
}

#[test]
fn read_commands_sync_remote_tasks_from_detached_head() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let writer = clone_repo(&origin);
    assert_success(writer.tiber(["init"]));
    assert_success(writer.tiber(["create", "Detached read task"]));
    assert_success(writer.tiber(["sync"]));

    let reader = clone_repo(&origin);
    let head = reader.git_output(["rev-parse", "HEAD"]);
    assert_success_ref(&head);
    let head = String::from_utf8(head.stdout)
        .expect("head ref utf8")
        .trim()
        .to_string();
    reader.git(["checkout", "--detach", &head]);

    let list = reader.tiber(["list"]);

    assert_success_ref(&list);
    assert!(String::from_utf8(list.stdout)
        .expect("list output utf8")
        .contains("Detached read task"));
}

#[test]
fn read_commands_fail_when_remote_tasks_cannot_merge() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let writer = clone_repo(&origin);
    let reader = clone_repo(&origin);
    assert_success(writer.tiber(["init"]));
    assert_success(reader.tiber(["init"]));

    assert_success(writer.tiber(["create", "Conflicting remote read"]));
    let remote_stem = task_stem(&writer, "backlog", "conflicting-remote-read");
    assert_success(writer.tiber(["sync"]));

    reader.insert_task_file(
        "backlog",
        &remote_stem,
        "---\ntitle: Local stale read copy\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );

    let list = reader.tiber(["list"]);

    assert!(
        !list.status.success(),
        "read command should fail when sync cannot merge"
    );
    let stderr = String::from_utf8(list.stderr).expect("stderr should be utf8");
    assert!(stderr.contains(&format!(
        "sync_conflict path={:?}",
        format!("backlog/{remote_stem}.md")
    )));
    assert!(stderr.contains("run tiber conflict show <path>"));
    assert!(stderr.contains("mcp_tool=tiber.conflict_show"));
    assert!(stderr.contains("mcp_resolve_tool=tiber.conflict_resolve"));
}

#[test]
fn sync_rejects_remote_tasks_non_fast_forward_rewrite() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let clone = clone_repo(&origin);
    assert_success(clone.tiber(["init"]));
    assert_success(clone.tiber(["create", "Rewrite protected task"]));
    assert_success(clone.tiber(["sync"]));
    clone.git(["fetch", "origin", "tasks:refs/remotes/origin/tasks"]);
    let known_good = clone.git_output(["rev-parse", "refs/remotes/origin/tasks"]);
    assert_success_ref(&known_good);
    let known_good = String::from_utf8(known_good.stdout)
        .expect("known good ref utf8")
        .trim()
        .to_string();
    let rewriter = clone_repo(&origin);
    rewriter.git(["fetch", "origin", "tasks:tasks"]);
    let tree = rewriter.git_output(["rev-parse", "tasks^{tree}"]);
    assert_success_ref(&tree);
    let tree = String::from_utf8(tree.stdout)
        .expect("tree ref utf8")
        .trim()
        .to_string();
    let main = rewriter.git_output(["rev-parse", "main"]);
    assert_success_ref(&main);
    let main = String::from_utf8(main.stdout)
        .expect("main ref utf8")
        .trim()
        .to_string();
    let rewritten = rewriter.git_output(["commit-tree", &tree, "-p", &main, "-m", "Rewrite tasks"]);
    assert_success_ref(&rewritten);
    let rewritten = String::from_utf8(rewritten.stdout)
        .expect("rewritten ref utf8")
        .trim()
        .to_string();
    rewriter.git([
        "push",
        "--force",
        "origin",
        &format!("{rewritten}:refs/heads/tasks"),
    ]);

    let list = clone.tiber(["list"]);

    assert!(
        !list.status.success(),
        "sync should fail when origin/tasks rewrites non-fast-forward"
    );
    let stderr = String::from_utf8(list.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("tasks_remote_rewritten"));
    assert!(stderr.contains("do not force-push or overwrite shared task state"));
    let tracking_ref = clone.git_output(["rev-parse", "refs/remotes/origin/tasks"]);
    assert_success_ref(&tracking_ref);
    assert_eq!(
        String::from_utf8(tracking_ref.stdout)
            .expect("tracking ref utf8")
            .trim(),
        known_good
    );
}

#[test]
fn sync_rejects_remote_tasks_deletion_after_tracking_ref_seen() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let clone = clone_repo(&origin);
    assert_success(clone.tiber(["init"]));
    assert_success(clone.tiber(["create", "Deletion protected task"]));
    assert_success(clone.tiber(["sync"]));
    clone.git(["fetch", "origin", "tasks:refs/remotes/origin/tasks"]);
    let known_good = clone.git_output(["rev-parse", "refs/remotes/origin/tasks"]);
    assert_success_ref(&known_good);
    let known_good = String::from_utf8(known_good.stdout)
        .expect("known good ref utf8")
        .trim()
        .to_string();

    origin.git(["update-ref", "-d", "refs/heads/tasks"]);

    let list = clone.tiber(["list"]);

    assert!(
        !list.status.success(),
        "sync should fail when origin/tasks disappears after being tracked"
    );
    let stderr = String::from_utf8(list.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("tasks_remote_rewritten"));
    assert!(stderr.contains("do not force-push or overwrite shared task state"));
    let tracking_ref = clone.git_output(["rev-parse", "refs/remotes/origin/tasks"]);
    assert_success_ref(&tracking_ref);
    assert_eq!(
        String::from_utf8(tracking_ref.stdout)
            .expect("tracking ref utf8")
            .trim(),
        known_good
    );
}

#[test]
fn sync_rejects_remote_tasks_deletion_when_local_tasks_ref_exists_without_tracking_ref() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let clone = clone_repo(&origin);
    assert_success(clone.tiber(["init"]));
    assert_success(clone.tiber(["create", "Untracked deletion protected task"]));
    assert_success(clone.tiber(["sync"]));
    clone.git(["update-ref", "-d", "refs/remotes/origin/tasks"]);
    origin.git(["update-ref", "-d", "refs/heads/tasks"]);

    let sync = clone.tiber(["sync"]);

    assert!(
        !sync.status.success(),
        "sync should fail when origin/tasks disappears while local tasks ref exists"
    );
    let stderr = String::from_utf8(sync.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("tasks_remote_rewritten"));
    assert!(stderr.contains("do not force-push or overwrite shared task state"));
    let remote = origin.git_output(["show-ref", "--verify", "refs/heads/tasks"]);
    assert!(
        !remote.status.success(),
        "sync must not recreate deleted origin/tasks"
    );
}

fn task_document(title: &str) -> String {
    format!("---\ntitle: {title}\nblocked_by: []\nblocks: []\ntags: []\n---\n")
}

fn clone_repo(origin: &TempRepo) -> TempRepo {
    let clone = TempRepo::new();
    assert_success(
        Command::new("git")
            .args(["clone", origin.path().to_str().expect("origin path utf8")])
            .arg(clone.path())
            .output()
            .expect("clone repository"),
    );
    clone.git(["config", "user.email", "tiber@example.test"]);
    clone.git(["config", "user.name", "Tiber Test"]);
    clone.git(["config", "commit.gpgsign", "false"]);
    clone
}

fn install_rejecting_origin_hook(origin: &TempRepo) -> std::path::PathBuf {
    let hook_dir = origin.path().join("hooks");
    fs::create_dir_all(&hook_dir).expect("create origin hook directory");
    let hook = hook_dir.join("pre-receive");
    fs::write(
        &hook,
        "#!/usr/bin/env bash\nset -euo pipefail\necho 'rejecting tiber push' >&2\nexit 1\n",
    )
    .expect("write rejecting hook");
    #[cfg(unix)]
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755))
        .expect("make rejecting hook executable");
    hook
}

fn install_one_shot_origin_push_race(origin: &TempRepo, racer_commit: &str) -> std::path::PathBuf {
    let hook_dir = origin.path().join("hooks");
    fs::create_dir_all(&hook_dir).expect("create origin hook directory");
    let marker = origin.path().join("tiber-pre-receive-raced");
    let hook = hook_dir.join("pre-receive");
    fs::write(
        &hook,
        format!(
            "#!/usr/bin/env bash\nset -euo pipefail\nif [[ ! -e '{}' ]]; then\n  touch '{}'\n  env -u GIT_QUARANTINE_PATH git --git-dir='{}' update-ref refs/heads/tasks '{}'\n  echo 'fetch first' >&2\n  exit 1\nfi\n",
            marker.display(),
            marker.display(),
            origin.path().display(),
            racer_commit
        ),
    )
    .expect("write origin pre-receive hook");
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&hook)
            .expect("origin hook metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&hook, permissions).expect("make origin hook executable");
    }
    marker
}

fn install_failing_pre_push_hook(repo: &TempRepo) {
    let hook_dir = repo.path().join(".git").join("hooks");
    fs::create_dir_all(&hook_dir).expect("create hooks directory");
    let hook = hook_dir.join("pre-push");
    fs::write(
        &hook,
        "#!/usr/bin/env bash\nset -euo pipefail\necho source pre-push hook should not run >&2\nexit 1\n",
    )
    .expect("write pre-push hook");
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&hook).expect("hook metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&hook, permissions).expect("make hook executable");
    }
}
