use std::io::{BufRead, Write};

use serde_json::{json, Value};

pub fn run_stdio(input: impl BufRead, mut output: impl Write) -> Result<(), tiber_git::Error> {
    for line in input.lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                writeln!(
                    output,
                    "{}",
                    error_response(Value::Null, -32603, &error.to_string())
                )?;
                output.flush()?;
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let request = match serde_json::from_str::<Value>(&line) {
            Ok(request) => request,
            Err(error) => {
                writeln!(
                    output,
                    "{}",
                    error_response(Value::Null, -32700, &format!("json_parse source={error}"))
                )?;
                output.flush()?;
                continue;
            }
        };
        if request.get("id").is_none() {
            continue;
        }
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let response = match handle_json_rpc(&request) {
            Ok(response) => response,
            Err(error) => error_response(id, -32603, &error.to_string()),
        };
        writeln!(output, "{response}")?;
        output.flush()?;
    }
    Ok(())
}

pub fn handle_json_rpc(request: &Value) -> Result<Value, tiber_git::Error> {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return Ok(error_response(id, -32600, "mcp_method_missing=true"));
    };

    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {
                "tools": {},
                "resources": {}
            },
            "serverInfo": {
                "name": "tiber",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
        "tools/list" => json!({ "tools": tools() }),
        "resources/list" => json!({ "resources": resources()? }),
        "tools/call" => {
            let Some(name) = request.pointer("/params/name").and_then(Value::as_str) else {
                return Ok(error_response(id, -32602, "mcp_tool_name_missing=true"));
            };
            let arguments = request
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            call_tool(name, &arguments)?
        }
        "resources/read" => {
            let Some(uri) = request.pointer("/params/uri").and_then(Value::as_str) else {
                return Ok(error_response(id, -32602, "mcp_resource_uri_missing=true"));
            };
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
        _ => {
            return Ok(error_response(
                id,
                -32601,
                &format!("unsupported method: {method}"),
            ))
        }
    };

    Ok(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    }))
}

