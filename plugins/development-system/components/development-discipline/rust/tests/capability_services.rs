use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
};

use serde_json::{json, Value};
use tempfile::TempDir;

fn mcp_call(root: &std::path::Path, name: &str, arguments: Value) -> Value {
    mcp_call_with_surface(root, name, arguments, None)
}

fn mcp_call_with_surface(
    root: &std::path::Path,
    name: &str,
    arguments: Value,
    surface: Option<&str>,
) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_development-discipline-mcp"));
    if let Some(surface) = surface {
        command.env("DEVELOPMENT_SYSTEM_SERVICE", surface);
    }
    let mut child = command
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start MCP");
    let request = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } });
    writeln!(child.stdin.take().expect("stdin"), "{request}").expect("request");
    let mut line = String::new();
    BufReader::new(child.stdout.take().expect("stdout"))
        .read_line(&mut line)
        .expect("response");
    child.wait().expect("exit");
    serde_json::from_str(&line).expect("JSON response")
}

fn mcp_call_with_environment(
    root: &std::path::Path,
    name: &str,
    arguments: Value,
    environment: &[(&str, &std::path::Path)],
) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_development-discipline-mcp"));
    for (key, value) in environment {
        command.env(key, value);
    }
    let mut child = command
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start MCP");
    let request = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } });
    writeln!(child.stdin.take().expect("stdin"), "{request}").expect("request");
    let mut line = String::new();
    BufReader::new(child.stdout.take().expect("stdout"))
        .read_line(&mut line)
        .expect("response");
    child.wait().expect("exit");
    serde_json::from_str(&line).expect("JSON response")
}

fn hook_call(root: &std::path::Path, input: Value) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_development-discipline-mcp"))
        .args(["--hook", "codex-pre-tool-use"])
        .env(
            "DEVELOPMENT_SYSTEM_DISCIPLINE_LAUNCHER",
            env!("CARGO_BIN_EXE_development-discipline-mcp"),
        )
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start hook");
    writeln!(child.stdin.take().expect("stdin"), "{input}").expect("hook input");
    let mut output = String::new();
    BufReader::new(child.stdout.take().expect("stdout"))
        .read_line(&mut output)
        .expect("hook output");
    assert!(child.wait().expect("hook exit").success());
    serde_json::from_str(&output).expect("hook JSON")
}

fn setup_bootstrap_command(root: &std::path::Path) -> String {
    format!(
        "'{}' --setup-apply --project '{}' --confirmed",
        env!("CARGO_BIN_EXE_development-discipline-mcp"),
        root.display()
    )
}

fn record_codex_boundary_proof(root: &std::path::Path, configuration_digest: &str) {
    fs::create_dir_all(root.join(".development-system")).expect("proof directory");
    fs::write(
        root.join(".development-system/boundary-proof.codex.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_version": 1,
            "harness": "codex",
            "harness_version": "0.147.0",
            "plugin_version": env!("CARGO_PKG_VERSION"),
            "configuration_digest": configuration_digest,
            "recorded_at": 1u64,
            "expires_at": 4_000_000_000u64,
            "observations": {
                "stable_caller_identity": true,
                "per_agent_tool_filtering": true,
                "root_mutation_denied": true,
                "implementer_builtin_mutation_denied": true,
                "role_service_isolation": true
            }
        }))
        .expect("proof JSON"),
    )
    .expect("proof");
}

fn git(root: &std::path::Path, arguments: &[&str]) {
    let status = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .status()
        .expect("git");
    assert!(status.success(), "git {arguments:?}");
}

#[test]
fn semantic_reader_is_inert_without_configuration_and_invalid_config_fails_closed() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    let absent = mcp_call(
        root.path(),
        "workspace-reader.status",
        json!({ "project_root": root.path() }),
    );
    assert_eq!(
        absent.pointer("/result/structuredContent/activation"),
        Some(&json!("absent"))
    );
    assert_eq!(
        absent.pointer("/result/structuredContent/mutations_available"),
        Some(&json!(false))
    );
    let status = mcp_call(
        root.path(),
        "workspace-reader.repository",
        json!({ "project_root": root.path(), "view": "status" }),
    );
    assert_eq!(
        status.pointer("/result/structuredContent/view"),
        Some(&json!("status"))
    );
    for view in ["diff", "log"] {
        let response = mcp_call(
            root.path(),
            "workspace-reader.repository",
            json!({ "project_root": root.path(), "view": view }),
        );
        assert_eq!(
            response.pointer("/result/structuredContent/view"),
            Some(&json!(view))
        );
    }

    fs::write(
        root.path().join(".development-system.toml"),
        "schema_version = 99\n",
    )
    .expect("invalid config");
    let invalid = mcp_call(
        root.path(),
        "workspace-reader.status",
        json!({ "project_root": root.path() }),
    );
    assert_eq!(
        invalid.pointer("/result/structuredContent/activation"),
        Some(&json!("invalid"))
    );
    assert_eq!(
        invalid.pointer("/result/structuredContent/mutations_available"),
        Some(&json!(false))
    );
}

#[test]
fn setup_preview_reports_legacy_configuration_as_migration_required() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::write(
        root.path().join(".development-system.toml"),
        "schema_version = 2\n\n[delivery]\nmode = \"direct-to-trunk\"\n\n[features]\ntiber = true\n\n[worktrees]\nroot = \".worktrees\"\n\n[final_review.models.codex]\npre_filter = \"gpt-5.6-sol\"\n",
    )
    .expect("legacy configuration");

    let response = mcp_call(
        root.path(),
        "setup.preview",
        json!({ "project_root": root.path() }),
    );

    assert_eq!(
        response.pointer("/result/structuredContent/configuration_state"),
        Some(&json!("migration_required"))
    );
    assert_eq!(
        response.pointer("/result/structuredContent/existing_schema_version"),
        Some(&json!(2))
    );
    assert_eq!(
        response.pointer("/result/structuredContent/mutations_available"),
        Some(&json!(false))
    );
    let preview = response
        .pointer("/result/structuredContent/configuration")
        .and_then(Value::as_str)
        .expect("migration configuration");
    for retained in [
        "[delivery]",
        "mode = \"direct-to-trunk\"",
        "[features]",
        "tiber = true",
        "[worktrees]",
        "root = \".worktrees\"",
        "[final_review.models.codex]",
        "pre_filter = \"gpt-5.6-sol\"",
        "[scopes.source]",
    ] {
        assert!(
            preview.contains(retained),
            "missing retained policy: {retained}"
        );
    }
}

