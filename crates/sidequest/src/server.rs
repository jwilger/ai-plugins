//! The MCP server surface for the control plane.
//!
//! Exposes the operator tools harnesses call. Launching a side-quest is
//! non-blocking: it creates the worktree, records the quest as running, and
//! spawns a detached worker that runs the session, delivers, and updates the
//! record. Worker (self-reporting) tools are added by later slices.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;
use sidequest_core::launch::{BranchName, Goal, branch_for_goal};
use sidequest_core::side_quest::{SideQuestRecord, SideQuestState};

use crate::{registry, steer, worktree};

/// Parameters for the `launch` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LaunchParams {
    /// The objective for the side-quest, in the user's own words.
    pub goal: String,
}

/// Parameters for the `answer` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AnswerParams {
    /// The side-quest's branch (its identifier).
    pub branch: String,
    /// The answer to the side-quest's pending question.
    pub answer: String,
}

/// The sidequest control-plane MCP server, rooted at the repository its
/// side-quests operate on.
#[derive(Clone)]
pub struct SidequestServer {
    project_root: Arc<Path>,
    session_command: Option<Arc<str>>,
}

#[tool_router]
impl SidequestServer {
    /// Build a server rooted at `project_root`. `session_command`, when present,
    /// is run (via `sh -c`) inside each worktree as the goal session.
    #[must_use]
    pub fn new(project_root: PathBuf, session_command: Option<String>) -> Self {
        Self {
            project_root: Arc::from(project_root),
            session_command: session_command.map(Arc::from),
        }
    }

    /// Launch a side-quest: create an isolated git worktree, record the quest as
    /// running, and start a detached background worker. Returns immediately.
    #[tool(
        description = "Launch a side-quest: create an isolated git worktree on a fresh branch derived from the goal, and run it in the background."
    )]
    async fn launch(
        &self,
        Parameters(params): Parameters<LaunchParams>,
    ) -> Result<CallToolResult, McpError> {
        let goal = Goal::try_new(params.goal)
            .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
        let branch = branch_for_goal(&goal)
            .map_err(|error| McpError::invalid_params(error.to_string(), None))?;

        let worktree_path = worktree::create(self.project_root.as_ref(), &branch)
            .await
            .map_err(|error| McpError::internal_error(error.to_string(), None))?;
        let worktree = worktree_path.display().to_string();

        registry::record(
            self.project_root.as_ref(),
            SideQuestRecord {
                goal: goal.clone(),
                branch: branch.clone(),
                worktree: worktree.clone(),
                state: SideQuestState::Running,
                question: None,
                answer: None,
            },
        )
        .await
        .map_err(|error| McpError::internal_error(error.to_string(), None))?;

        spawn_worker(
            self.project_root.as_ref(),
            &branch,
            self.session_command.as_deref(),
        )
        .map_err(|error| McpError::internal_error(error.to_string(), None))?;

        let payload = serde_json::json!({
            "branch": branch.as_ref(),
            "worktree_path": worktree,
            "state": "running",
        });
        Ok(CallToolResult::success(vec![Content::text(
            payload.to_string(),
        )]))
    }

    /// List the side-quests recorded for this project.
    #[tool(description = "List the side-quests recorded for this project.")]
    async fn list(&self) -> Result<CallToolResult, McpError> {
        let records = registry::list(self.project_root.as_ref())
            .await
            .map_err(|error| McpError::internal_error(error.to_string(), None))?;
        let value = serde_json::to_value(&records)
            .map_err(|error| McpError::internal_error(error.to_string(), None))?;
        Ok(CallToolResult::structured(value))
    }

    /// Answer a side-quest that is awaiting operator input.
    #[tool(description = "Answer a side-quest that is awaiting operator input.")]
    async fn answer(
        &self,
        Parameters(params): Parameters<AnswerParams>,
    ) -> Result<CallToolResult, McpError> {
        let branch = BranchName::try_new(params.branch)
            .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
        steer::answer(self.project_root.as_ref(), &branch, &params.answer)
            .await
            .map_err(|error| McpError::internal_error(error.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text("answered")]))
    }
}

/// Locate the sibling `sidequest` worker binary next to this executable, falling
/// back to the `PATH`.
fn worker_binary() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("sidequest")))
        .unwrap_or_else(|| PathBuf::from("sidequest"))
}

/// Spawn a detached `sidequest run-quest` worker for `branch`. The worker
/// outlives this server process.
fn spawn_worker(
    project_root: &Path,
    branch: &BranchName,
    session_command: Option<&str>,
) -> std::io::Result<()> {
    let mut command = std::process::Command::new(worker_binary());
    command
        .arg("run-quest")
        .arg("--project-root")
        .arg(project_root)
        .arg("--branch")
        .arg(branch.as_ref())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(session) = session_command {
        command.arg("--session-command").arg(session);
    }
    command.spawn().map(|_child| ())
}

#[tool_handler]
impl ServerHandler for SidequestServer {
    fn get_info(&self) -> ServerInfo {
        // `Implementation::from_build_env` reports rmcp's own crate name, so set
        // our identity explicitly.
        let mut implementation = Implementation::from_build_env();
        implementation.name = String::from("sidequest");
        implementation.version = String::from(env!("CARGO_PKG_VERSION"));
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(implementation)
    }
}