fn call_tool(name: &str, arguments: &Value) -> Result<Value, tiber_git::Error> {
    match name {
        "tiber.init" => {
            tiber_git::init_repository()?;
            Ok(text_content("initialized tiber".to_string()))
        }
        "tiber.sync" => {
            tiber_git::sync_repository()?;
            Ok(text_content("synced tiber".to_string()))
        }
        "tiber.create" => {
            let title = arguments
                .get("title")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    tiber_git::Error::Parse("mcp_tool_title_missing=true".to_string())
                })?;
            let created = tiber_git::create_task(title)?;
            Ok(text_content(format!("created {}", created.path)))
        }
        "tiber.list" => Ok(text_content(
            tiber_git::list_tasks()?
                .into_iter()
                .map(|task| format!("{}\t{}\n", task.path, task.title))
                .collect::<String>(),
        )),
        "tiber.show" => Ok(text_content(tiber_git::show_task(required_string(
            arguments, "ref",
        )?)?)),
        "tiber.metadata" => {
            let metadata = tiber_git::task_metadata(required_string(arguments, "ref")?)?;
            Ok(text_content(format!(
                "{}\t{}\tcommitted_at={}\n",
                metadata.path,
                metadata.title,
                metadata
                    .committed_at
                    .unwrap_or_else(|| "uncommitted".to_string())
            )))
        }
        "tiber.next" => Ok(text_content(
            tiber_git::next_task()?
                .map(|task| format!("{}\t{}\n", task.path, task.title))
                .unwrap_or_default(),
        )),
        "tiber.transition" => {
            let task_ref = required_string(arguments, "ref")?;
            let status = required_string(arguments, "status")?;
            let transitioned = tiber_git::transition_task(task_ref, status)?;
            Ok(text_content(format!("transitioned {}", transitioned.path)))
        }
        "tiber.prioritize" => {
            let task_ref = required_string(arguments, "ref")?;
            let before_ref = required_string(arguments, "before")?;
            tiber_git::prioritize_before(task_ref, before_ref)?;
            Ok(text_content(format!(
                "prioritized {task_ref} before {before_ref}"
            )))
        }
        "tiber.link" => {
            let from_ref = required_string(arguments, "from")?;
            let to_ref = required_string(arguments, "to")?;
            tiber_git::link_blocks(from_ref, to_ref)?;
            Ok(text_content(format!("linked {from_ref} blocks {to_ref}")))
        }
        "tiber.unlink" => {
            let from_ref = required_string(arguments, "from")?;
            let to_ref = required_string(arguments, "to")?;
            tiber_git::unlink_blocks(from_ref, to_ref)?;
            Ok(text_content(format!("unlinked {from_ref} blocks {to_ref}")))
        }
        "tiber.subtask.add" => {
            let task_ref = required_string(arguments, "ref")?;
            let title = required_string(arguments, "title")?;
            let after_refs = optional_string_array(arguments, "after")?.unwrap_or_default();
            tiber_git::add_subtask(task_ref, title, &after_refs)?;
            Ok(text_content(format!("updated {task_ref}")))
        }
        "tiber.subtask.check" => {
            let task_ref = required_string(arguments, "ref")?;
            let index = required_string(arguments, "index")?;
            tiber_git::set_subtask_checked(task_ref, index, true)?;
            Ok(text_content(format!("updated {task_ref}")))
        }
        "tiber.subtask.uncheck" => {
            let task_ref = required_string(arguments, "ref")?;
            let index = required_string(arguments, "index")?;
            tiber_git::set_subtask_checked(task_ref, index, false)?;
            Ok(text_content(format!("updated {task_ref}")))
        }
        "tiber.update" => {
            let task_ref = required_string(arguments, "ref")?;
            tiber_git::update_task(
                task_ref,
                tiber_git::TaskUpdate {
                    title: optional_string(arguments, "title"),
                    summary: optional_string(arguments, "summary"),
                    context: optional_string(arguments, "context"),
                    tags: optional_tags(arguments)?,
                    pr_mr_url: optional_string(arguments, "pr_mr_url"),
                    pr_mr_status: optional_string(arguments, "pr_mr_status"),
                },
            )?;
            Ok(text_content(format!("updated {task_ref}")))
        }
        "tiber.acceptance.add" => {
            let task_ref = required_string(arguments, "ref")?;
            let criterion = required_string(arguments, "criterion")?;
            tiber_git::add_acceptance(task_ref, criterion)?;
            Ok(text_content(format!("updated {task_ref}")))
        }
        "tiber.acceptance.check" => {
            let task_ref = required_string(arguments, "ref")?;
            let index = required_string(arguments, "index")?;
            tiber_git::set_acceptance_checked(task_ref, index, true)?;
            Ok(text_content(format!("updated {task_ref}")))
        }
        "tiber.acceptance.uncheck" => {
            let task_ref = required_string(arguments, "ref")?;
            let index = required_string(arguments, "index")?;
            tiber_git::set_acceptance_checked(task_ref, index, false)?;
            Ok(text_content(format!("updated {task_ref}")))
        }
        "tiber.acceptance.remove" => {
            let task_ref = required_string(arguments, "ref")?;
            let index = required_string(arguments, "index")?;
            tiber_git::remove_acceptance(task_ref, index)?;
            Ok(text_content(format!("updated {task_ref}")))
        }
        "tiber.note.add" => {
            let task_ref = required_string(arguments, "ref")?;
            let note = required_string(arguments, "note")?;
            tiber_git::add_note(task_ref, note)?;
            Ok(text_content(format!("updated {task_ref}")))
        }
        "tiber.validate_fix" => Ok(text_content(
            tiber_git::validate_fix()?
                .into_iter()
                .map(|message| format!("{message}\n"))
                .collect::<String>(),
        )),
        "tiber.close_from_trailers" => Ok(text_content(
            tiber_git::close_from_trailers()?
                .into_iter()
                .map(|closed| format!("closed {closed}\n"))
                .collect::<String>(),
        )),
        "tiber.scaffold_repo_dry_run" => Ok(text_content(
            tiber_git::scaffold_repo(false)?
                .into_iter()
                .map(|planned| format!("would write {planned}\n"))
                .collect::<String>(),
        )),
        "tiber.scaffold_repo_apply" => Ok(text_content(
            tiber_git::scaffold_repo(true)?
                .into_iter()
                .map(|written| format!("wrote {written}\n"))
                .collect::<String>(),
        )),
        "tiber.install_bin" => {
            let target_dir = required_string(arguments, "target_dir")?;
            let apply = arguments
                .get("apply")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let installed = tiber_git::install_bin(target_dir, apply)?;
            if apply {
                Ok(text_content(format!("installed {installed}")))
            } else {
                Ok(text_content(format!("would install {installed}")))
            }
        }
        _ => Err(tiber_git::Error::Parse(format!(
            "unsupported_mcp_tool name={name}"
        ))),
    }
}