#[test]
fn setup_requires_confirmation_and_emits_read_only_role_capability_profiles() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    let rejected = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path() }),
    );
    assert_eq!(
        rejected.pointer("/error/message"),
        Some(&json!("development_system.setup_confirmation_required"))
    );
    let applied = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true }),
    );
    assert_eq!(
        applied.pointer("/result/structuredContent/applied"),
        Some(&json!(true))
    );
    assert_eq!(
        applied.pointer("/result/structuredContent/mutations_available"),
        Some(&json!(false))
    );
    assert_eq!(
        applied.pointer("/result/structuredContent/harness_mutations"),
        Some(&json!({ "claude": false, "codex": false }))
    );
    assert!(root.path().join(".development-system.toml").is_file());
    let policy = fs::read_to_string(root.path().join(".development-system/agents/README.md"))
        .expect("policy");
    assert!(policy.contains("not proved privileged MCP isolation"));
    for role in [
        "coordinator",
        "explorer",
        "system-diagnostician",
        "test-author",
        "implementer",
        "documentation-author",
        "environment-maintainer",
        "verifier",
        "reviewer",
        "delivery",
        "ci-recovery",
        "setup",
    ] {
        let claude = fs::read_to_string(
            root.path()
                .join(format!(".claude/agents/development-system-{role}.md")),
        )
        .expect("Claude role profile");
        let frontmatter = claude
            .split("---")
            .nth(1)
            .expect("Claude profile frontmatter");
        assert!(frontmatter.contains("tools: Read,Grep,Glob"));
        assert!(!frontmatter.contains("Bash"));
        assert!(!frontmatter.contains("Write"));
        assert!(!frontmatter.contains("Edit"));
        assert!(!claude.contains("workspace-editor"));
        assert!(!claude.contains("project-runner"));

        let codex = fs::read_to_string(
            root.path()
                .join(format!(".codex/agents/development-system-{role}.toml")),
        )
        .expect("Codex role profile");
        assert!(codex.contains("sandbox_mode = \"read-only\""));
        assert!(!codex.contains("workspace-write"));
    }

    let coordinator = fs::read_to_string(
        root.path()
            .join(".codex/agents/development-system-coordinator.toml"),
    )
    .expect("coordinator profile");
    assert!(coordinator.contains("[mcp_servers.workspace_reader]"));
    assert!(!coordinator.contains("[mcp_servers.workflow_core]"));
    assert!(!coordinator.contains("[mcp_servers.workspace_editor]"));

    let test_author = fs::read_to_string(
        root.path()
            .join(".codex/agents/development-system-test-author.toml"),
    )
    .expect("test-author profile");
    assert!(test_author.contains("[mcp_servers.workspace_reader]"));
    assert!(!test_author.contains("[mcp_servers.workspace_editor]"));
    assert!(!test_author.contains("[mcp_servers.project_runner]"));
    assert!(!test_author.contains("[mcp_servers.workflow_core]"));

    let claude_reviewer = fs::read_to_string(
        root.path()
            .join(".claude/agents/development-system-reviewer.md"),
    )
    .expect("Claude reviewer profile");
    assert!(claude_reviewer
        .contains("mcp__plugin_development-system_development-discipline__workspace-reader_read"));
    assert!(!claude_reviewer.contains("workspace-editor"));

    let configuration_before = fs::read_to_string(root.path().join(".development-system.toml"))
        .expect("configuration before profile refresh");
    fs::remove_file(
        root.path()
            .join(".codex/agents/development-system-reviewer.toml"),
    )
    .expect("remove generated profile");
    let refreshed = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true }),
    );
    assert_eq!(
        refreshed.pointer("/result/structuredContent/configuration_changed"),
        Some(&json!(false))
    );
    assert!(root
        .path()
        .join(".codex/agents/development-system-reviewer.toml")
        .is_file());
    assert_eq!(
        fs::read_to_string(root.path().join(".development-system.toml"))
            .expect("configuration after profile refresh"),
        configuration_before
    );
}

#[test]
fn fresh_codex_boundary_proof_generates_enforced_role_profiles() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    let first = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true }),
    );
    let digest = first
        .pointer("/result/structuredContent/configuration_digest")
        .and_then(Value::as_str)
        .expect("configuration digest");
    record_codex_boundary_proof(root.path(), digest);

    let proved_only = mcp_call(
        root.path(),
        "workspace-reader.status",
        json!({ "project_root": root.path() }),
    );
    assert_eq!(
        proved_only.pointer("/result/structuredContent/mutations_available"),
        Some(&json!(false))
    );
    assert!(proved_only
        .pointer("/result/structuredContent/remediation")
        .and_then(Value::as_str)
        .is_some_and(|message| message.contains("setup.apply")));

    let applied = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true }),
    );
    assert_eq!(
        applied.pointer("/result/structuredContent/harness_mutations/codex"),
        Some(&json!(true))
    );
    assert_eq!(
        applied.pointer("/result/structuredContent/mutations_available"),
        Some(&json!(true))
    );
    assert_eq!(
        applied.pointer("/result/structuredContent/codex_launch_command"),
        Some(&json!("./.development-system/codex"))
    );
    let launcher =
        fs::read_to_string(root.path().join(".development-system/codex")).expect("Codex launcher");
    assert!(launcher.contains("exec codex --enable multi_agent_v2 --sandbox read-only"));
    assert!(launcher.contains("--dangerously-bypass-approvals-and-sandbox"));

    let coordinator = fs::read_to_string(
        root.path()
            .join(".codex/agents/development-system-coordinator.toml"),
    )
    .expect("coordinator");
    assert!(coordinator.contains("sandbox_mode = \"read-only\""));
    assert!(coordinator.contains("[mcp_servers.workflow_core]"));
    assert!(coordinator.contains("GIT_SSH_COMMAND = \"ssh -F /dev/null\""));
    assert!(coordinator.contains("env_vars = [\"SSH_AUTH_SOCK\"]"));
    assert!(!coordinator.contains("[mcp_servers.workspace_editor]"));

    let implementer = fs::read_to_string(
        root.path()
            .join(".codex/agents/development-system-implementer.toml"),
    )
    .expect("implementer");
    assert!(implementer.contains("[mcp_servers.workspace_editor]"));
    assert!(implementer.contains("[mcp_servers.project_runner]"));
    assert!(implementer.contains("tool_timeout_sec = 900"));
    assert!(implementer.contains("sandbox_mode = \"read-only\""));
    assert_eq!(
        implementer
            .matches("env_vars = [\"SSH_AUTH_SOCK\"]")
            .count(),
        2
    );
    assert!(!implementer.contains("[mcp_servers.repository_local]"));

    let explorer = fs::read_to_string(
        root.path()
            .join(".codex/agents/development-system-explorer.toml"),
    )
    .expect("explorer");
    assert!(explorer.contains("sandbox_mode = \"read-only\""));

    let delivery = fs::read_to_string(
        root.path()
            .join(".codex/agents/development-system-delivery.toml"),
    )
    .expect("delivery");
    assert!(delivery.contains("[mcp_servers.repository_local]"));
    assert!(delivery.contains("[mcp_servers.repository_remote]"));
    assert_eq!(
        delivery.matches("env_vars = [\"SSH_AUTH_SOCK\"]").count(),
        2
    );
    assert!(!delivery.contains("[mcp_servers.workspace_editor]"));
}

