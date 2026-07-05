mod support;

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use support::{assert_success, TempRepo};

#[test]
fn mcp_stdio_exposes_tools_and_task_resources() {
    let repo = TempRepo::initialized();
    assert_success(repo.taskbranch(["init"]));
    assert_success(repo.taskbranch(["create", "Expose MCP task"]));
    std::fs::create_dir_all(repo.path().join("docs/guides")).expect("create docs directory");
    std::fs::write(
        repo.path().join("docs/guides/taskbranch.md"),
        "# Taskbranch guide\n\nUse taskbranch mcp stdio.\n",
    )
    .expect("write doc");
    let mut child = Command::new(env!("CARGO_BIN_EXE_taskbranch"))
        .args(["mcp", "stdio"])
        .current_dir(repo.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn taskbranch mcp stdio");
    let mut stdin = child.stdin.take().expect("mcp stdin should be available");
    let stdout = child.stdout.take().expect("mcp stdout should be available");
    let mut stdout = BufReader::new(stdout);

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#,
    );
    let initialize = read_message(&mut stdout);
    assert!(initialize.contains(r#""id":1"#));
    assert!(initialize.contains(r#""name":"taskbranch""#));
    assert!(initialize.contains(r#""tools":{}"#));
    assert!(initialize.contains(r#""resources":{}"#));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
    );
    let tools = read_message(&mut stdout);
    assert!(tools.contains(r#""id":2"#));
    assert!(tools.contains(r#""name":"taskbranch.create""#));
    assert!(tools.contains(r#""name":"taskbranch.list""#));
    assert!(tools.contains(r#""name":"taskbranch.show""#));
    assert!(tools.contains(r#""name":"taskbranch.metadata""#));
    assert!(tools.contains(r#""name":"taskbranch.next""#));
    assert!(tools.contains(r#""name":"taskbranch.transition""#));
    assert!(tools.contains(r#""name":"taskbranch.prioritize""#));
    assert!(tools.contains(r#""name":"taskbranch.link""#));
    assert!(tools.contains(r#""name":"taskbranch.unlink""#));
    assert!(tools.contains(r#""name":"taskbranch.subtask.add""#));
    assert!(tools.contains(r#""name":"taskbranch.subtask.check""#));
    assert!(tools.contains(r#""name":"taskbranch.subtask.uncheck""#));
    assert!(tools.contains(r#""name":"taskbranch.validate_fix""#));
    assert!(tools.contains(r#""name":"taskbranch.close_from_trailers""#));
    assert!(tools.contains(r#""name":"taskbranch.scaffold_repo_dry_run""#));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":3,"method":"resources/list"}"#,
    );
    let resources = read_message(&mut stdout);
    assert!(resources.contains(r#""id":3"#));
    assert!(resources.contains(r#""uri":"tasks://board""#));
    assert!(resources.contains(r#""uri":"tasks://task/todo/expose-mcp-task.md""#));
    assert!(resources.contains(r#""uri":"tasks://docs/tree""#));
    assert!(resources.contains(r#""uri":"tasks://docs/guides/taskbranch.md""#));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":4,"method":"resources/read","params":{"uri":"tasks://task/todo/expose-mcp-task.md"}}"#,
    );
    let task = read_message(&mut stdout);
    assert!(task.contains(r#""id":4"#));
    assert!(task.contains("# Expose MCP task"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"taskbranch.create","arguments":{"title":"Created through MCP"}}}"#,
    );
    let create = read_message(&mut stdout);
    assert!(create.contains(r#""id":5"#));
    assert!(create.contains("todo/created-through-mcp.md"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"taskbranch.list","arguments":{}}}"#,
    );
    let list = read_message(&mut stdout);
    assert!(list.contains(r#""id":6"#));
    assert!(list.contains("todo/expose-mcp-task.md"));
    assert!(list.contains("todo/created-through-mcp.md"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":7,"method":"resources/read","params":{"uri":"tasks://task/todo/created-through-mcp.md"}}"#,
    );
    let created = read_message(&mut stdout);
    assert!(created.contains(r#""id":7"#));
    assert!(created.contains("# Created through MCP"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":8,"method":"resources/read","params":{"uri":"tasks://docs/tree"}}"#,
    );
    let docs_tree = read_message(&mut stdout);
    assert!(docs_tree.contains(r#""id":8"#));
    assert!(docs_tree.contains("docs/guides/taskbranch.md"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":9,"method":"resources/read","params":{"uri":"tasks://docs/guides/taskbranch.md"}}"#,
    );
    let doc = read_message(&mut stdout);
    assert!(doc.contains(r#""id":9"#));
    assert!(doc.contains("# Taskbranch guide"));
    assert!(doc.contains("taskbranch mcp stdio"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"taskbranch.show","arguments":{"ref":"todo/expose-mcp-task.md"}}}"#,
    );
    let show = read_message(&mut stdout);
    assert!(show.contains(r#""id":10"#));
    assert!(show.contains("# Expose MCP task"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"taskbranch.metadata","arguments":{"ref":"todo/expose-mcp-task.md"}}}"#,
    );
    let metadata = read_message(&mut stdout);
    assert!(metadata.contains(r#""id":11"#));
    assert!(
        metadata.contains(r#"todo/expose-mcp-task.md\tExpose MCP task\tcommitted_at=uncommitted"#)
    );

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"taskbranch.next","arguments":{}}}"#,
    );
    let next = read_message(&mut stdout);
    assert!(next.contains(r#""id":12"#));
    assert!(next.contains("todo/expose-mcp-task.md"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"taskbranch.subtask.add","arguments":{"ref":"todo/created-through-mcp.md","title":"Write MCP mirror tests"}}}"#,
    );
    let subtask_add = read_message(&mut stdout);
    assert!(subtask_add.contains(r#""id":13"#));
    assert!(subtask_add.contains("updated todo/created-through-mcp.md"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"taskbranch.subtask.check","arguments":{"ref":"todo/created-through-mcp.md","index":"1"}}}"#,
    );
    let subtask_check = read_message(&mut stdout);
    assert!(subtask_check.contains(r#""id":14"#));
    assert!(subtask_check.contains("updated todo/created-through-mcp.md"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":15,"method":"tools/call","params":{"name":"taskbranch.transition","arguments":{"ref":"todo/created-through-mcp.md","status":"doing"}}}"#,
    );
    let transition = read_message(&mut stdout);
    assert!(transition.contains(r#""id":15"#));
    assert!(transition.contains("doing/created-through-mcp.md"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":16,"method":"tools/call","params":{"name":"taskbranch.link","arguments":{"from":"doing/created-through-mcp.md","to":"todo/expose-mcp-task.md"}}}"#,
    );
    let link = read_message(&mut stdout);
    assert!(link.contains(r#""id":16"#));
    assert!(link.contains("linked doing/created-through-mcp.md blocks todo/expose-mcp-task.md"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":17,"method":"tools/call","params":{"name":"taskbranch.unlink","arguments":{"from":"doing/created-through-mcp.md","to":"todo/expose-mcp-task.md"}}}"#,
    );
    let unlink = read_message(&mut stdout);
    assert!(unlink.contains(r#""id":17"#));
    assert!(unlink.contains("unlinked doing/created-through-mcp.md blocks todo/expose-mcp-task.md"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":18,"method":"tools/call","params":{"name":"taskbranch.prioritize","arguments":{"ref":"doing/created-through-mcp.md","before":"todo/expose-mcp-task.md"}}}"#,
    );
    let prioritize = read_message(&mut stdout);
    assert!(prioritize.contains(r#""id":18"#));
    assert!(prioritize
        .contains("prioritized doing/created-through-mcp.md before todo/expose-mcp-task.md"));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":19,"method":"tools/call","params":{"name":"taskbranch.validate_fix","arguments":{}}}"#,
    );
    let validate = read_message(&mut stdout);
    assert!(validate.contains(r#""id":19"#));

    write_message(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":20,"method":"tools/call","params":{"name":"taskbranch.scaffold_repo_dry_run","arguments":{}}}"#,
    );
    let scaffold = read_message(&mut stdout);
    assert!(scaffold.contains(r#""id":20"#));
    assert!(scaffold.contains("would write .gitignore"));

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
    line
}
