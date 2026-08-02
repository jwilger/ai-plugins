use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use eventcore::{
    execute, CommandError, CommandLogic, CommandStreams, Event, NewEvents, RetryPolicy,
    StreamDeclarations, StreamId,
};
use eventcore_fs::FileEventStore;
use serde::{Deserialize, Serialize};

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match arguments.first().map(String::as_str) {
        None | Some("help") | Some("--help") | Some("-h") => {
            println!("Repository-local task board");
            println!("\nEventcore-backed Tiber is being initialized by development-system.");
            println!("\nusage: tiber <command> [options]");
            println!(
                "\ncommands:\n  init\n  create --title <title>\n  status\n  migrate-beads-to-tiber"
            );
            ExitCode::SUCCESS
        }
        Some("init") => initialize_store(),
        Some("create") => create_ticket(&arguments[1..]),
        Some(command) => {
            eprintln!("tiber.usage command={command}");
            ExitCode::from(2)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum TicketEvent {
    TicketCreatedV1 { ticket_id: StreamId, title: String },
}

impl Event for TicketEvent {
    fn stream_id(&self) -> &StreamId {
        match self {
            Self::TicketCreatedV1 { ticket_id, .. } => ticket_id,
        }
    }
    fn event_type_name() -> &'static str {
        "TiberTicketEvent"
    }
}

struct CreateTicket {
    ticket_id: StreamId,
    title: String,
}

impl CommandStreams for CreateTicket {
    fn stream_declarations(&self) -> StreamDeclarations {
        StreamDeclarations::try_from_streams(vec![self.ticket_id.clone()])
            .expect("valid ticket stream")
    }
}

impl CommandLogic for CreateTicket {
    type Event = TicketEvent;
    type State = bool;
    fn apply(&self, _state: bool, _event: &TicketEvent) -> bool {
        true
    }
    fn handle(&self, exists: bool) -> Result<NewEvents<TicketEvent>, CommandError> {
        if exists {
            return Err(CommandError::from("ticket already exists"));
        }
        Ok(vec![TicketEvent::TicketCreatedV1 {
            ticket_id: self.ticket_id.clone(),
            title: self.title.clone(),
        }]
        .into())
    }
}

fn create_ticket(arguments: &[String]) -> ExitCode {
    let Some(title) = arguments
        .windows(2)
        .find_map(|pair| (pair[0] == "--title").then(|| pair[1].clone()))
    else {
        eprintln!("tiber.create missing_title");
        return ExitCode::from(2);
    };
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let ticket_id =
        StreamId::try_new(format!("ticket-{timestamp}")).expect("valid generated ticket stream");
    let store_root = std::env::current_dir()
        .expect("repository directory")
        .join(".development-system/tiber/store");
    let store = match FileEventStore::open(store_root) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("tiber.create unable_to_initialize_store error={error}");
            return ExitCode::FAILURE;
        }
    };
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    match runtime.block_on(execute(
        &store,
        CreateTicket {
            ticket_id,
            title: title.clone(),
        },
        RetryPolicy::new(),
    )) {
        Ok(_) => {
            println!("tiber.create title={title}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("tiber.create failed error={error}");
            ExitCode::FAILURE
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
