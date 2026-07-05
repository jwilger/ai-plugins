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

fn run(args: impl IntoIterator<Item = String>) -> Result<(), taskbranch_git::Error> {
    let args = args.into_iter().collect::<Vec<_>>();
    match args.as_slice() {
        [command] if command == "init" => taskbranch_git::init_repository(),
        [command] if command == "sync" => taskbranch_git::sync_repository(),
        [command, action] if command == "dashboard" && action == "serve" => {
            let runtime = tokio::runtime::Runtime::new().map_err(taskbranch_git::Error::Io)?;
            runtime.block_on(async {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:7417")
                    .await
                    .map_err(taskbranch_git::Error::Io)?;
                taskbranch_server::serve(listener).await
            })
        }
        [command, transport] if command == "mcp" && transport == "stdio" => {
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            taskbranch_mcp::run_stdio(stdin.lock(), stdout.lock())
        }
        [command, title] if command == "create" => taskbranch_git::create_task(title).map(|_| ()),
        [command, task_ref] if command == "show" => {
            print!("{}", taskbranch_git::show_task(task_ref)?);
            Ok(())
        }
        [command, task_ref] if command == "metadata" => {
            let metadata = taskbranch_git::task_metadata(task_ref)?;
            println!(
                "{}\t{}\tcommitted_at={}",
                metadata.path,
                metadata.title,
                metadata.committed_at.unwrap_or_else(|| "uncommitted".to_string())
            );
            Ok(())
        }
        [command] if command == "list" => {
            for task in taskbranch_git::list_tasks()? {
                println!("{}\t{}", task.path, task.title);
            }
            Ok(())
        }
        [command] if command == "next" => {
            if let Some(task) = taskbranch_git::next_task()? {
                println!("{}\t{}", task.path, task.title);
            }
            Ok(())
        }
        [command, task_ref, status] if command == "transition" => {
            taskbranch_git::transition_task(task_ref, status)?;
            Ok(())
        }
        [command, task_ref, flag, before_ref] if command == "prioritize" && flag == "--before" => {
            taskbranch_git::prioritize_before(task_ref, before_ref)?;
            Ok(())
        }
        [command, from_ref, relation, to_ref] if command == "link" && relation == "blocks" => {
            taskbranch_git::link_blocks(from_ref, to_ref)?;
            Ok(())
        }
        [command, from_ref, relation, to_ref] if command == "unlink" && relation == "blocks" => {
            taskbranch_git::unlink_blocks(from_ref, to_ref)?;
            Ok(())
        }
        [command, action, task_ref, title] if command == "subtask" && action == "add" => {
            taskbranch_git::add_subtask(task_ref, title)?;
            Ok(())
        }
        [command, action, task_ref, index] if command == "subtask" && action == "check" => {
            taskbranch_git::set_subtask_checked(task_ref, index, true)?;
            Ok(())
        }
        [command, action, task_ref, index] if command == "subtask" && action == "uncheck" => {
            taskbranch_git::set_subtask_checked(task_ref, index, false)?;
            Ok(())
        }
        [command, flag] if command == "validate" && flag == "--fix" => {
            for message in taskbranch_git::validate_fix()? {
                println!("{message}");
            }
            Ok(())
        }
        [command] if command == "close-from-trailers" => {
            for closed in taskbranch_git::close_from_trailers()? {
                println!("closed {closed}");
            }
            Ok(())
        }
        [command, subject, flag] if command == "scaffold" && subject == "repo" && flag == "--dry-run" => {
            for planned in taskbranch_git::scaffold_repo(false)? {
                println!("would write {planned}");
            }
            Ok(())
        }
        [command, subject, flag] if command == "scaffold" && subject == "repo" && flag == "--apply" => {
            for written in taskbranch_git::scaffold_repo(true)? {
                println!("wrote {written}");
            }
            Ok(())
        }
        _ => Err(taskbranch_git::Error::Usage(
            "usage: taskbranch init|sync|dashboard serve|mcp stdio|create <title>|show <ref>|metadata <ref>|list|next|transition <ref> <status>|prioritize <ref> --before <ref>|link <ref> blocks <ref>|unlink <ref> blocks <ref>|subtask add|check|uncheck|validate --fix|close-from-trailers|scaffold repo --dry-run|--apply".to_string(),
        )),
    }
}
