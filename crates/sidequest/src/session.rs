//! Running a side-quest's goal session inside its worktree (imperative shell).
//!
//! The session command is the seam where a harness (`codex exec`, `claude`)
//! plugs in. It runs via `sh -c` inside the worktree, with the goal exposed
//! through the `SIDEQUEST_GOAL` environment variable. Its stdout/stderr are
//! redirected (not buffered) straight to the side-quest's log file, so the
//! log fills in live as the session runs and can be tailed while it is still
//! in progress.

use std::path::Path;

use sidequest_core::launch::{BranchName, Goal};
use thiserror::Error;
use tokio::process::Command;

use crate::logs;

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
/// session, logging its output to the side-quest's log file as it runs.
///
/// # Errors
///
/// Returns [`SessionError`] if the session cannot be spawned or exits non-zero.
pub async fn run(
    worktree: &Path,
    command: &str,
    goal: &Goal,
    project_root: &Path,
    branch: &BranchName,
) -> Result<(), SessionError> {
    let bin = std::env::current_exe().map_err(|error| SessionError::Spawn(error.to_string()))?;
    let log_path = logs::path(project_root, branch);
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| SessionError::Spawn(error.to_string()))?;
    }
    // Truncate rather than append: a branch name is deterministic from its
    // goal, so relaunching the same goal after a prior run reuses the same
    // log path -- each run's log must reflect only that run, not be a
    // mixture of stale content from a previous one.
    let log_out = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
        .map_err(|error| SessionError::Spawn(error.to_string()))?;
    let log_err = log_out
        .try_clone()
        .map_err(|error| SessionError::Spawn(error.to_string()))?;

    let status = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(worktree)
        .env("SIDEQUEST_GOAL", goal.as_ref())
        .env("SIDEQUEST_PROJECT_ROOT", project_root)
        .env("SIDEQUEST_BRANCH", branch.as_ref())
        .env("SIDEQUEST_BIN", &bin)
        .stdout(log_out)
        .stderr(log_err)
        .status()
        .await
        .map_err(|error| SessionError::Spawn(error.to_string()))?;

    if status.success() {
        Ok(())
    } else {
        Err(SessionError::Exit(format!(
            "session exited with {status}; call the logs tool for branch {} to see why",
            branch.as_ref()
        )))
    }
}
