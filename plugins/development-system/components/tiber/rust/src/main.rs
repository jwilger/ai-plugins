use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use eventcore::{
    collect_events, execute, CommandError, CommandLogic, CommandStreams, NewEvents, RetryPolicy,
    StreamDeclarations, StreamId,
};
use eventcore_fs::FileEventStore;
use eventcore_types::{BatchSize, EventFilter, EventPage, EventReader, EventStore, StreamPosition};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
mod application;
mod domain;
mod workflow;

use application::{
    AddTicketDependency, ClaimTicket, CompleteTicket, PrioritizeTicket, RemoveTicketDependency,
};
use domain::{apply_ticket_state, TicketEvent, TicketState};
use workflow::{
    RecordDiscoveryEvidence, RecordWorkflowTestEvent, WorkflowEvent, WorkflowProjection,
    WORKFLOW_STREAM,
};

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match arguments.first().map(String::as_str) {
        None | Some("help") | Some("--help") | Some("-h") => {
            println!("Repository-local task board");
            println!("\nEventcore-backed Tiber is being initialized by development-system.");
            println!("\nusage: tiber <command> [options]");
            println!(
                "\ncommands:\n  init\n  create --title <title>\n  list\n  claim <ticket-id> --owner <owner>\n  release <ticket-id> --owner <owner>\n  prioritize <ticket-id> --priority <0..4>\n  complete <ticket-id> --owner <owner>\n  next\n  depend <ticket-id> --on <dependency-id>\n  undepend <ticket-id> --on <dependency-id>"
            );
            ExitCode::SUCCESS
        }
        Some("init") => initialize_store(),
        Some("create") => create_ticket(&arguments[1..]),
        Some("list") => list_tickets(),
        Some("claim") => claim_ticket(&arguments[1..]),
        Some("release") => release_ticket(&arguments[1..]),
        Some("prioritize") => prioritize_ticket(&arguments[1..]),
        Some("complete") => complete_ticket(&arguments[1..]),
        Some("next") => next_ticket(),
        Some("depend") => depend_ticket(&arguments[1..]),
        Some("undepend") => undepend_ticket(&arguments[1..]),
        Some("workflow") => workflow_command(&arguments[1..]),
        Some(command) => {
            eprintln!("tiber.usage command={command}");
            ExitCode::from(2)
        }
    }
}

fn workflow_command(arguments: &[String]) -> ExitCode {
    match arguments.first().map(String::as_str) {
        Some("status") => workflow_status(arguments),
        Some("execute") => workflow_execute(&arguments[1..]),
        Some("test-append") => workflow_test_append(&arguments[1..]),
        _ => {
            eprintln!("tiber.workflow command_unknown");
            ExitCode::from(2)
        }
    }
}

fn workflow_test_append(arguments: &[String]) -> ExitCode {
    if std::env::var_os("TIBER_WORKFLOW_TEST_MODE").is_none() {
        eprintln!("tiber.workflow command_unknown");
        return ExitCode::from(2);
    }
    let command = arguments
        .windows(2)
        .find_map(|pair| (pair[0] == "--command").then(|| pair[1].clone()));
    let event = arguments
        .windows(2)
        .find_map(|pair| (pair[0] == "--event").then(|| pair[1].clone()));
    let observation = arguments
        .windows(2)
        .find_map(|pair| (pair[0] == "--observation").then(|| pair[1].clone()));
    let (Some(command), Some(event), Some(observation)) = (command, event, observation) else {
        eprintln!("tiber.workflow test_fixture_invalid");
        return ExitCode::from(2);
    };
    let root = match std::env::current_dir() {
        Ok(directory) => directory.join(".development-system/tiber/store"),
        Err(error) => {
            eprintln!("tiber.workflow unable_to_resolve_repository error={error}");
            return ExitCode::FAILURE;
        }
    };
    if !root.is_dir() {
        eprintln!("tiber.workflow not_initialized");
        return ExitCode::FAILURE;
    }
    let store = match FileEventStore::open(root) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("tiber.workflow unable_to_open_store error={error}");
            return ExitCode::FAILURE;
        }
    };
    let workflow_id = StreamId::try_new(WORKFLOW_STREAM.to_owned()).expect("workflow stream");
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    match runtime.block_on(execute(
        &store,
        RecordWorkflowTestEvent {
            workflow_id,
            command,
            event,
            observation,
        },
        RetryPolicy::new(),
    )) {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tiber.workflow test_fixture_failed error={error}");
            ExitCode::FAILURE
        }
    }
}

