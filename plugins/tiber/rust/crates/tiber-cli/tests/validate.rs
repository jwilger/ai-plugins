mod support;

use std::fs;

use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn validate_fix_repairs_missing_reciprocal_links_and_order_entries() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Build API"]));
    assert_success(repo.tiber(["create", "Build UI"]));

    fs::write(
        repo.path().join(".tasks").join("todo").join("build-api.md"),
        "# Build API\n\n## Blocks\n- todo/build-ui.md\n",
    )
    .expect("write one-sided dependency");
    fs::write(
        repo.path().join(".tasks").join("order.md"),
        "todo/build-api.md\ntodo/stale.md\n",
    )
    .expect("write stale order entry");

    let validate = repo.tiber(["validate", "--fix"]);

    assert_success_ref(&validate);
    assert_eq!(
        String::from_utf8(validate.stdout).expect("validate output should be utf8"),
        "fixed reciprocal-link todo/build-ui.md blocked-by todo/build-api.md\nfixed order stale todo/stale.md\nfixed order missing todo/build-ui.md\n"
    );

    let ui = repo.tiber(["show", "todo/build-ui.md"]);
    assert_success_ref(&ui);
    assert_eq!(
        String::from_utf8(ui.stdout).expect("ui task should be utf8"),
        "# Build UI\n\n## Blocked By\n- todo/build-api.md\n"
    );

    let order = fs::read_to_string(repo.path().join(".tasks").join("order.md"))
        .expect("order should be readable");
    assert_eq!(order, "todo/build-api.md\ntodo/build-ui.md\n");
}

#[test]
fn validate_fix_reports_dangling_dependency_refs_without_removing_them() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Dangling dependencies"]));

    let task_path = repo
        .path()
        .join(".tasks")
        .join("todo")
        .join("dangling-dependencies.md");
    fs::write(
        &task_path,
        "# Dangling dependencies\n\n## Blocks\n- todo/missing-blocked.md\n\n## Blocked By\n- todo/missing-blocker.md\n",
    )
    .expect("write dangling dependencies");

    let validate = repo.tiber(["validate", "--fix"]);

    assert_success_ref(&validate);
    assert_eq!(
        String::from_utf8(validate.stdout).expect("validate output should be utf8"),
        "dangling link todo/dangling-dependencies.md blocks todo/missing-blocked.md\ndangling link todo/dangling-dependencies.md blocked-by todo/missing-blocker.md\n"
    );
    assert_eq!(
        fs::read_to_string(task_path).expect("task should be readable"),
        "# Dangling dependencies\n\n## Blocks\n- todo/missing-blocked.md\n\n## Blocked By\n- todo/missing-blocker.md\n"
    );
}

#[test]
fn validate_fix_reports_dependency_cycles_without_rewriting_them() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Cycle A"]));
    assert_success(repo.tiber(["create", "Cycle B"]));

    let cycle_a = repo.path().join(".tasks").join("todo").join("cycle-a.md");
    let cycle_b = repo.path().join(".tasks").join("todo").join("cycle-b.md");
    let cycle_a_contents =
        "# Cycle A\n\n## Blocks\n- todo/cycle-b.md\n\n## Blocked By\n- todo/cycle-b.md\n";
    let cycle_b_contents =
        "# Cycle B\n\n## Blocks\n- todo/cycle-a.md\n\n## Blocked By\n- todo/cycle-a.md\n";
    fs::write(&cycle_a, cycle_a_contents).expect("write cycle a");
    fs::write(&cycle_b, cycle_b_contents).expect("write cycle b");

    let validate = repo.tiber(["validate", "--fix"]);

    assert_success_ref(&validate);
    assert_eq!(
        String::from_utf8(validate.stdout).expect("validate output should be utf8"),
        "cycle dependency todo/cycle-a.md -> todo/cycle-b.md -> todo/cycle-a.md\n"
    );
    assert_eq!(
        fs::read_to_string(cycle_a).expect("cycle a should be readable"),
        cycle_a_contents
    );
    assert_eq!(
        fs::read_to_string(cycle_b).expect("cycle b should be readable"),
        cycle_b_contents
    );
}

