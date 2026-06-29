//! Running a side-quest's goal session inside its worktree (imperative shell).
//!
//! The session command is the seam where a harness (`codex exec`, `claude`)
//! plugs in. It runs via `sh -c` inside the worktree, with the goal exposed
//! through the `SIDEQUEST_GOAL` environment variable.

use std::path::Path;

use sidequest_core::launch::Goal;
use thiserror::Error;
use tokio::process::Command;

/// A failure running the goal session.
#[derive(Debug, Error)]
pub enum SessionError {
    /// The session process could not be spawned.
    #[error("session-spawn-failed: {0}")]
    Spawn(String),
    /// The session process exited non-zero.
    #[error("session-failed: {0}")]
    Exit(String),
}

/// Run `command` (via `sh -c`) inside `worktree` as the side-quest's goal
/// session.
///
/// # Errors
///
/// Returns [`SessionError`] if the session cannot be spawned or exits non-zero.
pub async fn run(worktree: &Path, command: &str, goal: &Goal) -> Result<(), SessionError> {
    let goal_text: &str = goal.as_ref();
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(worktree)
        .env("SIDEQUEST_GOAL", goal_text)
        .output()
        .await
        .map_err(|error| SessionError::Spawn(error.to_string()))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(SessionError::Exit(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ))
    }
}
