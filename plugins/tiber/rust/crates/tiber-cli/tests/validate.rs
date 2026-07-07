mod support;

use support::{assert_success, assert_success_ref, task_stem, TempRepo};

#[test]
fn validate_fix_repairs_missing_reciprocal_links_and_order_entries() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Build API"]));
    assert_success(repo.tiber(["create", "Build UI"]));
    let api = task_stem(&repo, "backlog", "build-api");
    let ui = task_stem(&repo, "backlog", "build-ui");

    repo.insert_task_file(
        "backlog",
        &api,
        &task_document("Build API", &[], &[&ui], &[], ""),
    );
    repo.insert_tasks_tree_file("order.md", &format!("{api}\nstale-task\n"));

    let validate = repo.tiber(["validate", "--fix"]);

    assert_success_ref(&validate);
    assert_eq!(
        String::from_utf8(validate.stdout).expect("validate output should be utf8"),
        format!("fixed reciprocal-link {ui} blocked-by {api}\nfixed order stale stale-task\nfixed order missing {ui}\n")
    );

    let ui_task = repo.tiber(["show", "build-ui"]);
    assert_success_ref(&ui_task);
    assert!(String::from_utf8(ui_task.stdout)
        .expect("ui task should be utf8")
        .contains(&format!("blocked_by: [{api}]")));

    assert_eq!(repo.order_file(), format!("{api}\n{ui}\n"));
}

#[test]
fn validate_fix_reports_dangling_dependency_refs_without_removing_them() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Dangling dependencies"]));
    let task = task_stem(&repo, "backlog", "dangling-dependencies");

    let contents = task_document(
        "Dangling dependencies",
        &["missing-blocker"],
        &["missing-blocked"],
        &[],
        "",
    );
    repo.insert_task_file("backlog", &task, &contents);

    let validate = repo.tiber(["validate", "--fix"]);

    assert_success_ref(&validate);
    assert_eq!(
        String::from_utf8(validate.stdout).expect("validate output should be utf8"),
        format!("dangling link {task} blocks missing-blocked\ndangling link {task} blocked-by missing-blocker\n")
    );
    assert_eq!(repo.task_file("backlog", &task), contents);
}

#[test]
fn validate_fix_reports_dependency_cycles_without_rewriting_them() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Cycle A"]));
    assert_success(repo.tiber(["create", "Cycle B"]));
    let cycle_a = task_stem(&repo, "backlog", "cycle-a");
    let cycle_b = task_stem(&repo, "backlog", "cycle-b");

    let cycle_a_contents = task_document("Cycle A", &[&cycle_b], &[&cycle_b], &[], "");
    let cycle_b_contents = task_document("Cycle B", &[&cycle_a], &[&cycle_a], &[], "");
    repo.insert_task_file("backlog", &cycle_a, &cycle_a_contents);
    repo.insert_task_file("backlog", &cycle_b, &cycle_b_contents);

    let validate = repo.tiber(["validate", "--fix"]);

    assert_success_ref(&validate);
    let expected_cycle = if cycle_a < cycle_b {
        format!("cycle dependency {cycle_a} -> {cycle_b} -> {cycle_a}\n")
    } else {
        format!("cycle dependency {cycle_b} -> {cycle_a} -> {cycle_b}\n")
    };
    assert_eq!(
        String::from_utf8(validate.stdout).expect("validate output should be utf8"),
        expected_cycle
    );
    assert_eq!(repo.task_file("backlog", &cycle_a), cycle_a_contents);
    assert_eq!(repo.task_file("backlog", &cycle_b), cycle_b_contents);
}

#[test]
fn validate_fix_reports_subtask_dag_cycles_without_rewriting_them() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Parent task"]));
    let parent = task_stem(&repo, "backlog", "parent-task");
    let parent_contents = task_document(
        "Parent task",
        &[],
        &[],
        &[],
        "## Subtasks\n\n- [ ] (s1) First step — after: s2\n- [ ] (s2) Second step — after: s1\n\n## Notes / Log\n",
    );
    repo.insert_task_file("backlog", &parent, &parent_contents);

    let validate = repo.tiber(["validate", "--fix"]);

    assert_success_ref(&validate);
    assert_eq!(
        String::from_utf8(validate.stdout).expect("validate output should be utf8"),
        format!("cycle subtask {parent}:s1 -> s2 -> s1\n")
    );
    assert_eq!(repo.task_file("backlog", &parent), parent_contents);
}

#[test]
fn validate_fix_reports_task_schema_errors_without_rewriting_them() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    let contents = "---\nstatus: backlog\nblocked_by: []\nblocks: []\ntags: []\n---\n";
    repo.insert_task_file("backlog", "20260706-abcd-missing-title", contents);
    repo.insert_tasks_tree_file("order.md", "20260706-abcd-missing-title\n");

    let validate = repo.tiber(["validate", "--fix"]);

    assert_success_ref(&validate);
    assert_eq!(
        String::from_utf8(validate.stdout).expect("validate output should be utf8"),
        "schema title-missing 20260706-abcd-missing-title\nschema forbidden-key 20260706-abcd-missing-title status\n"
    );
    assert_eq!(
        repo.task_file("backlog", "20260706-abcd-missing-title"),
        contents
    );
}

#[test]
fn validate_fix_removes_misplaced_claims_from_unclaimed_tasks() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Unclaimed work"]));
    let task = task_stem(&repo, "backlog", "unclaimed-work");

    let contents = "---\ntitle: Unclaimed work\nblocked_by: []\nblocks: []\ntags: []\nclaim:\n  host: stale\n  session: stale-session\n---\n\n## Summary\n\n## Subtasks\n\n- [ ] (s1) Keep this work\n\n## Notes / Log\n";
    repo.insert_task_file("backlog", &task, contents);

    let validate = repo.tiber(["validate", "--fix"]);

    assert_success_ref(&validate);
    assert_eq!(
        String::from_utf8(validate.stdout).expect("validate output should be utf8"),
        format!("fixed misplaced-claim {task}\n")
    );
    assert_eq!(
        repo.task_file("backlog", &task),
        "---\ntitle: Unclaimed work\nblocked_by: []\nblocks: []\ntags: []\n---\n\n## Summary\n\n## Subtasks\n\n- [ ] (s1) Keep this work\n\n## Notes / Log\n"
    );
}

#[test]
fn validate_fix_preserves_claims_on_in_progress_tasks() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Claimed work"]));
    let task = task_stem(&repo, "backlog", "claimed-work");
    assert_success(repo.tiber(["transition", "claimed-work", "in-progress"]));

    let contents = "---\ntitle: Claimed work\nblocked_by: []\nblocks: []\ntags: []\nclaim:\n  host: active\n  session: active-session\n---\n";
    repo.insert_task_file("in-progress", &task, contents);

    let validate = repo.tiber(["validate", "--fix"]);

    assert_success_ref(&validate);
    assert_eq!(
        String::from_utf8(validate.stdout).expect("validate output should be utf8"),
        ""
    );
    assert_eq!(repo.task_file("in-progress", &task), contents);
}

fn task_document(
    title: &str,
    blocked_by: &[&str],
    blocks: &[&str],
    tags: &[&str],
    body: &str,
) -> String {
    let body = if body.is_empty() {
        "## Summary\n\n## Context / Why\n\n## Acceptance criteria\n\n## Subtasks\n\n## Notes / Log\n"
    } else {
        body
    };
    format!(
        "---\ntitle: {title}\nblocked_by: [{}]\nblocks: [{}]\ntags: [{}]\n---\n\n{body}",
        blocked_by.join(", "),
        blocks.join(", "),
        tags.join(", ")
    )
}
