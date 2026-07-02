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
    /// A bare origin remote, kept alive for the scenario.
    remote: Option<tempfile::TempDir>,
    /// Whether the most recent guarded launch attempt was rejected.
    launch_error: Option<bool>,
    /// Whether the most recent `list` attempt surfaced an error.
    list_error: Option<bool>,
    /// When true, `SIDEQUEST_SESSION_COMMAND` is left entirely unset rather
    /// than defaulted to a no-op, so a scenario can exercise the config-level
    /// override or the "no command resolved" fallback. Only ever paired with
    /// a harness name unrecognized by sidequest's built-in defaults, so a real
    /// `claude`/`codex` invocation is never actually reachable.
    allow_unset_session_command: bool,
}

/// Build a `sidequest-mcp` command. Asserting the sibling `sidequest` worker
/// binary exists makes cargo build it as a prerequisite of this integration
/// test (the server resolves it as a sibling of `sidequest-mcp`), so the suite
/// can never spawn a missing or never-built worker.
fn sidequest_mcp_command() -> Command {
    assert!(
        std::path::Path::new(env!("CARGO_BIN_EXE_sidequest")).exists(),
        "the sidequest worker binary should be built for the test"
    );
    Command::new(env!("CARGO_BIN_EXE_sidequest-mcp"))
}

/// Set `SIDEQUEST_SESSION_COMMAND` on `command` per the scenario's
/// configuration. Defaults to a safe no-op (`true`) unless the scenario has
/// explicitly opted into leaving it unset (see
/// `allow_unset_session_command`), so a scenario can never accidentally fall
/// through to a harness's real built-in default command.
fn set_session_command_env(world: &SideQuestWorld, command: &mut Command) {
    match (&world.session_command, world.allow_unset_session_command) {
        (Some(session_command), _) => {
            command.env("SIDEQUEST_SESSION_COMMAND", session_command);
        }
        (None, true) => {}
        (None, false) => {
            command.env("SIDEQUEST_SESSION_COMMAND", "true");
        }
    }
}

#[given("no session command is configured anywhere")]
fn no_session_command_configured(world: &mut SideQuestWorld) {
    world.allow_unset_session_command = true;
}

#[when("a harness connects to the sidequest control plane over MCP")]
async fn connects(world: &mut SideQuestWorld) {
    let transport = TokioChildProcess::new(sidequest_mcp_command())
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
    let mut command = sidequest_mcp_command();
    command.arg(repo.path());
    set_session_command_env(world, &mut command);
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

#[then("an isolated worktree exists with a branch name short enough for git")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
fn worktree_branch_name_is_git_safe(world: &mut SideQuestWorld) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    let worktrees = repo.path().join(".worktrees");
    let entry = std::fs::read_dir(&worktrees)
        .expect("the worktrees directory exists")
        .next()
        .expect("a worktree was created")
        .expect("the worktree entry is readable");
    assert!(entry.path().is_dir(), "the worktree entry is a directory");
    let head = git_stdout(&entry.path(), &["rev-parse", "--abbrev-ref", "HEAD"]);
    let branch = head.trim();
    assert!(
        branch.len() <= 100,
        "the branch name {branch:?} ({} chars) should be well within git's ref-name limits",
        branch.len()
    );
    assert!(
        branch.starts_with("side-quest/"),
        "the branch should still carry the side-quest/ prefix: {branch:?}"
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
    let repo = world
        .repo
        .as_ref()
        .expect("a git repository exists")
        .path()
        .to_owned();
    world.listing = Some(fetch_listing(&repo).await);
}

#[then(expr = "the list includes a side-quest on branch {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber step functions receive &mut World and own their parsed parameters"
)]
fn list_includes_branch(world: &mut SideQuestWorld, branch: String) {
    let listing = world.listing.as_ref().expect("a listing was retrieved");
    let records = side_quests(listing);
    let found = records.iter().any(|record| {
        record.get("branch").and_then(serde_json::Value::as_str) == Some(branch.as_str())
    });
    assert!(
        found,
        "the list should include a side-quest on branch {branch}"
    );
}

#[given("a project that allows cross-harness spawning")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
fn allows_cross_harness(world: &mut SideQuestWorld) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    std::fs::write(
        repo.path().join("sidequest.toml"),
        "[harness]\ndefault = \"claude\"\nallow_cross = true\n",
    )
    .expect("the config file is writable");
}

