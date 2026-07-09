mod support;

use support::{assert_success, assert_success_ref, task_stem, TempRepo};

#[test]
fn next_show_transition_and_prioritize_follow_order_md() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Write docs"]));
    assert_success(repo.tiber(["create", "Review docs"]));
    let write_docs = task_stem(&repo, "backlog", "write-docs");
    let review_docs = task_stem(&repo, "backlog", "review-docs");

    let next = repo.tiber(["next"]);
    assert_success_ref(&next);
    assert_eq!(
        String::from_utf8(next.stdout).expect("next output should be utf8"),
        format!("{write_docs}\tWrite docs\n")
    );

    let show = repo.tiber(["show", "write-docs"]);
    assert_success_ref(&show);
    assert!(String::from_utf8(show.stdout)
        .expect("show output should be utf8")
        .contains("title: Write docs"));

    assert_success(repo.tiber(["transition", "write-docs", "in-progress"]));
    assert_success_ref(&repo.git_output([
        "cat-file",
        "-e",
        &format!("tasks:in-progress/{write_docs}.md"),
    ]));
    let in_progress = repo.task_file("in-progress", &write_docs);
    assert!(in_progress.contains("claim:\n"));
    assert!(in_progress.contains("  host: "));
    assert!(in_progress.contains("  session: "));
    assert!(!repo
        .git_output(["cat-file", "-e", &format!("tasks:backlog/{write_docs}.md")])
        .status
        .success());

    assert_success(repo.tiber(["prioritize", "review-docs", "--before", "write-docs"]));

    assert_eq!(repo.order_file(), format!("{review_docs}\n{write_docs}\n"));
}

#[test]
fn transition_releases_claim_when_leaving_in_progress() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Claim lifecycle"]));
    let task = task_stem(&repo, "backlog", "claim-lifecycle");

    assert_success(repo.tiber_with_env(
        ["transition", "claim-lifecycle", "in-progress"],
        [
            ("TIBER_CLAIM_HOST", "test-host"),
            ("TIBER_CLAIM_SESSION", "test-session"),
        ],
    ));
    let in_progress = repo.task_file("in-progress", &task);
    assert!(in_progress.contains("claim:\n  host: test-host\n  session: test-session\n"));

    assert_success(repo.tiber(["transition", "claim-lifecycle", "done"]));
    let done = repo.task_file("done", &task);
    assert!(!done.contains("claim:"));
    assert!(!done.contains("test-session"));
}

#[test]
fn next_skips_tasks_blocked_by_open_dependencies() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Blocked task"]));
    assert_success(repo.tiber(["create", "Dependency task"]));
    let blocked = task_stem(&repo, "backlog", "blocked-task");
    let dependency = task_stem(&repo, "backlog", "dependency-task");
    assert_success(repo.tiber(["link", "dependency-task", "blocks", "blocked-task"]));
    assert_success(repo.tiber(["prioritize", "blocked-task", "--before", "dependency-task"]));

    let next = repo.tiber(["next"]);

    assert_success_ref(&next);
    assert_eq!(
        String::from_utf8(next.stdout).expect("next output should be utf8"),
        format!("{dependency}\tDependency task\n")
    );

    assert_success(repo.tiber(["transition", "dependency-task", "done"]));
    let next_after_dependency_done = repo.tiber(["next"]);

    assert_success_ref(&next_after_dependency_done);
    assert_eq!(
        String::from_utf8(next_after_dependency_done.stdout).expect("next output should be utf8"),
        format!("{blocked}\tBlocked task\n")
    );
}

#[test]
fn next_skips_agent_unresolvable_blocked_tasks_until_reason_is_cleared() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Externally blocked task"]));
    assert_success(repo.tiber(["create", "Available task"]));
    let blocked = task_stem(&repo, "backlog", "externally-blocked-task");
    let available = task_stem(&repo, "backlog", "available-task");
    assert_success(repo.tiber([
        "update",
        "externally-blocked-task",
        "--agent-blocked-reason",
        "Waiting on account access that the agent cannot grant.",
    ]));

    let next = repo.tiber(["next"]);

    assert_success_ref(&next);
    assert_eq!(
        String::from_utf8(next.stdout).expect("next output should be utf8"),
        format!("{available}\tAvailable task\n")
    );

    assert_success(repo.tiber([
        "update",
        "externally-blocked-task",
        "--agent-blocked-reason",
        "",
    ]));
    assert_success(repo.tiber([
        "prioritize",
        "externally-blocked-task",
        "--before",
        "available-task",
    ]));
    let next_after_clear = repo.tiber(["next"]);

    assert_success_ref(&next_after_clear);
    assert_eq!(
        String::from_utf8(next_after_clear.stdout).expect("next output should be utf8"),
        format!("{blocked}\tExternally blocked task\n")
    );
}

