use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use eventcore::{
    execute, CommandError, CommandLogic, CommandStreams, Event, NewEvents, RetryPolicy,
    StreamDeclarations, StreamId,
};
use eventcore_fs::FileEventStore;
use eventcore_types::{BatchSize, EventFilter, EventPage, EventReader, EventStoreError};
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
        Some("list") => list_tickets(),
        Some("claim") => claim_ticket(&arguments[1..]),
        Some("release") => release_ticket(&arguments[1..]),
        Some(command) => {
            eprintln!("tiber.usage command={command}");
            ExitCode::from(2)
        }
    }
}

fn list_tickets() -> ExitCode {
    let store_root = match std::env::current_dir() {
        Ok(directory) => directory.join(".development-system/tiber/store"),
        Err(error) => {
            eprintln!("tiber.list unable_to_resolve_repository error={error}");
            return ExitCode::FAILURE;
        }
    };
    let store = match FileEventStore::open(store_root) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("tiber.list unable_to_open_store error={error}");
            return ExitCode::FAILURE;
        }
    };
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    match runtime.block_on(async {
        let mut page = EventPage::first(BatchSize::new(100));
        let mut titles = std::collections::BTreeMap::new();
        let mut owners = std::collections::BTreeMap::new();
        loop {
            let events = store
                .read_events::<TicketEvent>(EventFilter::all(), page)
                .await?;
            let next_page = page.next_from_results(&events);
            for (event, _) in events {
                match event {
                    TicketEvent::TicketCreatedV1 { ticket_id, title } => {
                        titles.insert(ticket_id.to_string(), title);
                    }
                    TicketEvent::TicketClaimedV1 { ticket_id, owner } => {
                        owners.insert(ticket_id.to_string(), owner);
                    }
                    TicketEvent::TicketClaimReleasedV1 { ticket_id, owner } => {
                        if owners.get(ticket_id.as_str()) == Some(&owner) {
                            owners.remove(ticket_id.as_str());
                        }
                    }
                    TicketEvent::BoardTicketClaimedV1 { .. }
                    | TicketEvent::BoardTicketClaimReleasedV1 { .. } => {}
                }
            }
            let Some(next_page) = next_page else {
                break;
            };
            page = next_page;
        }
        Ok::<_, EventStoreError>((titles, owners))
    }) {
        Ok((titles, owners)) => {
            for (ticket_id, title) in titles {
                let owner = owners
                    .get(&ticket_id)
                    .map(String::as_str)
                    .unwrap_or("unclaimed");
                println!("tiber.ticket id={ticket_id} title={title} owner={owner}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("tiber.list unable_to_replay_tickets error={error}");
            ExitCode::FAILURE
        }
    }
}

fn release_ticket(arguments: &[String]) -> ExitCode {
    let Some(ticket_id) = arguments
        .first()
        .and_then(|value| StreamId::try_new(value.clone()).ok())
    else {
        eprintln!("tiber.release missing_ticket_id");
        return ExitCode::from(2);
    };
    let Some(owner) = arguments
        .windows(2)
        .find_map(|pair| (pair[0] == "--owner").then(|| pair[1].clone()))
    else {
        eprintln!("tiber.release missing_owner");
        return ExitCode::from(2);
    };
    let store_root = match std::env::current_dir() {
        Ok(directory) => directory.join(".development-system/tiber/store"),
        Err(error) => {
            eprintln!("tiber.release unable_to_resolve_repository error={error}");
            return ExitCode::FAILURE;
        }
    };
    let store = match FileEventStore::open(store_root) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("tiber.release unable_to_open_store error={error}");
            return ExitCode::FAILURE;
        }
    };
    let board_id = StreamId::try_new("board-local".to_owned()).expect("valid board stream");
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    match runtime.block_on(execute(
        &store,
        ReleaseTicket {
            ticket_id,
            board_id,
            owner: owner.clone(),
        },
        RetryPolicy::new(),
    )) {
        Ok(_) => {
            println!("tiber.release owner={owner}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("tiber.release failed error={error}");
            ExitCode::FAILURE
        }
    }
}

