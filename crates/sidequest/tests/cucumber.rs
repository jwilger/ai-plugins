#![expect(
    clippy::expect_used,
    reason = "acceptance tests use expect() for assertion clarity"
)]
//! Black-box acceptance tests for the sidequest control plane, driven by
//! Cucumber.
//!
//! Scenarios exercise only the public surface — the `sidequest-mcp` binary spoken
//! to as a real MCP client over stdio — never internal types. Cross-harness
//! behaviors will use `Examples: codex, claude`.

use std::path::Path;

use cucumber::{World, given, then, when};
use rmcp::ServiceExt as _;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::TokioChildProcess;
use tokio::process::Command;

/// State shared between steps of a scenario.
#[derive(Debug, Default, World)]
struct SideQuestWorld {
    /// The server identity reported during the MCP handshake.
    server_name: Option<String>,
    /// A temporary git repository the side-quest operates on.
    repo: Option<tempfile::TempDir>,
    /// The session command to run inside the worktree, if any.
    session_command: Option<String>,
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

#[given("a git repository")]
fn a_git_repository(world: &mut SideQuestWorld) {
    let repo = tempfile::tempdir().expect("a temp dir is creatable");
    let root = repo.path();
    run_git(root, &["init", "-q"]);
    run_git(root, &["config", "user.email", "test@example.com"]);
    run_git(root, &["config", "user.name", "sidequest tests"]);
    run_git(root, &["config", "commit.gpgsign", "false"]);
    std::fs::write(root.join("README.md"), "seed\n").expect("the seed file is writable");
    run_git(root, &["add", "README.md"]);
    run_git(root, &["commit", "-q", "-m", "seed"]);
    world.repo = Some(repo);
}

#[when(expr = "a harness launches a side-quest with the goal {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
async fn launches(world: &mut SideQuestWorld, goal: String) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    let binary = env!("CARGO_BIN_EXE_sidequest-mcp");
    let mut command = Command::new(binary);
    command.arg(repo.path());
    if let Some(session_command) = world.session_command.as_deref() {
        command.env("SIDEQUEST_SESSION_COMMAND", session_command);
    }
    let transport =
        TokioChildProcess::new(command).expect("spawning the sidequest-mcp binary should succeed");
    let client = ().serve(transport).await.expect("the MCP initialize handshake should succeed");

    let mut arguments = serde_json::Map::new();
    arguments.insert("goal".to_owned(), serde_json::Value::String(goal));
    let result = client
        .call_tool(CallToolRequestParams::new("launch").with_arguments(arguments))
        .await
        .expect("the launch tool call should succeed");
    assert_ne!(
        result.is_error,
        Some(true),
        "launch should not report an error"
    );

    client
        .cancel()
        .await
        .expect("the client should shut down cleanly");
}

#[then(expr = "an isolated worktree exists on branch {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber step functions receive &mut World and own their parsed parameters"
)]
fn worktree_exists(world: &mut SideQuestWorld, branch: String) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    let leaf = branch.rsplit('/').next().unwrap_or(branch.as_str());
    let worktree = repo.path().join(".worktrees").join(leaf);
    assert!(
        worktree.is_dir(),
        "the worktree {} should exist",
        worktree.display()
    );
    let head = git_stdout(&worktree, &["rev-parse", "--abbrev-ref", "HEAD"]);
    assert_eq!(
        head.trim(),
        branch,
        "the worktree should be checked out on the side-quest branch"
    );
}

#[given(expr = "a session runner that records the goal to {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber step functions own their parsed parameters"
)]
fn a_session_runner_recording_to(world: &mut SideQuestWorld, file: String) {
    world.session_command = Some(format!("printf '%s' \"$SIDEQUEST_GOAL\" > {file}"));
}

#[then(expr = "the worktree contains {string} with {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber step functions receive &mut World and own their parsed parameters"
)]
fn worktree_contains(world: &mut SideQuestWorld, file: String, content: String) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    let worktrees = repo.path().join(".worktrees");
    let entry = std::fs::read_dir(&worktrees)
        .expect("the worktrees directory exists")
        .next()
        .expect("a worktree was created")
        .expect("the worktree entry is readable");
    let actual = std::fs::read_to_string(entry.path().join(&file))
        .expect("the recorded file exists in the worktree");
    assert_eq!(actual, content, "the session recorded the goal into {file}");
}

fn run_git(dir: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .expect("git should be runnable");
    assert!(status.success(), "git {args:?} should succeed");
}

fn git_stdout(dir: &Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("git should be runnable");
    assert!(output.status.success(), "git {args:?} should succeed");
    String::from_utf8(output.stdout).expect("git output should be utf-8")
}

#[tokio::main]
async fn main() {
    SideQuestWorld::cucumber()
        .run_and_exit("tests/features")
        .await;
}
