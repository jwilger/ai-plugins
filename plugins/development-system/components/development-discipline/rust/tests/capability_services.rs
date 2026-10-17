use std::{
    fs,
    io::{BufRead, BufReader, Write},
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

fn git(root: &std::path::Path, arguments: &[&str]) {
    let status = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .status()
        .expect("git");
    assert!(status.success(), "git {arguments:?}");
    if arguments.first() == Some(&"init") {
        let status = Command::new("git")
            .args(["config", "commit.gpgsign", "false"])
            .current_dir(root)
            .status()
            .expect("disable signing in test repository");
        assert!(status.success(), "disable signing in test repository");
    }
}

#[test]
fn portable_mcp_requires_an_explicit_repository_when_sandbox_metadata_is_absent() {
    let plugin_root = TempDir::new().expect("plugin root");
    git(plugin_root.path(), &["init", "--quiet"]);
    fs::write(plugin_root.path().join("plugin.json"), "{}\n").expect("plugin manifest");
    let response = mcp_call_with_environment(
        plugin_root.path(),
        "workspace-reader.status",
        json!({}),
        &[("PLUGIN_ROOT", plugin_root.path())],
    );
    assert_eq!(
        response.pointer("/error/message"),
        Some(&json!(
            "development_workflow.project_root_required source=mcp_sandbox_cwd"
        ))
    );
}

fn test_executable(program: &str) -> String {
    std::env::var_os("PATH")
        .and_then(|path| {
            std::env::split_paths(&path)
                .map(|directory| directory.join(program))
                .find(|candidate| candidate.is_file())
        })
        .unwrap_or_else(|| panic!("test executable {program} should be available on PATH"))
        .display()
        .to_string()
}

fn assert_current_final_review_protocol(status: &Value) {
    assert_eq!(
        status.pointer("/result/structuredContent/final_review_protocol"),
        Some(&json!({
            "contract_version": 3,
            "minimum_clean_iterations": 1,
            "scoped_coverage": true,
            "explicit_policy_migration": true,
            "legacy_contract_versions": [2],
            "durable_pending_assignment_recovery": true
        })),
        "unexpected final-review protocol attestation: {status}"
    );
}

fn sqlite_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut found = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).expect("read state directory") {
            let path = entry.expect("state entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "sqlite")
            {
                found.push(path);
            }
        }
    }
    found
}

fn mcp_text_payload(response: &Value) -> Value {
    serde_json::from_str(
        response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .expect("MCP text payload"),
    )
    .expect("MCP payload JSON")
}

