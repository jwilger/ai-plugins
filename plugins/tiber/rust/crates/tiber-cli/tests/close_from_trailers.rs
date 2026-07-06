mod support;

use std::fs;

use support::{assert_success, assert_success_ref, task_stem, TempRepo};

#[test]
fn close_from_trailers_moves_closed_tasks_to_done() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Fix bug"]));
    let fix_bug = task_stem(&repo, "backlog", "fix-bug");
    fs::write(repo.path().join("fix.txt"), "fixed\n").expect("write fix file");
    repo.git(["add", "fix.txt"]);
    repo.git(["commit", "-m", "Fix bug\n\nCloses: fix-bug"]);

    let close = repo.tiber(["close-from-trailers"]);

    assert_success(close);
    assert_success_ref(&repo.git_output(["cat-file", "-e", &format!("tasks:done/{fix_bug}.md")]));
    assert!(!repo
        .git_output(["cat-file", "-e", &format!("tasks:backlog/{fix_bug}.md")])
        .status
        .success());
    assert_eq!(repo.order_file(), "");
}