fn workflow_status(arguments: &[String]) -> ExitCode {
    if arguments != ["status"] {
        eprintln!("tiber.workflow usage=workflow_status");
        return ExitCode::from(2);
    }
    let store_root = match std::env::current_dir() {
        Ok(directory) => directory.join(".development-system/tiber/store"),
        Err(error) => {
            eprintln!("tiber.workflow unable_to_resolve_repository error={error}");
            return ExitCode::FAILURE;
        }
    };
    if !store_root.is_dir() {
        eprintln!("tiber.workflow not_initialized");
        return ExitCode::FAILURE;
    }
    if let Err(error) = FileEventStore::open(&store_root) {
        eprintln!("tiber.workflow unable_to_open_store error={error}");
        return ExitCode::FAILURE;
    }
    let decision = match replay_workflow(&store_root) {
        Ok(projection) => projection.decision(),
        Err(error) => {
            eprintln!("tiber.workflow replay_failed reason={error}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "tiber.workflow frontier={} next={} evidence={} events={}",
        decision.frontier, decision.next, decision.evidence, decision.events
    );
    ExitCode::SUCCESS
}

fn replay_workflow(store_root: &std::path::Path) -> Result<WorkflowProjection, String> {
    let store = FileEventStore::open(store_root).map_err(|error| error.to_string())?;
    let workflow_id = StreamId::try_new(WORKFLOW_STREAM.to_owned()).expect("workflow stream");
    let runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;

    runtime.block_on(async {
        let stream = store
            .read_stream::<WorkflowEvent>(workflow_id)
            .await
            .map_err(|error| error.to_string())?;
        let events = collect_events(stream)
            .await
            .map_err(|error| error.to_string())?;
        events
            .into_iter()
            .try_fold(WorkflowProjection::initial(), |state, event| {
                state.try_apply(&event)
            })
            .map_err(|error| error.to_string())
    })
}

fn workflow_execute(arguments: &[String]) -> ExitCode {
    if arguments.first().map(String::as_str) == Some("FormProductHypothesis") {
        eprintln!("tiber.workflow command_not_eligible");
        return ExitCode::from(2);
    }
    if arguments.first().map(String::as_str) != Some("RecordDiscoveryEvidence") {
        eprintln!("tiber.workflow command_unknown");
        return ExitCode::from(2);
    }
    let Some(arguments) = parse_discovery_evidence_arguments(arguments) else {
        eprintln!("tiber.workflow command_unknown");
        return ExitCode::from(2);
    };
    if arguments.event != "DiscoveryEvidenceRecorded" {
        eprintln!("tiber.workflow event_not_emitted_by_command");
        return ExitCode::from(2);
    }
    let Some(expected) = arguments.expected_frontier.parse::<u8>().ok() else {
        eprintln!("tiber.workflow stale_frontier expected=invalid actual=0");
        return ExitCode::from(2);
    };
    let observation = arguments.observation;
    if observation.trim().is_empty() || observation.len() > 1024 {
        eprintln!("tiber.workflow invalid_observation");
        return ExitCode::from(2);
    }
    let root = match std::env::current_dir() {
        Ok(d) => d.join(".development-system/tiber/store"),
        Err(e) => {
            eprintln!("tiber.workflow unable_to_resolve_repository error={e}");
            return ExitCode::FAILURE;
        }
    };
    if !root.is_dir() {
        eprintln!("tiber.workflow not_initialized");
        return ExitCode::FAILURE;
    }
    let actual = match replay_workflow(&root) {
        Ok(value) => value.decision().frontier,
        Err(error) => {
            eprintln!("tiber.workflow replay_failed reason={error}");
            return ExitCode::FAILURE;
        }
    };
    if expected != actual {
        eprintln!("tiber.workflow stale_frontier expected={expected} actual={actual}");
        return ExitCode::from(2);
    }
    let store = match FileEventStore::open(root) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("tiber.workflow unable_to_open_store error={e}");
            return ExitCode::FAILURE;
        }
    };
    let id = StreamId::try_new(WORKFLOW_STREAM.to_owned()).expect("workflow stream");
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    match runtime.block_on(execute(
        &store,
        RecordDiscoveryEvidence {
            workflow_id: id,
            observation,
        },
        RetryPolicy::new(),
    )) {
        Ok(_) => {
            println!("tiber.workflow recorded=DiscoveryEvidenceRecorded frontier=1 next=FormProductHypothesis evidence=derived events=ProductHypothesisFormed");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("tiber.workflow failed error={e}");
            ExitCode::FAILURE
        }
    }
}

