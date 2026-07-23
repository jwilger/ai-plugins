mod support;

use std::fs;

use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn scaffold_repo_dry_run_previews_and_apply_writes_files() {
    let repo = TempRepo::initialized();

    let dry_run = repo.tiber(["scaffold", "repo", "--dry-run"]);

    assert_success_ref(&dry_run);
    assert_eq!(
        String::from_utf8(dry_run.stdout).expect("dry-run output should be utf8"),
        "would write .gitignore\nwould write .githooks/post-commit.tiber\nwould write .github/workflows/tiber-close-from-trailers.yml\n"
    );
    assert!(!repo.path().join(".gitignore").exists());
    assert!(!repo
        .path()
        .join(".githooks")
        .join("post-commit.tiber")
        .exists());
    assert!(!repo
        .path()
        .join(".github")
        .join("workflows")
        .join("tiber-close-from-trailers.yml")
        .exists());

    let apply = repo.tiber(["scaffold", "repo", "--apply"]);

    assert_success(apply);
    assert!(repo.path().join(".gitignore").exists());
    assert_eq!(
        fs::read_to_string(repo.path().join(".githooks").join("post-commit.tiber"))
            .expect("read hook snippet"),
        "#!/usr/bin/env bash\nset -euo pipefail\n\ntiber close-from-trailers\n"
    );
    assert!(repo
        .path()
        .join(".github")
        .join("workflows")
        .join("tiber-close-from-trailers.yml")
        .exists());
}

#[test]
fn scaffold_repo_preserves_existing_gitignore_entries() {
    let repo = TempRepo::initialized();
    let existing = "target/\n.env\ncoverage/\n";
    fs::write(repo.path().join(".gitignore"), existing).expect("write existing gitignore");

    let apply = repo.tiber(["scaffold", "repo", "--apply"]);

    assert_success(apply);
    let gitignore =
        fs::read_to_string(repo.path().join(".gitignore")).expect("read updated gitignore");
    assert!(gitignore.starts_with(existing));
    assert_eq!(
        gitignore
            .lines()
            .filter(|line| line.trim() == ".tasks")
            .count(),
        1
    );
}

#[test]
fn scaffold_repo_does_not_replace_non_utf8_gitignore() {
    let repo = TempRepo::initialized();
    let existing = b"target/\n\xff\n";
    fs::write(repo.path().join(".gitignore"), existing).expect("write non-utf8 gitignore");

    let apply = repo.tiber(["scaffold", "repo", "--apply"]);

    assert!(!apply.status.success());
    assert_eq!(
        fs::read(repo.path().join(".gitignore")).expect("read unchanged gitignore"),
        existing
    );
}

#[test]
fn scaffold_repo_detects_an_equivalent_existing_workflow() {
    let repo = TempRepo::initialized();
    let workflow_path = repo
        .path()
        .join(".github")
        .join("workflows")
        .join("close-tasks.yaml");
    fs::create_dir_all(workflow_path.parent().expect("workflow parent"))
        .expect("create workflow directory");
    let workflow =
        "name: close tasks\non: push\njobs:\n  close:\n    steps:\n      - run: tiber close-from-trailers\n";
    fs::write(&workflow_path, workflow).expect("write existing workflow");

    let dry_run = repo.tiber(["scaffold", "repo", "--dry-run"]);

    assert_success_ref(&dry_run);
    let stdout = String::from_utf8(dry_run.stdout).expect("dry-run output should be utf8");
    assert!(stdout.contains("already configured .github/workflows/close-tasks.yaml"));
    assert!(!stdout.contains(".github/workflows/tiber-close-from-trailers.yml"));

    let apply = repo.tiber(["scaffold", "repo", "--apply"]);

    assert_success(apply);
    assert_eq!(
        fs::read_to_string(&workflow_path).expect("read existing workflow"),
        workflow
    );
    assert!(!repo
        .path()
        .join(".github")
        .join("workflows")
        .join("tiber-close-from-trailers.yml")
        .exists());
}

