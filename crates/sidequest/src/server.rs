//! The MCP server surface for the control plane.
//!
//! At this stage the server is *connectable* and advertises its identity and
//! capabilities; the operator and worker tool families are added outside-in by
//! later behavioral slices.

use rmcp::{
    ServerHandler,
    model::{Implementation, ServerCapabilities, ServerInfo},
};

/// The sidequest control-plane MCP server.
///
/// Implements [`ServerHandler`]; served over stdio by the `sidequest-mcp`
/// binary so either harness can connect to it as an MCP client.
#[derive(Debug, Clone, Default)]
pub struct SidequestServer;

impl ServerHandler for SidequestServer {
    fn get_info(&self) -> ServerInfo {
        // `Implementation::from_build_env` reports rmcp's own crate name, so set
        // our identity explicitly. Capabilities stay empty until the tool
        // families land.
        let mut implementation = Implementation::from_build_env();
        implementation.name = String::from("sidequest");
        implementation.version = String::from(env!("CARGO_PKG_VERSION"));
        ServerInfo::new(ServerCapabilities::builder().build()).with_server_info(implementation)
    }
}