#[test]
fn advisory_final_review_resume_rebuilds_deleted_sqlite_projection_from_git_authority() {
    let root = TempDir::new().expect("repository");
    let state = TempDir::new().expect("state root");
    git(root.path(), &["init", "--quiet"]);
    fs::create_dir(root.path().join("src")).expect("source directory");
    fs::write(root.path().join("src/review.rs"), "const VALUE: u8 = 1;\n").expect("initial source");
    fs::write(
        root.path().join(".development-system.toml"),
        "schema_version = 3\n\n[delivery]\nmode = \"direct-to-trunk\"\ntrunk_branch = \"main\"\n\n[features]\ntiber = true\n\n[final_review.models.codex]\npre_filter = \"gpt-6-astra\"\nlens_review = \"gpt-6-sol\"\npost_filter = \"gpt-6-luna\"\nverifier = \"gpt-6-astra\"\n\n[scopes.source]\ncategory = \"source\"\ninclude = [\"src/**\"]\n",
    )
    .expect("configuration");
    git(root.path(), &["add", "."]);
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
    let baseline = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(root.path())
            .output()
            .expect("baseline")
            .stdout,
    )
    .expect("baseline UTF-8")
    .trim()
    .to_string();
    fs::write(root.path().join("src/review.rs"), "const VALUE: u8 = 2;\n").expect("changed source");

    let session_id = "production-resume-rebuild";
    let risk_arguments = json!({
        "session_id": session_id,
        "base": "HEAD",
        "baseline_commit": baseline,
        "scope": "uncommitted",
        "project_root": root.path(),
        "changed_files": ["src/review.rs"],
        "diff_hash": "production-resume-diff",
        "shared_test_evidence": {
            "id": "production-resume-tests",
            "diff_hash": "production-resume-diff",
            "status": "passed",
            "summary": "Fixture verification passed.",
            "commands": ["fixture:verification"],
            "artifact_reference": "fixture://production-resume"
        },
        "pre_filter_model_role": "gpt-6-astra"
    });
    let risk_response = mcp_call_with_environment(
        root.path(),
        "final_review.assess_risk",
        risk_arguments.clone(),
        &[("XDG_STATE_HOME", state.path())],
    );
    let risk = mcp_text_payload(&risk_response);
    let assignment = &risk["assignments"][0];
    let dimensions = assignment["review_dimensions"]
        .as_array()
        .expect("review dimensions")
        .iter()
        .map(|lens| {
            let lens = lens.as_str().expect("lens");
            let selected = lens == "correctness-behavior";
            json!({
                "lens": lens,
                "risk": if selected { "medium" } else { "none" },
                "evidence": if selected { "The changed recovery path controls durable final-review resumption." } else { "No concrete failure path in this fixture." },
                "plausible_failure": if selected { "A restarted coordinator cannot recover its pending assignments." } else { "none" },
                "material_impact": if selected { "Required final review becomes unavailable and blocks delivery." } else { "none" },
                "uncertain": false
            })
        })
        .collect::<Vec<_>>();
    let assessment = json!({
        "assignment_id": assignment["assignment_id"],
        "subagent_key": assignment["subagent_key"],
        "shared_test_evidence_id": assignment["shared_test_evidence"]["id"],
        "overall_risk": "medium",
        "dimensions": dimensions,
        "exceptional_triggers": [],
        "split_required": false,
        "plan_assumptions": [],
        "findings": [],
        "caller_attestation": {
            "model_role": assignment["model_role"],
            "fresh_context": true,
            "closed_after_result": true
        }
    });
    let mut plan_arguments = risk_arguments;
    plan_arguments["risk_assessment"] = assessment;
    plan_arguments["unrelated_finding_policy"] = json!({ "default": "report" });
    let plan_response = mcp_call_with_environment(
        root.path(),
        "final_review.plan",
        plan_arguments,
        &[("XDG_STATE_HOME", state.path())],
    );
    let plan = mcp_text_payload(&plan_response);
    assert!(
        plan.get("state_ref").is_some(),
        "plan failed: {plan_response}"
    );

    let resumed = mcp_call_with_environment(
        root.path(),
        "final_review.resume_latest",
        json!({ "project_root": root.path(), "session_id": session_id }),
        &[("XDG_STATE_HOME", state.path())],
    );
    let resumed_payload = mcp_text_payload(&resumed);
    assert!(
        resumed_payload.get("state_ref").is_some(),
        "restart recovery failed: {resumed}"
    );

    let projections = sqlite_files(state.path());
    assert_eq!(projections.len(), 1, "expected one review projection");
    fs::remove_file(&projections[0]).expect("delete rebuildable projection");
    for suffix in ["-wal", "-shm"] {
        let sidecar =
            std::path::PathBuf::from(format!("{}{}", projections[0].to_string_lossy(), suffix));
        if sidecar.exists() {
            fs::remove_file(sidecar).expect("delete projection sidecar");
        }
    }
    let rebuilt = mcp_call_with_environment(
        root.path(),
        "final_review.resume_latest",
        json!({ "project_root": root.path(), "session_id": session_id }),
        &[("XDG_STATE_HOME", state.path())],
    );
    let rebuilt_payload = mcp_text_payload(&rebuilt);
    assert!(
        rebuilt_payload.get("state_ref").is_some(),
        "projection rebuild failed: {rebuilt}"
    );
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
        absent.pointer("/result/structuredContent/configuration"),
        Some(&json!("absent"))
    );
    assert_eq!(
        absent.pointer("/result/structuredContent/authority"),
        Some(&json!("advisory"))
    );
    assert_current_final_review_protocol(&absent);
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
        invalid.pointer("/result/structuredContent/configuration"),
        Some(&json!("invalid"))
    );
    assert_eq!(
        invalid.pointer("/result/structuredContent/authority"),
        Some(&json!("advisory"))
    );
    assert_current_final_review_protocol(&invalid);
}

fn setup_arguments(root: &std::path::Path) -> Value {
    json!({ "project_root": root, "proposed_configuration": "schema_version = 3\n[scopes.application]\ncategory = 'source'\ninclude = ['apps/*/lib/**/*.ex']\nexclude = ['apps/*/lib/**/*_test.exs']\n[scopes.checks]\ncategory = 'tests'\ninclude = ['apps/*/test/**/*_test.exs']\n[commands.fast]\nargv = ['mix', 'test', '--stale']\ncapability = 'tests'\n[commands.full]\nargv = ['mix', 'test']\ncapability = 'verification'\n[delivery]\nmode = 'direct-to-trunk'\ntrunk_branch = 'main'\n[features]\ntiber = true\n", "pre_commit_command_ids": ["fast"], "pre_push_command_ids": ["full"] })
}

