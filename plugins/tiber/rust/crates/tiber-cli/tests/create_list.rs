mod support;

use std::fs;
use support::{assert_success, assert_success_ref, task_stem, TempRepo};

#[test]
fn create_stores_course_shaped_task_in_backlog_and_list_prints_ordered_summary() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));

    let create = repo.tiber(["create", "Write tiber docs"]);

    assert_success_ref(&create);
    let stem = task_stem(&repo, "backlog", "write-tiber-docs");
    assert_eq!(
        String::from_utf8(create.stdout).expect("create output should be utf8"),
        format!("created {stem}\n")
    );
    let file_name = format!("{stem}.md");
    assert!(file_name.ends_with("-write-tiber-docs.md"));
    let (date, rest) = stem
        .split_once('-')
        .expect("task stem should contain date and random code");
    let (code, nickname) = rest
        .split_once('-')
        .expect("task stem should contain random code and nickname");
    assert_eq!(date.len(), 8, "task id date should be YYYYMMDD");
    assert!(date.chars().all(|character| character.is_ascii_digit()));
    assert_eq!(code.len(), 4, "task id random code should be four chars");
    assert!(code
        .chars()
        .all(|character| "abcdefghijkmnpqrstuvwxyz23456789".contains(character)));
    assert_eq!(nickname, "write-tiber-docs");

    let task = repo.task_file("backlog", &stem);
    assert!(task.starts_with(
        "---\ntitle: Write tiber docs\nblocked_by: []\nblocks: []\ntags: []\npr_mr_url: \npr_mr_status: \n---\n"
    ));
    assert!(task.contains("## Summary\n\n"));
    assert!(task.contains("## Context / Why\n\n"));
    assert!(task.contains("## Acceptance criteria\n\n"));
    assert!(task.contains("## Subtasks\n\n"));
    assert!(task.contains("## Notes / Log\n"));

    assert_eq!(repo.order_file(), format!("{stem}\n"));

    let list = repo.tiber(["list"]);

    assert_success_ref(&list);
    assert_eq!(
        String::from_utf8(list.stdout).expect("list output should be utf8"),
        format!("{stem}\tWrite tiber docs\n")
    );
}

#[test]
fn create_refuses_when_configured_backlog_capacity_is_full() {
    let repo = TempRepo::initialized();
    fs::write(
        repo.path().join(".tiber.toml"),
        "[backlog]\nmax_queued = 1\n",
    )
    .expect("write tiber config");
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Keep this work"]));

    let create = repo.tiber(["create", "Overflow work"]);

    assert!(!create.status.success(), "create should refuse overflow");
    let stderr = String::from_utf8(create.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("backlog_capacity_exceeded"),
        "stderr should identify the refusal: {stderr}"
    );
    assert!(
        stderr.contains("queued=1") && stderr.contains("max_queued=1"),
        "stderr should report the current count and limit: {stderr}"
    );
    assert!(
        stderr.contains("replace") && stderr.contains("combine") && stderr.contains("reject"),
        "stderr should explain the available admission decisions: {stderr}"
    );
    assert!(
        !repo
            .git_output(["ls-tree", "-r", "--name-only", "tasks"])
            .stdout
            .windows(b"overflow-work".len())
            .any(|window| window == b"overflow-work"),
        "refused work should not be stored"
    );
}

#[test]
fn backlog_capacity_is_unlimited_when_project_config_is_absent() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));

    for title in ["First work", "Second work", "Third work"] {
        assert_success(repo.tiber(["create", title]));
    }

    let listing = repo.git_output(["ls-tree", "-r", "--name-only", "tasks", "backlog"]);
    assert_success_ref(&listing);
    assert_eq!(
        String::from_utf8(listing.stdout)
            .expect("task listing should be utf8")
            .lines()
            .filter(|path| path.ends_with(".md"))
            .count(),
        3
    );
}

#[test]
fn active_ticket_does_not_count_toward_backlog_capacity() {
    let repo = TempRepo::initialized();
    fs::write(
        repo.path().join(".tiber.toml"),
        "[backlog]\nmax_queued = 1\n",
    )
    .expect("write tiber config");
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Active work"]));
    assert_success(repo.tiber(["transition", "active-work", "in-progress"]));

    assert_success(repo.tiber(["create", "Queued work"]));

    task_stem(&repo, "in-progress", "active-work");
    task_stem(&repo, "backlog", "queued-work");
}

#[test]
fn malformed_project_config_fails_closed_before_task_creation() {
    let repo = TempRepo::initialized();
    fs::write(
        repo.path().join(".tiber.toml"),
        "[backlog]\nmax_queued = \"many\"\n",
    )
    .expect("write malformed tiber config");
    assert_success(repo.tiber(["init"]));

    let create = repo.tiber(["create", "Unsafe admission"]);

    assert!(!create.status.success(), "malformed config should refuse");
    let stderr = String::from_utf8(create.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("config_invalid") && stderr.contains(".tiber.toml"),
        "error should identify the configuration recovery surface: {stderr}"
    );
    let listing = repo.git_output(["ls-tree", "-r", "--name-only", "tasks", "backlog"]);
    assert_success_ref(&listing);
    assert!(
        !String::from_utf8(listing.stdout)
            .expect("task listing should be utf8")
            .contains("unsafe-admission"),
        "invalid configuration must not admit work"
    );
}

