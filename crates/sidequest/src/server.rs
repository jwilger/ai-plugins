//! The MCP server surface for the control plane.
//!
//! At this stage the server is *connectable* and advertises its identity and
//! capabilities; the operator and worker tool families are added outside-in by
//! later behavioral slices.

use rmcp::{
    ServerHandler,
    model::{ServerCapabilities, ServerInfo},
};

/// The sidequest control-plane MCP server.
///
/// Implements [`ServerHandler`]; served over stdio by the `sidequest-mcp`
/// binary so either harness can connect to it as an MCP client.
#[derive(Debug, Clone, Default)]
pub struct SidequestServer;

impl ServerHandler for SidequestServer {
    fn get_info(&self) -> ServerInfo {
        // Name and version are taken from the build environment (the `sidequest`
        // crate). Capabilities are empty until the tool families land.
        ServerInfo::new(ServerCapabilities::builder().build())
    }
}
