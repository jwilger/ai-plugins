//! Delivering a side-quest's work back to the project (imperative shell).

use std::path::Path;

use sidequest_core::launch::BranchName;
use thiserror::Error;
use tokio::process::Command;

/// A failure delivering a side-quest's work.
#[derive(Debug, Error)]
pub enum DeliverError {
    /// `git` could not be spawned.
    #[error("deliver-spawn-failed: {0}")]
    Spawn(String),
    /// The git command exited non-zero.
    #[error("deliver-git-failed: {0}")]
    Git(String),
}

async fn git(project_root: &Path, args: &[&str]) -> Result<(), DeliverError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
        .await
        .map_err(|error| DeliverError::Spawn(error.to_string()))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(DeliverError::Git(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ))
    }
}

/// Merge the side-quest `branch` into the project's current branch (the local
/// integration target).
///
/// # Errors
///
/// Returns a [`DeliverError`] if git cannot be spawned or the merge fails.
pub async fn local_merge(project_root: &Path, branch: &BranchName) -> Result<(), DeliverError> {
    let branch_ref: &str = branch.as_ref();
    git(project_root, &["merge", "--no-edit", branch_ref]).await
}

/// Merge the side-quest `branch` into the current branch, then push it to the
/// origin integration branch.
///
/// # Errors
///
/// Returns a [`DeliverError`] if git cannot be spawned or the merge or push fails.
pub async fn push_origin(project_root: &Path, branch: &BranchName) -> Result<(), DeliverError> {
    local_merge(project_root, branch).await?;
    git(project_root, &["push", "origin", "HEAD"]).await
}