fn approved_setup(root: &std::path::Path, mut arguments: Value) -> Value {
    let preview = mcp_call(root, "setup.preview", arguments.clone());
    let identity = preview
        .pointer("/result/structuredContent/preview_identity")
        .expect("valid full proposal preview")
        .clone();
    arguments["preview_identity"] = identity;
    arguments["confirmed"] = json!(true);
    arguments
}

#[test]
fn setup_discovery_returns_schema_and_evidence_without_a_template() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::create_dir_all(root.path().join("apps/web/lib")).expect("layout");
    fs::write(
        root.path().join("apps/web/lib/web.ex"),
        "defmodule Web do end",
    )
    .expect("source");
    let response = mcp_call(
        root.path(),
        "setup.preview",
        json!({"project_root":root.path()}),
    );
    let result = &response["result"]["structuredContent"];
    assert_eq!(result["proposal_required"], true);
    assert_eq!(
        result["configuration_schema"]["properties"]["schema_version"]["const"],
        3
    );
    assert!(result.get("configuration").is_none());
    assert!(result["repository_files"]
        .as_array()
        .expect("files")
        .contains(&json!("apps/web/lib/web.ex")));
    assert!(!root.path().join(".development-system.toml").exists());
}

#[test]
fn setup_accepts_custom_commands_and_planned_language_specific_scopes() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    let arguments = setup_arguments(root.path());
    let preview = mcp_call(root.path(), "setup.preview", arguments.clone());
    let result = &preview["result"]["structuredContent"];
    assert_eq!(result["configuration"], arguments["proposed_configuration"]);
    assert!(result["lefthook_configuration"]
        .as_str()
        .expect("hooks")
        .contains("--stale"));
    assert_eq!(result["pre_commit_command_ids"], json!(["fast"]));
    assert_eq!(result["pre_push_command_ids"], json!(["full"]));
    assert!(!root.path().join("apps").exists());
}

#[test]
fn setup_discovery_keeps_native_command_suggestions_and_nix_wrapping() {
    for (manifest, text, native) in [
        (
            "pyproject.toml",
            "[project]\nname='fixture'\nversion='0.1'\n",
            "pytest",
        ),
        ("go.mod", "module example.invalid/fixture\ngo 1.22\n", "go"),
        (
            "Cargo.toml",
            "[package]\nname='fixture'\nversion='0.1.0'\n",
            "cargo",
        ),
    ] {
        let root = TempDir::new().expect("repository");
        git(root.path(), &["init", "--quiet"]);
        fs::write(root.path().join(manifest), text).expect("manifest");
        fs::write(root.path().join("flake.nix"), "{}\n").expect("Nix manifest");
        let response = mcp_call(
            root.path(),
            "setup.preview",
            json!({"project_root":root.path()}),
        );
        let suggestions = response["result"]["structuredContent"]["detected_commands"].to_string();
        assert!(suggestions.contains(native), "{response}");
        assert!(suggestions.contains("nix"), "{response}");
        assert!(response["result"]["structuredContent"]
            .get("configuration")
            .is_none());
    }
}

#[test]
fn setup_apply_requires_confirmation_and_a_full_exact_preview() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    let old = mcp_call(
        root.path(),
        "setup.apply",
        json!({"confirmed":true,"selected_command_ids":["just-ci"]}),
    );
    assert!(old.get("error").is_some());
    let mut arguments = approved_setup(root.path(), setup_arguments(root.path()));
    arguments["confirmed"] = json!(false);
    let rejected = mcp_call(root.path(), "setup.apply", arguments);
    assert_eq!(
        rejected["error"]["message"],
        "development_system.setup_confirmation_required"
    );
    assert!(!root.path().join(".development-system.toml").exists());
}

