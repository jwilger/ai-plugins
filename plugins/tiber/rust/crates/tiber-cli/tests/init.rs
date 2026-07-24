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

#[test]
fn init_refuses_an_existing_source_tree_tasks_system_without_mutation() {
    let repo = TempRepo::initialized();
    fs::create_dir_all(repo.path().join(".tasks/backlog")).expect("create existing task system");
    fs::write(
        repo.path().join(".tasks/backlog/existing.md"),
        "# Existing task\n",
    )
    .expect("write existing task");
    let before = repo.git_output(["status", "--short"]);
    assert_success_ref(&before);

    let output = repo.tiber(["init"]);

    assert!(
        !output.status.success(),
        "parallel task board must be refused"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("existing_tasks_system"));
    assert!(stderr.contains("move, migrate, or explicitly integrate"));
    let task_ref = repo.git_output(["show-ref", "--verify", "refs/heads/tasks"]);
    assert!(
        !task_ref.status.success(),
        "tasks branch must not be created"
    );
    let after = repo.git_output(["status", "--short"]);
    assert_success_ref(&after);
    assert_eq!(
        after.stdout, before.stdout,
        "init refusal must not mutate files"
    );
}

#[test]
fn codex_sandbox_preview_prefers_narrow_git_prefixes() {
    let repo = TempRepo::initialized();

    let output = repo.tiber(["codex-sandbox", "--dry-run"]);

    assert_success_ref(&output);
    let stdout = String::from_utf8(output.stdout).expect("preview output should be utf8");
    assert!(stdout.contains("Tiber Codex sandbox setup preview"));
    assert!(stdout.contains("Prefer the narrowest approval"));
    assert!(stdout.contains("Couldn't get agent socket?"));
    assert!(stdout.contains("forwards SSH_AUTH_SOCK"));
    assert!(stdout.contains("env_vars = [\"SSH_AUTH_SOCK\"]"));
    assert!(stdout.contains("plugin MCP policy overlays do not change transport env"));
    assert!(stdout.contains("preserves the absolute installed launcher"));
    assert!(stdout.contains("Never forward SSH_AUTH_SOCK to a PATH-resolved"));
    assert!(stdout.contains("case-by-case approval for prefix_rule [\"git\", \"hash-object\"]"));
    assert!(stdout.contains("prefix_rule [\"git\", \"hash-object\"]"));
    assert!(stdout.contains("case-by-case approval for prefix_rule [\"git\", \"mktree\"]"));
    assert!(stdout.contains("prefix_rule [\"git\", \"mktree\"]"));
    assert!(stdout.contains("case-by-case approval for prefix_rule [\"git\", \"commit-tree\"]"));
    assert!(stdout.contains("prefix_rule [\"git\", \"commit-tree\"]"));
    assert!(stdout.contains("signed commit-tree -S"));
    assert!(stdout.contains(
        "case-by-case approval for prefix_rule [\"git\", \"update-ref\", \"refs/heads/tasks\"]"
    ));
    assert!(stdout.contains("prefix_rule [\"git\", \"update-ref\", \"refs/heads/tasks\"]"));
    assert!(stdout.contains(
        "case-by-case approval for prefix_rule [\"git\", \"fetch\", \"origin\", \"tasks:refs/remotes/origin/tasks\"]"
    ));
    assert!(stdout.contains(
        "prefix_rule [\"git\", \"fetch\", \"origin\", \"tasks:refs/remotes/origin/tasks\"]"
    ));
    assert!(stdout.contains("case-by-case approval for prefix_rule [\"git\", \"-c\", \"core.hooksPath=/dev/null\", \"push\", \"origin\", \"refs/heads/tasks:refs/heads/tasks\"]"));
    assert!(stdout.contains(
        "prefix_rule [\"git\", \"-c\", \"core.hooksPath=/dev/null\", \"push\", \"origin\", \"refs/heads/tasks:refs/heads/tasks\"]"
    ));
    assert!(stdout.contains(
        "Persist approval only when the harness can scope it to the exact Tiber-internal operation"
    ));
    assert!(stdout.contains(
        "Never persist a raw git, wildcard git, bash, sh, or whole-MCP-server permission"
    ));
    assert!(stdout.contains("retry the same structured Tiber MCP operation"));
    assert!(stdout.contains("Do not run the whole Tiber MCP server outside the sandbox"));
    assert!(
        stdout.contains("Do not ask the user to rerun an equivalent tiber CLI command manually")
    );
}
