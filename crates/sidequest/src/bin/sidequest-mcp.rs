//! `sidequest-mcp` — the MCP stdio server surface for the control plane.
//!
//! This is the PRIMARY surface: harnesses (Claude Code, Codex) connect to it and
//! invoke operator / worker tools. At this stage the server is connectable and
//! advertises its identity; the tool families are added by later slices.

use std::path::PathBuf;

use anyhow::Context as _;
use rmcp::ServiceExt as _;
use sidequest::server::SidequestServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let project_root = std::env::args().nth(1).map_or_else(
        || std::env::current_dir().context("the current directory must be accessible"),
        |arg| Ok(PathBuf::from(arg)),
    )?;

    let session_command = std::env::var("SIDEQUEST_SESSION_COMMAND").ok();
    let delivery = sidequest::config::load(&project_root)
        .await
        .context("loading sidequest.toml")?
        .delivery_mode();

    let service = SidequestServer::new(project_root, session_command, delivery)
        .serve(rmcp::transport::stdio())
        .await
        .context("MCP handshake failed")?;

    service
        .waiting()
        .await
        .context("MCP server exited with error")?;

    Ok(())
}

/// Send tracing to stderr; stdout is reserved for the MCP stdio channel.
fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();
}
