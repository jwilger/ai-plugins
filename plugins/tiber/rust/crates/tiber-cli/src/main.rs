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
            tiber_git::add_subtask(task_ref, title)?;
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
            "usage: tiber init|sync|dashboard serve|mcp stdio|create <title>|show <ref>|metadata <ref>|list|next|transition <ref> <status>|prioritize <ref> --before <ref>|link <ref> blocks <ref>|unlink <ref> blocks <ref>|subtask add|check|uncheck|validate --fix|close-from-trailers|scaffold repo --dry-run|--apply".to_string(),
        )),
    }
}
