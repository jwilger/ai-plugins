mod support;

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use support::{assert_success, assert_success_ref, task_stem, TempRepo};

#[test]
fn mcp_stdio_exposes_tools_and_task_resources() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Expose MCP task"]));
    let expose_mcp_task = task_stem(&repo, "backlog", "expose-mcp-task");
    let install_target_dir = repo.path().join("bin");
    let launcher = repo.path().join("plugin/bin/tiber");
    std::fs::create_dir_all(launcher.parent().expect("launcher parent"))
        .expect("create launcher dir");
    std::fs::write(&launcher, "#!/usr/bin/env bash\n").expect("write fake launcher");
    std::fs::create_dir_all(repo.path().join("docs/guides")).expect("create docs directory");
    std::fs::write(
        repo.path().join("docs/guides/tiber.md"),
        "# Tiber guide\n\nUse tiber mcp stdio.\n",
    )
    .expect("write doc");
    let mut child = Command::new(env!("CARGO_BIN_EXE_tiber"))
        .args(["mcp", "stdio"])
        .current_dir(repo.path())
        .env("TIBER_LAUNCHER_PATH", &launcher)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn tiber mcp stdio");
    let mut stdin = child.stdin.take().expect("mcp stdin should be available");
    let stdout = child.stdout.take().expect("mcp stdout should be available");
    let mut stdout = BufReader::new(stdout);

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#,
    );
    let initialize = read_json_message(&mut stdout);
    assert_eq!(initialize["id"], 1);
    assert_eq!(initialize["result"]["serverInfo"]["name"], "tiber");
    assert_eq!(
        initialize["result"]["capabilities"]["tools"],
        serde_json::json!({})
    );
    assert_eq!(
        initialize["result"]["capabilities"]["resources"],
        serde_json::json!({})
    );
    let instructions = initialize["result"]["instructions"]
        .as_str()
        .expect("initialize instructions should be a string");
    assert!(instructions.contains("tiber.codex_sandbox_setup"));
    assert!(instructions.contains("case-by-case approval for raw Git prefixes"));
    assert!(instructions.contains("exact Tiber-internal operation"));
    assert!(instructions
        .to_lowercase()
        .contains("do not run the whole tiber mcp server outside the sandbox"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
    );
    let tools = read_message(&mut stdout);
    assert!(tools.contains(r#""id":2"#));
    assert!(tools.contains(r#""name":"tiber.codex_sandbox_setup""#));
    assert!(tools.contains(r#""name":"tiber.create""#));
    assert!(tools.contains(r#""name":"tiber.list""#));
    assert!(tools.contains(r#""name":"tiber.show""#));
    assert!(tools.contains(r#""name":"tiber.metadata""#));
    assert!(tools.contains(r#""name":"tiber.conflict_show""#));
    assert!(tools.contains(r#""name":"tiber.conflict_resolve""#));
    assert!(tools.contains(r#""name":"tiber.conflict_resolve_many""#));
    assert!(tools.contains(r#""maxItems":25"#));
    assert!(tools.contains(r#""name":"tiber.next""#));
    assert!(tools.contains(r#""name":"tiber.transition""#));
    assert!(tools.contains(r#""name":"tiber.prioritize""#));
    assert!(tools.contains(r#""name":"tiber.link""#));
    assert!(tools.contains(r#""name":"tiber.unlink""#));
    assert!(tools.contains(r#""name":"tiber.subtask.add""#));
    assert!(tools.contains(r#""name":"tiber.subtask.check""#));
    assert!(tools.contains(r#""name":"tiber.subtask.uncheck""#));
    assert!(tools.contains(r#""name":"tiber.update""#));
    assert!(tools.contains(r#""name":"tiber.acceptance.add""#));
    assert!(tools.contains(r#""name":"tiber.acceptance.check""#));
    assert!(tools.contains(r#""name":"tiber.acceptance.uncheck""#));
    assert!(tools.contains(r#""name":"tiber.acceptance.remove""#));
    assert!(tools.contains(r#""name":"tiber.note.add""#));
    assert!(tools.contains(r#""name":"tiber.validate_fix""#));
    assert!(tools.contains(r#""name":"tiber.close_from_trailers""#));
    assert!(tools.contains(r#""name":"tiber.scaffold_repo_dry_run""#));
    assert!(tools.contains(r#""name":"tiber.install_bin""#));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":3,"method":"resources/list"}"#,
    );
    let resources = read_message(&mut stdout);
    assert!(resources.contains(r#""id":3"#));
    assert!(resources.contains(r#""uri":"tasks://board""#));
    assert!(resources.contains(r#""uri":"tasks://codex-sandbox""#));
    assert!(resources.contains(&format!(r#""uri":"tasks://task/{expose_mcp_task}""#)));
    assert!(resources.contains(r#""uri":"tasks://docs/tree""#));
    assert!(resources.contains(r#""uri":"tasks://docs/guides/tiber.md""#));

    write_message(
        &mut stdin,
        &format!(
            r#"{{"jsonrpc":"2.0","id":4,"method":"resources/read","params":{{"uri":"tasks://task/{expose_mcp_task}"}}}}"#
        ),
    );
    let task = read_message(&mut stdout);
    assert!(task.contains(r#""id":4"#));
    assert!(task.contains("title: Expose MCP task"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"tiber.create","arguments":{"title":"Created through MCP"}}}"#,
    );
    let create = read_message(&mut stdout);
    assert!(create.contains(r#""id":5"#));
    assert!(create.contains("-created-through-mcp"));
    let created_through_mcp = task_stem(&repo, "backlog", "created-through-mcp");

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"tiber.list","arguments":{}}}"#,
    );
    let list = read_message(&mut stdout);
    assert!(list.contains(r#""id":6"#));
    assert!(list.contains(&expose_mcp_task));
    assert!(list.contains(&created_through_mcp));

    write_message(
        &mut stdin,
        &format!(
            r#"{{"jsonrpc":"2.0","id":7,"method":"resources/read","params":{{"uri":"tasks://task/{created_through_mcp}"}}}}"#
        ),
    );
    let created = read_message(&mut stdout);
    assert!(created.contains(r#""id":7"#));
    assert!(created.contains("title: Created through MCP"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":8,"method":"resources/read","params":{"uri":"tasks://docs/tree"}}"#,
    );
    let docs_tree = read_message(&mut stdout);
    assert!(docs_tree.contains(r#""id":8"#));
    assert!(docs_tree.contains("docs/guides/tiber.md"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":9,"method":"resources/read","params":{"uri":"tasks://docs/guides/tiber.md"}}"#,
    );
    let doc = read_message(&mut stdout);
    assert!(doc.contains(r#""id":9"#));
    assert!(doc.contains("# Tiber guide"));
    assert!(doc.contains("tiber mcp stdio"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":91,"method":"tools/call","params":{"name":"tiber.codex_sandbox_setup","arguments":{}}}"#,
    );
    let codex_setup_tool = read_message(&mut stdout);
    assert!(codex_setup_tool.contains(r#""id":91"#));
    assert!(codex_setup_tool
        .contains("case-by-case approval for prefix_rule [\\\"git\\\", \\\"hash-object\\\"]"));
    assert!(codex_setup_tool.contains("prefix_rule [\\\"git\\\", \\\"commit-tree\\\"]"));
    assert!(codex_setup_tool
        .contains("case-by-case approval for prefix_rule [\\\"git\\\", \\\"update-ref\\\", \\\"refs/heads/tasks\\\"]"));
    assert!(codex_setup_tool.contains(
        "Persist approval only when the harness can scope it to the exact Tiber-internal operation"
    ));
    assert!(codex_setup_tool.contains("Never persist a raw git"));
    assert!(codex_setup_tool.contains("retry the same structured Tiber MCP operation"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":92,"method":"resources/read","params":{"uri":"tasks://codex-sandbox"}}"#,
    );
    let codex_setup_resource = read_message(&mut stdout);
    assert!(codex_setup_resource.contains(r#""id":92"#));
    assert!(
        codex_setup_resource.contains("Do not run the whole Tiber MCP server outside the sandbox")
    );

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"tiber.show","arguments":{"ref":"expose-mcp-task"}}}"#,
    );
    let show = read_message(&mut stdout);
    assert!(show.contains(r#""id":10"#));
    assert!(show.contains("title: Expose MCP task"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"tiber.metadata","arguments":{"ref":"expose-mcp-task"}}}"#,
    );
    let metadata = read_message(&mut stdout);
    assert!(metadata.contains(r#""id":11"#));
    assert!(metadata.contains(&format!(
        "{expose_mcp_task}\\tExpose MCP task\\tcommitted_at=20"
    )));

    write_message(
        &mut stdin,
        &format!(
            r#"{{"jsonrpc":"2.0","id":111,"method":"tools/call","params":{{"name":"tiber.conflict_show","arguments":{{"path":"backlog/{expose_mcp_task}.md"}}}}}}"#
        ),
    );
    let conflict = read_message(&mut stdout);
    assert!(conflict.contains(r#""id":111"#));
    let conflict_response: serde_json::Value =
        serde_json::from_str(&conflict).expect("conflict response should be json");
    let conflict_text = conflict_response["result"]["content"][0]["text"]
        .as_str()
        .expect("conflict tool text");
    let conflict_payload: serde_json::Value =
        serde_json::from_str(conflict_text).expect("conflict text should be json");
    assert_eq!(
        conflict_payload["path"],
        format!("backlog/{expose_mcp_task}.md")
    );
    assert_eq!(
        conflict_payload["local_path"],
        format!("backlog/{expose_mcp_task}.md")
    );
    assert!(conflict_payload["remote_path"].is_null());
    assert!(conflict_payload["local"]
        .as_str()
        .expect("local conflict side")
        .contains("title: Expose MCP task"));
    assert!(conflict_payload["remote"].is_null());

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"tiber.next","arguments":{}}}"#,
    );
    let next = read_message(&mut stdout);
    assert!(next.contains(r#""id":12"#));
    assert!(next.contains(&expose_mcp_task));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"tiber.subtask.add","arguments":{"ref":"created-through-mcp","title":"Write MCP mirror tests"}}}"#,
    );
    let subtask_add = read_message(&mut stdout);
    assert!(subtask_add.contains(r#""id":13"#));
    assert!(subtask_add.contains("updated created-through-mcp"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"tiber.subtask.check","arguments":{"ref":"created-through-mcp","index":"1"}}}"#,
    );
    let subtask_check = read_message(&mut stdout);
    assert!(subtask_check.contains(r#""id":14"#));
    assert!(subtask_check.contains("updated created-through-mcp"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":141,"method":"tools/call","params":{"name":"tiber.subtask.add","arguments":{"ref":"created-through-mcp","title":"Wire dependency","after":["s1"]}}}"#,
    );
    let dependent_subtask_add = read_message(&mut stdout);
    assert!(dependent_subtask_add.contains(r#""id":141"#));
    assert!(dependent_subtask_add.contains("updated created-through-mcp"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":15,"method":"tools/call","params":{"name":"tiber.transition","arguments":{"ref":"created-through-mcp","status":"in-progress"}}}"#,
    );
    let transition = read_message(&mut stdout);
    assert!(transition.contains(r#""id":15"#));
    assert!(transition.contains(&created_through_mcp));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":16,"method":"tools/call","params":{"name":"tiber.link","arguments":{"from":"created-through-mcp","to":"expose-mcp-task"}}}"#,
    );
    let link = read_message(&mut stdout);
    assert!(link.contains(r#""id":16"#));
    assert!(link.contains("linked created-through-mcp blocks expose-mcp-task"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":17,"method":"tools/call","params":{"name":"tiber.unlink","arguments":{"from":"created-through-mcp","to":"expose-mcp-task"}}}"#,
    );
    let unlink = read_message(&mut stdout);
    assert!(unlink.contains(r#""id":17"#));
    assert!(unlink.contains("unlinked created-through-mcp blocks expose-mcp-task"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":18,"method":"tools/call","params":{"name":"tiber.prioritize","arguments":{"ref":"created-through-mcp","before":"expose-mcp-task"}}}"#,
    );
    let prioritize = read_message(&mut stdout);
    assert!(prioritize.contains(r#""id":18"#));
    assert!(prioritize.contains("prioritized created-through-mcp before expose-mcp-task"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":181,"method":"tools/call","params":{"name":"tiber.update","arguments":{"ref":"created-through-mcp","summary":"MCP summary","context":"MCP context","tags":["mcp","structured"]}}}"#,
    );
    let update = read_message(&mut stdout);
    assert!(update.contains(r#""id":181"#));
    assert!(update.contains("updated created-through-mcp"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":182,"method":"tools/call","params":{"name":"tiber.acceptance.add","arguments":{"ref":"created-through-mcp","criterion":"MCP criterion"}}}"#,
    );
    let acceptance_add = read_message(&mut stdout);
    assert!(acceptance_add.contains(r#""id":182"#));
    assert!(acceptance_add.contains("updated created-through-mcp"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":183,"method":"tools/call","params":{"name":"tiber.acceptance.check","arguments":{"ref":"created-through-mcp","index":"1"}}}"#,
    );
    let acceptance_check = read_message(&mut stdout);
    assert!(acceptance_check.contains(r#""id":183"#));
    assert!(acceptance_check.contains("updated created-through-mcp"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":184,"method":"tools/call","params":{"name":"tiber.note.add","arguments":{"ref":"created-through-mcp","note":"MCP note"}}}"#,
    );
    let note_add = read_message(&mut stdout);
    assert!(note_add.contains(r#""id":184"#));
    assert!(note_add.contains("updated created-through-mcp"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":185,"method":"tools/call","params":{"name":"tiber.show","arguments":{"ref":"created-through-mcp"}}}"#,
    );
    let structured_show = read_message(&mut stdout);
    assert!(structured_show.contains(r#""id":185"#));
    assert!(structured_show.contains("MCP summary"));
    assert!(structured_show.contains("tags: [mcp, structured]"));
    assert!(structured_show.contains("- [x] MCP criterion"));
    assert!(structured_show.contains("- [ ] (s2) Wire dependency — after: s1"));
    assert!(structured_show.contains("MCP note"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":19,"method":"tools/call","params":{"name":"tiber.validate_fix","arguments":{}}}"#,
    );
    let validate = read_message(&mut stdout);
    assert!(validate.contains(r#""id":19"#));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":20,"method":"tools/call","params":{"name":"tiber.scaffold_repo_dry_run","arguments":{}}}"#,
    );
    let scaffold = read_message(&mut stdout);
    assert!(scaffold.contains(r#""id":20"#));
    assert!(scaffold.contains("would write .githooks/post-commit.tiber"));
    assert!(scaffold.contains("would write .github/workflows/tiber-close-from-trailers.yml"));
    assert!(!scaffold.contains("would write .gitignore"));

    write_message(&mut stdin, r#"{"jsonrpc":"2.0","id":21}"#);
    let missing_method = read_json_message(&mut stdout);
    assert_eq!(missing_method["id"], 21);
    assert_eq!(missing_method["error"]["code"], -32600);
    assert!(missing_method["error"]["message"]
        .as_str()
        .expect("error message")
        .contains("mcp_method_missing"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":22,"method":"tools/call","params":{"arguments":{}}}"#,
    );
    let missing_tool_name = read_json_message(&mut stdout);
    assert_eq!(missing_tool_name["id"], 22);
    assert_eq!(missing_tool_name["error"]["code"], -32602);
    assert!(missing_tool_name["error"]["message"]
        .as_str()
        .expect("error message")
        .contains("mcp_tool_name_missing"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":23,"method":"resources/read","params":{}}"#,
    );
    let missing_resource_uri = read_json_message(&mut stdout);
    assert_eq!(missing_resource_uri["id"], 23);
    assert_eq!(missing_resource_uri["error"]["code"], -32602);
    assert!(missing_resource_uri["error"]["message"]
        .as_str()
        .expect("error message")
        .contains("mcp_resource_uri_missing"));

    write_message(
        &mut stdin,
        &format!(
            r#"{{"jsonrpc":"2.0","id":24,"method":"tools/call","params":{{"name":"tiber.install_bin","arguments":{{"target_dir":"{}","apply":false}}}}}}"#,
            install_target_dir.display()
        ),
    );
    let install_bin = read_message(&mut stdout);
    assert!(install_bin.contains(r#""id":24"#));
    assert!(install_bin.contains(&format!(
        "would install {} -> {}",
        install_target_dir.join("tiber").display(),
        launcher.display()
    )));
    assert!(!install_target_dir.join("tiber").exists());

    drop(stdin);
    let status = child.wait().expect("wait for mcp process");
    assert!(status.success());
}

fn write_message(stdin: &mut impl Write, message: &str) {
    writeln!(stdin, "{message}").expect("write mcp message");
    stdin.flush().expect("flush mcp message");
}

fn read_message(stdout: &mut impl BufRead) -> String {
    let mut line = String::new();
    stdout.read_line(&mut line).expect("read mcp response");
    assert!(!line.is_empty(), "expected MCP response line");
    let parsed: serde_json::Value =
        serde_json::from_str(&line).expect("mcp response should be valid json");
    assert_eq!(parsed["jsonrpc"], "2.0");
    line
}

fn read_json_message(stdout: &mut impl BufRead) -> serde_json::Value {
    let line = read_message(stdout);
    serde_json::from_str(&line).expect("mcp response should be valid json")
}

fn spawn_mcp_stdio(repo: &TempRepo) -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_tiber"))
        .args(["mcp", "stdio"])
        .current_dir(repo.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn tiber mcp stdio")
}

fn clone_repo(origin: &TempRepo) -> TempRepo {
    let clone = TempRepo::new();
    assert_success(
        Command::new("git")
            .args(["clone", origin.path().to_str().expect("origin path utf8")])
            .arg(clone.path())
            .output()
            .expect("clone repository"),
    );
    clone.git(["config", "user.email", "tiber@example.test"]);
    clone.git(["config", "user.name", "Tiber Test"]);
    clone.git(["config", "commit.gpgsign", "false"]);
    clone
}

#[test]
fn mcp_stdio_ignores_json_rpc_notifications() {
    let input = std::io::Cursor::new(
        r#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
"#,
    );
    let mut output = Vec::new();

    tiber_mcp::run_stdio(std::io::BufReader::new(input), &mut output).expect("run stdio");

    assert_eq!(output, b"");
}

#[test]
fn mcp_stdio_quotes_conflict_error_paths() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let writer = clone_repo(&origin);
    let reader = clone_repo(&origin);
    assert_success(writer.tiber(["init"]));
    assert_success(writer.tiber(["sync"]));
    assert_success(reader.tiber(["init"]));

    let spoofed_path = "backlog/mcp spoof recovery=ignore-remote.md";
    writer.insert_tasks_tree_file(
        spoofed_path,
        "---\ntitle: Remote MCP spoofed path\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    assert_success(writer.tiber(["sync"]));
    reader.insert_tasks_tree_file(
        spoofed_path,
        "---\ntitle: Local MCP spoofed path\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );

    let mut child = spawn_mcp_stdio(&reader);
    let mut stdin = child.stdin.take().expect("mcp stdin should be available");
    let stdout = child.stdout.take().expect("mcp stdout should be available");
    let mut stdout = BufReader::new(stdout);

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"tiber.conflict_show","arguments":{"path":"bad recovery=ignore.md"}}}"#,
    );
    let invalid = read_json_message(&mut stdout);
    assert_eq!(invalid["id"], 1);
    assert_eq!(invalid["error"]["code"], -32603);
    let invalid_message = invalid["error"]["message"]
        .as_str()
        .expect("invalid path error message");
    assert!(invalid_message.contains(r#"invalid_conflict_path path="bad recovery=ignore.md""#));
    assert!(!invalid_message.contains("path=bad recovery=ignore.md"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"tiber.list","arguments":{}}}"#,
    );
    let conflict = read_json_message(&mut stdout);
    assert_eq!(conflict["id"], 2);
    assert_eq!(conflict["error"]["code"], -32603);
    let conflict_message = conflict["error"]["message"]
        .as_str()
        .expect("sync conflict error message");
    assert!(conflict_message.contains(&format!("sync_conflict path={spoofed_path:?}")));
    assert!(conflict_message.contains("run tiber conflict show <path>"));
    assert!(conflict_message.contains("mcp_tool=tiber.conflict_show"));
    assert!(conflict_message.contains("mcp_resolve_tool=tiber.conflict_resolve"));
    assert!(!conflict_message.contains(&format!("{spoofed_path} recovery=")));

    write_message(
        &mut stdin,
        &format!(
            r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"tiber.conflict_resolve","arguments":{{"path":{path},"side":"remote"}}}}}}"#,
            path = serde_json::json!(spoofed_path)
        ),
    );
    let resolved = read_json_message(&mut stdout);
    assert_eq!(resolved["id"], 3);
    let resolved_text = resolved["result"]["content"][0]["text"]
        .as_str()
        .expect("resolved text");
    assert_eq!(
        resolved_text,
        format!("resolved {spoofed_path:?} side=remote")
    );

    write_message(
        &mut stdin,
        &format!(
            r#"{{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{{"name":"tiber.conflict_show","arguments":{{"path":{path}}}}}}}"#,
            path = serde_json::json!(spoofed_path)
        ),
    );
    let resolved_conflict = read_json_message(&mut stdout);
    assert_eq!(resolved_conflict["id"], 4);
    let resolved_conflict_text = resolved_conflict["result"]["content"][0]["text"]
        .as_str()
        .expect("resolved conflict text");
    let resolved_conflict_payload: serde_json::Value =
        serde_json::from_str(resolved_conflict_text).expect("resolved conflict text json");
    assert!(resolved_conflict_payload["local"]
        .as_str()
        .expect("local side")
        .contains("Remote MCP spoofed path"));

    drop(stdin);
    let status = child.wait().expect("wait for mcp process");
    assert!(status.success());
}

#[test]
fn mcp_stdio_redacts_generic_sync_errors() {
    let repo = TempRepo::initialized();
    repo.git([
        "remote",
        "add",
        "origin",
        "https://user:secret-token@example.invalid/private/repo.git",
    ]);
    assert_success(repo.tiber(["init"]));

    let mut child = spawn_mcp_stdio(&repo);
    let mut stdin = child.stdin.take().expect("mcp stdin should be available");
    let stdout = child.stdout.take().expect("mcp stdout should be available");
    let mut stdout = BufReader::new(stdout);

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"tiber.list","arguments":{}}}"#,
    );
    let failed = read_json_message(&mut stdout);

    assert_eq!(failed["id"], 1);
    assert_eq!(failed["error"]["code"], -32603);
    let message = failed["error"]["message"].as_str().expect("error message");
    assert!(message.contains("args_redacted=true"));
    assert!(message.contains("stderr_redacted=true"));
    assert!(!message.contains("secret-token"));
    assert!(!message.contains("private/repo.git"));

    drop(stdin);
    let status = child.wait().expect("wait for mcp process");
    assert!(status.success());
}

#[test]
fn mcp_stdio_conflict_show_truncates_large_conflict_sides() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let writer = clone_repo(&origin);
    let reader = clone_repo(&origin);
    assert_success(writer.tiber(["init"]));
    assert_success(reader.tiber(["init"]));
    assert_success(writer.tiber(["create", "Large MCP conflict"]));
    let stem = task_stem(&writer, "backlog", "large-mcp-conflict");
    assert_success(writer.tiber(["sync"]));
    assert_success(reader.tiber(["list"]));
    writer.git(["fetch", "origin", "+tasks:refs/heads/tasks"]);

    let remote_body = format!(
        "---\ntitle: Remote large MCP conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n{}",
        "R".repeat(5 * 1024 * 1024)
    );
    writer.insert_task_file("backlog", &stem, &remote_body);
    writer.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);
    let local_body = format!(
        "---\ntitle: Local large MCP conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n{}",
        "L".repeat(5 * 1024 * 1024)
    );
    reader.insert_task_file("backlog", &stem, &local_body);

    let oversized_read = reader.tiber(["list"]);
    assert!(
        !oversized_read.status.success(),
        "normal read sync should reject oversized remote task blobs before merging"
    );
    let oversized_read_stderr = String::from_utf8(oversized_read.stderr).expect("stderr utf8");
    assert!(oversized_read_stderr.contains("task_blob_too_large"));
    assert!(oversized_read_stderr.contains("recovery="));
    assert!(oversized_read_stderr.contains("inspect with tiber conflict show <path>"));
    assert!(
        oversized_read_stderr.contains("without force-pushing or overwriting shared task state")
    );

    let cli_conflict = reader.tiber(["conflict", "show", &format!("backlog/{stem}.md")]);
    assert_success_ref(&cli_conflict);
    let cli_payload: serde_json::Value =
        serde_json::from_slice(&cli_conflict.stdout).expect("cli conflict output should be json");
    assert!(cli_payload["local"]
        .as_str()
        .expect("cli local side")
        .contains("[truncated: conflict side exceeded 65536 bytes]"));
    assert!(cli_payload["remote"]
        .as_str()
        .expect("cli remote side")
        .contains("[truncated: conflict side exceeded 65536 bytes]"));

    let oversized_remote_resolve = reader.tiber([
        "conflict",
        "resolve",
        &format!("backlog/{stem}.md"),
        "--remote",
    ]);
    assert!(
        !oversized_remote_resolve.status.success(),
        "oversized selected remote conflict side should be rejected before resolution"
    );
    let oversized_remote_resolve_stderr =
        String::from_utf8(oversized_remote_resolve.stderr).expect("stderr should be utf8");
    assert!(oversized_remote_resolve_stderr.contains("task_blob_too_large"));
    assert!(oversized_remote_resolve_stderr.contains("recovery="));

    let mut child = spawn_mcp_stdio(&reader);
    let mut stdin = child.stdin.take().expect("mcp stdin should be available");
    let stdout = child.stdout.take().expect("mcp stdout should be available");
    let mut stdout = BufReader::new(stdout);

    write_message(
        &mut stdin,
        &format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"tiber.conflict_show","arguments":{{"path":"backlog/{stem}.md"}}}}}}"#
        ),
    );
    let conflict = read_json_message(&mut stdout);
    assert_eq!(conflict["id"], 1);
    let conflict_text = conflict["result"]["content"][0]["text"]
        .as_str()
        .expect("conflict text");
    let conflict_payload: serde_json::Value =
        serde_json::from_str(conflict_text).expect("conflict text should be json");
    let local = conflict_payload["local"]
        .as_str()
        .expect("local conflict side");
    let remote = conflict_payload["remote"]
        .as_str()
        .expect("remote conflict side");
    assert!(local.contains("[truncated: conflict side exceeded 65536 bytes]"));
    assert!(remote.contains("[truncated: conflict side exceeded 65536 bytes]"));
    let max_truncated_len = 65_536 + "\n[truncated: conflict side exceeded 65536 bytes]".len();
    assert!(local.len() <= max_truncated_len);
    assert!(remote.len() <= max_truncated_len);

    drop(stdin);
    let status = child.wait().expect("wait for mcp process");
    assert!(status.success());
}

#[test]
fn conflict_resolve_local_rejects_oversized_selected_local_side() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let writer = clone_repo(&origin);
    let reader = clone_repo(&origin);
    assert_success(writer.tiber(["init"]));
    assert_success(reader.tiber(["init"]));
    assert_success(writer.tiber(["create", "Local oversized conflict"]));
    let stem = task_stem(&writer, "backlog", "local-oversized-conflict");
    assert_success(writer.tiber(["sync"]));
    assert_success(reader.tiber(["list"]));
    writer.git(["fetch", "origin", "+tasks:refs/heads/tasks"]);

    writer.insert_task_file(
        "in-progress",
        &stem,
        "---\ntitle: Remote small MCP conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    writer.remove_tasks_tree_file(&format!("backlog/{stem}.md"));
    writer.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);
    let local_body = format!(
        "---\ntitle: Local oversized MCP conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n{}",
        "L".repeat(5 * 1024 * 1024)
    );
    reader.insert_task_file("done", &stem, &local_body);
    reader.remove_tasks_tree_file(&format!("backlog/{stem}.md"));

    let oversized_local_resolve = reader.tiber([
        "conflict",
        "resolve",
        &format!("in-progress/{stem}.md"),
        "--local",
    ]);
    assert!(
        !oversized_local_resolve.status.success(),
        "oversized selected local conflict side should be rejected before resolution"
    );
    let oversized_local_resolve_stderr =
        String::from_utf8(oversized_local_resolve.stderr).expect("stderr should be utf8");
    assert!(oversized_local_resolve_stderr.contains("task_blob_too_large"));
    assert!(
        oversized_local_resolve_stderr.contains(&format!("path={:?}", format!("done/{stem}.md")))
    );
    assert!(!oversized_local_resolve_stderr
        .contains(&format!("path={:?}", format!("in-progress/{stem}.md"))));
    assert!(oversized_local_resolve_stderr.contains("recovery="));
}

#[test]
fn mcp_stdio_conflict_resolve_many_resolves_two_conflicts() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let writer = clone_repo(&origin);
    let reader = clone_repo(&origin);
    assert_success(writer.tiber(["init"]));
    assert_success(writer.tiber(["create", "First MCP batch conflict"]));
    let first = task_stem(&writer, "backlog", "first-mcp-batch-conflict");
    assert_success(writer.tiber(["create", "Second MCP batch conflict"]));
    let second = task_stem(&writer, "backlog", "second-mcp-batch-conflict");
    assert_success(writer.tiber(["sync"]));
    assert_success(reader.tiber(["list"]));

    writer.insert_task_file(
        "backlog",
        &first,
        "---\ntitle: Remote first MCP batch conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    writer.insert_task_file(
        "backlog",
        &second,
        "---\ntitle: Remote second MCP batch conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    writer.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);
    reader.insert_task_file(
        "backlog",
        &first,
        "---\ntitle: Local first MCP batch conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    reader.insert_task_file(
        "backlog",
        &second,
        "---\ntitle: Local second MCP batch conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );

    let mut child = spawn_mcp_stdio(&reader);
    let mut stdin = child.stdin.take().expect("mcp stdin should be available");
    let stdout = child.stdout.take().expect("mcp stdout should be available");
    let mut stdout = BufReader::new(stdout);
    let resolutions = serde_json::json!([
        {"path": format!("backlog/{first}.md"), "side": "local"},
        {"path": format!("backlog/{second}.md"), "side": "remote"}
    ]);

    write_message(
        &mut stdin,
        &format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"tiber.conflict_resolve_many","arguments":{{"resolutions":{resolutions}}}}}}}"#,
        ),
    );
    let resolved = read_json_message(&mut stdout);
    assert_eq!(resolved["id"], 1);
    let resolved_text = resolved["result"]["content"][0]["text"]
        .as_str()
        .expect("resolved text");
    assert!(resolved_text.contains(&format!(
        "resolved {:?} side=local",
        format!("backlog/{first}.md")
    )));
    assert!(resolved_text.contains(&format!(
        "resolved {:?} side=remote",
        format!("backlog/{second}.md")
    )));

    drop(stdin);
    let status = child.wait().expect("wait for mcp process");
    assert!(status.success());

    let verification = clone_repo(&origin);
    let first_task = verification.git_output(["show", &format!("origin/tasks:backlog/{first}.md")]);
    assert_success_ref(&first_task);
    assert!(String::from_utf8(first_task.stdout)
        .expect("first task utf8")
        .contains("title: Local first MCP batch conflict"));
    let second_task =
        verification.git_output(["show", &format!("origin/tasks:backlog/{second}.md")]);
    assert_success_ref(&second_task);
    assert!(String::from_utf8(second_task.stdout)
        .expect("second task utf8")
        .contains("title: Remote second MCP batch conflict"));
}

#[test]
fn mcp_stdio_conflict_resolve_many_rejects_oversized_batches() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    let mut child = spawn_mcp_stdio(&repo);
    let mut stdin = child.stdin.take().expect("mcp stdin should be available");
    let stdout = child.stdout.take().expect("mcp stdout should be available");
    let mut stdout = BufReader::new(stdout);
    let resolutions = (0..26)
        .map(|index| serde_json::json!({"path": format!("backlog/{index:02}-too-many.md"), "side": "local"}))
        .collect::<Vec<_>>();

    write_message(
        &mut stdin,
        &format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"tiber.conflict_resolve_many","arguments":{{"resolutions":{}}}}}}}"#,
            serde_json::Value::Array(resolutions)
        ),
    );
    let failed = read_json_message(&mut stdout);
    assert_eq!(failed["id"], 1);
    assert_eq!(failed["error"]["code"], -32603);
    let error_message = failed["error"]["message"].as_str().expect("error message");
    assert!(
        error_message.contains("mcp_argument_too_many name=resolutions max_items=25"),
        "unexpected oversized batch failure: {error_message}"
    );

    drop(stdin);
    let status = child.wait().expect("wait for mcp process");
    assert!(status.success());
}

#[test]
fn mcp_stdio_conflict_resolve_many_accepts_max_sized_batch_for_validation() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    let mut child = spawn_mcp_stdio(&repo);
    let mut stdin = child.stdin.take().expect("mcp stdin should be available");
    let stdout = child.stdout.take().expect("mcp stdout should be available");
    let mut stdout = BufReader::new(stdout);
    let resolutions = (0..25)
        .map(|index| serde_json::json!({"path": format!("backlog/{index:02}-max-sized.md"), "side": "local"}))
        .collect::<Vec<_>>();

    write_message(
        &mut stdin,
        &format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"tiber.conflict_resolve_many","arguments":{{"resolutions":{}}}}}}}"#,
            serde_json::Value::Array(resolutions)
        ),
    );
    let failed = read_json_message(&mut stdout);
    assert_eq!(failed["id"], 1);
    assert_eq!(failed["error"]["code"], -32603);
    let error_message = failed["error"]["message"].as_str().expect("error message");
    assert!(
        !error_message.contains("mcp_argument_too_many"),
        "max-sized batch should pass MCP size validation: {error_message}"
    );

    drop(stdin);
    let status = child.wait().expect("wait for mcp process");
    assert!(status.success());
}

#[test]
fn mcp_stdio_conflict_resolve_many_failure_leaves_remote_unchanged() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);

    let writer = clone_repo(&origin);
    let reader = clone_repo(&origin);
    assert_success(writer.tiber(["init"]));
    assert_success(writer.tiber(["create", "Atomic MCP batch conflict"]));
    let conflict_stem = task_stem(&writer, "backlog", "atomic-mcp-batch-conflict");
    assert_success(writer.tiber(["create", "Atomic MCP non conflict"]));
    let non_conflict_stem = task_stem(&writer, "backlog", "atomic-mcp-non-conflict");
    assert_success(writer.tiber(["sync"]));
    assert_success(reader.tiber(["list"]));

    writer.insert_task_file(
        "backlog",
        &conflict_stem,
        "---\ntitle: Remote atomic MCP conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );
    writer.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);
    reader.insert_task_file(
        "backlog",
        &conflict_stem,
        "---\ntitle: Local atomic MCP conflict\nblocked_by: []\nblocks: []\ntags: []\n---\n",
    );

    let before_conflict = clone_repo(&origin)
        .git_output(["show", &format!("origin/tasks:backlog/{conflict_stem}.md")]);
    assert_success_ref(&before_conflict);
    let before_conflict = before_conflict.stdout;
    let before_non_conflict = clone_repo(&origin).git_output([
        "show",
        &format!("origin/tasks:backlog/{non_conflict_stem}.md"),
    ]);
    assert_success_ref(&before_non_conflict);
    let before_non_conflict = before_non_conflict.stdout;

    let mut child = spawn_mcp_stdio(&reader);
    let mut stdin = child.stdin.take().expect("mcp stdin should be available");
    let stdout = child.stdout.take().expect("mcp stdout should be available");
    let mut stdout = BufReader::new(stdout);
    let resolutions = serde_json::json!([
        {"path": format!("backlog/{conflict_stem}.md"), "side": "local"},
        {"path": format!("backlog/{non_conflict_stem}.md"), "side": "remote"}
    ]);

    write_message(
        &mut stdin,
        &format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"tiber.conflict_resolve_many","arguments":{{"resolutions":{resolutions}}}}}}}"#,
        ),
    );
    let failed = read_json_message(&mut stdout);
    assert_eq!(failed["id"], 1);
    assert_eq!(failed["error"]["code"], -32603);
    let error_message = failed["error"]["message"].as_str().expect("error message");
    assert!(
        error_message.contains("local_conflict_side_missing")
            || error_message.contains("conflict_side_not_in_conflict"),
        "unexpected MCP batch failure: {error_message}"
    );

    drop(stdin);
    let status = child.wait().expect("wait for mcp process");
    assert!(status.success());

    let verification = clone_repo(&origin);
    let after_conflict =
        verification.git_output(["show", &format!("origin/tasks:backlog/{conflict_stem}.md")]);
    assert_success_ref(&after_conflict);
    assert_eq!(after_conflict.stdout, before_conflict);
    let after_non_conflict = verification.git_output([
        "show",
        &format!("origin/tasks:backlog/{non_conflict_stem}.md"),
    ]);
    assert_success_ref(&after_non_conflict);
    assert_eq!(after_non_conflict.stdout, before_non_conflict);
}

#[test]
fn mcp_stdio_conflict_resolve_does_not_initialize_tiber_storage() {
    let repo = TempRepo::initialized();
    let mut child = spawn_mcp_stdio(&repo);
    let mut stdin = child.stdin.take().expect("mcp stdin should be available");
    let stdout = child.stdout.take().expect("mcp stdout should be available");
    let mut stdout = BufReader::new(stdout);

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"tiber.conflict_resolve","arguments":{"path":"backlog/missing.md","side":"local"}}}"#,
    );
    let failed = read_json_message(&mut stdout);
    assert_eq!(failed["id"], 1);
    assert_eq!(failed["error"]["code"], -32603);
    assert!(failed["error"]["message"]
        .as_str()
        .expect("error message")
        .contains("tiber_not_initialized=true"));

    drop(stdin);
    let status = child.wait().expect("wait for mcp process");
    assert!(status.success());
    assert!(
        !repo
            .git_output(["show-ref", "--verify", "refs/heads/tasks"])
            .status
            .success(),
        "MCP conflict resolution should not create Tiber storage"
    );
}

#[test]
fn mcp_stdio_create_sync_failure_reports_created_ref_and_redacts_stderr() {
    let (origin, hook_path) = TempRepo::bare_with_rejecting_hook();
    let repo = TempRepo::initialized();
    repo.git([
        "remote",
        "add",
        "origin",
        origin.path().to_str().expect("origin path should be utf8"),
    ]);
    assert_success(repo.tiber(["init"]));

    let mut child = Command::new(env!("CARGO_BIN_EXE_tiber"))
        .args(["mcp", "stdio"])
        .current_dir(repo.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn tiber mcp stdio");
    let mut stdin = child.stdin.take().expect("mcp stdin should be available");
    let stdout = child.stdout.take().expect("mcp stdout should be available");
    let mut stdout = BufReader::new(stdout);

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"tiber.create","arguments":{"title":"MCP partial create"}}}"#,
    );
    let create = read_json_message(&mut stdout);

    assert_eq!(create["id"], 1);
    assert_eq!(create["error"]["code"], -32603);
    let message = create["error"]["message"]
        .as_str()
        .expect("error message should be a string");
    assert!(
        message.contains("tiber.create_sync_failed created="),
        "MCP error should include partial-success error with created ref: {message}"
    );
    assert!(
        message.contains("-mcp-partial-create"),
        "MCP error should include the created task nickname: {message}"
    );
    assert!(
        message.contains("run tiber sync after resolving the sync error"),
        "MCP error should include recovery guidance: {message}"
    );
    assert!(
        message.contains("stderr_redacted=true"),
        "MCP error should report redaction instead of raw sync output: {message}"
    );
    assert!(
        message.contains("args_redacted=true"),
        "MCP error should report redacted sync command arguments: {message}"
    );
    assert!(
        !message.contains("secret@example.invalid"),
        "MCP error should not leak token-bearing remote details: {message}"
    );
    assert!(
        !message.contains("private/repo.git"),
        "MCP error should not leak private remote paths: {message}"
    );
    assert!(
        !message.contains(repo.path().to_str().expect("repo path should be utf8")),
        "MCP error should not leak local repository paths: {message}"
    );
    let stem = task_stem(&repo, "backlog", "mcp-partial-create");
    assert!(
        message.contains(&stem),
        "MCP error should include the exact locally created task ref {stem}: {message}"
    );

    fs::remove_file(&hook_path).expect("remove rejecting hook");
    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"tiber.sync","arguments":{}}}"#,
    );
    let sync = read_message(&mut stdout);
    assert!(sync.contains(r#""id":2"#));
    assert!(sync.contains("synced tiber"));

    let remote_listing = origin.git_output(["ls-tree", "-r", "--name-only", "tasks"]);
    assert_success_ref(&remote_listing);
    assert!(
        String::from_utf8(remote_listing.stdout)
            .expect("remote task listing should be utf8")
            .contains(&format!("backlog/{stem}.md")),
        "tiber sync should recover the locally created MCP task to origin/tasks"
    );

    drop(stdin);
    let status = child.wait().expect("wait for mcp process");
    assert!(status.success());
}