#[test]
fn validate_fix_reports_subtask_dag_cycles_without_rewriting_them() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Parent task"]));
    assert_success(repo.tiber(["create", "Child task"]));

    let parent = repo
        .path()
        .join(".tasks")
        .join("todo")
        .join("parent-task.md");
    let child = repo
        .path()
        .join(".tasks")
        .join("todo")
        .join("child-task.md");
    let parent_contents = "# Parent task\n\n## Subtasks\n- [ ] todo/child-task.md\n";
    let child_contents = "# Child task\n\n## Subtasks\n- [ ] todo/parent-task.md\n";
    fs::write(&parent, parent_contents).expect("write parent task");
    fs::write(&child, child_contents).expect("write child task");

    let validate = repo.tiber(["validate", "--fix"]);

    assert_success_ref(&validate);
    assert_eq!(
        String::from_utf8(validate.stdout).expect("validate output should be utf8"),
        "cycle subtask todo/child-task.md -> todo/parent-task.md -> todo/child-task.md\n"
    );
    assert_eq!(
        fs::read_to_string(parent).expect("parent task should be readable"),
        parent_contents
    );
    assert_eq!(
        fs::read_to_string(child).expect("child task should be readable"),
        child_contents
    );
}

#[test]
fn validate_fix_reports_task_schema_errors_without_rewriting_them() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    fs::create_dir_all(repo.path().join(".tasks").join("todo")).expect("create todo directory");
    let malformed = repo
        .path()
        .join(".tasks")
        .join("todo")
        .join("missing-title.md");
    let contents = "body without a markdown title\n";
    fs::write(&malformed, contents).expect("write malformed task");
    fs::write(
        repo.path().join(".tasks").join("order.md"),
        "todo/missing-title.md\n",
    )
    .expect("write order");

    let validate = repo.tiber(["validate", "--fix"]);

    assert_success_ref(&validate);
    assert_eq!(
        String::from_utf8(validate.stdout).expect("validate output should be utf8"),
        "schema title-missing todo/missing-title.md\n"
    );
    assert_eq!(
        fs::read_to_string(malformed).expect("task should be readable"),
        contents
    );
}

#[test]
fn validate_fix_removes_misplaced_claims_from_unclaimed_tasks() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Unclaimed work"]));

    let task_path = repo
        .path()
        .join(".tasks")
        .join("todo")
        .join("unclaimed-work.md");
    fs::write(
        &task_path,
        "# Unclaimed work\n\n## Claims\n- stale-agent\n\n## Subtasks\n- [ ] Keep this work\n",
    )
    .expect("write misplaced claim");

    let validate = repo.tiber(["validate", "--fix"]);

    assert_success_ref(&validate);
    assert_eq!(
        String::from_utf8(validate.stdout).expect("validate output should be utf8"),
        "fixed misplaced-claim todo/unclaimed-work.md\n"
    );
    assert_eq!(
        fs::read_to_string(task_path).expect("task should be readable"),
        "# Unclaimed work\n\n## Subtasks\n- [ ] Keep this work\n"
    );
}

#[test]
fn validate_fix_preserves_claims_on_doing_tasks() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Claimed work"]));
    assert_success(repo.tiber(["transition", "todo/claimed-work.md", "doing"]));

    let task_path = repo
        .path()
        .join(".tasks")
        .join("doing")
        .join("claimed-work.md");
    let contents = "# Claimed work\n\n## Claims\n- active-agent\n";
    fs::write(&task_path, contents).expect("write claim");

    let validate = repo.tiber(["validate", "--fix"]);

    assert_success_ref(&validate);
    assert_eq!(
        String::from_utf8(validate.stdout).expect("validate output should be utf8"),
        ""
    );
    assert_eq!(
        fs::read_to_string(task_path).expect("task should be readable"),
        contents
    );
}
