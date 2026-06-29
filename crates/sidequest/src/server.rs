//! The MCP server surface for the control plane.
//!
//! Exposes the operator tools harnesses call. Worker tools (self-reporting) and
//! the remaining operator tools are added outside-in by later slices.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;
use sidequest_core::launch::{Goal, branch_for_goal};

use crate::{deliver, session, worktree};

/// Parameters for the `launch` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LaunchParams {
    /// The objective for the side-quest, in the user's own words.
    pub goal: String,
}

/// The sidequest control-plane MCP server, rooted at the repository its
/// side-quests operate on.
#[derive(Clone)]
pub struct SidequestServer {
    project_root: Arc<Path>,
    session_command: Option<Arc<str>>,
    delivery: Option<Arc<str>>,
}

#[tool_router]
impl SidequestServer {
    /// Build a server rooted at `project_root`. `session_command`, when present,
    /// is run (via `sh -c`) inside each new worktree as the goal session;
    /// `delivery` selects how the work is delivered (`"local-merge"`).
    #[must_use]
    pub fn new(
        project_root: PathBuf,
        session_command: Option<String>,
        delivery: Option<String>,
    ) -> Self {
        Self {
            project_root: Arc::from(project_root),
            session_command: session_command.map(Arc::from),
            delivery: delivery.map(Arc::from),
        }
    }

    /// Launch a side-quest: create an isolated git worktree on a fresh branch
    /// derived from the goal.
    #[tool(
        description = "Launch a side-quest: create an isolated git worktree on a fresh branch derived from the goal."
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

        if let Some(command) = self.session_command.as_deref() {
            session::run(&worktree_path, command, &goal)
                .await
                .map_err(|error| McpError::internal_error(error.to_string(), None))?;
        }

        if matches!(self.delivery.as_deref(), Some("local-merge")) {
            deliver::local_merge(self.project_root.as_ref(), &branch)
                .await
                .map_err(|error| McpError::internal_error(error.to_string(), None))?;
        }

        let payload = serde_json::json!({
            "branch": branch.as_ref(),
            "worktree_path": worktree_path.display().to_string(),
        });
        Ok(CallToolResult::success(vec![Content::text(
            payload.to_string(),
        )]))
    }
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