struct DiscoveryEvidenceArguments {
    event: String,
    expected_frontier: String,
    observation: String,
}

fn parse_discovery_evidence_arguments(arguments: &[String]) -> Option<DiscoveryEvidenceArguments> {
    let mut event = None;
    let mut expected_frontier = None;
    let mut observation = None;
    let mut index = 1;

    while index < arguments.len() {
        let flag = arguments.get(index)?;
        let value = arguments.get(index + 1)?;
        if value.starts_with("--") {
            return None;
        }
        match flag.as_str() {
            "--event" if event.is_none() => event = Some(value.clone()),
            "--expected-frontier" if expected_frontier.is_none() => {
                expected_frontier = Some(value.clone())
            }
            "--observation" if observation.is_none() => observation = Some(value.clone()),
            _ => return None,
        }
        index += 2;
    }

    Some(DiscoveryEvidenceArguments {
        event: event?,
        expected_frontier: expected_frontier?,
        observation: observation?,
    })
}

struct BoardTicketRow {
    priority: u8,
    creation_position: u64,
    ticket_id: String,
    title: String,
    state: TicketState,
    blocked_by: Vec<String>,
}

const BOARD_PROJECTION_REVISION: u8 = 1;
const BOARD_PROJECTION_FILE: &str = "board-projection-v1.json";
const BOARD_PROJECTION_BATCH_SIZE: usize = 100;

#[derive(Default, Serialize, Deserialize)]
struct BoardProjection {
    tickets: BTreeMap<String, ProjectedTicket>,
    priorities: BTreeMap<String, u8>,
    dependencies: BTreeMap<String, BTreeSet<String>>,
    completed: BTreeSet<String>,
    next_creation_position: u64,
}

#[derive(Serialize, Deserialize)]
struct ProjectedTicket {
    title: String,
    owner: Option<String>,
    completed: bool,
    creation_position: u64,
}

#[derive(Serialize, Deserialize)]
struct BoardProjectionSnapshot {
    revision: u8,
    cursor: Option<String>,
    projection: BoardProjection,
}

impl BoardProjection {
    fn apply(&mut self, event: TicketEvent) {
        match event {
            TicketEvent::TicketCreatedV1 { ticket_id, title } => {
                let ticket_id = ticket_id.to_string();
                self.tickets.entry(ticket_id).or_insert_with(|| {
                    let creation_position = self.next_creation_position;
                    self.next_creation_position += 1;
                    ProjectedTicket {
                        title,
                        owner: None,
                        completed: false,
                        creation_position,
                    }
                });
            }
            TicketEvent::TicketClaimedV1 { ticket_id, owner } => {
                if let Some(ticket) = self.tickets.get_mut(ticket_id.as_str()) {
                    if !ticket.completed {
                        ticket.owner = Some(owner);
                    }
                }
            }
            TicketEvent::TicketClaimReleasedV1 { ticket_id, owner } => {
                if let Some(ticket) = self.tickets.get_mut(ticket_id.as_str()) {
                    if ticket.owner.as_deref() == Some(owner.as_str()) {
                        ticket.owner = None;
                    }
                }
            }
            TicketEvent::TicketCompletedV1 { ticket_id, owner } => {
                if let Some(ticket) = self.tickets.get_mut(ticket_id.as_str()) {
                    if ticket.owner.as_deref() == Some(owner.as_str()) {
                        ticket.owner = None;
                        ticket.completed = true;
                        self.completed.insert(ticket_id.to_string());
                    }
                }
            }
            TicketEvent::BoardTicketPrioritySetV1 {
                ticket_id,
                priority,
                ..
            } => {
                self.priorities.insert(ticket_id.to_string(), priority);
            }
            TicketEvent::BoardTicketDependencyAddedV1 {
                ticket_id,
                dependency_id,
                ..
            } => {
                self.dependencies
                    .entry(ticket_id.to_string())
                    .or_default()
                    .insert(dependency_id.to_string());
            }
            TicketEvent::BoardTicketDependencyRemovedV1 {
                ticket_id,
                dependency_id,
                ..
            } => {
                if let Some(dependencies) = self.dependencies.get_mut(ticket_id.as_str()) {
                    dependencies.remove(dependency_id.as_str());
                }
            }
            TicketEvent::BoardTicketCompletedV1 { ticket_id, .. } => {
                self.completed.insert(ticket_id.to_string());
            }
            TicketEvent::BoardTicketClaimedV1 { .. }
            | TicketEvent::BoardTicketClaimReleasedV1 { .. } => {}
        }
    }

