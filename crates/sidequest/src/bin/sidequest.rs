//! `sidequest` — the CLI surface (secondary) and the background `run-quest`
//! worker spawned by the control plane.

use std::path::PathBuf;

use anyhow::Context as _;
use clap::{Parser, Subcommand};
use sidequest_core::launch::BranchName;

#[derive(Parser)]
#[command(name = "sidequest", about = "Side-quest control-plane CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run a launched side-quest to completion. Spawned by the control plane;
    /// not normally invoked by hand.
    RunQuest {
        /// The project the side-quest operates on.
        #[arg(long)]
        project_root: PathBuf,
        /// The side-quest's branch (its identifier).
        #[arg(long)]
        branch: String,
        /// The session command to run inside the worktree, if any.
        #[arg(long)]
        session_command: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::RunQuest {
            project_root,
            branch,
            session_command,
        } => {
            let branch = BranchName::try_new(branch)
                .map_err(|error| anyhow::anyhow!("invalid branch: {error}"))?;
            sidequest::quest::execute(&project_root, &branch, session_command.as_deref())
                .await
                .context("running the side-quest")
        }
    }
}