#[test]
fn codex_hook_requires_named_agents_and_rejects_cross_role_mutation() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    let applied = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true }),
    );
    let digest = applied
        .pointer("/result/structuredContent/configuration_digest")
        .and_then(Value::as_str)
        .expect("configuration digest");
    record_codex_boundary_proof(root.path(), digest);
    mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true }),
    );

    let root_shell = hook_call(
        root.path(),
        json!({
            "session_id": "root", "turn_id": "turn", "cwd": root.path(),
            "hook_event_name": "PreToolUse", "model": "gpt-5.6-sol",
            "permission_mode": "workspace-write", "tool_name": "functions.exec_command",
            "tool_input": {}, "tool_use_id": "tool-root"
        }),
    );
    assert_eq!(
        root_shell.pointer("/hookSpecificOutput/permissionDecision"),
        Some(&json!("deny"))
    );

    let implementer_delivery = hook_call(
        root.path(),
        json!({
            "session_id": "root", "turn_id": "turn", "agent_id": "agent-1",
            "agent_type": "development-system-implementer", "cwd": root.path(),
            "hook_event_name": "PreToolUse", "model": "gpt-5.6-sol",
            "permission_mode": "read-only", "tool_name": "mcp__repository-remote__push-ref",
            "tool_input": {}, "tool_use_id": "tool-agent"
        }),
    );
    assert_eq!(
        implementer_delivery.pointer("/hookSpecificOutput/permissionDecision"),
        Some(&json!("deny"))
    );

    let implementer_edit = hook_call(
        root.path(),
        json!({
            "session_id": "root", "turn_id": "turn", "agent_id": "agent-1",
            "agent_type": "development-system-implementer", "cwd": root.path(),
            "hook_event_name": "PreToolUse", "model": "gpt-5.6-sol",
            "permission_mode": "read-only", "tool_name": "mcp__workspace-editor__patch",
            "tool_input": {}, "tool_use_id": "tool-agent"
        }),
    );
    assert_eq!(
        implementer_edit.pointer("/hookSpecificOutput/permissionDecision"),
        Some(&json!("allow"))
    );
}

#[test]
fn codex_hook_allows_only_the_exact_pending_setup_bootstrap() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    let first = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true }),
    );
    let digest = first
        .pointer("/result/structuredContent/configuration_digest")
        .and_then(Value::as_str)
        .expect("configuration digest");
    record_codex_boundary_proof(root.path(), digest);

    let exact = hook_call(
        root.path(),
        json!({
            "session_id": "root", "turn_id": "turn", "cwd": root.path(),
            "hook_event_name": "PreToolUse", "model": "gpt-5.6-sol",
            "permission_mode": "workspace-write", "tool_name": "functions.exec_command",
            "tool_input": { "cmd": setup_bootstrap_command(root.path()) },
            "tool_use_id": "tool-setup"
        }),
    );
    assert_eq!(
        exact.pointer("/hookSpecificOutput/permissionDecision"),
        Some(&json!("allow"))
    );

    let code_mode = hook_call(
        root.path(),
        json!({
            "session_id": "root", "turn_id": "turn", "cwd": root.path(),
            "hook_event_name": "PreToolUse", "model": "gpt-5.6-sol",
            "permission_mode": "workspace-write", "tool_name": "functions.exec",
            "tool_input": format!(
                "const r = await tools.exec_command({{cmd:{:?}}}); text(r)",
                setup_bootstrap_command(root.path())
            ),
            "tool_use_id": "tool-setup-code-mode"
        }),
    );
    assert_eq!(
        code_mode.pointer("/hookSpecificOutput/permissionDecision"),
        Some(&json!("allow"))
    );

    for command in [
        format!("{} --extra", setup_bootstrap_command(root.path())),
        format!(
            "'{}' --setup-apply --project '/tmp/other' --confirmed",
            env!("CARGO_BIN_EXE_development-discipline-mcp")
        ),
    ] {
        let denied = hook_call(
            root.path(),
            json!({
                "session_id": "root", "turn_id": "turn", "cwd": root.path(),
                "hook_event_name": "PreToolUse", "model": "gpt-5.6-sol",
                "permission_mode": "workspace-write", "tool_name": "functions.exec_command",
                "tool_input": { "cmd": command }, "tool_use_id": "tool-setup-invalid"
            }),
        );
        assert_eq!(
            denied.pointer("/hookSpecificOutput/permissionDecision"),
            Some(&json!("deny"))
        );
    }
}

#[test]
fn setup_bootstrap_cli_applies_the_confirmed_configuration() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    let first = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true }),
    );
    let digest = first
        .pointer("/result/structuredContent/configuration_digest")
        .and_then(Value::as_str)
        .expect("configuration digest");
    record_codex_boundary_proof(root.path(), digest);

    let output = Command::new(env!("CARGO_BIN_EXE_development-discipline-mcp"))
        .args([
            "--setup-apply",
            "--project",
            root.path().to_str().expect("UTF-8 root"),
            "--confirmed",
        ])
        .output()
        .expect("run setup bootstrap");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: Value = serde_json::from_slice(&output.stdout).expect("bootstrap JSON");
    assert_eq!(result.get("mutations_available"), Some(&json!(true)));
    assert!(root
        .path()
        .join(".development-system/activation.codex.json")
        .is_file());
}