    fn rows(self) -> Vec<BoardTicketRow> {
        let mut rows = self
            .tickets
            .into_iter()
            .map(|(ticket_id, ticket)| BoardTicketRow {
                priority: self.priorities.get(&ticket_id).copied().unwrap_or(2),
                creation_position: ticket.creation_position,
                blocked_by: self
                    .dependencies
                    .get(&ticket_id)
                    .into_iter()
                    .flatten()
                    .filter(|dependency_id| !self.completed.contains(*dependency_id))
                    .cloned()
                    .collect(),
                ticket_id,
                title: ticket.title,
                state: TicketState {
                    exists: true,
                    owner: ticket.owner,
                    completed: ticket.completed,
                },
            })
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then(left.creation_position.cmp(&right.creation_position))
                .then(left.ticket_id.cmp(&right.ticket_id))
        });
        rows
    }
}

fn board_projection_path(store_root: &Path) -> PathBuf {
    store_root.join("checkpoints").join(BOARD_PROJECTION_FILE)
}

fn load_board_projection(store_root: &Path) -> (BoardProjection, Option<StreamPosition>) {
    let path = board_projection_path(store_root);
    let Ok(contents) = fs::read_to_string(path) else {
        return (BoardProjection::default(), None);
    };
    let Ok(snapshot) = serde_json::from_str::<BoardProjectionSnapshot>(&contents) else {
        return (BoardProjection::default(), None);
    };
    if snapshot.revision != BOARD_PROJECTION_REVISION {
        return (BoardProjection::default(), None);
    }
    let cursor = match snapshot.cursor {
        None => None,
        Some(cursor) => match Uuid::parse_str(&cursor) {
            Ok(cursor) => Some(StreamPosition::new(cursor)),
            Err(_) => return (BoardProjection::default(), None),
        },
    };
    (snapshot.projection, cursor)
}

fn save_board_projection(
    store_root: &Path,
    projection: BoardProjection,
    cursor: Option<StreamPosition>,
) -> Result<(), String> {
    let path = board_projection_path(store_root);
    let parent = path
        .parent()
        .expect("board projection has parent directory");
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let snapshot = BoardProjectionSnapshot {
        revision: BOARD_PROJECTION_REVISION,
        cursor: cursor.map(|cursor| cursor.into_inner().to_string()),
        projection,
    };
    let contents = serde_json::to_vec(&snapshot).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    let mut file = fs::File::create(&temporary).map_err(|error| error.to_string())?;
    file.write_all(&contents)
        .map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    fs::rename(temporary, path).map_err(|error| error.to_string())
}

fn replay_board_rows() -> Result<Vec<BoardTicketRow>, String> {
    let store_root = match std::env::current_dir() {
        Ok(directory) => directory.join(".development-system/tiber/store"),
        Err(error) => return Err(error.to_string()),
    };
    let store = match FileEventStore::open(&store_root) {
        Ok(store) => store,
        Err(error) => return Err(error.to_string()),
    };
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime
        .block_on(async {
            let (mut projection, cursor) = load_board_projection(&store_root);
            let mut page = cursor.map_or_else(
                || EventPage::first(BatchSize::new(BOARD_PROJECTION_BATCH_SIZE)),
                |cursor| EventPage::after(cursor, BatchSize::new(BOARD_PROJECTION_BATCH_SIZE)),
            );
            let mut last_position = cursor;
            loop {
                let events = store
                    .read_events::<TicketEvent>(EventFilter::all(), page)
                    .await
                    .map_err(|error| error.to_string())?;
                let next_page = page.next_from_results(&events);
                for (event, position) in events {
                    projection.apply(event);
                    last_position = Some(position);
                }
                let Some(next_page) = next_page else {
                    break;
                };
                page = next_page;
            }
            save_board_projection(&store_root, projection, last_position)?;
            Ok::<_, String>(())
        })
        .map(|()| {
            let (projection, _) = load_board_projection(&store_root);
            projection.rows()
        })
}

