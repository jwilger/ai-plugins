//! Git worktree operations (imperative shell).

use std::path::{Path, PathBuf};

use sidequest_core::launch::BranchName;
use thiserror::Error;
use tokio::process::Command;

/// A failure creating a worktree.
#[derive(Debug, Error)]
pub enum WorktreeError {
    /// `git` could not be spawned.
    #[error("worktree-spawn-failed: {0}")]
    Spawn(String),
    /// `git worktree add` exited non-zero.
    #[error("worktree-git-failed: {0}")]
    Git(String),
}

/// Create an isolated worktree for `branch` under the project's `.worktrees/`
/// directory, returning its path. The leaf directory is the final branch
/// segment.
///
/// # Errors
///
/// Returns [`WorktreeError`] if `git` cannot be spawned or `git worktree add`
/// fails.
pub async fn create(project_root: &Path, branch: &BranchName) -> Result<PathBuf, WorktreeError> {
    let branch_ref: &str = branch.as_ref();
    let leaf = branch_ref.rsplit('/').next().unwrap_or(branch_ref);
    let path = project_root.join(".worktrees").join(leaf);

    let output = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(["worktree", "add", "-b"])
        .arg(branch_ref)
        .arg(&path)
        .output()
        .await
        .map_err(|error| WorktreeError::Spawn(error.to_string()))?;

    if output.status.success() {
        Ok(path)
    } else {
        Err(WorktreeError::Git(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ))
    }
}