#[test]
fn setup_apply_writes_exact_proposal_and_preserves_unrelated_configuration() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::create_dir_all(root.path().join(".codex")).expect("Codex directory");
    fs::write(
        root.path().join(".codex/config.toml"),
        "model = 'gpt-6-sol'\n",
    )
    .expect("config");
    fs::write(root.path().join(".mcp.json"), "unrelated sentinel").expect("sentinel");
    let arguments = approved_setup(root.path(), setup_arguments(root.path()));
    let response = mcp_call(root.path(), "setup.apply", arguments.clone());
    assert_eq!(
        response["result"]["structuredContent"]["applied"], true,
        "{response}"
    );
    assert_eq!(
        fs::read_to_string(root.path().join(".development-system.toml")).expect("config"),
        arguments["proposed_configuration"]
            .as_str()
            .expect("proposal")
    );
    assert_eq!(
        fs::read_to_string(root.path().join(".mcp.json")).expect("sentinel"),
        "unrelated sentinel"
    );
    assert_eq!(
        fs::read_to_string(root.path().join(".codex/config.toml")).expect("config"),
        "model = 'gpt-6-sol'\n"
    );
    assert!(root.path().join(".git/hooks/pre-commit").is_file());
    assert!(root.path().join(".git/hooks/pre-push").is_file());
}

#[test]
fn setup_reconciles_existing_policy_and_migrates_only_managed_mcp_entries() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::write(
        root.path().join(".development-system.toml"),
        "schema_version = 2\n[features]\ntiber = true\n",
    )
    .expect("legacy config");
    fs::create_dir_all(root.path().join(".codex")).expect("Codex directory");
    fs::write(root.path().join(".codex/config.toml"), "model = 'gpt-6-sol'\n# >>> development-system MCP servers >>>\n[mcp_servers.development-discipline]\ncommand = 'old'\n# <<< development-system MCP servers <<<\n").expect("legacy MCP");
    let arguments = approved_setup(root.path(), setup_arguments(root.path()));
    let response = mcp_call(root.path(), "setup.apply", arguments);
    assert_eq!(
        response["result"]["structuredContent"]["applied"], true,
        "{response}"
    );
    let config = fs::read_to_string(root.path().join(".development-system.toml")).expect("config");
    assert!(config.contains("tiber = true"));
    assert!(config.contains("apps/*/lib/**/*.ex"));
    let codex = fs::read_to_string(root.path().join(".codex/config.toml")).expect("Codex config");
    assert!(codex.contains("model = 'gpt-6-sol'"));
    assert!(!codex.contains("mcp_servers"));
}

#[test]
fn setup_rejects_changed_proposal_hook_selection_or_repository_before_writes() {
    for change in [
        "proposal",
        "selection",
        "files",
        "config",
        "hooks",
        "replacement",
        "manifest",
    ] {
        let root = TempDir::new().expect("repository");
        git(root.path(), &["init", "--quiet"]);
        fs::write(root.path().join("package.json"), "{}").expect("manifest");
        let mut arguments = approved_setup(root.path(), setup_arguments(root.path()));
        match change {
            "proposal" => {
                arguments["proposed_configuration"] = json!(format!(
                    "{}\n# changed",
                    arguments["proposed_configuration"]
                        .as_str()
                        .expect("proposal")
                ))
            }
            "selection" => arguments["pre_commit_command_ids"] = json!(["full"]),
            "files" => fs::write(root.path().join("new-file"), "new").expect("new file"),
            "config" => fs::write(root.path().join(".development-system.toml"), "# changed")
                .expect("config"),
            "hooks" => {
                fs::write(root.path().join("lefthook.yml"), "pre-commit: {}\n").expect("hooks")
            }
            "replacement" => arguments["replace_lefthook"] = json!(true),
            "manifest" => fs::write(root.path().join("package.json"), "{\"name\":\"changed\"}")
                .expect("manifest"),
            _ => unreachable!(),
        }
        let before = fs::read(root.path().join(".development-system.toml")).ok();
        let response = mcp_call(root.path(), "setup.apply", arguments);
        assert_eq!(
            response["error"]["message"], "development_system.setup_preview_stale",
            "{change}: {response}"
        );
        assert_eq!(
            fs::read(root.path().join(".development-system.toml")).ok(),
            before
        );
        assert!(!root.path().join(".git/hooks/pre-push").exists());
    }
}