#[test]
fn scaffold_repo_does_not_treat_workflow_comments_as_automation() {
    let repo = TempRepo::initialized();
    let workflow_path = repo
        .path()
        .join(".github")
        .join("workflows")
        .join("build.yml");
    fs::create_dir_all(workflow_path.parent().expect("workflow parent"))
        .expect("create workflow directory");
    fs::write(
        workflow_path,
        "name: build\n# tiber close-from-trailers\njobs:\n  build:\n    steps:\n      - run: echo build\n",
    )
    .expect("write unrelated workflow");

    let dry_run = repo.tiber(["scaffold", "repo", "--dry-run"]);

    assert_success_ref(&dry_run);
    let stdout = String::from_utf8(dry_run.stdout).expect("dry-run output should be utf8");
    assert!(stdout.contains("would write .github/workflows/tiber-close-from-trailers.yml"));
    assert!(!stdout.contains("already configured .github/workflows/build.yml"));
}

#[test]
fn scaffold_repo_does_not_treat_action_inputs_as_automation() {
    let repo = TempRepo::initialized();
    let workflow_path = repo
        .path()
        .join(".github")
        .join("workflows")
        .join("action-input.yml");
    fs::create_dir_all(workflow_path.parent().expect("workflow parent"))
        .expect("create workflow directory");
    fs::write(
        workflow_path,
        "name: action input\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: example/action@v1\n        with:\n          run: tiber close-from-trailers\n",
    )
    .expect("write unrelated workflow");

    let dry_run = repo.tiber(["scaffold", "repo", "--dry-run"]);

    assert_success_ref(&dry_run);
    let stdout = String::from_utf8(dry_run.stdout).expect("dry-run output should be utf8");
    assert!(stdout.contains("would write .github/workflows/tiber-close-from-trailers.yml"));
    assert!(!stdout.contains("already configured .github/workflows/action-input.yml"));
}

#[test]
fn scaffold_repo_detects_an_equivalent_existing_hook() {
    let repo = TempRepo::initialized();
    repo.git(["config", "core.hooksPath", ".githooks"]);
    let hook_path = repo.path().join(".githooks").join("post-commit");
    fs::create_dir_all(hook_path.parent().expect("hook parent")).expect("create hook directory");
    let hook = "#!/usr/bin/env bash\nset -euo pipefail\ntiber close-from-trailers\n";
    fs::write(&hook_path, hook).expect("write existing hook");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755))
            .expect("make hook executable");
    }

    let dry_run = repo.tiber(["scaffold", "repo", "--dry-run"]);

    assert_success_ref(&dry_run);
    let stdout = String::from_utf8(dry_run.stdout).expect("dry-run output should be utf8");
    assert!(stdout.contains("already configured .githooks/post-commit"));
    assert!(!stdout.contains(".githooks/post-commit.tiber"));

    let apply = repo.tiber(["scaffold", "repo", "--apply"]);

    assert_success(apply);
    assert_eq!(
        fs::read_to_string(&hook_path).expect("read existing hook"),
        hook
    );
    assert!(!repo
        .path()
        .join(".githooks")
        .join("post-commit.tiber")
        .exists());
}

#[test]
fn scaffold_repo_does_not_treat_an_inactive_hook_file_as_automation() {
    let repo = TempRepo::initialized();
    repo.git(["config", "core.hooksPath", ".githooks"]);
    let hook_path = repo.path().join(".githooks").join("post-commit");
    fs::create_dir_all(hook_path.parent().expect("hook parent")).expect("create hook directory");
    fs::write(
        hook_path,
        "#!/usr/bin/env bash\ntiber close-from-trailers\n",
    )
    .expect("write non-executable hook");

    let dry_run = repo.tiber(["scaffold", "repo", "--dry-run"]);

    assert_success_ref(&dry_run);
    let stdout = String::from_utf8(dry_run.stdout).expect("dry-run output should be utf8");
    assert!(stdout.contains("would write .githooks/post-commit.tiber"));
    assert!(!stdout.contains("already configured .githooks/post-commit"));
}