#[test]
fn tampered_codex_profile_invalidates_activation_and_fails_closed() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    let first = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true }),
    );
    let digest = first
        .pointer("/result/structuredContent/configuration_digest")
        .and_then(Value::as_str)
        .expect("configuration digest");
    record_codex_boundary_proof(root.path(), digest);
    mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true }),
    );
    fs::write(
        root.path()
            .join(".codex/agents/development-system-implementer.toml"),
        "sandbox_mode = \"workspace-write\"\n",
    )
    .expect("tamper profile");

    let status = mcp_call(
        root.path(),
        "workspace-reader.status",
        json!({ "project_root": root.path() }),
    );
    assert_eq!(
        status.pointer("/result/structuredContent/mutations_available"),
        Some(&json!(false))
    );
    let editor = hook_call(
        root.path(),
        json!({
            "session_id": "root", "turn_id": "turn", "agent_id": "agent-1",
            "agent_type": "development-system-implementer", "cwd": root.path(),
            "hook_event_name": "PreToolUse", "model": "gpt-5.6-sol",
            "permission_mode": "read-only", "tool_name": "mcp__workspace-editor__patch",
            "tool_input": {}, "tool_use_id": "tool-agent"
        }),
    );
    assert_eq!(
        editor.pointer("/hookSpecificOutput/permissionDecision"),
        Some(&json!("deny"))
    );
}

#[test]
fn setup_probe_records_only_observed_codex_enforcement() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true }),
    );
    let fake_codex = root.path().join("fake-codex");
    fs::write(
        &fake_codex,
        "#!/usr/bin/env bash\nset -eu\nif [ \"${1:-}\" = --version ]; then printf 'codex-cli 0.147.0\\n'; exit 0; fi\nout=\nwhile [ \"$#\" -gt 0 ]; do if [ \"$1\" = -o ]; then out=$2; shift 2; else shift; fi; done\nprintf '%s\\n' '{\"root_mutation_denied\":true,\"implementer_builtin_mutation_denied\":true,\"implementer_workspace_editor_visible\":true,\"implementer_repository_remote_visible\":false,\"raw_error_labels\":[\"development_system.agent_capability_denied\",\"development_system.argument_required\"]}' >\"$out\"\n",
    )
    .expect("fake Codex");
    let mut permissions = fs::metadata(&fake_codex).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_codex, permissions).expect("executable");
    let auth_home = root.path().join("auth-home");
    fs::create_dir_all(&auth_home).expect("auth home");
    fs::write(auth_home.join("auth.json"), "{}\n").expect("auth");

    let probed = mcp_call_with_environment(
        root.path(),
        "setup.probe",
        json!({ "project_root": root.path(), "harness": "codex", "confirmed": true }),
        &[
            ("DEVELOPMENT_SYSTEM_CODEX_BIN", fake_codex.as_path()),
            ("CODEX_HOME", auth_home.as_path()),
        ],
    );
    assert_eq!(
        probed.pointer("/result/structuredContent/harness"),
        Some(&json!("codex"))
    );
    assert!(root
        .path()
        .join(".development-system/boundary-proof.codex.json")
        .is_file());
    let applied = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true }),
    );
    assert_eq!(
        applied.pointer("/result/structuredContent/harness_mutations/codex"),
        Some(&json!(true))
    );
}

#[test]
fn stale_or_configuration_mismatched_codex_proof_fails_closed() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    let first = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true }),
    );
    let digest = first
        .pointer("/result/structuredContent/configuration_digest")
        .and_then(Value::as_str)
        .expect("configuration digest");
    record_codex_boundary_proof(root.path(), digest);
    let path = root
        .path()
        .join(".development-system/boundary-proof.codex.json");
    let proof = fs::read_to_string(&path)
        .expect("proof")
        .replace(digest, "0000000000000000");
    fs::write(&path, proof).expect("tampered proof");

    let applied = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true }),
    );
    assert_eq!(
        applied.pointer("/result/structuredContent/mutations_available"),
        Some(&json!(false))
    );
    assert_eq!(
        applied.pointer("/result/structuredContent/harness_mutations/codex"),
        Some(&json!(false))
    );
    let root_shell = hook_call(
        root.path(),
        json!({
            "session_id": "root", "turn_id": "turn", "cwd": root.path(),
            "hook_event_name": "PreToolUse", "model": "gpt-5.6-sol",
            "permission_mode": "workspace-write", "tool_name": "functions.exec_command",
            "tool_input": {}, "tool_use_id": "tool-root"
        }),
    );
    assert_eq!(
        root_shell.pointer("/hookSpecificOutput/permissionDecision"),
        Some(&json!("deny"))
    );
}

#[test]
fn setup_apply_migrates_legacy_configuration_without_dropping_project_policy() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::create_dir_all(root.path().join(".agents")).expect("agents directory");
    fs::create_dir_all(root.path().join(".codex")).expect("codex directory");
    fs::write(root.path().join("justfile"), "ci:\n    true\n").expect("justfile");
    fs::write(
        root.path().join(".development-system.toml"),
        "schema_version = 2\n\n[delivery]\nmode = \"direct-to-trunk\"\ntrunk_branch = \"main\"\n\n[features]\ntiber = true\n\n[final_review.models.codex]\npre_filter = \"gpt-5.6-sol\"\n",
    )
    .expect("legacy configuration");
    let applied = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true, "selected_command_ids": ["just-ci"] }),
    );
    assert_eq!(
        applied.pointer("/result/structuredContent/applied"),
        Some(&json!(true)),
        "unexpected setup response: {applied}"
    );
    let migrated = fs::read_to_string(root.path().join(".development-system.toml"))
        .expect("migrated configuration");
    for retained in [
        "schema_version = 3",
        "[delivery]",
        "trunk_branch = \"main\"",
        "[features]",
        "tiber = true",
        "[final_review.models.codex]",
        "pre_filter = \"gpt-5.6-sol\"",
        "[scopes.source]",
    ] {
        assert!(
            migrated.contains(retained),
            "missing retained policy: {retained}"
        );
    }
    let status = mcp_call(
        root.path(),
        "workspace-reader.status",
        json!({ "project_root": root.path() }),
    );
    assert_eq!(
        status.pointer("/result/structuredContent/activation"),
        Some(&json!("configured"))
    );
}

#[test]
fn setup_preview_reports_unenabled_named_command_candidates() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::write(root.path().join("justfile"), "ci:\n    true\n").expect("justfile");
    fs::write(root.path().join("package.json"), "{}\n").expect("package");
    let preview = mcp_call(
        root.path(),
        "setup.preview",
        json!({ "project_root": root.path() }),
    );
    let candidates = preview
        .pointer("/result/structuredContent/detected_commands")
        .and_then(Value::as_array)
        .expect("candidates");
    assert!(candidates.iter().any(|candidate| candidate
        == &json!({
            "id": "just-ci",
            "argv": ["just", "ci"],
            "capability": "verification",
            "requires_confirmation": true
        })));
    assert!(candidates.iter().any(|candidate| candidate
        == &json!({
                "id": "npm-test",
                "argv": ["npm", "test"],
                "capability": "tests",
                "requires_confirmation": true
        })));
    assert_eq!(
        preview.pointer("/result/structuredContent/recommended_command_ids"),
        Some(&json!(["just-ci"]))
    );
}

