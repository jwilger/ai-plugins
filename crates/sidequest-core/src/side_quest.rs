//! Records describing launched side-quests, as kept in the registry.

use serde::{Deserialize, Serialize};

use crate::launch::{BranchName, Goal};

/// The lifecycle state of a side-quest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SideQuestState {
    /// Work is in progress.
    Running,
    /// Blocked, waiting for the operator to answer a question.
    AwaitingInput,
    /// The work was delivered per the project's delivery mode.
    Delivered,
    /// The work finished, produced commits, but the project has no delivery
    /// mode configured (left on its branch).
    Done,
    /// The goal session ran to completion but produced no new commits, so
    /// there was nothing to deliver.
    DoneNoChanges,
    /// The session or delivery step failed; see `detail`.
    Failed,
}

/// A registry record for one launched side-quest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideQuestRecord {
    /// The side-quest's goal.
    pub goal: Goal,
    /// The branch its work lands on (also its identifier).
    pub branch: BranchName,
    /// The worktree path (a filesystem boundary value).
    pub worktree: String,
    /// Its lifecycle state.
    pub state: SideQuestState,
    /// The question it is awaiting an answer to, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    /// The operator's answer to a pending question, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    /// The harness the side-quest runs in, if specified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness: Option<String>,
    /// Human-readable context for `Failed` or `DoneNoChanges` states (e.g. why
    /// the session failed, or that no session command could be resolved).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}