#[test]
fn scaffold_repo_resolves_hooks_from_a_linked_worktree() {
    let repo = TempRepo::initialized();
    let hook_path = repo.path().join(".git").join("hooks").join("post-commit");
    let hook = "#!/usr/bin/env bash\ntiber close-from-trailers\n";
    fs::write(&hook_path, hook).expect("write common hook");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755))
            .expect("make hook executable");
    }
    let linked = repo.path().join(".worktrees").join("feature");
    repo.git(["worktree", "add", ".worktrees/feature", "-b", "feature"]);

    let dry_run = repo.tiber_at(&linked, ["scaffold", "repo", "--dry-run"]);

    assert_success_ref(&dry_run);
    let stdout = String::from_utf8(dry_run.stdout).expect("dry-run output should be utf8");
    assert!(stdout.contains("already configured"));
    assert!(!stdout.contains(".githooks/post-commit.tiber"));
}

#[test]
fn scaffold_repo_reports_repeated_setup_as_already_configured() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["scaffold", "repo", "--apply"]));

    let dry_run = repo.tiber(["scaffold", "repo", "--dry-run"]);

    assert_success_ref(&dry_run);
    let stdout = String::from_utf8(dry_run.stdout).expect("dry-run output should be utf8");
    assert!(stdout.contains("already configured .gitignore"));
    assert!(stdout.contains("already configured .githooks/post-commit.tiber"));
    assert!(stdout.contains("already configured .github/workflows/tiber-close-from-trailers.yml"));
    assert!(!stdout.contains("would write"));
    assert!(!stdout.contains("conflict"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for path in [
            ".gitignore",
            ".githooks/post-commit.tiber",
            ".github/workflows/tiber-close-from-trailers.yml",
        ] {
            fs::set_permissions(repo.path().join(path), fs::Permissions::from_mode(0o444))
                .expect("make generated file read-only");
        }
    }
    let second_apply = repo.tiber(["scaffold", "repo", "--apply"]);

    assert_success_ref(&second_apply);
    let stdout = String::from_utf8(second_apply.stdout).expect("apply output should be utf8");
    assert!(stdout.contains("already configured .gitignore"));
    assert!(stdout.contains("already configured .githooks/post-commit.tiber"));
    assert!(stdout.contains("already configured .github/workflows/tiber-close-from-trailers.yml"));
    assert!(!stdout.contains("wrote"));
}

#[test]
fn scaffold_repo_adds_show_tasks_recipe_when_justfile_exists() {
    let repo = TempRepo::initialized();
    fs::write(repo.path().join("justfile"), "test:\n  cargo test\n")
        .expect("write existing justfile");

    let dry_run = repo.tiber(["scaffold", "repo", "--dry-run"]);

    assert_success_ref(&dry_run);
    assert_eq!(
        String::from_utf8(dry_run.stdout).expect("dry-run output should be utf8"),
        "would write .gitignore\nwould write .githooks/post-commit.tiber\nwould write .github/workflows/tiber-close-from-trailers.yml\nwould write justfile\n"
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("justfile")).expect("read justfile"),
        "test:\n  cargo test\n"
    );

    let apply = repo.tiber(["scaffold", "repo", "--apply"]);

    assert_success(apply);
    assert_eq!(
        fs::read_to_string(repo.path().join("justfile")).expect("read justfile"),
        "test:\n  cargo test\n\nshow-tasks:\n  tiber list\n"
    );

    let second_apply = repo.tiber(["scaffold", "repo", "--apply"]);
    assert_success(second_apply);
    assert_eq!(
        fs::read_to_string(repo.path().join("justfile")).expect("read justfile"),
        "test:\n  cargo test\n\nshow-tasks:\n  tiber list\n"
    );
}
