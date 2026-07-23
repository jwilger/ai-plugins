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
    assert!(stderr.contains(&format!("sync_conflict path=in-progress/{stem}.md")));
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
        "---\ntitle: Shared remote task\nblocked_by: []\nblocks: []\ntags: []\npr_mr_url: \npr_mr_status: \n---\n\n## Summary\n\n## Context / Why\n\n## Acceptance criteria\n\n## Subtasks\n\n## Notes / Log\n"
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
fn create_revalidates_capacity_across_multiple_concurrent_remote_admissions() {
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

    let racer = clone_repo(&origin);
    racer.git(["remote", "remove", "origin"]);
    assert_success(racer.tiber(["init"]));
    assert_success(racer.tiber(["create", "First concurrent admission"]));
    racer.git(["branch", "capacity-racer-one", "tasks"]);
    assert_success(racer.tiber(["create", "Second concurrent admission"]));
    racer.git([
        "remote",
        "add",
        "origin",
        origin.path().to_str().expect("origin path utf8"),
    ]);
    racer.git([
        "push",
        "origin",
        "capacity-racer-one:refs/heads/capacity-racer-one",
    ]);
    racer.git(["push", "origin", "tasks:refs/heads/capacity-racer-two"]);

    let candidate = clone_repo(&origin);
    fs::write(
        candidate.path().join(".tiber.toml"),
        "[backlog]\nmax_queued = 2\n",
    )
    .expect("write tiber config");
    assert_success(candidate.tiber(["init"]));
    install_remote_admission_race(&origin);

    let create = candidate.tiber(["create", "Losing admission"]);

    assert!(
        !create.status.success(),
        "candidate should lose the concurrent admission race"
    );
    let stderr = String::from_utf8(create.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("backlog_capacity_exceeded"),
        "retries should return an actionable capacity refusal: {stderr}"
    );
    assert!(
        !stderr.contains("created="),
        "a rejected admission must not be reported as locally created: {stderr}"
    );
    let candidate_tree = candidate.git_output(["ls-tree", "-r", "--name-only", "tasks", "backlog"]);
    assert_success_ref(&candidate_tree);
    assert!(
        !String::from_utf8(candidate_tree.stdout)
            .expect("candidate tree output should be utf8")
            .contains("losing-admission"),
        "rejected admission must be rolled back from the local tasks ref"
    );
    assert_success(candidate.tiber(["sync"]));
    let verification = clone_repo(&origin);
    let tree = verification.git_output(["ls-tree", "-r", "--name-only", "origin/tasks", "backlog"]);
    assert_success_ref(&tree);
    let tree = String::from_utf8(tree.stdout).expect("tree output should be utf8");
    assert_eq!(
        tree.lines().filter(|path| path.ends_with(".md")).count(),
        2,
        "remote backlog must retain only the two winning admissions: {tree}"
    );
}

#[test]
fn create_capacity_rejection_preserves_preexisting_local_only_tasks() {
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
    assert_success(remote.tiber(["create", "Remote capacity winner"]));

    let candidate = clone_repo(&origin);
    candidate.git(["remote", "remove", "origin"]);
    assert_success(candidate.tiber(["init"]));
    assert_success(candidate.tiber(["create", "Preserved local task"]));
    let preserved_stem = task_stem(&candidate, "backlog", "preserved-local-task");
    candidate.git([
        "remote",
        "add",
        "origin",
        origin.path().to_str().expect("origin path utf8"),
    ]);
    fs::write(
        candidate.path().join(".tiber.toml"),
        "[backlog]\nmax_queued = 2\n",
    )
    .expect("write tiber config");

    let create = candidate.tiber(["create", "Rejected local admission"]);

    assert!(
        !create.status.success(),
        "remote winner should fill capacity during admission"
    );
    let stderr = String::from_utf8(create.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("backlog_capacity_exceeded"), "{stderr}");
    let candidate_tree = candidate.git_output(["ls-tree", "-r", "--name-only", "tasks", "backlog"]);
    assert_success_ref(&candidate_tree);
    let candidate_tree =
        String::from_utf8(candidate_tree.stdout).expect("candidate tree should be utf8");
    assert!(
        candidate_tree.contains(&format!("backlog/{preserved_stem}.md")),
        "capacity rollback must preserve unrelated local-only task state: {candidate_tree}"
    );
    assert!(
        !candidate_tree.contains("rejected-local-admission"),
        "capacity rollback must remove only the rejected admission: {candidate_tree}"
    );
}

