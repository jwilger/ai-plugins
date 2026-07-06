use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run(env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: impl IntoIterator<Item = String>) -> Result<(), tiber_git::Error> {
    let args = args.into_iter().collect::<Vec<_>>();
    match args.as_slice() {
        [command] if command == "init" => tiber_git::init_repository(),
        [command] if command == "sync" => tiber_git::sync_repository(),
        [command, action] if command == "dashboard" && action == "serve" => {
            let runtime = tokio::runtime::Runtime::new().map_err(tiber_git::Error::Io)?;
            runtime.block_on(async {
                let port = env::var("TIBER_DASHBOARD_PORT")
                    .ok()
                    .and_then(|value| value.parse::<u16>().ok())
                    .unwrap_or(7417);
                let addr = format!("127.0.0.1:{port}");
                let listener = tokio::net::TcpListener::bind(&addr)
                    .await
                    .map_err(tiber_git::Error::Io)?;
                println!("tiber dashboard listening on http://{addr}");
                tiber_server::serve(listener).await
            })
        }
        [command, transport] if command == "mcp" && transport == "stdio" => {
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            tiber_mcp::run_stdio(stdin.lock(), stdout.lock())
        }
        [command, target_flag, target_dir, mode]
            if command == "install-bin" && target_flag == "--target-dir" =>
        {
            let apply = match mode.as_str() {
                "--dry-run" => false,
                "--apply" => true,
                _ => {
                    return Err(tiber_git::Error::Usage(
                        "install-bin requires --dry-run or --apply".to_string(),
                    ))
                }
            };
            let installed = tiber_git::install_bin(target_dir, apply)?;
            if apply {
                println!("installed {installed}");
            } else {
                println!("would install {installed}");
            }
            Ok(())
        }
        [command, title] if command == "create" => tiber_git::create_task(title).map(|_| ()),
        [command, task_ref] if command == "show" => {
            print!("{}", tiber_git::show_task(task_ref)?);
            Ok(())
        }
        [command, task_ref] if command == "metadata" => {
            let metadata = tiber_git::task_metadata(task_ref)?;
            println!(
                "{}\t{}\tcommitted_at={}",
                metadata.path,
                metadata.title,
                metadata.committed_at.unwrap_or_else(|| "uncommitted".to_string())
            );
            Ok(())
        }
        [command] if command == "list" => {
            for task in tiber_git::list_tasks()? {
                println!("{}\t{}", task.path, task.title);
            }
            Ok(())
        }
        [command] if command == "next" => {
            if let Some(task) = tiber_git::next_task()? {
                println!("{}\t{}", task.path, task.title);
            }
            Ok(())
        }
        [command, task_ref, status] if command == "transition" => {
            tiber_git::transition_task(task_ref, status)?;
            Ok(())
        }
        [command, task_ref, flag, before_ref] if command == "prioritize" && flag == "--before" => {
            tiber_git::prioritize_before(task_ref, before_ref)?;
            Ok(())
        }
        [command, from_ref, relation, to_ref] if command == "link" && relation == "blocks" => {
            tiber_git::link_blocks(from_ref, to_ref)?;
            Ok(())
        }
        [command, from_ref, relation, to_ref] if command == "unlink" && relation == "blocks" => {
            tiber_git::unlink_blocks(from_ref, to_ref)?;
            Ok(())
        }
        [command, action, task_ref, title] if command == "subtask" && action == "add" => {
            tiber_git::add_subtask(task_ref, title, &[])?;
            Ok(())
        }
        [command, action, task_ref, title, flag, after]
            if command == "subtask" && action == "add" && flag == "--after" =>
        {
            let after_refs = parse_comma_list(after);
            tiber_git::add_subtask(task_ref, title, &after_refs)?;
            Ok(())
        }
        [command, action, task_ref, index] if command == "subtask" && action == "check" => {
            tiber_git::set_subtask_checked(task_ref, index, true)?;
            Ok(())
        }
        [command, action, task_ref, index] if command == "subtask" && action == "uncheck" => {
            tiber_git::set_subtask_checked(task_ref, index, false)?;
            Ok(())
        }
        [command, task_ref, rest @ ..] if command == "update" => {
            let update = parse_update_args(rest)?;
            tiber_git::update_task(
                task_ref,
                update.title.as_deref(),
                update.summary.as_deref(),
                update.context.as_deref(),
                update.tags,
            )?;
            Ok(())
        }
        [command, action, task_ref, criterion]
            if command == "acceptance" && action == "add" =>
        {
            tiber_git::add_acceptance(task_ref, criterion)?;
            Ok(())
        }
        [command, action, task_ref, index] if command == "acceptance" && action == "check" => {
            tiber_git::set_acceptance_checked(task_ref, index, true)?;
            Ok(())
        }
        [command, action, task_ref, index] if command == "acceptance" && action == "uncheck" => {
            tiber_git::set_acceptance_checked(task_ref, index, false)?;
            Ok(())
        }
        [command, action, task_ref, index] if command == "acceptance" && action == "remove" => {
            tiber_git::remove_acceptance(task_ref, index)?;
            Ok(())
        }
        [command, action, task_ref, note] if command == "note" && action == "add" => {
            tiber_git::add_note(task_ref, note)?;
            Ok(())
        }
        [command, flag] if command == "validate" && flag == "--fix" => {
            for message in tiber_git::validate_fix()? {
                println!("{message}");
            }
            Ok(())
        }
        [command] if command == "close-from-trailers" => {
            for closed in tiber_git::close_from_trailers()? {
                println!("closed {closed}");
            }
            Ok(())
        }
        [command, subject, flag] if command == "scaffold" && subject == "repo" && flag == "--dry-run" => {
            for planned in tiber_git::scaffold_repo(false)? {
                println!("would write {planned}");
            }
            Ok(())
        }
        [command, subject, flag] if command == "scaffold" && subject == "repo" && flag == "--apply" => {
            for written in tiber_git::scaffold_repo(true)? {
                println!("wrote {written}");
            }
            Ok(())
        }
        _ => Err(tiber_git::Error::Usage(
            "usage: tiber init|sync|dashboard serve|mcp stdio|install-bin --target-dir <dir> --dry-run|--apply|create <title>|show <ref>|metadata <ref>|list|next|transition <ref> <status>|prioritize <ref> --before <ref>|link <ref> blocks <ref>|unlink <ref> blocks <ref>|subtask add <ref> <title> [--after s1,s2]|subtask check|uncheck|update <ref> [--title|--summary|--context|--tags]|acceptance add|check|uncheck|remove|note add|validate --fix|close-from-trailers|scaffold repo --dry-run|--apply".to_string(),
        )),
    }
}

