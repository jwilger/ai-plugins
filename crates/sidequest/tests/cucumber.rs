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

use std::path::{Path, PathBuf};

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
    /// The structured result of the most recent `list` call.
    listing: Option<serde_json::Value>,
    /// A signal file a blocking session waits for.
    signal_path: Option<PathBuf>,
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
    let actual = wait_for(|| {
        let entry = std::fs::read_dir(&worktrees).ok()?.next()?.ok()?;
        std::fs::read_to_string(entry.path().join(&file)).ok()
    });
    assert_eq!(
        actual.as_deref(),
        Some(content.as_str()),
        "the session should record the goal into {file}"
    );
}

#[given("a project configured for local-merge delivery")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
fn configured_for_local_merge(world: &mut SideQuestWorld) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    std::fs::write(
        repo.path().join("sidequest.toml"),
        "[delivery]\nmode = \"local-merge\"\n",
    )
    .expect("the config file is writable");
}

#[given(expr = "a session runner that commits {string} with {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber step functions own their parsed parameters"
)]
fn a_session_runner_committing(world: &mut SideQuestWorld, file: String, content: String) {
    world.session_command = Some(format!(
        "printf '%s' '{content}' > {file} && git add {file} && git commit -q -m work"
    ));
}

#[then(expr = "the main checkout contains {string} with {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber step functions receive &mut World and own their parsed parameters"
)]
fn main_checkout_contains(world: &mut SideQuestWorld, file: String, content: String) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    let path = repo.path().join(&file);
    let actual = wait_for(|| std::fs::read_to_string(&path).ok());
    assert_eq!(
        actual.as_deref(),
        Some(content.as_str()),
        "the work should be delivered to the main checkout"
    );
}

#[when("the harness lists the side-quests")]
async fn lists(world: &mut SideQuestWorld) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    let binary = env!("CARGO_BIN_EXE_sidequest-mcp");
    let mut command = Command::new(binary);
    command.arg(repo.path());
    let transport =
        TokioChildProcess::new(command).expect("spawning the sidequest-mcp binary should succeed");
    let client = ().serve(transport).await.expect("the MCP initialize handshake should succeed");

    let result = client
        .call_tool(CallToolRequestParams::new("list"))
        .await
        .expect("the list tool call should succeed");
    world.listing = result.structured_content;

    client
        .cancel()
        .await
        .expect("the client should shut down cleanly");
}

#[then(expr = "the list includes a side-quest on branch {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber step functions receive &mut World and own their parsed parameters"
)]
fn list_includes_branch(world: &mut SideQuestWorld, branch: String) {
    let listing = world.listing.as_ref().expect("a listing was retrieved");
    let records = listing.as_array().expect("the listing is a JSON array");
    let found = records.iter().any(|record| {
        record.get("branch").and_then(serde_json::Value::as_str) == Some(branch.as_str())
    });
    assert!(
        found,
        "the list should include a side-quest on branch {branch}"
    );
}

#[given(expr = "a session runner that waits for a signal then commits {string} with {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber step functions own their parsed parameters"
)]
fn session_waits_then_commits(world: &mut SideQuestWorld, file: String, content: String) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    let signal = repo.path().join(".signal");
    world.session_command = Some(format!(
        "until [ -f '{signal}' ]; do sleep 0.05; done; printf '%s' '{content}' > {file} && git add {file} && git commit -q -m work",
        signal = signal.display()
    ));
    world.signal_path = Some(signal);
}

#[when("the side-quest is signaled to finish")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
fn signal_finish(world: &mut SideQuestWorld) {
    let signal = world.signal_path.as_ref().expect("a signal path was set");
    std::fs::write(signal, "").expect("the signal file is writable");
}

#[then(expr = "the side-quest {string} is running")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber step functions receive &mut World and own their parsed parameters"
)]
fn quest_is_running(world: &mut SideQuestWorld, branch: String) {
    let listing = world.listing.as_ref().expect("a listing was retrieved");
    let records = listing.as_array().expect("the listing is a JSON array");
    let found = records.iter().any(|record| {
        record.get("branch").and_then(serde_json::Value::as_str) == Some(branch.as_str())
            && record.get("state").and_then(serde_json::Value::as_str) == Some("running")
    });
    assert!(found, "the side-quest {branch} should be running");
}

fn wait_for<T>(probe: impl Fn() -> Option<T>) -> Option<T> {
    for _ in 0..200u32 {
        if let Some(value) = probe() {
            return Some(value);
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    probe()
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
