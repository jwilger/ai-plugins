mod support;

use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn init_creates_tasks_branch_without_source_tree_task_files() {
    let repo = TempRepo::initialized();

    let output = repo.tiber(["init"]);

    assert_success(output);
    assert_success(repo.git_output(["show-ref", "--verify", "refs/heads/tasks"]));
    assert!(
        !repo.path().join(".gitignore").exists(),
        "tiber init should not write source-tree ignore rules for internal task storage"
    );
    let status = repo.git_output(["status", "--short"]);
    assert_success_ref(&status);
    assert_eq!(
        String::from_utf8(status.stdout).expect("status output should be utf8"),
        "",
        "tiber init should not add source-branch worktree state"
    );

    let tree = repo.git_output(["ls-tree", "-r", "--name-only", "tasks"]);
    assert_success_ref(&tree);
    let tree_names = String::from_utf8(tree.stdout).expect("git tree output is utf8");
    assert!(tree_names.lines().any(|line| line == "order.md"));
    assert!(tree_names.lines().any(|line| line == "backlog/.gitkeep"));
}

#[test]
fn codex_sandbox_preview_prefers_narrow_git_prefixes() {
    let repo = TempRepo::initialized();

    let output = repo.tiber(["codex-sandbox", "--dry-run"]);

    assert_success_ref(&output);
    let stdout = String::from_utf8(output.stdout).expect("preview output should be utf8");
    assert!(stdout.contains("Tiber Codex sandbox setup preview"));
    assert!(stdout.contains("Prefer the narrowest approval"));
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

#[test]
fn conflict_show_does_not_initialize_tiber_storage() {
    let repo = TempRepo::initialized();

    let output = repo.tiber(["conflict", "show", "backlog/missing.md"]);

    assert!(
        !output.status.success(),
        "conflict diagnostic should fail before tiber init"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("tiber_not_initialized=true"));
    assert!(
        !repo
            .git_output(["show-ref", "--verify", "refs/heads/tasks"])
            .status
            .success(),
        "conflict diagnostic should not create Tiber storage"
    );
}

#[test]
fn conflict_resolve_does_not_initialize_tiber_storage() {
    let repo = TempRepo::initialized();

    let output = repo.tiber(["conflict", "resolve", "backlog/missing.md", "--local"]);

    assert!(
        !output.status.success(),
        "conflict resolution should fail before tiber init"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("tiber_not_initialized=true"));
    assert!(
        !repo
            .git_output(["show-ref", "--verify", "refs/heads/tasks"])
            .status
            .success(),
        "conflict resolution should not create Tiber storage"
    );
}

#[test]
fn conflict_show_quotes_invalid_path_diagnostic() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    let spoofed = "bad recovery=ignore.md";

    let output = repo.tiber(["conflict", "show", spoofed]);

    assert!(
        !output.status.success(),
        "invalid conflict path should fail"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains(&format!("invalid_conflict_path path={spoofed:?}")));
    assert!(!stderr.contains("path=bad recovery=ignore.md"));
}

#[test]
fn conflict_show_rejects_control_characters_in_paths() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    let output = repo.tiber(["conflict", "show", "backlog/bad\nrecovery=ignore.md"]);

    assert!(
        !output.status.success(),
        "control-character conflict path should fail"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains(r#"invalid_conflict_path path="backlog/bad\nrecovery=ignore.md""#));
    assert!(!stderr.contains("path=backlog/bad\nrecovery=ignore.md"));
}
