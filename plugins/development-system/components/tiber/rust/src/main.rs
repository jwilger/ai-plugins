use std::process::ExitCode;

use eventcore_fs::FileEventStore;

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        None | Some("help") | Some("--help") | Some("-h") => {
            println!("Repository-local task board");
            println!("\nEventcore-backed Tiber is being initialized by development-system.");
            println!("\nusage: tiber <command> [options]");
            println!("\ncommands:\n  init\n  status\n  migrate-beads-to-tiber");
            ExitCode::SUCCESS
        }
        Some("init") => initialize_store(),
        Some(command) => {
            eprintln!("tiber.usage command={command}");
            ExitCode::from(2)
        }
    }
}

fn initialize_store() -> ExitCode {
    let store_root = match std::env::current_dir() {
        Ok(directory) => directory.join(".development-system/tiber/store"),
        Err(error) => {
            eprintln!("tiber.init unable_to_resolve_repository error={error}");
            return ExitCode::FAILURE;
        }
    };

    match FileEventStore::open(&store_root) {
        Ok(_) => {
            println!("tiber.init store={}", store_root.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("tiber.init unable_to_initialize_store error={error}");
            ExitCode::FAILURE
        }
    }
}