#[when(expr = "a harness launches a side-quest with the goal {string} targeting {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
async fn launches_targeting(world: &mut SideQuestWorld, goal: String, harness: String) {
    let ok = try_launch(world, &goal, Some(&harness)).await;
    assert!(ok, "launch targeting {harness} should succeed");
}

#[when(expr = "a harness tries to launch a side-quest with the goal {string} targeting {string}")]
async fn tries_launch_targeting(world: &mut SideQuestWorld, goal: String, harness: String) {
    let ok = try_launch(world, &goal, Some(&harness)).await;
    world.launch_error = Some(!ok);
}

#[when(expr = "a harness tries to launch a side-quest with the goal {string}")]
async fn tries_launch(world: &mut SideQuestWorld, goal: String) {
    let ok = try_launch(world, &goal, None).await;
    world.launch_error = Some(!ok);
}

#[given("the config path is a directory")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
fn config_path_is_a_directory(world: &mut SideQuestWorld) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    std::fs::create_dir_all(repo.path().join("sidequest.toml"))
        .expect("the config path can be made a directory");
}

#[given("the registry path is a directory")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
fn registry_path_is_a_directory(world: &mut SideQuestWorld) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    let registry = repo
        .path()
        .join(".git")
        .join("sidequest")
        .join("registry.json");
    std::fs::create_dir_all(&registry).expect("the registry path can be made a directory");
}

#[when("the harness tries to list the side-quests")]
async fn tries_to_list(world: &mut SideQuestWorld) {
    let repo = world
        .repo
        .as_ref()
        .expect("a git repository exists")
        .path()
        .to_owned();
    let ok = list_call_succeeds(&repo).await;
    world.list_error = Some(!ok);
}

#[then("listing the side-quests fails")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
fn listing_fails(world: &mut SideQuestWorld) {
    assert_eq!(
        world.list_error,
        Some(true),
        "listing should surface an error for a corrupt registry, not silently report none"
    );
}

#[then(expr = "the side-quest {string} targets harness {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber step functions receive &mut World and own their parsed parameters"
)]
fn targets_harness(world: &mut SideQuestWorld, branch: String, harness: String) {
    let listing = world.listing.as_ref().expect("a listing was retrieved");
    let records = side_quests(listing);
    let found = records.iter().any(|record| {
        record.get("branch").and_then(serde_json::Value::as_str) == Some(branch.as_str())
            && record.get("harness").and_then(serde_json::Value::as_str) == Some(harness.as_str())
    });
    assert!(
        found,
        "the side-quest {branch} should target harness {harness}"
    );
}

#[then("the launch is rejected")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
fn launch_rejected(world: &mut SideQuestWorld) {
    assert_eq!(
        world.launch_error,
        Some(true),
        "the cross-harness launch should be rejected"
    );
}

async fn try_launch(world: &SideQuestWorld, goal: &str, harness: Option<&str>) -> bool {
    let repo = world.repo.as_ref().expect("a git repository exists");
    let mut command = sidequest_mcp_command();
    command.arg(repo.path());
    set_session_command_env(world, &mut command);
    let transport =
        TokioChildProcess::new(command).expect("spawning the sidequest-mcp binary should succeed");
    let client = ().serve(transport).await.expect("the MCP initialize handshake should succeed");

    let mut arguments = serde_json::Map::new();
    arguments.insert(
        "goal".to_owned(),
        serde_json::Value::String(goal.to_owned()),
    );
    if let Some(harness) = harness {
        arguments.insert(
            "harness".to_owned(),
            serde_json::Value::String(harness.to_owned()),
        );
    }
    let outcome = client
        .call_tool(CallToolRequestParams::new("launch").with_arguments(arguments))
        .await;
    client
        .cancel()
        .await
        .expect("the client should shut down cleanly");
    outcome.is_ok()
}

#[given("a bare origin remote")]
fn a_bare_origin_remote(world: &mut SideQuestWorld) {
    let remote = tempfile::tempdir().expect("a temp dir is creatable");
    run_git(remote.path(), &["init", "--bare", "-q"]);
    let repo = world.repo.as_ref().expect("a git repository exists");
    let remote_path = remote.path().to_string_lossy().into_owned();
    run_git(repo.path(), &["remote", "add", "origin", &remote_path]);
    run_git(repo.path(), &["push", "-q", "origin", "HEAD"]);
    world.remote = Some(remote);
}

