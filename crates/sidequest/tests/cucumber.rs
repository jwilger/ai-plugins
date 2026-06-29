#![expect(
    clippy::expect_used,
    reason = "acceptance tests use expect() for assertion clarity"
)]
//! Black-box acceptance tests for the sidequest control plane, driven by
//! Cucumber.
//!
//! Scenarios exercise only the public surface — here, connecting to the
//! `sidequest-mcp` binary as a real MCP client over stdio — never internal
//! types. Cross-harness behaviors will use `Examples: codex, claude`.

use cucumber::{World, then, when};
use rmcp::ServiceExt as _;
use rmcp::transport::TokioChildProcess;
use tokio::process::Command;

/// State shared between steps of a scenario.
#[derive(Debug, Default, World)]
struct SideQuestWorld {
    /// The server identity reported during the MCP handshake.
    server_name: Option<String>,
}

#[when("a harness connects to the sidequest control plane over MCP")]
async fn connects(world: &mut SideQuestWorld) {
    let binary = env!("CARGO_BIN_EXE_sidequest-mcp");
    let transport = TokioChildProcess::new(Command::new(binary))
        .expect("spawning the sidequest-mcp binary should succeed");

    let client = ().serve(transport).await.expect("the MCP initialize handshake should succeed");

    let info = client
        .peer_info()
        .expect("the server must report handshake info");
    world.server_name = Some(info.server_info.name.clone());

    client
        .cancel()
        .await
        .expect("the client should shut down cleanly");
}

#[then(expr = "the control plane identifies itself as {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber step functions receive &mut World and own their parsed parameters"
)]
fn identifies_as(world: &mut SideQuestWorld, expected: String) {
    assert_eq!(
        world.server_name.as_deref(),
        Some(expected.as_str()),
        "the control plane should identify itself as {expected}"
    );
}

#[tokio::main]
async fn main() {
    SideQuestWorld::cucumber()
        .run_and_exit("tests/features")
        .await;
}
