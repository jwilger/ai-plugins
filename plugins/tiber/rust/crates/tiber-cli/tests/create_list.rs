mod support;

use std::fs;

use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn create_stores_task_in_todo_and_list_prints_ordered_summary() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));

    let create = repo.tiber(["create", "Write tiber docs"]);

    assert_success(create);
    let task_file = repo
        .path()
        .join(".tasks")
        .join("todo")
        .join("write-tiber-docs.md");
    let task = fs::read_to_string(task_file).expect("task file should be written");
    assert!(task.contains("# Write tiber docs"));

    let order = fs::read_to_string(repo.path().join(".tasks").join("order.md"))
        .expect("order file should be written");
    assert_eq!(order, "todo/write-tiber-docs.md\n");

    let list = repo.tiber(["list"]);

    assert_success_ref(&list);
    assert_eq!(
        String::from_utf8(list.stdout).expect("list output should be utf8"),
        "todo/write-tiber-docs.md\tWrite tiber docs\n"
    );
}
