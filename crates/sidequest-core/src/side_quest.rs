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
    /// The work finished without delivery (left on its branch).
    Done,
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
}
