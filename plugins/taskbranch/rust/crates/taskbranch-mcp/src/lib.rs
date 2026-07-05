use std::io::{BufRead, Write};

use serde_json::{json, Value};

pub fn run_stdio(input: impl BufRead, mut output: impl Write) -> Result<(), taskbranch_git::Error> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request = serde_json::from_str::<Value>(&line)
            .map_err(|error| taskbranch_git::Error::Parse(format!("json_parse source={error}")))?;
        let response = handle_json_rpc(&request)?;
        writeln!(output, "{response}")?;
        output.flush()?;
    }
    Ok(())
}

pub fn handle_json_rpc(request: &Value) -> Result<Value, taskbranch_git::Error> {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| taskbranch_git::Error::Parse("mcp_method_missing=true".to_string()))?;

    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {
                "tools": {},
                "resources": {}
            },
            "serverInfo": {
                "name": "taskbranch",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
        "tools/list" => json!({ "tools": tools() }),
        "resources/list" => json!({ "resources": resources()? }),
        "tools/call" => {
            let name = request
                .pointer("/params/name")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    taskbranch_git::Error::Parse("mcp_tool_name_missing=true".to_string())
                })?;
            let arguments = request
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            call_tool(name, &arguments)?
        }
        "resources/read" => {
            let uri = request
                .pointer("/params/uri")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    taskbranch_git::Error::Parse("mcp_resource_uri_missing=true".to_string())
                })?;
            json!({
                "contents": [
                    {
                        "uri": uri,
                        "mimeType": "text/markdown",
                        "text": read_resource(uri)?
                    }
                ]
            })
        }
        _ => json!({ "error": format!("unsupported method: {method}") }),
    };

    Ok(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    }))
}

fn call_tool(name: &str, arguments: &Value) -> Result<Value, taskbranch_git::Error> {
    match name {
        "taskbranch.init" => {
            taskbranch_git::init_repository()?;
            Ok(text_content("initialized taskbranch".to_string()))
        }
        "taskbranch.sync" => {
            taskbranch_git::sync_repository()?;
            Ok(text_content("synced taskbranch".to_string()))
        }
        "taskbranch.create" => {
            let title = arguments
                .get("title")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    taskbranch_git::Error::Parse("mcp_tool_title_missing=true".to_string())
                })?;
            let created = taskbranch_git::create_task(title)?;
            Ok(text_content(format!("created {}", created.path)))
        }
        "taskbranch.list" => Ok(text_content(
            taskbranch_git::list_tasks()?
                .into_iter()
                .map(|task| format!("{}\t{}\n", task.path, task.title))
                .collect::<String>(),
        )),
        "taskbranch.show" => Ok(text_content(taskbranch_git::show_task(required_string(
            arguments, "ref",
        )?)?)),
        "taskbranch.metadata" => {
            let metadata = taskbranch_git::task_metadata(required_string(arguments, "ref")?)?;
            Ok(text_content(format!(
                "{}\t{}\tcommitted_at={}\n",
                metadata.path,
                metadata.title,
                metadata
                    .committed_at
                    .unwrap_or_else(|| "uncommitted".to_string())
            )))
        }
        "taskbranch.next" => Ok(text_content(
            taskbranch_git::next_task()?
                .map(|task| format!("{}\t{}\n", task.path, task.title))
                .unwrap_or_default(),
        )),
        "taskbranch.transition" => {
            let task_ref = required_string(arguments, "ref")?;
            let status = required_string(arguments, "status")?;
            let transitioned = taskbranch_git::transition_task(task_ref, status)?;
            Ok(text_content(format!("transitioned {}", transitioned.path)))
        }
        "taskbranch.prioritize" => {
            let task_ref = required_string(arguments, "ref")?;
            let before_ref = required_string(arguments, "before")?;
            taskbranch_git::prioritize_before(task_ref, before_ref)?;
            Ok(text_content(format!(
                "prioritized {task_ref} before {before_ref}"
            )))
        }
        "taskbranch.link" => {
            let from_ref = required_string(arguments, "from")?;
            let to_ref = required_string(arguments, "to")?;
            taskbranch_git::link_blocks(from_ref, to_ref)?;
            Ok(text_content(format!("linked {from_ref} blocks {to_ref}")))
        }
        "taskbranch.unlink" => {
            let from_ref = required_string(arguments, "from")?;
            let to_ref = required_string(arguments, "to")?;
            taskbranch_git::unlink_blocks(from_ref, to_ref)?;
            Ok(text_content(format!("unlinked {from_ref} blocks {to_ref}")))
        }
        "taskbranch.subtask.add" => {
            let task_ref = required_string(arguments, "ref")?;
            let title = required_string(arguments, "title")?;
            taskbranch_git::add_subtask(task_ref, title)?;
            Ok(text_content(format!("updated {task_ref}")))
        }
        "taskbranch.subtask.check" => {
            let task_ref = required_string(arguments, "ref")?;
            let index = required_string(arguments, "index")?;
            taskbranch_git::set_subtask_checked(task_ref, index, true)?;
            Ok(text_content(format!("updated {task_ref}")))
        }
        "taskbranch.subtask.uncheck" => {
            let task_ref = required_string(arguments, "ref")?;
            let index = required_string(arguments, "index")?;
            taskbranch_git::set_subtask_checked(task_ref, index, false)?;
            Ok(text_content(format!("updated {task_ref}")))
        }
        "taskbranch.validate_fix" => Ok(text_content(
            taskbranch_git::validate_fix()?
                .into_iter()
                .map(|message| format!("{message}\n"))
                .collect::<String>(),
        )),
        "taskbranch.close_from_trailers" => Ok(text_content(
            taskbranch_git::close_from_trailers()?
                .into_iter()
                .map(|closed| format!("closed {closed}\n"))
                .collect::<String>(),
        )),
        "taskbranch.scaffold_repo_dry_run" => Ok(text_content(
            taskbranch_git::scaffold_repo(false)?
                .into_iter()
                .map(|planned| format!("would write {planned}\n"))
                .collect::<String>(),
        )),
        "taskbranch.scaffold_repo_apply" => Ok(text_content(
            taskbranch_git::scaffold_repo(true)?
                .into_iter()
                .map(|written| format!("wrote {written}\n"))
                .collect::<String>(),
        )),
        _ => Ok(text_content(format!("unsupported tool: {name}"))),
    }
}

