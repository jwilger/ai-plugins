mod support;

use std::process::Command;

use support::{assert_success, assert_success_ref, task_stem, TempRepo};

#[test]
fn metadata_reports_task_commit_time_from_tasks_branch() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber_with_env(
        ["create", "Date stamped task"],
        [
            ("GIT_AUTHOR_DATE", "2024-01-02T03:04:05Z"),
            ("GIT_COMMITTER_DATE", "2024-01-02T03:04:05Z"),
        ],
    ));
    let stem = task_stem(&repo, "backlog", "date-stamped-task");

    let metadata = repo.tiber(["metadata", "date-stamped-task"]);

    assert_success_ref(&metadata);
    assert_eq!(
        String::from_utf8(metadata.stdout).expect("metadata output should be utf8"),
        format!("{stem}\tDate stamped task\tcommitted_at=2024-01-02T03:04:05Z\n")
    );
}

#[test]
fn metadata_reports_remote_task_commit_time_after_read_sync() {
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
    assert_success(writer.tiber_with_env(
        ["create", "Remote dated task"],
        [
            ("GIT_AUTHOR_DATE", "2024-03-04T05:06:07Z"),
            ("GIT_COMMITTER_DATE", "2024-03-04T05:06:07Z"),
        ],
    ));
    let stem = task_stem(&writer, "backlog", "remote-dated-task");
    assert_success(writer.tiber(["sync"]));

    let metadata = reader.tiber(["metadata", "remote-dated-task"]);

    assert_success_ref(&metadata);
    assert_eq!(
        String::from_utf8(metadata.stdout).expect("metadata output should be utf8"),
        format!("{stem}\tRemote dated task\tcommitted_at=2024-03-04T05:06:07Z\n")
    );
}

#[test]
fn sync_preserves_unrelated_local_task_history_after_remote_advances() {
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
    assert_success(remote.tiber(["create", "Shared base task"]));
    assert_success(remote.tiber(["sync"]));
    assert_success(local.tiber(["list"]));

    local.insert_task_file(
        "backlog",
        "9999-local-sync-history",
        "---\ntitle: Local sync history\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    assert_success(remote.tiber(["create", "Remote advanced task"]));
    assert_success(remote.tiber(["sync"]));

    assert_success(local.tiber(["sync"]));

    let local_history = local.git_output([
        "log",
        "-1",
        "--format=%s",
        "refs/heads/tasks",
        "--",
        "backlog/9999-local-sync-history.md",
    ]);
    assert_success_ref(&local_history);
    assert_eq!(
        String::from_utf8(local_history.stdout).expect("git log output should be utf8"),
        "Insert test task\n"
    );
}

#[test]
fn conflict_resolution_preserves_unrelated_local_task_history_for_metadata() {
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
    assert_success(remote.tiber(["create", "Conflicted dated task"]));
    let conflict_stem = task_stem(&remote, "backlog", "conflicted-dated-task");
    assert_success(remote.tiber(["sync"]));
    assert_success(local.tiber(["list"]));

    local.insert_task_file(
        "backlog",
        "9999-unrelated-local-history",
        "---\ntitle: Unrelated local history\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    remote.insert_task_file(
        "backlog",
        &conflict_stem,
        "---\ntitle: Remote selected conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    remote.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);
    local.insert_task_file(
        "backlog",
        &conflict_stem,
        "---\ntitle: Local rejected conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    let setup_conflict = local.tiber(["sync"]);
    assert!(
        !setup_conflict.status.success(),
        "setup should produce a conflict"
    );

    assert_success(local.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{conflict_stem}.md"),
        "--remote",
    ]));

    let local_history = local.git_output([
        "log",
        "-1",
        "--format=%s",
        "refs/heads/tasks",
        "--",
        "backlog/9999-unrelated-local-history.md",
    ]);
    assert_success_ref(&local_history);
    assert_eq!(
        String::from_utf8(local_history.stdout).expect("git log output should be utf8"),
        "Insert test task\n"
    );
}

#[test]
fn metadata_rejects_stale_origin_tasks_when_remote_branch_disappears() {
    let first_origin = TempRepo::new();
    first_origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(first_origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    first_origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let writer = clone_repo(&first_origin);
    let reader = clone_repo(&first_origin);
    assert_success(writer.tiber(["init"]));
    assert_success(writer.tiber_with_env(
        ["create", "Stale remote task"],
        [
            ("GIT_AUTHOR_DATE", "2024-05-06T07:08:09Z"),
            ("GIT_COMMITTER_DATE", "2024-05-06T07:08:09Z"),
        ],
    ));
    let stem = task_stem(&writer, "backlog", "stale-remote-task");
    assert_success(writer.tiber(["sync"]));
    assert_success(reader.tiber(["metadata", "stale-remote-task"]));

    let second_origin = TempRepo::new();
    second_origin.git(["init", "--bare"]);
    let second_seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(second_origin.path())
            .current_dir(second_seed.path())
            .output()
            .expect("add replacement origin remote"),
    );
    second_seed.git(["push", "origin", "main"]);
    second_origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);
    assert_success(reader.git_output([
        "remote",
        "set-url",
        "origin",
        second_origin.path().to_str().unwrap(),
    ]));

    let metadata = reader.tiber(["metadata", &stem]);

    assert!(
        !metadata.status.success(),
        "metadata should hard-stop after origin loses a previously tracked tasks branch"
    );
    let stderr = String::from_utf8(metadata.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("tasks_remote_rewritten"));
    assert!(stderr.contains("do not force-push or overwrite shared task state"));
    let stale_ref = reader.git_output(["show-ref", "--verify", "refs/remotes/origin/tasks"]);
    assert!(
        stale_ref.status.success(),
        "read sync should preserve stale origin/tasks for coordinated inspection"
    );
}

fn clone_repo(origin: &TempRepo) -> TempRepo {
    let clone = TempRepo::new();
    assert_success(
        Command::new("git")
            .args(["clone"])
            .arg(origin.path())
            .arg(clone.path())
            .output()
            .expect("clone repo"),
    );
    clone.git(["config", "user.email", "tiber@example.test"]);
    clone.git(["config", "user.name", "Tiber Test"]);
    clone.git(["config", "commit.gpgsign", "false"]);
    clone
}