#[test]
fn setup_requires_both_explicit_hook_selections_and_safe_complete_configuration() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    for key in ["pre_commit_command_ids", "pre_push_command_ids"] {
        let mut arguments = setup_arguments(root.path());
        arguments.as_object_mut().expect("arguments").remove(key);
        let response = mcp_call(root.path(), "setup.preview", arguments);
        assert!(response.get("error").is_some(), "{response}");
    }
    for bad in [
        "schema_version = 3\n[scopes.source]\ncategory='source'\ninclude=['../escape/**']\n",
        "schema_version=3\n[scopes.source]\ncategory='source'\ninclude=['future/**/*.rs']\n",
    ] {
        let mut arguments = setup_arguments(root.path());
        arguments["proposed_configuration"] = json!(bad);
        assert!(mcp_call(root.path(), "setup.preview", arguments)
            .get("error")
            .is_some());
    }
}

#[test]
fn setup_rejects_parameterized_hook_selections_before_writes() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    let mut base = setup_arguments(root.path());
    base["proposed_configuration"] = json!(format!(
        "{}\n[commands.targeted]\nargv = ['mix', 'test', '{{target}}']\ncapability = 'tests'\n[commands.targeted.parameters]\ntarget = 'string'\n",
        base["proposed_configuration"].as_str().expect("proposal")
    ));
    let approved = approved_setup(root.path(), base.clone());
    for key in ["pre_commit_command_ids", "pre_push_command_ids"] {
        for tool in ["setup.preview", "setup.apply"] {
            let mut arguments = if tool == "setup.apply" {
                approved.clone()
            } else {
                base.clone()
            };
            arguments[key] = json!(["targeted"]);
            let response = mcp_call(root.path(), tool, arguments);
            assert!(
                response["error"]["message"]
                    .as_str()
                    .is_some_and(|message| {
                        message.contains("setup_hook_command_parameters_unsupported id=targeted")
                    }),
                "{tool} {key}: {response}"
            );
            assert!(!root.path().join(".development-system.toml").exists());
            assert!(!root.path().join("lefthook.yml").exists());
        }
    }
}

#[cfg(unix)]
#[test]
fn setup_rejects_symlinked_write_paths_without_changing_external_files() {
    use std::os::unix::fs::symlink;
    for relative in [
        ".development-system.toml",
        "lefthook.yml",
        ".codex/config.toml",
        ".codex",
    ] {
        let root = TempDir::new().expect("repository");
        let external = TempDir::new().expect("external files");
        git(root.path(), &["init", "--quiet"]);
        let arguments = approved_setup(root.path(), setup_arguments(root.path()));
        let target = external.path().join("config.toml");
        fs::write(&target, "external sentinel\n").expect("external file");
        let link = root.path().join(relative);
        if relative == ".codex/config.toml" {
            fs::create_dir(root.path().join(".codex")).expect("config directory");
        }
        symlink(
            if relative == ".codex" {
                external.path()
            } else {
                &target
            },
            &link,
        )
        .expect("symlink");
        for (tool, input) in [
            ("setup.preview", setup_arguments(root.path())),
            ("setup.apply", arguments),
        ] {
            let result = mcp_call(root.path(), tool, input);
            assert!(
                result["error"]["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("setup_symlink_not_supported")),
                "{relative}: {result}"
            );
            assert_eq!(
                fs::read_to_string(&target).expect("external file"),
                "external sentinel\n"
            );
            assert!(!root.path().join(".git/hooks/pre-push").exists());
        }
    }
}

#[test]
fn setup_validates_proposed_final_review_policy_before_preview_or_write() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    for policy in [
        "[final_review]\nunexpected=true\n",
        "[final_review.models]\npre_filter=42\n",
        "[final_review.models]\nunknown='model'\n",
        "[final_review.models]\ncodex='model'\n",
        "[final_review.dispositions.MAJOR]\ncorrectness-behavior='invalid'\n",
    ] {
        let mut arguments = setup_arguments(root.path());
        let proposal = format!(
            "{}\n{policy}",
            arguments["proposed_configuration"]
                .as_str()
                .expect("proposal")
        );
        arguments["proposed_configuration"] = json!(proposal);
        let result = mcp_call(root.path(), "setup.preview", arguments);
        assert!(result.get("error").is_some(), "{policy}: {result}");
        assert!(!root.path().join(".development-system.toml").exists());
    }
}