fn print_ticket_row(row: &BoardTicketRow) {
    let owner = row.state.owner.as_deref().unwrap_or("unclaimed");
    println!(
        "tiber.ticket id={} title={} owner={owner} priority={} completed={} blocked_by={}",
        row.ticket_id,
        row.title,
        row.priority,
        row.state.completed,
        if row.blocked_by.is_empty() {
            "none".to_owned()
        } else {
            row.blocked_by.join(",")
        }
    );
}

fn list_tickets() -> ExitCode {
    match replay_board_rows() {
        Ok(rows) => {
            for row in &rows {
                print_ticket_row(row);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("tiber.list unable_to_replay_tickets error={error}");
            ExitCode::FAILURE
        }
    }
}

fn next_ticket() -> ExitCode {
    match replay_board_rows() {
        Ok(rows) => {
            if let Some(row) = rows.iter().find(|row| {
                row.state.owner.is_none() && !row.state.completed && row.blocked_by.is_empty()
            }) {
                print_ticket_row(row);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("tiber.next unable_to_replay_tickets error={error}");
            ExitCode::FAILURE
        }
    }
}

fn prioritize_ticket(arguments: &[String]) -> ExitCode {
    let Some(ticket_id) = arguments
        .first()
        .and_then(|value| StreamId::try_new(value.clone()).ok())
    else {
        eprintln!("tiber.prioritize missing_ticket_id");
        return ExitCode::from(2);
    };
    let Some(priority) = arguments
        .windows(2)
        .find_map(|pair| {
            (pair[0] == "--priority")
                .then(|| pair[1].parse::<u8>().ok())
                .flatten()
        })
        .filter(|priority| *priority <= 4)
    else {
        eprintln!("tiber.prioritize invalid_priority");
        return ExitCode::from(2);
    };
    let store_root = match std::env::current_dir() {
        Ok(directory) => directory.join(".development-system/tiber/store"),
        Err(error) => {
            eprintln!("tiber.prioritize unable_to_resolve_repository error={error}");
            return ExitCode::FAILURE;
        }
    };
    let store = match FileEventStore::open(store_root) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("tiber.prioritize unable_to_open_store error={error}");
            return ExitCode::FAILURE;
        }
    };
    let board_id = StreamId::try_new("board-local".to_owned()).expect("valid board stream");
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    match runtime.block_on(execute(
        &store,
        PrioritizeTicket {
            ticket_id,
            board_id,
            priority,
        },
        RetryPolicy::new(),
    )) {
        Ok(_) => {
            println!("tiber.prioritize priority={priority}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("tiber.prioritize failed error={error}");
            ExitCode::FAILURE
        }
    }
}

fn complete_ticket(arguments: &[String]) -> ExitCode {
    let Some(ticket_id) = arguments
        .first()
        .and_then(|value| StreamId::try_new(value.clone()).ok())
    else {
        eprintln!("tiber.complete missing_ticket_id");
        return ExitCode::from(2);
    };
    let Some(owner) = arguments
        .windows(2)
        .find_map(|pair| (pair[0] == "--owner").then(|| pair[1].clone()))
    else {
        eprintln!("tiber.complete missing_owner");
        return ExitCode::from(2);
    };
    let store_root = match std::env::current_dir() {
        Ok(directory) => directory.join(".development-system/tiber/store"),
        Err(error) => {
            eprintln!("tiber.complete unable_to_resolve_repository error={error}");
            return ExitCode::FAILURE;
        }
    };
    let store = match FileEventStore::open(store_root) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("tiber.complete unable_to_open_store error={error}");
            return ExitCode::FAILURE;
        }
    };
    let board_id = StreamId::try_new("board-local".to_owned()).expect("valid board stream");
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    match runtime.block_on(execute(
        &store,
        CompleteTicket {
            ticket_id,
            board_id,
            owner: owner.clone(),
        },
        RetryPolicy::new(),
    )) {
        Ok(_) => {
            println!("tiber.complete owner={owner}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("tiber.complete failed error={error}");
            ExitCode::FAILURE
        }
    }
}