#[given("a project configured for push-origin delivery")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
fn configured_for_push_origin(world: &mut SideQuestWorld) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    std::fs::write(
        repo.path().join("sidequest.toml"),
        "[delivery]\nmode = \"push-origin\"\n",
    )
    .expect("the config file is writable");
}

#[then(expr = "the origin integration branch contains {string} with {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber step functions receive &mut World and own their parsed parameters"
)]
fn origin_contains(world: &mut SideQuestWorld, file: String, content: String) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    let head = git_stdout(repo.path(), &["rev-parse", "--abbrev-ref", "HEAD"]);
    let branch = head.trim();
    let actual = wait_for(|| {
        let output = std::process::Command::new("git")
            .current_dir(repo.path())
            .args(["show", &format!("origin/{branch}:{file}")])
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
    });
    assert_eq!(
        actual.as_deref(),
        Some(content.as_str()),
        "the work should be on the origin integration branch"
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
    let records = side_quests(listing);
    let found = records.iter().any(|record| {
        record.get("branch").and_then(serde_json::Value::as_str) == Some(branch.as_str())
            && record.get("state").and_then(serde_json::Value::as_str) == Some("running")
    });
    assert!(found, "the side-quest {branch} should be running");
}

#[then(expr = "the side-quest {string} is delivered")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
async fn quest_is_delivered(world: &mut SideQuestWorld, branch: String) {
    let repo = world
        .repo
        .as_ref()
        .expect("a git repository exists")
        .path()
        .to_owned();
    let mut found = false;
    for _ in 0..200u32 {
        let listing = fetch_listing(&repo).await;
        if side_quests(&listing).iter().any(|record| {
            record.get("branch").and_then(serde_json::Value::as_str) == Some(branch.as_str())
                && record.get("state").and_then(serde_json::Value::as_str) == Some("delivered")
        }) {
            found = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(found, "the side-quest {branch} should be delivered");
}

#[then(expr = "the side-quest {string} has state {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
async fn quest_has_state(world: &mut SideQuestWorld, branch: String, state: String) {
    let repo = world
        .repo
        .as_ref()
        .expect("a git repository exists")
        .path()
        .to_owned();
    let mut found = false;
    for _ in 0..200u32 {
        let listing = fetch_listing(&repo).await;
        if side_quests(&listing).iter().any(|record| {
            record.get("branch").and_then(serde_json::Value::as_str) == Some(branch.as_str())
                && record.get("state").and_then(serde_json::Value::as_str) == Some(state.as_str())
        }) {
            found = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(found, "the side-quest {branch} should have state {state}");
}

#[then(expr = "the side-quest {string} has a detail containing {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
async fn quest_has_detail_containing(
    world: &mut SideQuestWorld,
    branch: String,
    substring: String,
) {
    let repo = world
        .repo
        .as_ref()
        .expect("a git repository exists")
        .path()
        .to_owned();
    let mut found = false;
    for _ in 0..200u32 {
        let listing = fetch_listing(&repo).await;
        if side_quests(&listing).iter().any(|record| {
            record.get("branch").and_then(serde_json::Value::as_str) == Some(branch.as_str())
                && record
                    .get("detail")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|detail| detail.contains(&substring))
        }) {
            found = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(
        found,
        "the side-quest {branch} should have a detail containing {substring:?}"
    );
}

#[given(expr = "a project configured with harness command {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber step functions receive &mut World and own their parsed parameters"
)]
fn configured_with_harness_command(world: &mut SideQuestWorld, command: String) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    std::fs::write(
        repo.path().join("sidequest.toml"),
        format!("[harness]\nallow_cross = true\ncommand = \"{command}\"\n"),
    )
    .expect("the config file is writable");
}

#[given(expr = "a session runner that fails with {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber step functions own their parsed parameters"
)]
fn a_session_runner_that_fails(world: &mut SideQuestWorld, message: String) {
    world.session_command = Some(format!("echo '{message}' >&2; exit 1"));
}

