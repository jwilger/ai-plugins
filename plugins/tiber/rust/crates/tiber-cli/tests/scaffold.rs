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