#[test]
fn create_failure_after_local_task_creation_reports_created_ref_for_recovery() {
    let (origin, hook_path) = TempRepo::bare_with_rejecting_hook();
    let repo = TempRepo::initialized();
    repo.git([
        "remote",
        "add",
        "origin",
        origin.path().to_str().expect("origin path should be utf8"),
    ]);
    assert_success(repo.tiber(["init"]));

    let create = repo.tiber(["create", "Release smoke"]);

    assert!(
        !create.status.success(),
        "create should surface sync failure"
    );
    let stderr = String::from_utf8(create.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("tiber.create_sync_failed created="),
        "stderr should include partial-success error with created ref: {stderr}"
    );
    assert!(
        stderr.contains("-release-smoke"),
        "stderr should include the created task nickname: {stderr}"
    );
    assert!(
        stderr.contains("run tiber sync after resolving the sync error"),
        "stderr should include recovery guidance: {stderr}"
    );
    assert!(
        stderr.contains("stderr_redacted=true"),
        "stderr should report redaction instead of raw sync output: {stderr}"
    );
    assert!(
        stderr.contains("args_redacted=true"),
        "stderr should report redacted sync command arguments: {stderr}"
    );
    assert!(
        !stderr.contains("secret@example.invalid"),
        "stderr should not leak token-bearing remote details: {stderr}"
    );
    assert!(
        !stderr.contains("private/repo.git"),
        "stderr should not leak private remote paths: {stderr}"
    );
    assert!(
        !stderr.contains(repo.path().to_str().expect("repo path should be utf8")),
        "stderr should not leak local repository paths: {stderr}"
    );
    let stem = task_stem(&repo, "backlog", "release-smoke");
    assert!(
        stderr.contains(&stem),
        "stderr should include the exact locally created task ref {stem}: {stderr}"
    );

    fs::remove_file(&hook_path).expect("remove rejecting hook");
    assert_success(repo.tiber(["sync"]));

    let remote_listing = origin.git_output(["ls-tree", "-r", "--name-only", "tasks"]);
    assert_success_ref(&remote_listing);
    assert!(
        String::from_utf8(remote_listing.stdout)
            .expect("remote task listing should be utf8")
            .contains(&format!("backlog/{stem}.md")),
        "tiber sync should recover the locally created task to origin/tasks"
    );
}

#[test]
fn create_failure_before_local_task_commit_does_not_report_unrecoverable_ref() {
    let repo = TempRepo::initialized();
    let missing_origin = repo
        .path()
        .join("private")
        .join("user-secret@example.invalid")
        .join("missing-origin.git");
    repo.git([
        "remote",
        "add",
        "origin",
        missing_origin
            .to_str()
            .expect("missing origin path should be utf8"),
    ]);
    assert_success(repo.tiber(["init"]));

    let create = repo.tiber(["create", "Lost before sync"]);

    assert!(
        !create.status.success(),
        "create should surface sync failure"
    );
    let stderr = String::from_utf8(create.stderr).expect("stderr should be utf8");
    assert!(
        !stderr.contains("tiber.create_sync_failed created="),
        "stderr should not report a recoverable created ref when refs/heads/tasks was not updated: {stderr}"
    );
    assert!(
        !stderr.contains("-lost-before-sync"),
        "stderr should not include an unrecoverable task nickname: {stderr}"
    );
    assert!(
        stderr.contains("stderr_redacted=true"),
        "stderr should report redaction instead of raw sync output: {stderr}"
    );
    assert!(
        stderr.contains("args_redacted=true"),
        "stderr should report redacted sync command arguments: {stderr}"
    );
    assert!(
        !stderr.contains("secret@example.invalid"),
        "stderr should not leak token-bearing remote details: {stderr}"
    );
    assert!(
        !stderr.contains("private/missing-origin.git"),
        "stderr should not leak private remote paths: {stderr}"
    );
    let listing = repo.git_output(["ls-tree", "-r", "--name-only", "tasks"]);
    assert_success_ref(&listing);
    assert!(
        !String::from_utf8(listing.stdout)
            .expect("tasks listing should be utf8")
            .contains("-lost-before-sync"),
        "task should not be present in refs/heads/tasks when sync failed before local ref update"
    );
}

#[test]
fn show_resolves_by_id_nickname_or_full_stem_without_storage_paths() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Write tiber docs"]));
    let stem = task_stem(&repo, "backlog", "write-tiber-docs");
    let id = stem
        .split_once("-write-tiber-docs")
        .map(|(id, _)| id)
        .expect("stem includes nickname")
        .to_string();

    for task_ref in [id.as_str(), "write-tiber-docs", stem.as_str()] {
        let show = repo.tiber(["show", task_ref]);

        assert_success_ref(&show);
        assert!(
            String::from_utf8(show.stdout)
                .expect("show output should be utf8")
                .contains("title: Write tiber docs"),
            "show should print task for ref {task_ref}"
        );
    }
}