#[test]
fn next_reports_when_all_open_tasks_are_agent_unresolvable_blocked() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Externally blocked task"]));
    let blocked = task_stem(&repo, "backlog", "externally-blocked-task");
    assert_success(repo.tiber([
        "update",
        "externally-blocked-task",
        "--agent-blocked-reason",
        "Waiting on account access that the agent cannot grant.",
    ]));

    let next = repo.tiber(["next"]);

    assert_success_ref(&next);
    assert_eq!(
        String::from_utf8(next.stdout).expect("next stdout should be utf8"),
        ""
    );
    let stderr = String::from_utf8(next.stderr).expect("next stderr should be utf8");
    assert!(stderr.contains("no ready tasks; 1 task(s) have agent_blocked_reason"));
    assert!(stderr.contains(&blocked));
    assert!(stderr.contains("tiber update <ref> --agent-blocked-reason \"\""));
}

#[test]
fn next_does_not_report_agent_blocked_count_for_dependency_blocked_tasks() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Dependency blocked task"]));
    assert_success(repo.tiber(["create", "Open dependency"]));
    let open_dependency = task_stem(&repo, "backlog", "open-dependency");
    assert_success(repo.tiber([
        "update",
        "dependency-blocked-task",
        "--agent-blocked-reason",
        "Waiting on account access that the agent cannot grant.",
    ]));
    assert_success(repo.tiber([
        "link",
        "open-dependency",
        "blocks",
        "dependency-blocked-task",
    ]));
    assert_success(repo.tiber([
        "prioritize",
        "dependency-blocked-task",
        "--before",
        "open-dependency",
    ]));
    assert_success(repo.tiber([
        "update",
        "open-dependency",
        "--agent-blocked-reason",
        "Waiting on a user decision.",
    ]));

    let next = repo.tiber(["next"]);

    assert_success_ref(&next);
    assert_eq!(
        String::from_utf8(next.stdout).expect("next stdout should be utf8"),
        ""
    );
    let stderr = String::from_utf8(next.stderr).expect("next stderr should be utf8");
    assert!(stderr.contains("no ready tasks; 1 task(s) have agent_blocked_reason"));
    assert!(stderr.contains(&open_dependency));
    assert!(!stderr.contains("dependency-blocked-task"));
}

#[test]
fn task_refs_can_use_unique_filename_identity_and_report_ambiguity() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Write docs"]));
    assert_success(repo.tiber(["create", "Review docs"]));
    let write_docs = task_stem(&repo, "backlog", "write-docs");
    let review_docs = task_stem(&repo, "backlog", "review-docs");

    let show = repo.tiber(["show", "write-docs"]);
    assert_success_ref(&show);
    assert!(String::from_utf8(show.stdout)
        .expect("show output should be utf8")
        .contains("title: Write docs"));

    assert_success(repo.tiber(["transition", "write-docs", "in-progress"]));
    assert_success(repo.tiber(["prioritize", "review-docs", "--before", "write-docs"]));
    assert_eq!(repo.order_file(), format!("{review_docs}\n{write_docs}\n"));

    repo.insert_task_file(
        "done",
        "20260706-abcd-write-docs",
        "---\ntitle: Archived duplicate\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );

    let ambiguous = repo.tiber(["show", "write-docs"]);
    assert!(!ambiguous.status.success(), "ambiguous ref should fail");
    let stderr = String::from_utf8(ambiguous.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("ambiguous_task_ref ref=write-docs"));
    assert!(stderr.contains("in-progress/"));
    assert!(stderr.contains("done/"));
}
