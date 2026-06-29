//! Records describing launched side-quests, as kept in the registry.

use serde::{Deserialize, Serialize};

use crate::launch::{BranchName, Goal};

/// The lifecycle state of a side-quest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SideQuestState {
    /// Work is in progress.
    Running,
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
}