#[test]
fn setup_preview_wraps_detected_commands_in_the_repository_nix_devshell() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::write(root.path().join("flake.nix"), "{}\n").expect("flake");
    fs::write(root.path().join("justfile"), "ci:\n    true\n").expect("justfile");
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname='fixture'\nversion='0.1.0'\n",
    )
    .expect("cargo manifest");
    fs::write(root.path().join("package.json"), "{}\n").expect("package");

    let preview = mcp_call(
        root.path(),
        "setup.preview",
        json!({ "project_root": root.path() }),
    );
    let candidates = preview
        .pointer("/result/structuredContent/detected_commands")
        .and_then(Value::as_array)
        .expect("candidates");

    for (id, command) in [
        ("just-ci", vec!["nix", "develop", "-c", "just", "ci"]),
        ("cargo-test", vec!["nix", "develop", "-c", "cargo", "test"]),
        ("npm-test", vec!["nix", "develop", "-c", "npm", "test"]),
    ] {
        assert!(candidates.iter().any(|candidate| {
            candidate.get("id") == Some(&json!(id))
                && candidate.get("argv") == Some(&json!(command))
        }));
    }
}

#[test]
fn setup_preview_rejects_a_linked_worktree_and_reports_the_primary_checkout() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::write(root.path().join("README.md"), "fixture\n").expect("fixture");
    git(root.path(), &["add", "README.md"]);
    git(
        root.path(),
        &[
            "-c",
            "user.name=Development System Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ],
    );
    let linked = root.path().join("linked");
    git(
        root.path(),
        &["worktree", "add", "--quiet", "-b", "linked-setup", "linked"],
    );

    let response = mcp_call(&linked, "setup.preview", json!({ "project_root": linked }));
    let error = response
        .pointer("/error/message")
        .and_then(Value::as_str)
        .expect("setup error");
    assert!(error.starts_with("development_system.setup_primary_checkout_required"));
    assert!(error.contains(root.path().to_string_lossy().as_ref()));
}

#[test]
fn setup_apply_enables_only_explicitly_selected_detected_commands() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::write(root.path().join("justfile"), "ci:\n    true\n").expect("justfile");
    fs::write(root.path().join("package.json"), "{}\n").expect("package");
    let rejected = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true, "selected_command_ids": ["git-push"] }),
    );
    assert_eq!(
        rejected.pointer("/error/message"),
        Some(&json!("development_system.setup_command_not_detected"))
    );
    let applied = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true, "selected_command_ids": ["npm-test"] }),
    );
    assert_eq!(
        applied.pointer("/result/structuredContent/applied"),
        Some(&json!(true))
    );
    let configuration =
        fs::read_to_string(root.path().join(".development-system.toml")).expect("configuration");
    assert!(configuration.contains("[commands.npm-test]"));
    assert!(configuration.contains("argv = [\"npm\", \"test\"]"));
    assert!(!configuration.contains("[commands.just-ci]"));
}

#[test]
fn setup_apply_refuses_an_unusable_empty_command_catalog() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::write(root.path().join("justfile"), "ci:\n    true\n").expect("justfile");

    let rejected = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true }),
    );

    assert_eq!(
        rejected.pointer("/error/message"),
        Some(&json!(
            "development_system.setup_command_selection_required"
        ))
    );
    assert!(!root.path().join(".development-system.toml").exists());
}

#[test]
fn setup_apply_can_add_a_detected_command_to_existing_configuration() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::write(root.path().join("justfile"), "ci:\n    true\n").expect("justfile");
    fs::write(
        root.path().join(".development-system.toml"),
        "schema_version = 3\n\n[scopes.source]\ncategory = \"source\"\ninclude = [\"src/**\"]\n",
    )
    .expect("configuration");

    let applied = mcp_call(
        root.path(),
        "setup.apply",
        json!({ "project_root": root.path(), "confirmed": true, "selected_command_ids": ["just-ci"] }),
    );

    assert_eq!(
        applied.pointer("/result/structuredContent/applied"),
        Some(&json!(true))
    );
    let configuration =
        fs::read_to_string(root.path().join(".development-system.toml")).expect("configuration");
    assert!(configuration.contains("[commands.just-ci]"));
}

#[test]
fn lifecycle_transition_invalidates_existing_editor_assignments() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::create_dir_all(root.path().join("src")).expect("source directory");
    fs::write(root.path().join("src/lib.rs"), "before").expect("source file");
    fs::write(
        root.path().join(".development-system.toml"),
        "schema_version = 3\n\n[scopes.source]\ncategory = \"source\"\ninclude = [\"src/**\"]\n",
    )
    .expect("configuration");

    let started = mcp_call_with_surface(
        root.path(),
        "workflow.start",
        json!({ "project_root": root.path(), "change_kind": "production" }),
        Some("workflow-core"),
    );
    assert_eq!(
        started.pointer("/result/structuredContent/phase"),
        Some(&json!("awaiting_red"))
    );
    let issued = mcp_call_with_surface(
        root.path(),
        "workflow.assignment.issue",
        json!({
            "project_root": root.path(),
            "assignment_id": "source-edit",
            "role": "implementer",
            "state_epoch": 1,
            "scope_ids": ["source"],
            "expires_at": 4_000_000_000u64
        }),
        Some("workflow-core"),
    );
    assert!(issued.get("result").is_some());
    let abandoned = mcp_call_with_surface(
        root.path(),
        "workflow.abandon",
        json!({ "project_root": root.path() }),
        Some("workflow-core"),
    );
    assert_eq!(
        abandoned.pointer("/result/structuredContent/phase"),
        Some(&json!("abandoned"))
    );

    let edit = mcp_call_with_surface(
        root.path(),
        "workspace-editor.replace",
        json!({
            "project_root": root.path(),
            "assignment_id": "source-edit",
            "role": "implementer",
            "scope_id": "source",
            "path": "src/lib.rs",
            "expected_digest": "0000000000000000",
            "content": "after"
        }),
        Some("workspace-editor"),
    );
    assert_eq!(
        edit.pointer("/error/message"),
        Some(&json!("development_system.assignment_stale_epoch"))
    );
}

