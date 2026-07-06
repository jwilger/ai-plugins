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
    assert!(stderr.contains("commit-tree"));
    assert!(stderr.contains("-S"));
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
    assert!(stderr.contains(&format!("sync_conflict path=backlog/{remote_stem}.md")));
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
        "---\ntitle: Locally divergent task\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );

    let sync = second.tiber(["sync"]);

    assert!(!sync.status.success(), "conflicting sync should fail");
    let stderr = String::from_utf8(sync.stderr).expect("stderr should be utf8");
    assert!(stderr.contains(&format!("sync_conflict path=backlog/{remote_stem}.md")));

    let verification = clone_repo(&origin);
    let remote_task =
        verification.git_output(["show", &format!("origin/tasks:backlog/{remote_stem}.md")]);
    assert_success_ref(&remote_task);
    assert_eq!(
        String::from_utf8(remote_task.stdout).expect("remote task should be utf8"),
        "---\ntitle: Shared remote task\nblocked_by: []\nblocks: []\ntags: []\n---\n\n## Summary\n\n## Context / Why\n\n## Acceptance criteria\n\n## Subtasks\n\n## Notes / Log\n"
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
    assert_success(second.tiber(["create", "Second raced task"]));
    let second_stem = task_stem(&second, "backlog", "second-raced-task");
    assert_success(racer.tiber(["init"]));
    assert_success(racer.tiber(["create", "Racer remote task"]));
    let racer_stem = task_stem(&racer, "backlog", "racer-remote-task");
    install_one_shot_pre_push_race(&second, &racer);

    assert_success(second.tiber(["sync"]));

    let verification = clone_repo(&origin);
    let tree = verification.git_output(["ls-tree", "-r", "--name-only", "origin/tasks", "backlog"]);
    assert_success_ref(&tree);
    let tree = String::from_utf8(tree.stdout).expect("tree output should be utf8");
    assert!(tree.contains(&format!("backlog/{base_stem}.md")));
    assert!(tree.contains(&format!("backlog/{racer_stem}.md")));
    assert!(tree.contains(&format!("backlog/{second_stem}.md")));
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
    assert!(stderr.contains(&format!("sync_conflict path=backlog/{remote_stem}.md")));
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

fn install_one_shot_pre_push_race(repo: &TempRepo, racer: &TempRepo) {
    let hook_dir = repo.path().join(".git").join("hooks");
    fs::create_dir_all(&hook_dir).expect("create hooks directory");
    let marker = repo.path().join(".git").join("tiber-pre-push-raced");
    let hook = hook_dir.join("pre-push");
    fs::write(
        &hook,
        format!(
            "#!/usr/bin/env bash\nset -euo pipefail\nif [[ ! -e '{}' ]]; then\n  touch '{}'\n  cd '{}'\n  '{}' sync\nfi\n",
            marker.display(),
            marker.display(),
            racer.path().display(),
            env!("CARGO_BIN_EXE_tiber")
        ),
    )
    .expect("write pre-push hook");
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&hook).expect("hook metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&hook, permissions).expect("make hook executable");
    }
    assert!(
        racer.path().exists(),
        "racer repo must live long enough for the pre-push hook"
    );
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
