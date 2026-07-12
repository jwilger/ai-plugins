mod support;

use support::TempRepo;

#[test]
fn help_succeeds_for_the_root_and_every_command_path() {
    let repo = TempRepo::initialized();
    let help_paths: &[&[&str]] = &[
        &["--help"],
        &["-h"],
        &["init", "--help"],
        &["sync", "--help"],
        &["codex-sandbox", "--help"],
        &["dashboard", "--help"],
        &["dashboard", "serve", "--help"],
        &["mcp", "--help"],
        &["mcp", "stdio", "--help"],
        &["install-bin", "--help"],
        &["create", "--help"],
        &["show", "--help"],
        &["metadata", "--help"],
        &["list", "--help"],
        &["next", "--help"],
        &["transition", "--help"],
        &["prioritize", "--help"],
        &["link", "--help"],
        &["link", "source", "blocks", "--help"],
        &["unlink", "--help"],
        &["unlink", "source", "blocks", "--help"],
        &["subtask", "--help"],
        &["subtask", "add", "--help"],
        &["subtask", "check", "--help"],
        &["subtask", "uncheck", "--help"],
        &["update", "--help"],
        &["acceptance", "--help"],
        &["acceptance", "add", "--help"],
        &["acceptance", "check", "--help"],
        &["acceptance", "uncheck", "--help"],
        &["acceptance", "remove", "--help"],
        &["note", "--help"],
        &["note", "add", "--help"],
        &["validate", "--help"],
        &["close-from-trailers", "--help"],
        &["scaffold", "--help"],
        &["scaffold", "repo", "--help"],
    ];

    for args in help_paths {
        let output = repo.tiber(*args);
        assert!(
            output.status.success(),
            "help failed for {args:?}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.stderr.is_empty(),
            "help wrote stderr for {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("help output should be utf8");
        assert!(
            stdout.contains("Usage: tiber"),
            "help lacked generated usage for {args:?}: {stdout}"
        );
        assert!(
            stdout.contains("-h, --help"),
            "help lacked generated help option for {args:?}: {stdout}"
        );
    }
}