fn required_string<'a>(arguments: &'a Value, name: &str) -> Result<&'a str, tiber_git::Error> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| tiber_git::Error::Parse(format!("mcp_argument_missing name={name}")))
}

fn optional_string<'a>(arguments: &'a Value, name: &str) -> Option<&'a str> {
    arguments.get(name).and_then(Value::as_str)
}

fn optional_tags(arguments: &Value) -> Result<Option<Vec<String>>, tiber_git::Error> {
    optional_string_array(arguments, "tags")
}

fn optional_string_array(
    arguments: &Value,
    name: &str,
) -> Result<Option<Vec<String>>, tiber_git::Error> {
    let Some(values) = arguments.get(name) else {
        return Ok(None);
    };
    let values = values
        .as_array()
        .ok_or_else(|| tiber_git::Error::Parse(format!("mcp_argument_invalid name={name}")))?;
    Ok(Some(
        values
            .iter()
            .map(|value| {
                value.as_str().map(str::to_string).ok_or_else(|| {
                    tiber_git::Error::Parse(format!("mcp_argument_invalid name={name}"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn tools() -> Vec<Value> {
    vec![
        tool(
            "tiber.init",
            "Initialize tiber",
            "Initialize tiber in the current repository.",
            json!({}),
            vec![],
        ),
        tool(
            "tiber.sync",
            "Sync tiber",
            "Sync local task state into the Git-backed tasks branch.",
            json!({}),
            vec![],
        ),
        tool(
            "tiber.create",
            "Create task",
            "Create a tiber task in backlog status.",
            json!({ "title": { "type": "string" } }),
            vec!["title"],
        ),
        tool(
            "tiber.list",
            "List tasks",
            "List tiber tasks in board order.",
            json!({}),
            vec![],
        ),
        tool(
            "tiber.show",
            "Show task",
            "Read a task by ref.",
            json!({ "ref": { "type": "string" } }),
            vec!["ref"],
        ),
        tool(
            "tiber.metadata",
            "Read task metadata",
            "Read task path, title, and tasks-branch commit time by ref.",
            json!({ "ref": { "type": "string" } }),
            vec!["ref"],
        ),
        tool(
            "tiber.next",
            "Next task",
            "Read the next task in board order.",
            json!({}),
            vec![],
        ),
        tool(
            "tiber.transition",
            "Transition task",
            "Move a task to another status.",
            json!({ "ref": { "type": "string" }, "status": { "type": "string" } }),
            vec!["ref", "status"],
        ),
        tool(
            "tiber.prioritize",
            "Prioritize task",
            "Move a task before another task in board order.",
            json!({ "ref": { "type": "string" }, "before": { "type": "string" } }),
            vec!["ref", "before"],
        ),
        tool(
            "tiber.link",
            "Link task dependency",
            "Add a blocks relationship between two tasks.",
            json!({ "from": { "type": "string" }, "to": { "type": "string" } }),
            vec!["from", "to"],
        ),
        tool(
            "tiber.unlink",
            "Unlink task dependency",
            "Remove a blocks relationship between two tasks.",
            json!({ "from": { "type": "string" }, "to": { "type": "string" } }),
            vec!["from", "to"],
        ),
        tool(
            "tiber.subtask.add",
            "Add subtask",
            "Add a checklist subtask to a task.",
            json!({
                "ref": { "type": "string" },
                "title": { "type": "string" },
                "after": { "type": "array", "items": { "type": "string" } }
            }),
            vec!["ref", "title"],
        ),
        tool(
            "tiber.subtask.check",
            "Check subtask",
            "Mark a subtask checked by one-based index.",
            json!({ "ref": { "type": "string" }, "index": { "type": "string" } }),
            vec!["ref", "index"],
        ),
        tool(
            "tiber.subtask.uncheck",
            "Uncheck subtask",
            "Mark a subtask unchecked by one-based index.",
            json!({ "ref": { "type": "string" }, "index": { "type": "string" } }),
            vec!["ref", "index"],
        ),
        tool(
            "tiber.update",
            "Update task",
            "Update task title, summary, context, tags, or PR/MR tracking fields.",
            json!({
                "ref": { "type": "string" },
                "title": { "type": "string" },
                "summary": { "type": "string" },
                "context": { "type": "string" },
                "tags": { "type": "array", "items": { "type": "string" } },
                "pr_mr_url": { "type": "string" },
                "pr_mr_status": { "type": "string" }
            }),
            vec!["ref"],
        ),
        tool(
            "tiber.acceptance.add",
            "Add acceptance criterion",
            "Add an acceptance criterion to a task.",
            json!({ "ref": { "type": "string" }, "criterion": { "type": "string" } }),
            vec!["ref", "criterion"],
        ),
        tool(
            "tiber.acceptance.check",
            "Check acceptance criterion",
            "Mark an acceptance criterion checked by one-based index.",
            json!({ "ref": { "type": "string" }, "index": { "type": "string" } }),
            vec!["ref", "index"],
        ),
        tool(
            "tiber.acceptance.uncheck",
            "Uncheck acceptance criterion",
            "Mark an acceptance criterion unchecked by one-based index.",
            json!({ "ref": { "type": "string" }, "index": { "type": "string" } }),
            vec!["ref", "index"],
        ),
        tool(
            "tiber.acceptance.remove",
            "Remove acceptance criterion",
            "Remove an acceptance criterion by one-based index.",
            json!({ "ref": { "type": "string" }, "index": { "type": "string" } }),
            vec!["ref", "index"],
        ),
        tool(
            "tiber.note.add",
            "Add note",
            "Append a dated note to a task.",
            json!({ "ref": { "type": "string" }, "note": { "type": "string" } }),
            vec!["ref", "note"],
        ),
        tool(
            "tiber.validate_fix",
            "Validate and safely fix",
            "Run tiber validation with safe autofixes.",
            json!({}),
            vec![],
        ),
        tool(
            "tiber.close_from_trailers",
            "Close from trailers",
            "Close tasks referenced by Closes trailers in Git history.",
            json!({}),
            vec![],
        ),
        tool(
            "tiber.scaffold_repo_dry_run",
            "Preview repository scaffold",
            "Preview repository files tiber can scaffold.",
            json!({}),
            vec![],
        ),
        tool(
            "tiber.scaffold_repo_apply",
            "Apply repository scaffold",
            "Write repository files tiber scaffolds.",
            json!({}),
            vec![],
        ),
        tool(
            "tiber.install_bin",
            "Install tiber launcher",
            "Preview or install the bundled tiber launcher into a target directory.",
            json!({
                "target_dir": { "type": "string" },
                "apply": { "type": "boolean" }
            }),
            vec!["target_dir"],
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

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

fn resources() -> Result<Vec<Value>, tiber_git::Error> {
    let mut resources = vec![
        json!({
            "uri": "tasks://board",
            "name": "Tiber board",
            "mimeType": "text/markdown"
        }),
        json!({
            "uri": "tasks://docs/tree",
            "name": "Tiber docs tree",
            "mimeType": "text/markdown"
        }),
    ];
    for task in tiber_git::list_tasks()? {
        resources.push(json!({
            "uri": format!("tasks://task/{}", task.path),
            "name": task.title,
            "mimeType": "text/markdown"
        }));
    }
    for doc in tiber_git::list_docs()? {
        resources.push(json!({
            "uri": format!("tasks://{doc}"),
            "name": doc,
            "mimeType": "text/markdown"
        }));
    }
    Ok(resources)
}

fn read_resource(uri: &str) -> Result<String, tiber_git::Error> {
    if uri == "tasks://board" {
        return tiber_git::list_tasks().map(|tasks| {
            tasks
                .into_iter()
                .map(|task| format!("{}\t{}\n", task.path, task.title))
                .collect::<String>()
        });
    }
    if uri == "tasks://docs/tree" {
        return tiber_git::list_docs().map(|docs| {
            docs.into_iter()
                .map(|doc| format!("{doc}\n"))
                .collect::<String>()
        });
    }
    if let Some(task_ref) = uri.strip_prefix("tasks://task/") {
        return tiber_git::show_task(task_ref);
    }
    if let Some(doc_ref) = uri.strip_prefix("tasks://docs/") {
        return tiber_git::read_doc(&format!("docs/{doc_ref}"));
    }
    Err(tiber_git::Error::Parse(format!(
        "unsupported_resource uri={uri}"
    )))
}
