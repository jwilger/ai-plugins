mod support;

use std::fs;
use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn init_creates_tasks_branch_and_ignores_accidental_tasks_checkout() {
    let repo = TempRepo::initialized();

    let output = repo.tiber(["init"]);

    assert_success(output);
    assert_success(repo.git_output(["show-ref", "--verify", "refs/heads/tasks"]));
    let gitignore =
        fs::read_to_string(repo.path().join(".gitignore")).expect(".gitignore should be readable");
    assert!(
        gitignore.lines().any(|line| line.trim() == ".tasks"),
        ".tasks should be ignored through source-branch .gitignore"
    );
    assert!(
        !repo.path().join(".tasks").exists(),
        "tiber should not keep a persistent .tasks checkout"
    );
    let status = repo.git_output(["status", "--short", "--", ".tasks"]);
    assert_success_ref(&status);
    assert_eq!(
        String::from_utf8(status.stdout).expect("status output should be utf8"),
        "",
        ".tasks should not appear as source-branch worktree state"
    );

    let tree = repo.git_output(["ls-tree", "-r", "--name-only", "tasks"]);
    assert_success_ref(&tree);
    let tree_names = String::from_utf8(tree.stdout).expect("git tree output is utf8");
    assert!(tree_names.lines().any(|line| line == "order.md"));
    assert!(tree_names.lines().any(|line| line == "backlog/.gitkeep"));
}
