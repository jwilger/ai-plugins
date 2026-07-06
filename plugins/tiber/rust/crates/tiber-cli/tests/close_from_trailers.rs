mod support;

use std::fs;

use support::{assert_success, TempRepo};

#[test]
fn close_from_trailers_moves_closed_tasks_to_done() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Fix bug"]));
    fs::write(repo.path().join("fix.txt"), "fixed\n").expect("write fix file");
    repo.git(["add", "fix.txt"]);
    repo.git(["commit", "-m", "Fix bug\n\nCloses: todo/fix-bug.md"]);

    let close = repo.tiber(["close-from-trailers"]);

    assert_success(close);
    assert!(repo
        .path()
        .join(".tasks")
        .join("done")
        .join("fix-bug.md")
        .exists());
    assert!(!repo
        .path()
        .join(".tasks")
        .join("todo")
        .join("fix-bug.md")
        .exists());
    let order = fs::read_to_string(repo.path().join(".tasks").join("order.md"))
        .expect("order should be readable");
    assert_eq!(order, "done/fix-bug.md\n");
}
