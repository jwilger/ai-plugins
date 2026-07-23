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
    assert!(codex_setup_tool.contains("Couldn't get agent socket?"));
    assert!(codex_setup_tool.contains("SSH_AUTH_SOCK"));
    assert!(codex_setup_tool.contains("env_vars = [\\\"SSH_AUTH_SOCK\\\"]"));
    assert!(codex_setup_tool.contains("preserves the absolute installed launcher"));
    assert!(codex_setup_tool.contains("Never forward SSH_AUTH_SOCK to a PATH-resolved"));
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
    assert!(scaffold.contains("already configured .gitignore"));

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