fn required_string<'a>(arguments: &'a Value, name: &str) -> Result<&'a str, taskbranch_git::Error> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| taskbranch_git::Error::Parse(format!("mcp_argument_missing name={name}")))
}

fn tools() -> Vec<Value> {
    vec![
        tool(
            "taskbranch.init",
            "Initialize taskbranch",
            "Initialize taskbranch in the current repository.",
            json!({}),
            vec![],
        ),
        tool(
            "taskbranch.sync",
            "Sync taskbranch",
            "Sync local task state into the Git-backed tasks branch.",
            json!({}),
            vec![],
        ),
        tool(
            "taskbranch.create",
            "Create task",
            "Create a taskbranch task in todo status.",
            json!({ "title": { "type": "string" } }),
            vec!["title"],
        ),
        tool(
            "taskbranch.list",
            "List tasks",
            "List taskbranch tasks in board order.",
            json!({}),
            vec![],
        ),
        tool(
            "taskbranch.show",
            "Show task",
            "Read a task by ref.",
            json!({ "ref": { "type": "string" } }),
            vec!["ref"],
        ),
        tool(
            "taskbranch.metadata",
            "Read task metadata",
            "Read task path, title, and tasks-branch commit time by ref.",
            json!({ "ref": { "type": "string" } }),
            vec!["ref"],
        ),
        tool(
            "taskbranch.next",
            "Next task",
            "Read the next task in board order.",
            json!({}),
            vec![],
        ),
        tool(
            "taskbranch.transition",
            "Transition task",
            "Move a task to another status.",
            json!({ "ref": { "type": "string" }, "status": { "type": "string" } }),
            vec!["ref", "status"],
        ),
        tool(
            "taskbranch.prioritize",
            "Prioritize task",
            "Move a task before another task in board order.",
            json!({ "ref": { "type": "string" }, "before": { "type": "string" } }),
            vec!["ref", "before"],
        ),
        tool(
            "taskbranch.link",
            "Link task dependency",
            "Add a blocks relationship between two tasks.",
            json!({ "from": { "type": "string" }, "to": { "type": "string" } }),
            vec!["from", "to"],
        ),
        tool(
            "taskbranch.unlink",
            "Unlink task dependency",
            "Remove a blocks relationship between two tasks.",
            json!({ "from": { "type": "string" }, "to": { "type": "string" } }),
            vec!["from", "to"],
        ),
        tool(
            "taskbranch.subtask.add",
            "Add subtask",
            "Add a checklist subtask to a task.",
            json!({ "ref": { "type": "string" }, "title": { "type": "string" } }),
            vec!["ref", "title"],
        ),
        tool(
            "taskbranch.subtask.check",
            "Check subtask",
            "Mark a subtask checked by one-based index.",
            json!({ "ref": { "type": "string" }, "index": { "type": "string" } }),
            vec!["ref", "index"],
        ),
        tool(
            "taskbranch.subtask.uncheck",
            "Uncheck subtask",
            "Mark a subtask unchecked by one-based index.",
            json!({ "ref": { "type": "string" }, "index": { "type": "string" } }),
            vec!["ref", "index"],
        ),
        tool(
            "taskbranch.validate_fix",
            "Validate and safely fix",
            "Run taskbranch validation with safe autofixes.",
            json!({}),
            vec![],
        ),
        tool(
            "taskbranch.close_from_trailers",
            "Close from trailers",
            "Close tasks referenced by Closes trailers in Git history.",
            json!({}),
            vec![],
        ),
        tool(
            "taskbranch.scaffold_repo_dry_run",
            "Preview repository scaffold",
            "Preview repository files taskbranch can scaffold.",
            json!({}),
            vec![],
        ),
        tool(
            "taskbranch.scaffold_repo_apply",
            "Apply repository scaffold",
            "Write repository files taskbranch scaffolds.",
            json!({}),
            vec![],
        ),
    ]
}

