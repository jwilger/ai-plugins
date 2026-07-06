mod support;

use std::fs;
use std::path::PathBuf;

use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn init_creates_tasks_branch_and_links_worktree_tasks_dir() {
    let repo = TempRepo::initialized();

    let output = repo.tiber(["init"]);

    assert_success(output);
    assert_success(repo.git_output(["show-ref", "--verify", "refs/heads/tasks"]));
    let git_common_dir = repo.git_output(["rev-parse", "--git-common-dir"]);
    assert_success_ref(&git_common_dir);
    let git_common_dir = PathBuf::from(
        String::from_utf8(git_common_dir.stdout)
            .expect("git common dir is utf8")
            .trim(),
    );
    assert_eq!(
        fs::read_link(repo.path().join(".tasks")).expect(".tasks should be a symlink"),
        repo.path().join(git_common_dir).join("tiber/main/.tasks")
    );

    let tree = repo.git_output(["ls-tree", "-r", "--name-only", "tasks"]);
    assert_success_ref(&tree);
    let tree_names = String::from_utf8(tree.stdout).expect("git tree output is utf8");
    assert!(tree_names
        .lines()
        .any(|line| line == "main/.tasks/order.md"));
}