#[test]
fn setup_requires_explicit_approval_to_replace_lefthook() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    fs::write(root.path().join("lefthook.yml"), "pre-commit: {}\n").expect("hooks");
    let arguments = approved_setup(root.path(), setup_arguments(root.path()));
    let rejected = mcp_call(root.path(), "setup.apply", arguments);
    assert_eq!(
        rejected["error"]["message"],
        "development_system.setup_lefthook_conflict replace_lefthook_required=true"
    );
    assert!(!root.path().join(".development-system.toml").exists());
    let mut arguments = setup_arguments(root.path());
    arguments["replace_lefthook"] = json!(true);
    let arguments = approved_setup(root.path(), arguments);
    let applied = mcp_call(root.path(), "setup.apply", arguments);
    assert_eq!(
        applied["result"]["structuredContent"]["applied"], true,
        "{applied}"
    );
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
        format!(
            "schema_version = 3\n\n[scopes.tests]\ncategory = \"tests\"\ninclude = [\"tests/**\"]\n\n[commands.red]\nargv = [\"{}\"]\ncapability = \"tests\"\n",
            test_executable("false")
        ),
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
        format!(
            "schema_version = 3\n\n[scopes.tests]\ncategory = \"tests\"\ninclude = [\"tests/**\"]\n\n[scopes.source]\ncategory = \"source\"\ninclude = [\"src/**\"]\n\n[commands.red]\nargv = [\"{}\"]\ncapability = \"tests\"\n\n[commands.green]\nargv = [\"{}\"]\ncapability = \"implementation\"\n",
            test_executable("false"),
            test_executable("true")
        ),
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
        format!(
            "schema_version = 3\n\n[scopes.tests]\ncategory = \"tests\"\ninclude = [\"tests/**\"]\n\n[scopes.source]\ncategory = \"source\"\ninclude = [\"src/**\"]\n\n[commands.red]\nargv = [\"{}\"]\ncapability = \"tests\"\n",
            test_executable("false")
        ),
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
fn plugin_surface_exposes_advisory_coordination_and_denies_project_mutation_tools() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    let denied = mcp_call_with_surface(
        root.path(),
        "workspace-editor.patch",
        json!({}),
        Some("plugin-advisory"),
    );
    assert_eq!(
        denied.pointer("/error/message"),
        Some(&json!("development_system.service_capability_unavailable"))
    );

    let mut command = Command::new(env!("CARGO_BIN_EXE_development-discipline-mcp"));
    command.env("DEVELOPMENT_SYSTEM_SERVICE", "plugin-advisory");
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
    assert!(names.contains(&"workspace-reader.status"));
    assert!(names.contains(&"setup.preview"));
    assert!(names.contains(&"final_review.plan"));
    assert!(names.contains(&"final_review.reopen"));
    assert!(names.contains(&"final_review.migrate_policy"));
    assert!(names.contains(&"final_review.continue_review"));
    assert!(names.contains(&"final_review.yield_report"));
    assert!(names.contains(&"final_review.evidence"));
    assert!(!names.iter().any(|name| name.starts_with("workflow.")));
    assert!(names.iter().all(|name| {
        matches!(
            *name,
            "workspace-reader.status"
                | "workspace-reader.read"
                | "workspace-reader.list"
                | "workspace-reader.search"
                | "workspace-reader.repository"
                | "setup.preview"
                | "setup.apply"
                | "development_system.codex_sandbox_setup"
        ) || matches!(
            *name,
            "final_review.plan"
                | "final_review.filter_findings"
                | "final_review.advance"
                | "final_review.confirm_split"
                | "final_review.clean_status"
                | "final_review.out_of_scope_report"
                | "final_review.resume_latest"
                | "final_review.reopen"
                | "final_review.migrate_policy"
                | "final_review.continue_review"
                | "final_review.yield_report"
                | "final_review.evidence"
                | "final_review.pending_assignments"
                | "final_review.assess_risk"
        )
    }));
    for tool in listed["result"]["tools"].as_array().expect("tools") {
        let Some(name) = tool["name"].as_str() else {
            continue;
        };
        if name.starts_with("final_review.") {
            let serialized = tool.to_string().to_ascii_lowercase();
            assert!(serialized.contains("advisory coordination"));
            assert!(!serialized.contains("authoritative"));
            assert!(!serialized.contains("mechanical development lifecycle gate"));
        }
    }
}

#[test]
fn stale_environment_service_name_fails_with_an_actionable_error() {
    let root = TempDir::new().expect("repository");
    git(root.path(), &["init", "--quiet"]);
    let output = Command::new(env!("CARGO_BIN_EXE_development-discipline-mcp"))
        .env("DEVELOPMENT_SYSTEM_SERVICE", "plugin-read-only")
        .current_dir(root.path())
        .output()
        .expect("run dispatcher");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("development_system.dispatcher_service_unknown"));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("recovery=use_plugin-advisory_or_update_plugin_registration"));
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