fn depend_ticket(arguments: &[String]) -> ExitCode {
    let Some(ticket_id) = arguments
        .first()
        .and_then(|value| StreamId::try_new(value.clone()).ok())
    else {
        eprintln!("tiber.depend missing_ticket_id");
        return ExitCode::from(2);
    };
    let Some(dependency_id) = arguments
        .windows(2)
        .find_map(|pair| (pair[0] == "--on").then(|| StreamId::try_new(pair[1].clone()).ok()))
        .flatten()
    else {
        eprintln!("tiber.depend missing_dependency_id");
        return ExitCode::from(2);
    };
    let store_root = match std::env::current_dir() {
        Ok(directory) => directory.join(".development-system/tiber/store"),
        Err(error) => {
            eprintln!("tiber.depend unable_to_resolve_repository error={error}");
            return ExitCode::FAILURE;
        }
    };
    let store = match FileEventStore::open(store_root) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("tiber.depend unable_to_open_store error={error}");
            return ExitCode::FAILURE;
        }
    };
    let board_id = StreamId::try_new("board-local".to_owned()).expect("valid board stream");
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    match runtime.block_on(execute(
        &store,
        AddTicketDependency {
            ticket_id,
            dependency_id,
            board_id,
        },
        RetryPolicy::new(),
    )) {
        Ok(_) => {
            println!("tiber.depend added=true");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("tiber.depend failed error={error}");
            ExitCode::FAILURE
        }
    }
}

fn undepend_ticket(arguments: &[String]) -> ExitCode {
    let Some(ticket_id) = arguments
        .first()
        .and_then(|value| StreamId::try_new(value.clone()).ok())
    else {
        eprintln!("tiber.undepend missing_ticket_id");
        return ExitCode::from(2);
    };
    let Some(dependency_id) = arguments
        .windows(2)
        .find_map(|pair| (pair[0] == "--on").then(|| StreamId::try_new(pair[1].clone()).ok()))
        .flatten()
    else {
        eprintln!("tiber.undepend missing_dependency_id");
        return ExitCode::from(2);
    };
    let store_root = match std::env::current_dir() {
        Ok(directory) => directory.join(".development-system/tiber/store"),
        Err(error) => {
            eprintln!("tiber.undepend unable_to_resolve_repository error={error}");
            return ExitCode::FAILURE;
        }
    };
    let store = match FileEventStore::open(store_root) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("tiber.undepend unable_to_open_store error={error}");
            return ExitCode::FAILURE;
        }
    };
    let board_id = StreamId::try_new("board-local".to_owned()).expect("valid board stream");
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    match runtime.block_on(execute(
        &store,
        RemoveTicketDependency {
            ticket_id,
            dependency_id,
            board_id,
        },
        RetryPolicy::new(),
    )) {
        Ok(_) => {
            println!("tiber.undepend removed=true");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("tiber.undepend failed error={error}");
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

struct CreateTicket {
    ticket_id: StreamId,
    title: String,
}

struct ReleaseTicket {
    ticket_id: StreamId,
    board_id: StreamId,
    owner: String,
}

impl CommandStreams for ReleaseTicket {
    fn stream_declarations(&self) -> StreamDeclarations {
        StreamDeclarations::try_from_streams(vec![self.ticket_id.clone(), self.board_id.clone()])
            .expect("valid release streams")
    }
}

impl CommandLogic for ReleaseTicket {
    type Event = TicketEvent;
    type State = TicketState;
    fn apply(&self, state: TicketState, event: &TicketEvent) -> TicketState {
        apply_ticket_state(state, &self.ticket_id, event)
    }
    fn handle(&self, state: TicketState) -> Result<NewEvents<TicketEvent>, CommandError> {
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