#[test]
fn repository_local_checkpoint_is_semantic_and_delivery_fenced() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::create_dir_all(root.path().join("src")).expect("source directory");
    fs::write(
        root.path().join(".development-system.toml"),
        "schema_version = 3\n\n[scopes.source]\ncategory = \"source\"\ninclude = [\"src/**\"]\n",
    )
    .expect("configuration");
    let started = mcp_call_with_surface(
        root.path(),
        "workflow.start",
        json!({ "project_root": root.path(), "change_kind": "production" }),
        Some("workflow-core"),
    );
    assert!(started.get("result").is_some());
    let issued = mcp_call_with_surface(
        root.path(),
        "workflow.assignment.issue",
        json!({
            "project_root": root.path(),
            "assignment_id": "deliver",
            "role": "delivery",
            "state_epoch": 1,
            "scope_ids": [],
            "expires_at": 4_000_000_000u64
        }),
        Some("workflow-core"),
    );
    assert!(issued.get("result").is_some());
    let checkpoint = mcp_call_with_surface(
        root.path(),
        "repository-local.checkpoint",
        json!({
            "project_root": root.path(),
            "assignment_id": "deliver",
            "role": "delivery"
        }),
        Some("repository-local"),
    );
    assert!(checkpoint
        .pointer("/result/structuredContent/index_tree")
        .and_then(Value::as_str)
        .is_some_and(|tree| matches!(tree.len(), 40 | 64)));
    assert_eq!(
        checkpoint.pointer("/result/structuredContent/authorized_scope_ids"),
        Some(&json!([]))
    );
    assert!(checkpoint
        .pointer("/result/structuredContent/command_policy_digest")
        .and_then(Value::as_str)
        .is_some_and(|digest| !digest.is_empty()));
    assert_eq!(
        checkpoint.pointer("/result/structuredContent/evidence_ids"),
        Some(&json!([]))
    );
    assert_eq!(
        checkpoint.pointer("/result/structuredContent/predecessor"),
        Some(&Value::Null)
    );
    let successor = mcp_call_with_surface(
        root.path(),
        "repository-local.checkpoint",
        json!({
            "project_root": root.path(),
            "assignment_id": "deliver",
            "role": "delivery"
        }),
        Some("repository-local"),
    );
    assert_eq!(
        successor.pointer("/result/structuredContent/predecessor"),
        checkpoint.pointer("/result/structuredContent/id")
    );
    let denied = mcp_call_with_surface(
        root.path(),
        "repository-local.checkpoint",
        json!({
            "project_root": root.path(),
            "assignment_id": "deliver",
            "role": "implementer"
        }),
        Some("repository-local"),
    );
    assert_eq!(
        denied.pointer("/error/message"),
        Some(&json!("development_system.assignment_role_denied"))
    );
}

#[test]
fn failed_named_command_receipt_is_required_to_record_red() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::write(
        root.path().join(".development-system.toml"),
        "schema_version = 3\n\n[scopes.tests]\ncategory = \"tests\"\ninclude = [\"tests/**\"]\n\n[commands.red]\nargv = [\"/run/current-system/sw/bin/false\"]\ncapability = \"tests\"\n",
    )
    .expect("configuration");
    let started = mcp_call_with_surface(
        root.path(),
        "workflow.start",
        json!({ "project_root": root.path(), "change_kind": "production" }),
        Some("workflow-core"),
    );
    assert_eq!(
        started.pointer("/result/structuredContent/phase"),
        Some(&json!("awaiting_red"))
    );
    let assignment = mcp_call_with_surface(
        root.path(),
        "workflow.assignment.issue",
        json!({
            "project_root": root.path(),
            "assignment_id": "red-test",
            "role": "test_author",
            "state_epoch": 1,
            "scope_ids": ["tests"],
            "command_ids": ["red"],
            "expires_at": 4_000_000_000u64
        }),
        Some("workflow-core"),
    );
    assert!(assignment.get("result").is_some());
    let test_write = mcp_call_with_surface(
        root.path(),
        "workspace-editor.create",
        json!({
            "project_root": root.path(),
            "assignment_id": "red-test",
            "role": "test_author",
            "scope_id": "tests",
            "path": "tests/red.rs",
            "expected_digest": "cbf29ce484222325",
            "content": "#[test]\nfn expected_behavior() {}\n"
        }),
        Some("workspace-editor"),
    );
    assert!(test_write.get("result").is_some(), "{test_write}");
    let result = mcp_call_with_surface(
        root.path(),
        "project-runner.run",
        json!({
            "project_root": root.path(),
            "assignment_id": "red-test",
            "role": "test_author",
            "command_id": "red"
        }),
        Some("project-runner"),
    );
    assert_eq!(
        result.pointer("/result/structuredContent/succeeded"),
        Some(&json!(false))
    );
    let evidence_id = result
        .pointer("/result/structuredContent/evidence_id")
        .and_then(Value::as_str)
        .expect("durable failed-command receipt");
    let recorded = mcp_call_with_surface(
        root.path(),
        "workflow.record_red",
        json!({ "project_root": root.path(), "evidence_id": evidence_id }),
        Some("workflow-core"),
    );
    assert_eq!(
        recorded.pointer("/result/structuredContent/phase"),
        Some(&json!("awaiting_implementation_authorization"))
    );
    let staged = Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .current_dir(root.path())
        .output()
        .expect("inspect RED checkpoint");
    assert_eq!(
        String::from_utf8(staged.stdout).expect("paths"),
        "tests/red.rs\n"
    );
    assert!(Command::new("git")
        .args([
            "diff",
            "--quiet",
            "--cached",
            "--",
            ".development-system.toml"
        ])
        .current_dir(root.path())
        .status()
        .expect("inspect unrelated config index")
        .success());
}

