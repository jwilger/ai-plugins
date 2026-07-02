//! Running a side-quest to completion in the background (imperative shell).
//!
//! Invoked by the detached `sidequest run-quest` worker: it resolves which
//! command drives the goal session, runs it inside the worktree, delivers per
//! the project's config, and always updates the registry record's state --
//! including on failure, so a side-quest never gets stuck at `Running` with
//! no explanation.

use std::path::{Path, PathBuf};

use sidequest_core::config::{Config, DeliveryMode};
use sidequest_core::harness::default_session_command;
use sidequest_core::launch::BranchName;
use sidequest_core::side_quest::{SideQuestRecord, SideQuestState};
use thiserror::Error;

use crate::{config, deliver, registry, session};

/// A failure executing a side-quest.
#[derive(Debug, Error)]
pub enum QuestError {
    /// No registry record exists for the branch.
    #[error("quest-not-found: {0}")]
    NotFound(String),
    /// A registry operation failed.
    #[error(transparent)]
    Registry(#[from] registry::RegistryError),
}

/// Run the side-quest on `branch` to completion: resolve its session command,
/// run the session, deliver the result, and record the final state.
///
/// # Errors
///
/// Returns a [`QuestError`] if the quest is unknown, or the registry itself
/// cannot be read or written -- there is then nowhere to record an outcome.
/// Every other failure (an unreadable `sidequest.toml`, the session, or
/// delivery) is captured as a `Failed` registry state instead of an `Err`,
/// since this runs unattended and nobody reads its exit code.
pub async fn execute(
    project_root: &Path,
    branch: &BranchName,
    session_command: Option<&str>,
) -> Result<(), QuestError> {
    let records = registry::list(project_root).await?;
    let record = records
        .into_iter()
        .find(|candidate| &candidate.branch == branch)
        .ok_or_else(|| QuestError::NotFound(branch.as_ref().to_owned()))?;

    let (state, detail) = match config::load(project_root).await {
        Ok(config) => {
            run_to_completion(project_root, branch, &record, session_command, &config).await
        }
        Err(error) => (
            SideQuestState::Failed,
            Some(format!("quest-config-failed: {error}")),
        ),
    };

    registry::record(
        project_root,
        SideQuestRecord {
            state,
            detail,
            ..record
        },
    )
    .await?;
    Ok(())
}

/// Resolve the session command (an explicit override, then the project's
/// `sidequest.toml` override, then the harness's built-in default), run it,
/// and deliver the result.
async fn run_to_completion(
    project_root: &Path,
    branch: &BranchName,
    record: &SideQuestRecord,
    session_command: Option<&str>,
    config: &Config,
) -> (SideQuestState, Option<String>) {
    let worktree = PathBuf::from(&record.worktree);

    let resolved_command = session_command
        .map(str::to_owned)
        .or_else(|| config.harness_command().map(str::to_owned))
        .or_else(|| {
            record
                .harness
                .as_deref()
                .and_then(default_session_command)
                .map(str::to_owned)
        });

    let Some(command) = resolved_command else {
        let harness = record.harness.as_deref().unwrap_or("(none)");
        return (
            SideQuestState::Failed,
            Some(format!(
                "no session command resolved for harness {harness:?}; set [harness] command in sidequest.toml"
            )),
        );
    };

    if let Err(error) = session::run(&worktree, &command, &record.goal, project_root, branch).await
    {
        return (SideQuestState::Failed, Some(error.to_string()));
    }

    // A transient failure checking for commits (e.g. a git lock held by a
    // concurrent side-quest) must not turn an otherwise-successful session
    // into a false `Failed` -- fall back to assuming there may be commits, the
    // same as if this check didn't exist at all.
    let has_commits = deliver::has_new_commits(project_root, branch)
        .await
        .unwrap_or(true);
    if !has_commits {
        return (SideQuestState::DoneNoChanges, None);
    }

    match config.delivery_mode() {
        Some(DeliveryMode::LocalMerge) => {
            deliver_result(deliver::local_merge(project_root, branch).await)
        }
        Some(DeliveryMode::PushOrigin) => {
            deliver_result(deliver::push_origin(project_root, branch).await)
        }
        Some(DeliveryMode::Pr) => deliver_result(deliver::push_branch(project_root, branch).await),
        None => (SideQuestState::Done, None),
    }
}

fn deliver_result(result: Result<(), deliver::DeliverError>) -> (SideQuestState, Option<String>) {
    match result {
        Ok(()) => (SideQuestState::Delivered, None),
        Err(error) => (SideQuestState::Failed, Some(error.to_string())),
    }
}

#[cfg(test)]
#[expect(clippy::expect_used, reason = "unit tests use expect() for clarity")]
mod tests {
    use sidequest_core::launch::Goal;

    use super::*;

    fn a_record(branch: &BranchName, worktree: &Path) -> SideQuestRecord {
        SideQuestRecord {
            goal: Goal::try_new("do the thing").expect("a valid goal"),
            branch: branch.clone(),
            worktree: worktree.display().to_string(),
            state: SideQuestState::Running,
            question: None,
            answer: None,
            harness: None,
            detail: None,
        }
    }

    #[tokio::test]
    async fn an_unreadable_config_marks_the_quest_failed_instead_of_hanging_at_running() {
        let dir = tempfile::tempdir().expect("a temp dir is creatable");
        let project_root = dir.path();
        let branch = BranchName::try_new("side-quest/do-the-thing").expect("a valid branch");
        registry::record(project_root, a_record(&branch, project_root))
            .await
            .expect("the registry is writable");

        // Make sidequest.toml a directory, so config::load fails to read it.
        std::fs::create_dir_all(project_root.join("sidequest.toml"))
            .expect("the config path can be made a directory");

        execute(project_root, &branch, Some("true"))
            .await
            .expect("execute should not error even when config is unreadable");

        let records = registry::list(project_root)
            .await
            .expect("the registry is readable");
        let record = records
            .into_iter()
            .find(|candidate| candidate.branch == branch)
            .expect("the record still exists");
        assert_eq!(
            record.state,
            SideQuestState::Failed,
            "an unreadable config should mark the quest failed, not leave it stuck at running"
        );
        assert!(
            record
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("quest-config-failed")),
            "the detail should explain the config failure: {:?}",
            record.detail
        );
    }

    #[tokio::test]
    async fn a_transient_commit_check_failure_does_not_mark_a_successful_session_failed() {
        let dir = tempfile::tempdir().expect("a temp dir is creatable");
        let project_root = dir.path(); // deliberately not a git repository
        let worktree = project_root.join("worktree");
        std::fs::create_dir_all(&worktree).expect("the worktree dir is creatable");

        let branch = BranchName::try_new("side-quest/do-the-thing").expect("a valid branch");
        let record = a_record(&branch, &worktree);
        let config = Config::default();

        let (state, detail) =
            run_to_completion(project_root, &branch, &record, Some("true"), &config).await;

        assert_eq!(
            state,
            SideQuestState::Done,
            "a commit-check failure (not a git repository) should not turn a successful \
             session into Failed; detail: {detail:?}"
        );
    }
}
