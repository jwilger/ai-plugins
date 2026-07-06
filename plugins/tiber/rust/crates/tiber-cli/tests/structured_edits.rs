mod support;

use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn update_edits_title_summary_context_and_tags_without_raw_markdown_paths() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Original title"]));

    let update = repo.tiber([
        "update",
        "original-title",
        "--title",
        "Updated title",
        "--summary",
        "The new summary.",
        "--context",
        "The new context.",
        "--tags",
        "alpha,beta",
    ]);

    assert_success(update);
    let show = repo.tiber(["show", "original-title"]);
    assert_success_ref(&show);
    let task = String::from_utf8(show.stdout).expect("show output should be utf8");
    assert!(task.contains("title: Updated title\n"));
    assert!(task.contains("tags: [alpha, beta]\n"));
    assert!(
        task.contains("## Summary\n\nThe new summary.\n\n## Context / Why\n\nThe new context.\n")
    );
}

#[test]
fn acceptance_add_check_uncheck_and_remove_edits_acceptance_section() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Acceptance task"]));

    assert_success(repo.tiber([
        "acceptance",
        "add",
        "acceptance-task",
        "First observable condition",
    ]));
    assert_success(repo.tiber(["acceptance", "check", "acceptance-task", "1"]));
    let checked = repo.tiber(["show", "acceptance-task"]);
    assert_success_ref(&checked);
    assert!(String::from_utf8(checked.stdout)
        .expect("checked task should be utf8")
        .contains("## Acceptance criteria\n\n- [x] First observable condition\n"));

    assert_success(repo.tiber(["acceptance", "uncheck", "acceptance-task", "1"]));
    assert_success(repo.tiber(["acceptance", "remove", "acceptance-task", "1"]));
    let removed = repo.tiber(["show", "acceptance-task"]);
    assert_success_ref(&removed);
    assert!(String::from_utf8(removed.stdout)
        .expect("removed task should be utf8")
        .contains("## Acceptance criteria\n\n## Subtasks\n"));
}

#[test]
fn note_add_appends_dated_note_to_notes_log() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Noted task"]));

    assert_success(repo.tiber(["note", "add", "noted-task", "Made progress."]));

    let show = repo.tiber(["show", "noted-task"]);
    assert_success_ref(&show);
    let task = String::from_utf8(show.stdout).expect("task should be utf8");
    assert!(task.contains("## Notes / Log\n\n- 20"));
    assert!(task.contains(": Made progress.\n"));
}