#[test]
fn transition_capacity_rejection_restores_the_preexisting_local_status() {
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
    assert_success(remote.tiber(["create", "Remote capacity winner"]));

    let candidate = clone_repo(&origin);
    candidate.git(["remote", "remove", "origin"]);
    assert_success(candidate.tiber(["init"]));
    assert_success(candidate.tiber(["create", "Preserved completed task"]));
    let preserved_stem = task_stem(&candidate, "backlog", "preserved-completed-task");
    assert_success(candidate.tiber(["transition", &preserved_stem, "done"]));
    candidate.git([
        "remote",
        "add",
        "origin",
        origin.path().to_str().expect("origin path utf8"),
    ]);
    fs::write(
        candidate.path().join(".tiber.toml"),
        "[backlog]\nmax_queued = 1\n",
    )
    .expect("write tiber config");

    let transition = candidate.tiber(["transition", &preserved_stem, "backlog"]);

    assert!(
        !transition.status.success(),
        "remote winner should fill capacity during transition"
    );
    let stderr = String::from_utf8(transition.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("backlog_capacity_exceeded"), "{stderr}");
    let completed =
        candidate.git_output(["cat-file", "-e", &format!("tasks:done/{preserved_stem}.md")]);
    assert_success_ref(&completed);
    let reopened = candidate.git_output([
        "cat-file",
        "-e",
        &format!("tasks:backlog/{preserved_stem}.md"),
    ]);
    assert!(
        !reopened.status.success(),
        "capacity rollback must restore the pre-admission status"
    );
}

#[test]
fn create_rolls_back_local_admission_when_concurrent_retry_budget_is_exhausted() {
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

    let candidate = clone_repo(&origin);
    fs::write(
        candidate.path().join(".tiber.toml"),
        "[backlog]\nmax_queued = 1\n",
    )
    .expect("write tiber config");
    assert_success(candidate.tiber(["init"]));
    install_always_racing_hook(&origin);

    let create = candidate.tiber(["create", "Exhausted admission"]);

    assert!(
        !create.status.success(),
        "create should stop after the retry budget"
    );
    let stderr = String::from_utf8(create.stderr).expect("stderr should be utf8");
    assert!(
        !stderr.contains("created="),
        "exhausted admission must not be reported as locally created: {stderr}"
    );
    let candidate_tree = candidate.git_output(["ls-tree", "-r", "--name-only", "tasks", "backlog"]);
    assert_success_ref(&candidate_tree);
    assert!(
        !String::from_utf8(candidate_tree.stdout)
            .expect("candidate tree output should be utf8")
            .contains("exhausted-admission"),
        "exhausted admission must be rolled back from the local tasks ref"
    );

    fs::remove_file(origin.path().join("hooks").join("pre-receive")).expect("remove racing hook");
    assert_success(candidate.tiber(["sync"]));
    let remote_tree =
        candidate.git_output(["ls-tree", "-r", "--name-only", "origin/tasks", "backlog"]);
    assert_success_ref(&remote_tree);
    assert!(
        !String::from_utf8(remote_tree.stdout)
            .expect("remote tree output should be utf8")
            .contains("exhausted-admission"),
        "later sync must not propagate the rejected admission"
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

fn install_remote_admission_race(origin: &TempRepo) {
    let hook = origin.path().join("hooks").join("pre-receive");
    let counter = origin.path().join("capacity-race-count");
    fs::write(
        &hook,
        format!(
            "#!/usr/bin/env bash\nset -euo pipefail\ncount=0\nif [[ -e '{}' ]]; then\n  count=$(< '{}')\nfi\nif [[ \"$count\" -eq 0 ]]; then\n  echo 1 > '{}'\n  env -u GIT_QUARANTINE_PATH git update-ref refs/heads/tasks refs/heads/capacity-racer-one\n  echo 'non-fast-forward first concurrent admission' >&2\n  exit 1\nfi\nif [[ \"$count\" -eq 1 ]]; then\n  echo 2 > '{}'\n  env -u GIT_QUARANTINE_PATH git update-ref refs/heads/tasks refs/heads/capacity-racer-two\n  echo 'non-fast-forward second concurrent admission' >&2\n  exit 1\nfi\n",
            counter.display(),
            counter.display(),
            counter.display(),
            counter.display()
        ),
    )
    .expect("write remote admission race hook");
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&hook).expect("hook metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&hook, permissions).expect("make race hook executable");
    }
}

fn install_always_racing_hook(origin: &TempRepo) {
    let hook = origin.path().join("hooks").join("pre-receive");
    fs::write(
        &hook,
        "#!/usr/bin/env bash\nset -euo pipefail\necho 'non-fast-forward concurrent admission' >&2\nexit 1\n",
    )
    .expect("write persistent race hook");
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&hook).expect("hook metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&hook, permissions).expect("make race hook executable");
    }
}