#[test]
fn successful_named_command_receipt_is_required_to_record_green() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::write(
        root.path().join(".development-system.toml"),
        "schema_version = 3\n\n[scopes.tests]\ncategory = \"tests\"\ninclude = [\"tests/**\"]\n\n[scopes.source]\ncategory = \"source\"\ninclude = [\"src/**\"]\n\n[commands.red]\nargv = [\"/run/current-system/sw/bin/false\"]\ncapability = \"tests\"\n\n[commands.green]\nargv = [\"/run/current-system/sw/bin/true\"]\ncapability = \"implementation\"\n",
    )
    .expect("configuration");
    let call = |name: &str, arguments: Value, surface: &str| {
        mcp_call_with_surface(root.path(), name, arguments, Some(surface))
    };
    assert_eq!(
        call(
            "workflow.start",
            json!({ "project_root": root.path(), "change_kind": "production" }),
            "workflow-core",
        )
        .pointer("/result/structuredContent/phase"),
        Some(&json!("awaiting_red"))
    );
    assert!(call(
        "workflow.assignment.issue",
        json!({ "project_root": root.path(), "assignment_id": "red", "role": "test_author", "state_epoch": 1, "scope_ids": ["tests"], "command_ids": ["red"], "expires_at": 4_000_000_000u64 }),
        "workflow-core",
    )
    .get("result")
    .is_some());
    let red_write = call(
        "workspace-editor.create",
        json!({ "project_root": root.path(), "assignment_id": "red", "role": "test_author", "scope_id": "tests", "path": "tests/red.rs", "expected_digest": "cbf29ce484222325", "content": "#[test]\nfn expected_behavior() {}\n" }),
        "workspace-editor",
    );
    assert!(red_write.get("result").is_some(), "{red_write}");
    let red = call(
        "project-runner.run",
        json!({ "project_root": root.path(), "assignment_id": "red", "role": "test_author", "command_id": "red" }),
        "project-runner",
    );
    let red_receipt = red
        .pointer("/result/structuredContent/evidence_id")
        .and_then(Value::as_str)
        .expect("failed receipt");
    assert_eq!(
        call(
            "workflow.record_red",
            json!({ "project_root": root.path(), "evidence_id": red_receipt }),
            "workflow-core",
        )
        .pointer("/result/structuredContent/phase"),
        Some(&json!("awaiting_implementation_authorization"))
    );
    assert_eq!(
        call(
            "workflow.authorize_implementation",
            json!({ "project_root": root.path() }),
            "workflow-core",
        )
        .pointer("/result/structuredContent/phase"),
        Some(&json!("implementing"))
    );
    assert!(call(
        "workflow.assignment.issue",
        json!({ "project_root": root.path(), "assignment_id": "green", "role": "implementer", "state_epoch": 3, "scope_ids": ["source"], "command_ids": ["green"], "expires_at": 4_000_000_000u64 }),
        "workflow-core",
    )
    .get("result")
    .is_some());
    let green_write = call(
        "workspace-editor.create",
        json!({ "project_root": root.path(), "assignment_id": "green", "role": "implementer", "scope_id": "source", "path": "src/lib.rs", "expected_digest": "cbf29ce484222325", "content": "pub fn expected_behavior() {}\n" }),
        "workspace-editor",
    );
    assert!(green_write.get("result").is_some(), "{green_write}");
    let green = call(
        "project-runner.run",
        json!({ "project_root": root.path(), "assignment_id": "green", "role": "implementer", "command_id": "green" }),
        "project-runner",
    );
    let green_receipt = green
        .pointer("/result/structuredContent/evidence_id")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("successful receipt response={green}"));
    assert_ne!(
        green_receipt, red_receipt,
        "distinct semantic command operations must never reuse a receipt identity"
    );
    assert_eq!(
        call(
            "workflow.record_green",
            json!({ "project_root": root.path(), "evidence_id": green_receipt }),
            "workflow-core",
        )
        .pointer("/result/structuredContent/phase"),
        Some(&json!("awaiting_verification"))
    );
    let staged = Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .current_dir(root.path())
        .output()
        .expect("inspect GREEN checkpoint");
    assert_eq!(
        String::from_utf8(staged.stdout).expect("paths"),
        "src/lib.rs\ntests/red.rs\n"
    );
}

#[test]
fn checkpoint_abort_archives_implementation_and_preserves_unrelated_index_entries() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::write(
        root.path().join(".development-system.toml"),
        "schema_version = 3\n\n[scopes.tests]\ncategory = \"tests\"\ninclude = [\"tests/**\"]\n\n[scopes.source]\ncategory = \"source\"\ninclude = [\"src/**\"]\n\n[commands.red]\nargv = [\"/run/current-system/sw/bin/false\"]\ncapability = \"tests\"\n",
    )
    .expect("configuration");
    fs::write(root.path().join("notes.txt"), "unrelated staged content\n")
        .expect("unrelated content");
    git(
        root.path(),
        &["add", ".development-system.toml", "notes.txt"],
    );
    let call = |name: &str, arguments: Value, surface: &str| {
        mcp_call_with_surface(root.path(), name, arguments, Some(surface))
    };
    assert!(call(
        "workflow.start",
        json!({ "project_root": root.path(), "change_kind": "production" }),
        "workflow-core",
    )
    .get("result")
    .is_some());
    assert!(call(
        "workflow.assignment.issue",
        json!({ "project_root": root.path(), "assignment_id": "red-abort", "role": "test_author", "state_epoch": 1, "scope_ids": ["tests"], "command_ids": ["red"], "expires_at": 4_000_000_000u64 }),
        "workflow-core",
    )
    .get("result")
    .is_some());
    assert!(call(
        "workspace-editor.create",
        json!({ "project_root": root.path(), "assignment_id": "red-abort", "role": "test_author", "scope_id": "tests", "path": "tests/red.rs", "expected_digest": "cbf29ce484222325", "content": "#[test]\nfn wrong_contract() {}\n" }),
        "workspace-editor",
    )
    .get("result")
    .is_some());
    let red = call(
        "project-runner.run",
        json!({ "project_root": root.path(), "assignment_id": "red-abort", "role": "test_author", "command_id": "red" }),
        "project-runner",
    );
    let receipt = red
        .pointer("/result/structuredContent/evidence_id")
        .and_then(Value::as_str)
        .expect("RED receipt");
    assert!(call(
        "workflow.record_red",
        json!({ "project_root": root.path(), "evidence_id": receipt }),
        "workflow-core",
    )
    .get("result")
    .is_some());
    assert!(call(
        "workflow.authorize_implementation",
        json!({ "project_root": root.path() }),
        "workflow-core",
    )
    .get("result")
    .is_some());
    assert!(call(
        "workflow.assignment.issue",
        json!({ "project_root": root.path(), "assignment_id": "implementation-abort", "role": "implementer", "state_epoch": 3, "scope_ids": ["source"], "expires_at": 4_000_000_000u64 }),
        "workflow-core",
    )
    .get("result")
    .is_some());
    assert!(call(
        "workspace-editor.create",
        json!({ "project_root": root.path(), "assignment_id": "implementation-abort", "role": "implementer", "scope_id": "source", "path": "src/lib.rs", "expected_digest": "cbf29ce484222325", "content": "pub fn built_for_wrong_test() {}\n" }),
        "workspace-editor",
    )
    .get("result")
    .is_some());

    let preview = call(
        "repository-local.preview-checkpoint-abort",
        json!({ "project_root": root.path() }),
        "repository-local",
    );
    let operation = preview
        .pointer("/result/structuredContent")
        .cloned()
        .unwrap_or_else(|| panic!("rollback preview={preview}"));
    assert_eq!(operation["affected_paths"], json!(["src/lib.rs"]));
    fs::write(root.path().join("src/lib.rs"), "concurrent user edit\n").expect("concurrent edit");
    let refused = call(
        "repository-local.apply-checkpoint-abort",
        json!({ "project_root": root.path(), "confirmed": true, "operation": operation.clone() }),
        "repository-local",
    );
    assert_eq!(
        refused.pointer("/error/message"),
        Some(&json!(
            "development_system.checkpoint_abort_concurrent_change"
        ))
    );
    fs::write(
        root.path().join("src/lib.rs"),
        "pub fn built_for_wrong_test() {}\n",
    )
    .expect("restore previewed implementation");
    let applied = call(
        "repository-local.apply-checkpoint-abort",
        json!({ "project_root": root.path(), "confirmed": true, "operation": operation.clone() }),
        "repository-local",
    );
    assert!(applied.get("result").is_some(), "rollback apply={applied}");
    let replayed = call(
        "repository-local.apply-checkpoint-abort",
        json!({ "project_root": root.path(), "confirmed": true, "operation": operation }),
        "repository-local",
    );
    assert_eq!(replayed.get("result"), applied.get("result"));
    assert!(!root.path().join("src/lib.rs").exists());
    assert!(root.path().join("tests/red.rs").exists());
    let staged = Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .current_dir(root.path())
        .output()
        .expect("staged paths after rollback");
    assert_eq!(
        String::from_utf8(staged.stdout).expect("staged UTF-8"),
        ".development-system.toml\nnotes.txt\ntests/red.rs\n"
    );
    let archive = applied
        .pointer("/result/structuredContent/archive_relative_path")
        .and_then(Value::as_str)
        .expect("archive path");
    assert!(root
        .path()
        .join(".git")
        .join(archive)
        .join("files/src/lib.rs")
        .exists());
    let status = call(
        "workflow.status",
        json!({ "project_root": root.path() }),
        "workflow-core",
    );
    assert_eq!(
        status.pointer("/result/structuredContent/phase"),
        Some(&json!("awaiting_red"))
    );
}

