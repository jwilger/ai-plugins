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

/// Run `git` with `args` in `project_root`, returning its stdout.
async fn git(project_root: &Path, args: &[&str]) -> Result<String, DeliverError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
        .await
        .map_err(|error| DeliverError::Spawn(error.to_string()))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(DeliverError::Git(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ))
    }
}

/// Whether `branch` has any commits not reachable from the project's current
/// `HEAD` — i.e. whether its goal session actually produced work worth
/// delivering.
///
/// # Errors
///
/// Returns a [`DeliverError`] if git cannot be spawned or the check fails.
pub async fn has_new_commits(
    project_root: &Path,
    branch: &BranchName,
) -> Result<bool, DeliverError> {
    let branch_ref: &str = branch.as_ref();
    let stdout = git(
        project_root,
        &["rev-list", "--count", &format!("HEAD..{branch_ref}")],
    )
    .await?;
    Ok(parse_commit_count(&stdout)? > 0)
}

/// Parse `git rev-list --count`'s stdout. A non-numeric result (which should
/// never happen on a successful `rev-list --count`, but would otherwise be
/// silently treated as "zero commits") is surfaced as an error instead.
fn parse_commit_count(stdout: &str) -> Result<u64, DeliverError> {
    stdout.trim().parse().map_err(|error| {
        DeliverError::Git(format!("unexpected rev-list output {stdout:?}: {error}"))
    })
}

/// Merge the side-quest `branch` into the project's current branch (the local
/// integration target).
///
/// # Errors
///
/// Returns a [`DeliverError`] if git cannot be spawned or the merge fails.
pub async fn local_merge(project_root: &Path, branch: &BranchName) -> Result<(), DeliverError> {
    let branch_ref: &str = branch.as_ref();
    git(project_root, &["merge", "--no-edit", branch_ref]).await?;
    Ok(())
}

/// Merge the side-quest `branch` into the current branch, then push it to the
/// origin integration branch.
///
/// # Errors
///
/// Returns a [`DeliverError`] if git cannot be spawned or the merge or push fails.
pub async fn push_origin(project_root: &Path, branch: &BranchName) -> Result<(), DeliverError> {
    local_merge(project_root, branch).await?;
    git(project_root, &["push", "origin", "HEAD"]).await?;
    Ok(())
}

/// Push the side-quest `branch` to origin as a feature branch (without merging
/// it), so a pull/merge request can be opened for it (by the babysit-pr skill).
///
/// # Errors
///
/// Returns a [`DeliverError`] if git cannot be spawned or the push fails.
pub async fn push_branch(project_root: &Path, branch: &BranchName) -> Result<(), DeliverError> {
    let branch_ref: &str = branch.as_ref();
    git(project_root, &["push", "origin", branch_ref]).await?;
    Ok(())
}

#[cfg(test)]
#[expect(clippy::expect_used, reason = "unit tests use expect() for clarity")]
mod tests {
    use super::*;

    #[test]
    fn parses_a_valid_count() {
        assert_eq!(
            parse_commit_count("5\n").expect("a numeric count parses"),
            5,
            "rev-list --count's trailing newline is trimmed"
        );
    }

    #[test]
    fn unexpected_output_is_an_error_not_a_silent_zero() {
        assert!(
            parse_commit_count("fatal: ambiguous argument").is_err(),
            "non-numeric rev-list output must not be silently treated as zero commits"
        );
    }
}