fn parse_comma_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

struct UpdateArgs {
    title: Option<String>,
    summary: Option<String>,
    context: Option<String>,
    tags: Option<Vec<String>>,
}

fn parse_update_args(args: &[String]) -> Result<UpdateArgs, tiber_git::Error> {
    let mut update = UpdateArgs {
        title: None,
        summary: None,
        context: None,
        tags: None,
    };
    let mut index = 0;
    while index < args.len() {
        let flag = &args[index];
        let value = args.get(index + 1).ok_or_else(|| {
            tiber_git::Error::Usage(format!("missing value for update flag {flag}"))
        })?;
        match flag.as_str() {
            "--title" => update.title = Some(value.clone()),
            "--summary" => update.summary = Some(value.clone()),
            "--context" => update.context = Some(value.clone()),
            "--tags" => {
                update.tags = Some(parse_comma_list(value));
            }
            _ => {
                return Err(tiber_git::Error::Usage(format!(
                    "unknown update flag {flag}"
                )))
            }
        }
        index += 2;
    }
    if update.title.is_none()
        && update.summary.is_none()
        && update.context.is_none()
        && update.tags.is_none()
    {
        return Err(tiber_git::Error::Usage(
            "update requires at least one field".to_string(),
        ));
    }
    Ok(update)
}