fn tool(
    name: &str,
    title: &str,
    description: &str,
    properties: Value,
    required: Vec<&str>,
) -> Value {
    json!({
        "name": name,
        "title": title,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required
        }
    })
}

fn text_content(text: String) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ]
    })
}

fn resources() -> Result<Vec<Value>, taskbranch_git::Error> {
    let mut resources = vec![
        json!({
            "uri": "tasks://board",
            "name": "Taskbranch board",
            "mimeType": "text/markdown"
        }),
        json!({
            "uri": "tasks://docs/tree",
            "name": "Taskbranch docs tree",
            "mimeType": "text/markdown"
        }),
    ];
    for task in taskbranch_git::list_tasks()? {
        resources.push(json!({
            "uri": format!("tasks://task/{}", task.path),
            "name": task.title,
            "mimeType": "text/markdown"
        }));
    }
    for doc in taskbranch_git::list_docs()? {
        resources.push(json!({
            "uri": format!("tasks://{doc}"),
            "name": doc,
            "mimeType": "text/markdown"
        }));
    }
    Ok(resources)
}

fn read_resource(uri: &str) -> Result<String, taskbranch_git::Error> {
    if uri == "tasks://board" {
        return taskbranch_git::list_tasks().map(|tasks| {
            tasks
                .into_iter()
                .map(|task| format!("{}\t{}\n", task.path, task.title))
                .collect::<String>()
        });
    }
    if uri == "tasks://docs/tree" {
        return taskbranch_git::list_docs().map(|docs| {
            docs.into_iter()
                .map(|doc| format!("{doc}\n"))
                .collect::<String>()
        });
    }
    if let Some(task_ref) = uri.strip_prefix("tasks://task/") {
        return taskbranch_git::show_task(task_ref);
    }
    if let Some(doc_ref) = uri.strip_prefix("tasks://docs/") {
        return taskbranch_git::read_doc(&format!("docs/{doc_ref}"));
    }
    Err(taskbranch_git::Error::Parse(format!(
        "unsupported_resource uri={uri}"
    )))
}