fn claim_ticket(arguments: &[String]) -> ExitCode {
    let Some(ticket_id) = arguments
        .first()
        .and_then(|value| StreamId::try_new(value.clone()).ok())
    else {
        eprintln!("tiber.claim missing_ticket_id");
        return ExitCode::from(2);
    };
    let Some(owner) = arguments
        .windows(2)
        .find_map(|pair| (pair[0] == "--owner").then(|| pair[1].clone()))
    else {
        eprintln!("tiber.claim missing_owner");
        return ExitCode::from(2);
    };
    let store_root = match std::env::current_dir() {
        Ok(directory) => directory.join(".development-system/tiber/store"),
        Err(error) => {
            eprintln!("tiber.claim unable_to_resolve_repository error={error}");
            return ExitCode::FAILURE;
        }
    };
    let store = match FileEventStore::open(store_root) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("tiber.claim unable_to_open_store error={error}");
            return ExitCode::FAILURE;
        }
    };
    let board_id = StreamId::try_new("board-local".to_owned()).expect("valid board stream");
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    match runtime.block_on(execute(
        &store,
        ClaimTicket {
            ticket_id,
            board_id,
            owner: owner.clone(),
        },
        RetryPolicy::new(),
    )) {
        Ok(_) => {
            println!("tiber.claim owner={owner}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("tiber.claim failed error={error}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum TicketEvent {
    TicketCreatedV1 {
        ticket_id: StreamId,
        title: String,
    },
    TicketClaimedV1 {
        ticket_id: StreamId,
        owner: String,
    },
    TicketClaimReleasedV1 {
        ticket_id: StreamId,
        owner: String,
    },
    BoardTicketClaimedV1 {
        board_id: StreamId,
        ticket_id: StreamId,
        owner: String,
    },
    BoardTicketClaimReleasedV1 {
        board_id: StreamId,
        ticket_id: StreamId,
        owner: String,
    },
}

impl Event for TicketEvent {
    fn stream_id(&self) -> &StreamId {
        match self {
            Self::TicketCreatedV1 { ticket_id, .. }
            | Self::TicketClaimedV1 { ticket_id, .. }
            | Self::TicketClaimReleasedV1 { ticket_id, .. } => ticket_id,
            Self::BoardTicketClaimedV1 { board_id, .. }
            | Self::BoardTicketClaimReleasedV1 { board_id, .. } => board_id,
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

#[derive(Default)]
struct ClaimState {
    exists: bool,
    owner: Option<String>,
}

struct ClaimTicket {
    ticket_id: StreamId,
    board_id: StreamId,
    owner: String,
}

struct ReleaseTicket {
    ticket_id: StreamId,
    board_id: StreamId,
    owner: String,
}

impl CommandStreams for ClaimTicket {
    fn stream_declarations(&self) -> StreamDeclarations {
        StreamDeclarations::try_from_streams(vec![self.ticket_id.clone(), self.board_id.clone()])
            .expect("valid claim streams")
    }
}

impl CommandStreams for ReleaseTicket {
    fn stream_declarations(&self) -> StreamDeclarations {
        StreamDeclarations::try_from_streams(vec![self.ticket_id.clone(), self.board_id.clone()])
            .expect("valid release streams")
    }
}

impl CommandLogic for ClaimTicket {
    type Event = TicketEvent;
    type State = ClaimState;
    fn apply(&self, mut state: ClaimState, event: &TicketEvent) -> ClaimState {
        match event {
            TicketEvent::TicketCreatedV1 { ticket_id, .. } if ticket_id == &self.ticket_id => {
                state.exists = true
            }
            TicketEvent::TicketClaimedV1 { ticket_id, owner } if ticket_id == &self.ticket_id => {
                state.owner = Some(owner.clone())
            }
            TicketEvent::TicketClaimReleasedV1 { ticket_id, owner }
                if ticket_id == &self.ticket_id && state.owner.as_deref() == Some(owner) =>
            {
                state.owner = None
            }
            _ => {}
        }
        state
    }
    fn handle(&self, state: ClaimState) -> Result<NewEvents<TicketEvent>, CommandError> {
        if !state.exists {
            return Err(CommandError::from("ticket does not exist"));
        }
        if state.owner.is_some() {
            return Err(CommandError::from("ticket already claimed"));
        }
        Ok(vec![
            TicketEvent::TicketClaimedV1 {
                ticket_id: self.ticket_id.clone(),
                owner: self.owner.clone(),
            },
            TicketEvent::BoardTicketClaimedV1 {
                board_id: self.board_id.clone(),
                ticket_id: self.ticket_id.clone(),
                owner: self.owner.clone(),
            },
        ]
        .into())
    }
}

impl CommandLogic for ReleaseTicket {
    type Event = TicketEvent;
    type State = ClaimState;
    fn apply(&self, mut state: ClaimState, event: &TicketEvent) -> ClaimState {
        match event {
            TicketEvent::TicketCreatedV1 { ticket_id, .. } if ticket_id == &self.ticket_id => {
                state.exists = true
            }
            TicketEvent::TicketClaimedV1 { ticket_id, owner } if ticket_id == &self.ticket_id => {
                state.owner = Some(owner.clone())
            }
            TicketEvent::TicketClaimReleasedV1 { ticket_id, owner }
                if ticket_id == &self.ticket_id && state.owner.as_deref() == Some(owner) =>
            {
                state.owner = None
            }
            _ => {}
        }
        state
    }
    fn handle(&self, state: ClaimState) -> Result<NewEvents<TicketEvent>, CommandError> {
        if !state.exists {
            return Err(CommandError::from("ticket does not exist"));
        }
        if state.owner.as_deref() != Some(self.owner.as_str()) {
            return Err(CommandError::from("ticket is not claimed by owner"));
        }
        Ok(vec![
            TicketEvent::TicketClaimReleasedV1 {
                ticket_id: self.ticket_id.clone(),
                owner: self.owner.clone(),
            },
            TicketEvent::BoardTicketClaimReleasedV1 {
                board_id: self.board_id.clone(),
                ticket_id: self.ticket_id.clone(),
                owner: self.owner.clone(),
            },
        ]
        .into())
    }
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