#[test]
fn workflow_core_rejects_role_incompatible_assignment_scope() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::write(
        root.path().join(".development-system.toml"),
        "schema_version = 3\n\n[scopes.source]\ncategory = \"source\"\ninclude = [\"src/**\"]\n",
    )
    .expect("configuration");
    let started = mcp_call_with_surface(
        root.path(),
        "workflow.start",
        json!({ "project_root": root.path(), "change_kind": "production" }),
        Some("workflow-core"),
    );
    assert_eq!(
        started.pointer("/result/structuredContent/phase"),
        Some(&json!("awaiting_red"))
    );
    let assignment = mcp_call_with_surface(
        root.path(),
        "workflow.assignment.issue",
        json!({
            "project_root": root.path(),
            "assignment_id": "wrong-role",
            "role": "test_author",
            "state_epoch": 1,
            "scope_ids": ["source"],
            "expires_at": 4_000_000_000u64
        }),
        Some("workflow-core"),
    );
    assert_eq!(
        assignment.pointer("/error/message"),
        Some(&json!("development_system.assignment_scope_role_denied"))
    );
}

#[test]
fn workflow_assignment_uses_the_git_common_directory_from_a_linked_worktree() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::write(
        root.path().join(".development-system.toml"),
        "schema_version = 3\n\n[scopes.tests]\ncategory = \"tests\"\ninclude = [\"tests/**\"]\n",
    )
    .expect("configuration");
    git(root.path(), &["add", ".development-system.toml"]);
    git(
        root.path(),
        &[
            "-c",
            "user.name=Development System Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "test fixture",
        ],
    );
    let linked = root.path().join("linked");
    git(
        root.path(),
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "linked-worktree",
            "linked",
        ],
    );
    let started = mcp_call_with_surface(
        root.path(),
        "workflow.start",
        json!({ "project_root": root.path(), "change_kind": "production" }),
        Some("workflow-core"),
    );
    assert_eq!(
        started.pointer("/result/structuredContent/phase"),
        Some(&json!("awaiting_red"))
    );
    let assignment = mcp_call_with_surface(
        &linked,
        "workflow.assignment.issue",
        json!({
            "project_root": linked,
            "assignment_id": "linked-test",
            "role": "test_author",
            "state_epoch": 1,
            "scope_ids": ["tests"],
            "expires_at": 4_000_000_000u64
        }),
        Some("workflow-core"),
    );
    assert!(assignment.get("result").is_some());
}

#[test]
fn plugin_surface_hides_and_denies_privileged_tools_before_dispatch() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    let denied = mcp_call_with_surface(
        root.path(),
        "workflow.start",
        json!({ "project_root": root.path(), "change_kind": "production" }),
        Some("plugin-read-only"),
    );
    assert_eq!(
        denied.pointer("/error/message"),
        Some(&json!("development_system.service_capability_unavailable"))
    );

    let mut command = Command::new(env!("CARGO_BIN_EXE_development-discipline-mcp"));
    command.env("DEVELOPMENT_SYSTEM_SERVICE", "plugin-read-only");
    let mut child = command
        .current_dir(root.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start MCP");
    writeln!(
        child.stdin.take().expect("stdin"),
        "{}",
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" })
    )
    .expect("request");
    let mut line = String::new();
    BufReader::new(child.stdout.take().expect("stdout"))
        .read_line(&mut line)
        .expect("response");
    child.wait().expect("exit");
    let listed: Value = serde_json::from_str(&line).expect("JSON response");
    let names = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "workspace-reader.status",
            "workspace-reader.read",
            "workspace-reader.list",
            "workspace-reader.search",
            "workspace-reader.repository",
            "setup.preview",
            "setup.probe",
            "setup.apply"
        ]
    );
}

#[test]
fn direct_dispatcher_selects_the_requested_service_surface() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    let mut command = Command::new(env!("CARGO_BIN_EXE_development-discipline-mcp"));
    command
        .args(["--service", "workspace-editor"])
        .current_dir(root.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    let mut child = command.spawn().expect("start direct dispatcher");
    writeln!(
        child.stdin.take().expect("stdin"),
        "{}",
        json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" })
    )
    .expect("request");
    let mut line = String::new();
    BufReader::new(child.stdout.take().expect("stdout"))
        .read_line(&mut line)
        .expect("response");
    child.wait().expect("exit");
    let response: Value = serde_json::from_str(&line).expect("JSON response");
    let names = response["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        vec![
            "workspace-editor.create",
            "workspace-editor.patch",
            "workspace-editor.replace",
            "workspace-editor.delete",
            "workspace-editor.move"
        ]
    );
}
