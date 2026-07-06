mod support;

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn sync_commits_local_tasks_state_to_tasks_branch() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Sync durable task"]));

    let sync = repo.tiber(["sync"]);

    assert_success(sync);
    let tree = repo.git_output([
        "ls-tree",
        "-r",
        "--name-only",
        "tasks",
        "main/.tasks/todo/sync-durable-task.md",
    ]);
    assert_success_ref(&tree);
    assert_eq!(
        String::from_utf8(tree.stdout).expect("tree output should be utf8"),
        "main/.tasks/todo/sync-durable-task.md\n"
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
        "main/.tasks/todo/remote-sync-task.md",
    ]);
    assert_success_ref(&tree);
    assert_eq!(
        String::from_utf8(tree.stdout).expect("tree output should be utf8"),
        "main/.tasks/todo/remote-sync-task.md\n"
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

    assert_success(first.tiber(["create", "First remote task"]));
    assert_success(first.tiber(["sync"]));

    assert_success(second.tiber(["create", "Second remote task"]));
    assert_success(second.tiber(["sync"]));

    let verification = clone_repo(&origin);
    let tree = verification.git_output([
        "ls-tree",
        "-r",
        "--name-only",
        "origin/tasks",
        "main/.tasks/todo",
    ]);
    assert_success_ref(&tree);
    assert_eq!(
        String::from_utf8(tree.stdout).expect("tree output should be utf8"),
        "main/.tasks/todo/first-remote-task.md\nmain/.tasks/todo/second-remote-task.md\n"
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
    assert_success(first.tiber(["sync"]));

    assert_success(second.tiber(["create", "Shared remote task"]));
    fs::write(
        second
            .path()
            .join(".tasks")
            .join("todo")
            .join("shared-remote-task.md"),
        "# Locally divergent task\n",
    )
    .expect("write divergent local task");

    let sync = second.tiber(["sync"]);

    assert!(!sync.status.success(), "conflicting sync should fail");
    let stderr = String::from_utf8(sync.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("sync_conflict path=todo/shared-remote-task.md"));

    let verification = clone_repo(&origin);
    let remote_task = verification.git_output([
        "show",
        "origin/tasks:main/.tasks/todo/shared-remote-task.md",
    ]);
    assert_success_ref(&remote_task);
    assert_eq!(
        String::from_utf8(remote_task.stdout).expect("remote task should be utf8"),
        "# Shared remote task\n"
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
    assert_success(first.tiber(["sync"]));

    assert_success(second.tiber(["init"]));
    assert_success(second.tiber(["create", "Second raced task"]));
    assert_success(racer.tiber(["init"]));
    assert_success(racer.tiber(["create", "Racer remote task"]));
    install_one_shot_pre_push_race(&second, &racer);

    assert_success(second.tiber(["sync"]));

    let verification = clone_repo(&origin);
    let tree = verification.git_output([
        "ls-tree",
        "-r",
        "--name-only",
        "origin/tasks",
        "main/.tasks/todo",
    ]);
    assert_success_ref(&tree);
    assert_eq!(
        String::from_utf8(tree.stdout).expect("tree output should be utf8"),
        "main/.tasks/todo/base-remote-task.md\nmain/.tasks/todo/racer-remote-task.md\nmain/.tasks/todo/second-raced-task.md\n"
    );
}

#[test]
fn read_commands_soft_sync_remote_tasks_without_explicit_sync() {
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
    assert_success(writer.tiber(["sync"]));

    let list = reader.tiber(["list"]);

    assert_success_ref(&list);
    assert_eq!(
        String::from_utf8(list.stdout).expect("list output should be utf8"),
        "todo/remote-read-task.md\tRemote read task\n"
    );
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
