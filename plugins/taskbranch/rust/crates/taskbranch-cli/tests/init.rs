mod support;

use std::fs;
use std::path::PathBuf;

use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn init_creates_tasks_branch_and_links_worktree_tasks_dir() {
    let repo = TempRepo::initialized();

    let output = repo.taskbranch(["init"]);

    assert_success(output);
    assert_success(repo.git_output(["show-ref", "--verify", "refs/heads/tasks"]));
    assert_eq!(
        fs::read_link(repo.path().join(".tasks")).expect(".tasks should be a symlink"),
        PathBuf::from(".git/taskbranch/main/.tasks")
    );

    let tree = repo.git_output(["ls-tree", "-r", "--name-only", "tasks"]);
    assert_success_ref(&tree);
    let tree_names = String::from_utf8(tree.stdout).expect("git tree output is utf8");
    assert!(tree_names
        .lines()
        .any(|line| line == "main/.tasks/order.md"));
}
