mod support;

use std::fs;

use support::{assert_success, TempRepo};

#[test]
fn update_without_a_field_is_a_parser_error() {
    let repo = TempRepo::initialized();

    let output = repo.tiber(["update", "missing-task"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("parser error should be utf8");
    assert!(stderr.contains("error:"), "missing parser error: {stderr}");
    assert!(
        stderr.contains("Usage: tiber update"),
        "missing parser-generated usage: {stderr}"
    );
}

#[test]
fn subtask_after_option_requires_a_positional_title() {
    let repo = TempRepo::initialized();

    let output = repo.tiber(["subtask", "add", "missing-task", "--after", "s1"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("parser error should be utf8");
    assert!(stderr.contains("error:"), "missing parser error: {stderr}");
    assert!(
        stderr.contains("Usage: tiber subtask add"),
        "missing parser-generated usage: {stderr}"
    );
}

#[test]
fn subtask_after_option_requires_a_value_after_the_title() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Example"]));

    let output = repo.tiber(["subtask", "add", "example", "Normal title", "--after"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("parser error should be utf8");
    assert!(stderr.contains("error:"), "missing parser error: {stderr}");
    assert!(
        stderr.contains("Usage: tiber subtask add"),
        "missing parser-generated usage: {stderr}"
    );
    let shown = repo.tiber(["show", "example"]);
    assert!(
        !String::from_utf8(shown.stdout)
            .expect("show output should be utf8")
            .contains("Normal title"),
        "invalid invocation must not write a subtask"
    );
}

#[test]
fn install_mode_flag_cannot_be_consumed_as_a_reordered_target_value() {
    let repo = TempRepo::initialized();
    let launcher = repo.path().join("plugin/bin/tiber");
    fs::create_dir_all(launcher.parent().expect("launcher parent"))
        .expect("create launcher directory");
    fs::write(&launcher, "#!/usr/bin/env bash\n").expect("write fake launcher");

    let output = repo.tiber_with_env(
        ["install-bin", "--apply", "--target-dir", "--dry-run"],
        [(
            "TIBER_LAUNCHER_PATH",
            launcher.to_str().expect("launcher path utf8"),
        )],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("parser error should be utf8");
    assert!(stderr.contains("error:"), "missing parser error: {stderr}");
    assert!(
        stderr.contains("Usage: tiber install-bin"),
        "missing parser-generated usage: {stderr}"
    );
    assert!(
        !repo.path().join("--dry-run/tiber").exists(),
        "invalid invocation must not install a launcher"
    );
}
