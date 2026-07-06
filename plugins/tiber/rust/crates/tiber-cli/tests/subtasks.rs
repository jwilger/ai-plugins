mod support;

use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn subtask_add_check_and_uncheck_update_task_checklist() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Ship feature"]));

    let add = repo.tiber(["subtask", "add", "ship-feature", "Write tests"]);

    assert_success(add);
    let task = repo.tiber(["show", "ship-feature"]);
    assert_success_ref(&task);
    assert!(String::from_utf8(task.stdout)
        .expect("task should be utf8")
        .contains("## Subtasks\n\n- [ ] (s1) Write tests\n"));

    assert_success(repo.tiber(["subtask", "add", "ship-feature", "Wire UI", "--after", "s1"]));
    let task = repo.tiber(["show", "ship-feature"]);
    assert_success_ref(&task);
    assert!(String::from_utf8(task.stdout)
        .expect("task should be utf8")
        .contains("- [ ] (s2) Wire UI — after: s1\n"));

    assert_success(repo.tiber(["subtask", "check", "ship-feature", "s1"]));
    let task = repo.tiber(["show", "ship-feature"]);
    assert_success_ref(&task);
    assert!(String::from_utf8(task.stdout)
        .expect("task should be utf8")
        .contains("## Subtasks\n\n- [x] (s1) Write tests\n"));

    assert_success(repo.tiber(["subtask", "uncheck", "ship-feature", "s1"]));
    let task = repo.tiber(["show", "ship-feature"]);
    assert_success_ref(&task);
    assert!(String::from_utf8(task.stdout)
        .expect("task should be utf8")
        .contains("## Subtasks\n\n- [ ] (s1) Write tests\n"));
}
