//! Steering a running side-quest: the `ask` / `answer` protocol, mediated by the
//! shared registry (imperative shell).
//!
//! The worker calls [`ask`] (it posts a question and blocks until answered). The
//! operator calls [`answer`] (it records the answer). Both go through the
//! registry, so they work across processes.

use std::path::Path;
use std::time::Duration;

use sidequest_core::launch::BranchName;
use sidequest_core::side_quest::{SideQuestRecord, SideQuestState};
use thiserror::Error;

use crate::registry;

/// A failure steering a side-quest.
#[derive(Debug, Error)]
pub enum SteerError {
    /// No registry record exists for the branch.
    #[error("steer-not-found: {0}")]
    NotFound(String),
    /// A registry operation failed.
    #[error(transparent)]
    Registry(#[from] registry::RegistryError),
}

async fn modify(
    project_root: &Path,
    branch: &BranchName,
    change: impl FnOnce(SideQuestRecord) -> SideQuestRecord,
) -> Result<(), SteerError> {
    let record = find(project_root, branch).await?;
    registry::record(project_root, change(record)).await?;
    Ok(())
}

async fn find(project_root: &Path, branch: &BranchName) -> Result<SideQuestRecord, SteerError> {
    registry::list(project_root)
        .await?
        .into_iter()
        .find(|candidate| &candidate.branch == branch)
        .ok_or_else(|| SteerError::NotFound(branch.as_ref().to_owned()))
}

/// Worker side: post `question` for `branch`, block until the operator answers,
/// then clear the question and return the answer.
///
/// # Errors
///
/// Returns a [`SteerError`] if the quest is unknown or a registry operation fails.
pub async fn ask(
    project_root: &Path,
    branch: &BranchName,
    question: &str,
) -> Result<String, SteerError> {
    modify(project_root, branch, |mut record| {
        record.state = SideQuestState::AwaitingInput;
        record.question = Some(question.to_owned());
        record.answer = None;
        record
    })
    .await?;

    loop {
        let record = find(project_root, branch).await?;
        if let Some(answer) = record.answer.clone() {
            modify(project_root, branch, |mut record| {
                record.state = SideQuestState::Running;
                record.question = None;
                record.answer = None;
                record
            })
            .await?;
            return Ok(answer);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Operator side: record `answer` for `branch` (the worker picks it up).
///
/// # Errors
///
/// Returns a [`SteerError`] if the quest is unknown or a registry operation fails.
pub async fn answer(
    project_root: &Path,
    branch: &BranchName,
    answer: &str,
) -> Result<(), SteerError> {
    modify(project_root, branch, |mut record| {
        record.answer = Some(answer.to_owned());
        record
    })
    .await
}
