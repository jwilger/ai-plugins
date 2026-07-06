mod support;

use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn subtask_add_check_and_uncheck_update_task_checklist() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Ship feature"]));

    let add = repo.tiber(["subtask", "add", "todo/ship-feature.md", "Write tests"]);

    assert_success(add);
    let task = repo.tiber(["show", "todo/ship-feature.md"]);
    assert_success_ref(&task);
    assert_eq!(
        String::from_utf8(task.stdout).expect("task should be utf8"),
        "# Ship feature\n\n## Subtasks\n- [ ] Write tests\n"
    );

    assert_success(repo.tiber(["subtask", "check", "todo/ship-feature.md", "1"]));
    let task = repo.tiber(["show", "todo/ship-feature.md"]);
    assert_success_ref(&task);
    assert_eq!(
        String::from_utf8(task.stdout).expect("task should be utf8"),
        "# Ship feature\n\n## Subtasks\n- [x] Write tests\n"
    );

    assert_success(repo.tiber(["subtask", "uncheck", "todo/ship-feature.md", "1"]));
    let task = repo.tiber(["show", "todo/ship-feature.md"]);
    assert_success_ref(&task);
    assert_eq!(
        String::from_utf8(task.stdout).expect("task should be utf8"),
        "# Ship feature\n\n## Subtasks\n- [ ] Write tests\n"
    );
}