#[given(expr = "a session runner that asks {string} and records the answer to {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber step functions own their parsed parameters"
)]
fn session_asks_and_records(world: &mut SideQuestWorld, question: String, file: String) {
    world.session_command = Some(format!(
        "answer=$(\"$SIDEQUEST_BIN\" ask --project-root \"$SIDEQUEST_PROJECT_ROOT\" --branch \"$SIDEQUEST_BRANCH\" --question '{question}'); printf '%s' \"$answer\" > {file}"
    ));
}

#[then(expr = "the side-quest {string} is awaiting input with question {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
async fn awaiting_input(world: &mut SideQuestWorld, branch: String, question: String) {
    let repo = world
        .repo
        .as_ref()
        .expect("a git repository exists")
        .path()
        .to_owned();
    let mut found = false;
    for _ in 0..200u32 {
        let listing = fetch_listing(&repo).await;
        if side_quests(&listing).iter().any(|record| {
            record.get("branch").and_then(serde_json::Value::as_str) == Some(branch.as_str())
                && record.get("state").and_then(serde_json::Value::as_str) == Some("awaiting-input")
                && record.get("question").and_then(serde_json::Value::as_str)
                    == Some(question.as_str())
        }) {
            found = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(
        found,
        "the side-quest {branch} should be awaiting input with question {question}"
    );
}

#[when(expr = "the operator answers {string} to {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
async fn operator_answers(world: &mut SideQuestWorld, answer: String, branch: String) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    let mut command = sidequest_mcp_command();
    command.arg(repo.path());
    let transport =
        TokioChildProcess::new(command).expect("spawning the sidequest-mcp binary should succeed");
    let client = ().serve(transport).await.expect("the MCP initialize handshake should succeed");

    let mut arguments = serde_json::Map::new();
    arguments.insert("branch".to_owned(), serde_json::Value::String(branch));
    arguments.insert("answer".to_owned(), serde_json::Value::String(answer));
    let result = client
        .call_tool(CallToolRequestParams::new("answer").with_arguments(arguments))
        .await
        .expect("the answer tool call should succeed");
    assert_ne!(
        result.is_error,
        Some(true),
        "answer should not report an error"
    );

    client
        .cancel()
        .await
        .expect("the client should shut down cleanly");
}

/// Connect to `sidequest-mcp` for `repo`, call `tool` with `arguments`, and
/// return the raw result -- callers decide how to interpret success (which
/// can fail either as an `Err` here, e.g. a protocol-level error response, or
/// as `Ok(result)` with `result.is_error == Some(true)`), content, or
/// structured content.
async fn call_tool(
    repo: &Path,
    tool: &'static str,
    arguments: serde_json::Map<String, serde_json::Value>,
) -> Result<rmcp::model::CallToolResult, rmcp::service::ServiceError> {
    let mut command = sidequest_mcp_command();
    command.arg(repo);
    let transport =
        TokioChildProcess::new(command).expect("spawning the sidequest-mcp binary should succeed");
    let client = ().serve(transport).await.expect("the MCP initialize handshake should succeed");
    let result = client
        .call_tool(CallToolRequestParams::new(tool).with_arguments(arguments))
        .await;
    client
        .cancel()
        .await
        .expect("the client should shut down cleanly");
    result
}

async fn list_call_succeeds(repo: &Path) -> bool {
    call_tool(repo, "list", serde_json::Map::new())
        .await
        .is_ok_and(|outcome| outcome.is_error != Some(true))
}

async fn fetch_listing(repo: &Path) -> serde_json::Value {
    call_tool(repo, "list", serde_json::Map::new())
        .await
        .expect("the list tool call should succeed")
        .structured_content
        .unwrap_or_else(|| serde_json::json!({ "side_quests": [] }))
}

/// The `side_quests` array out of a `list` tool result's structured content.
/// The server wraps the listing in an object because MCP structured content
/// must be a JSON object, never a bare array.
fn side_quests(listing: &serde_json::Value) -> &[serde_json::Value] {
    listing
        .get("side_quests")
        .and_then(serde_json::Value::as_array)
        .expect("the list result's structured content should be an object with a side_quests array")
}

#[given(expr = "a session runner that prints {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber step functions own their parsed parameters"
)]
fn a_session_runner_that_prints(world: &mut SideQuestWorld, message: String) {
    world.session_command = Some(format!("echo '{message}'"));
}

#[given("a session runner that writes a very large log")]
fn a_session_runner_that_writes_a_large_log(world: &mut SideQuestWorld) {
    world.session_command = Some("head -c 400000 /dev/zero | tr '\\0' 'A'".to_owned());
}

