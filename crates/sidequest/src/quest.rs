//! Running a side-quest to completion in the background (imperative shell).
//!
//! Invoked by the detached `sidequest run-quest` worker: it runs the goal
//! session inside the worktree, delivers per the project's config, and updates
//! the registry record's state.

use std::path::{Path, PathBuf};

use sidequest_core::config::DeliveryMode;
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
    /// The goal session failed.
    #[error(transparent)]
    Session(#[from] session::SessionError),
    /// Delivery failed.
    #[error(transparent)]
    Deliver(#[from] deliver::DeliverError),
    /// Loading config failed.
    #[error("quest-config-failed: {0}")]
    Config(String),
}

/// Run the side-quest on `branch` to completion: session, delivery, and the
/// final registry state update.
///
/// # Errors
///
/// Returns a [`QuestError`] if the quest is unknown or any step fails.
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

    let worktree = PathBuf::from(&record.worktree);

    if let Some(command) = session_command {
        session::run(&worktree, command, &record.goal).await?;
    }

    let delivery = config::load(project_root)
        .await
        .map_err(|error| QuestError::Config(error.to_string()))?
        .delivery_mode();
    let delivered = matches!(delivery, Some(DeliveryMode::LocalMerge));
    if delivered {
        deliver::local_merge(project_root, branch).await?;
    }

    let state = if delivered {
        SideQuestState::Delivered
    } else {
        SideQuestState::Done
    };
    registry::record(project_root, SideQuestRecord { state, ..record }).await?;
    Ok(())
}
