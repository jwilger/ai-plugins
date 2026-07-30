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

#[test]
fn subtask_predecessor_list_trims_whitespace_and_ignores_empty_entries() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Normalize predecessors"]));
    assert_success(repo.tiber([
        "subtask",
        "add",
        "normalize-predecessors",
        "First dependency",
    ]));
    assert_success(repo.tiber([
        "subtask",
        "add",
        "normalize-predecessors",
        "Second dependency",
    ]));

    let add = repo.tiber([
        "subtask",
        "add",
        "normalize-predecessors",
        "Dependent task",
        "--after",
        "s1, s2, ,",
    ]);

    assert_success(add);
    let task = repo.tiber(["show", "normalize-predecessors"]);
    assert_success_ref(&task);
    assert!(String::from_utf8(task.stdout)
        .expect("task should be utf8")
        .contains("- [ ] (s3) Dependent task — after: s1, s2\n"));
}

#[test]
fn subtask_add_rejects_missing_and_self_predecessors_without_writing() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Validate predecessors"]));

    let self_cycle = repo.tiber([
        "subtask",
        "add",
        "validate-predecessors",
        "Invalid self cycle",
        "--after",
        "s1",
    ]);
    assert!(!self_cycle.status.success());
    assert!(String::from_utf8_lossy(&self_cycle.stderr).contains("subtask_self_dependency"));

    assert_success(repo.tiber([
        "subtask",
        "add",
        "validate-predecessors",
        "Valid first subtask",
    ]));
    let missing = repo.tiber([
        "subtask",
        "add",
        "validate-predecessors",
        "Missing predecessor",
        "--after",
        "s9",
    ]);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("subtask_predecessor_missing"));

    let task = repo.tiber(["show", "validate-predecessors"]);
    assert_success_ref(&task);
    let task = String::from_utf8(task.stdout).expect("task should be utf8");
    assert!(task.contains("(s1) Valid first subtask"));
    assert!(!task.contains("Invalid self cycle"));
    assert!(!task.contains("Missing predecessor"));

    let injected = repo.tiber([
        "subtask",
        "add",
        "validate-predecessors",
        "Injected — after: s999",
    ]);
    assert!(!injected.status.success());
    assert!(String::from_utf8_lossy(&injected.stderr).contains("subtask_title_invalid"));
}

#[test]
fn subtask_after_repairs_an_invalid_self_cycle_without_reusing_its_id() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Repair subtask"]));
    let task = support::task_stem(&repo, "backlog", "repair-subtask");
    let contents = repo.task_file("backlog", &task).replace(
        "## Subtasks\n",
        "## Subtasks\n\n- [ ] (s1) Invalid — after: s1\n- [ ] (s2) Also invalid — after: s2\n",
    );
    repo.insert_task_file("backlog", &task, &contents);

    assert_success(repo.tiber(["subtask", "after", "repair-subtask", "s1"]));
    assert_success(repo.tiber(["subtask", "after", "repair-subtask", "s2"]));
    assert_success(repo.tiber(["subtask", "add", "repair-subtask", "Replacement"]));
    let updated = repo.tiber(["show", "repair-subtask"]);
    assert_success_ref(&updated);
    let updated = String::from_utf8(updated.stdout).expect("task should be utf8");
    assert!(updated.contains("(s1) Invalid\n"));
    assert!(!updated.contains("(s1) Invalid — after:"));
    assert!(updated.contains("(s2) Also invalid\n"));
    assert!(!updated.contains("(s2) Also invalid — after:"));
    assert!(updated.contains("(s3) Replacement"));
}

#[test]
fn subtask_add_rejects_completing_a_cycle_in_a_legacy_graph() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Legacy graph"]));
    let task = support::task_stem(&repo, "backlog", "legacy-graph");
    let contents = repo.task_file("backlog", &task).replace(
        "## Subtasks\n",
        "## Subtasks\n\n- [ ] (s1) Legacy dangling edge — after: s2\n",
    );
    repo.insert_task_file("backlog", &task, &contents);

    let add = repo.tiber([
        "subtask",
        "add",
        "legacy-graph",
        "Would complete cycle",
        "--after",
        "s1",
    ]);
    assert!(!add.status.success());
    assert!(String::from_utf8_lossy(&add.stderr).contains("subtask_dependency_cycle"));
    assert!(!repo
        .task_file("backlog", &task)
        .contains("Would complete cycle"));
}

#[test]
fn subtask_check_only_updates_subtasks_section() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Scoped subtask check"]));
    assert_success(repo.tiber(["subtask", "add", "scoped-subtask-check", "Real subtask"]));
    let task = support::task_stem(&repo, "backlog", "scoped-subtask-check");
    let contents = repo.task_file("backlog", &task).replace(
        "## Acceptance criteria\n\n",
        "## Acceptance criteria\n\n- [ ] (s1) Acceptance item with matching marker\n\n",
    );
    repo.insert_task_file("backlog", &task, &contents);

    assert_success(repo.tiber(["subtask", "check", "scoped-subtask-check", "s1"]));

    let updated = repo.task_file("backlog", &task);
    assert!(updated.contains("## Acceptance criteria\n\n- [ ] (s1) Acceptance item"));
    assert!(updated.contains("## Subtasks\n\n- [x] (s1) Real subtask"));
}