#[then(expr = "the side-quest {string}'s log is at most 300000 characters")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
async fn quest_log_is_bounded(world: &mut SideQuestWorld, branch: String) {
    let repo = world
        .repo
        .as_ref()
        .expect("a git repository exists")
        .path()
        .to_owned();
    let mut logs = String::new();
    for _ in 0..200u32 {
        logs = fetch_logs(&repo, &branch).await;
        if !logs.is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(
        logs.len() <= 300_000,
        "the logs tool should never return more than a bounded tail, got {} characters",
        logs.len()
    );
}

#[then(expr = "the side-quest {string}'s log contains {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
async fn quest_log_contains(world: &mut SideQuestWorld, branch: String, substring: String) {
    let repo = world
        .repo
        .as_ref()
        .expect("a git repository exists")
        .path()
        .to_owned();
    let mut found = false;
    let mut last_seen = String::new();
    for _ in 0..200u32 {
        let logs = fetch_logs(&repo, &branch).await;
        if logs.contains(&substring) {
            found = true;
            break;
        }
        last_seen = logs;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(
        found,
        "the side-quest {branch}'s log should contain {substring:?}; last seen: {last_seen:?}"
    );
}

#[then(expr = "the side-quest {string}'s log is empty")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
async fn quest_log_is_empty(world: &mut SideQuestWorld, branch: String) {
    let repo = world
        .repo
        .as_ref()
        .expect("a git repository exists")
        .path()
        .to_owned();
    let logs = fetch_logs(&repo, &branch).await;
    assert_eq!(
        logs, "",
        "a side-quest with no log file yet should return an empty log, not error"
    );
}

#[given(expr = "the log path for {string} is a directory")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber step functions receive &mut World and own their parsed parameters"
)]
fn log_path_is_a_directory(world: &mut SideQuestWorld, branch: String) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    let file_name = format!("{}.log", branch.replace('/', "-"));
    let path = repo
        .path()
        .join(".git")
        .join("sidequest")
        .join("logs")
        .join(file_name);
    std::fs::create_dir_all(&path).expect("the log path can be made a directory");
}

#[then(expr = "reading the logs for {string} fails")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
async fn reading_logs_fails(world: &mut SideQuestWorld, branch: String) {
    let repo = world
        .repo
        .as_ref()
        .expect("a git repository exists")
        .path()
        .to_owned();
    let ok = logs_call_succeeds(&repo, &branch).await;
    assert!(
        !ok,
        "reading logs for a directory-shaped log path should surface an error, not silently \
         return an empty log"
    );
}

fn branch_argument(branch: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut arguments = serde_json::Map::new();
    arguments.insert(
        "branch".to_owned(),
        serde_json::Value::String(branch.to_owned()),
    );
    arguments
}

async fn logs_call_succeeds(repo: &Path, branch: &str) -> bool {
    call_tool(repo, "logs", branch_argument(branch))
        .await
        .is_ok_and(|outcome| outcome.is_error != Some(true))
}

async fn fetch_logs(repo: &Path, branch: &str) -> String {
    call_tool(repo, "logs", branch_argument(branch))
        .await
        .expect("the logs tool call should succeed")
        .content
        .first()
        .and_then(|content| content.as_text())
        .map(|text| text.text.clone())
        .unwrap_or_default()
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

#[given("a project configured for PR delivery")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "cucumber step functions receive &mut World"
)]
fn configured_for_pr(world: &mut SideQuestWorld) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    std::fs::write(
        repo.path().join("sidequest.toml"),
        "[delivery]\nmode = \"pr\"\n",
    )
    .expect("the config file is writable");
}

#[then(expr = "the origin branch {string} contains {string} with {string}")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber step functions receive &mut World and own their parsed parameters"
)]
fn origin_branch_contains(
    world: &mut SideQuestWorld,
    branch: String,
    file: String,
    content: String,
) {
    let repo = world.repo.as_ref().expect("a git repository exists");
    let actual = wait_for(|| {
        let output = std::process::Command::new("git")
            .current_dir(repo.path())
            .args(["show", &format!("origin/{branch}:{file}")])
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
    });
    assert_eq!(
        actual.as_deref(),
        Some(content.as_str()),
        "origin branch {branch} should contain {file}"
    );
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
