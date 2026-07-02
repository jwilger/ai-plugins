//! Built-in session-command templates for harnesses sidequest knows how to
//! drive out of the box. Pure — no I/O.
//!
//! `$SIDEQUEST_GOAL` is left as a literal environment-variable reference: the
//! shell substitutes it at run time (see `session::run`), so arbitrary goal
//! text — including quotes, apostrophes, and newlines — passes through as a
//! single argument rather than being string-interpolated into the command.
//! Neither harness commits its work on its own, so each template explicitly
//! instructs it to.

/// The built-in headless invocation template for `harness`, if known.
///
/// Returns `None` for an unrecognized harness name (a project must then set
/// `[harness] command` in `sidequest.toml` explicitly).
#[must_use]
pub fn default_session_command(harness: &str) -> Option<&'static str> {
    match harness {
        "claude" => Some(
            "claude --print \"$SIDEQUEST_GOAL\n\nWhen you have finished, commit your changes \
             with git before finishing.\" --dangerously-skip-permissions --output-format \
             stream-json --verbose",
        ),
        "codex" => Some(
            "codex exec --dangerously-bypass-approvals-and-sandbox \"$SIDEQUEST_GOAL\n\nWhen you \
             have finished, commit your changes with git before finishing.\"",
        ),
        _ => None,
    }
}

#[cfg(test)]
#[expect(clippy::expect_used, reason = "unit tests use expect() for clarity")]
mod tests {
    use super::*;

    #[test]
    fn known_harnesses_have_a_default_command() {
        for harness in ["claude", "codex"] {
            let command = default_session_command(harness)
                .expect("claude and codex should have a built-in default command");
            assert!(
                command.contains("$SIDEQUEST_GOAL"),
                "{harness}'s default command must pass the goal via the environment: {command:?}"
            );
            assert!(
                command.contains("commit"),
                "{harness}'s default command must instruct it to commit its work: {command:?}"
            );
        }
    }

    #[test]
    fn stream_json_output_is_paired_with_verbose() {
        // Claude Code's CLI rejects `--print --output-format stream-json`
        // without `--verbose`, failing the session immediately.
        let command = default_session_command("claude").expect("claude has a default command");
        assert!(
            !command.contains("--output-format stream-json") || command.contains("--verbose"),
            "a command using stream-json output must also pass --verbose: {command:?}"
        );
    }

    #[test]
    fn unknown_harnesses_have_no_default_command() {
        assert_eq!(
            default_session_command("some-future-harness"),
            None,
            "an unrecognized harness name has no built-in default"
        );
    }
}
