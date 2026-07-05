mod support;

use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn subtask_add_check_and_uncheck_update_task_checklist() {
    let repo = TempRepo::initialized();
    assert_success(repo.taskbranch(["init"]));
    assert_success(repo.taskbranch(["create", "Ship feature"]));

    let add = repo.taskbranch(["subtask", "add", "todo/ship-feature.md", "Write tests"]);

    assert_success(add);
    let task = repo.taskbranch(["show", "todo/ship-feature.md"]);
    assert_success_ref(&task);
    assert_eq!(
        String::from_utf8(task.stdout).expect("task should be utf8"),
        "# Ship feature\n\n## Subtasks\n- [ ] Write tests\n"
    );

    assert_success(repo.taskbranch(["subtask", "check", "todo/ship-feature.md", "1"]));
    let task = repo.taskbranch(["show", "todo/ship-feature.md"]);
    assert_success_ref(&task);
    assert_eq!(
        String::from_utf8(task.stdout).expect("task should be utf8"),
        "# Ship feature\n\n## Subtasks\n- [x] Write tests\n"
    );

    assert_success(repo.taskbranch(["subtask", "uncheck", "todo/ship-feature.md", "1"]));
    let task = repo.taskbranch(["show", "todo/ship-feature.md"]);
    assert_success_ref(&task);
    assert_eq!(
        String::from_utf8(task.stdout).expect("task should be utf8"),
        "# Ship feature\n\n## Subtasks\n- [ ] Write tests\n"
    );
}
