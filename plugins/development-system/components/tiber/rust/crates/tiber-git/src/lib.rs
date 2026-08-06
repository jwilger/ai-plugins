use crate::git_event_store::{GitEventStore, SynchronizeOutcome};
use eventcore_types::{
    BatchSize, Event, EventFilter, EventPage, EventReader, EventStore, EventStoreError, StreamId,
    StreamVersion, StreamWrites,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::fs::{OpenOptions, TryLockError};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tiber_core::task::{ChecklistItem, Claim, Note, Subtask, Task, ValidationRepair};
use tiber_core::{
    events::TiberEvent, BoardSnapshot, DependencyGraph, OrderReconciliation, TaskDependencies,
    TaskSnapshot, TaskTitle,
};

pub mod git_event_store;

const STATUS_DIRS: &[&str] = &["backlog", "in-progress", "done", "abandoned"];
const OPEN_STATUS_DIRS: &[&str] = &["backlog", "in-progress"];
const TASK_ID_ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789";
const DEFAULT_LOCK_RETRY_TIMEOUT: Duration = Duration::from_secs(3);
const DEFAULT_LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(50);
const CONFIG_FILE: &str = ".tiber.toml";
const MAX_SYNC_ATTEMPTS: usize = 8;
const CI_RECOVERY_LEASE_SECONDS: u64 = 60 * 60;
const CI_RECOVERY_TEXT_MAX_BYTES: usize = 16 * 1024;
const REPOSITORY_STREAM: &str = "tiber:repository";
const BOARD_STREAM: &str = "tiber:board";
const CI_RECOVERY_STREAM: &str = "tiber:ci-recovery";

#[derive(Clone, Copy)]
enum TaskMutation {
    Create,
    Transition,
    Prioritize,
    Dependencies,
    AddSubtask,
    CheckSubtask,
    UpdateDetails,
    UpdatePullRequest,
    UpdateDetailsAndPullRequest,
    AddAcceptance,
    CheckAcceptance,
    RemoveAcceptance,
    AddNote,
    ValidateRepair,
    CloseFromTrailer,
}

#[derive(Clone, Default)]
struct TiberProjection {
    initialized: bool,
    tasks: std::collections::BTreeMap<String, Task>,
    order: Vec<String>,
    ci_recovery: Option<CiRecoveryState>,
    versions: std::collections::HashMap<StreamId, StreamVersion>,
}

fn stream_id(value: impl Into<String>) -> Result<StreamId, Error> {
    StreamId::try_new(value.into())
        .map_err(|error| Error::Parse(format!("event_stream_invalid source={error}")))
}

fn run_async<T>(future: impl std::future::Future<Output = T> + Send + 'static) -> T
where
    T: Send + 'static,
{
    let run = move || {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("Tiber's bundled Tokio runtime must initialize")
            .block_on(future)
    };
    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::spawn(run)
            .join()
            .expect("Tiber's event-store worker must complete")
    } else {
        run()
    }
}

fn event_store_error(_error: impl std::fmt::Display) -> Error {
    Error::Parse("event_store_failed source_redacted=true".to_string())
}

fn load_tiber_projection(root: &Path) -> Result<TiberProjection, Error> {
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    run_async(async move {
        let mut projection = TiberProjection::default();
        let mut page = EventPage::first(BatchSize::new(1024));
        loop {
            let events = store
                .read_events::<TiberEvent>(EventFilter::all(), page)
                .await
                .map_err(event_store_error)?;
            if events.is_empty() {
                break;
            }
            for (event, _) in &events {
                let version = projection
                    .versions
                    .entry(event.stream_id().clone())
                    .or_insert(StreamVersion::new(0));
                *version = version.increment();
                apply_tiber_event(&mut projection, event)?;
            }
            page = page.next(events.last().expect("nonempty page").1);
            if events.len() < 1024 {
                break;
            }
        }
        Ok(projection)
    })
}

fn task_mut<'a>(projection: &'a mut TiberProjection, stem: &str) -> Result<&'a mut Task, Error> {
    projection
        .tasks
        .get_mut(stem)
        .ok_or_else(|| Error::Parse(format!("task_event_without_creation ref={stem}")))
}

fn apply_tiber_event(projection: &mut TiberProjection, event: &TiberEvent) -> Result<(), Error> {
    match event {
        TiberEvent::RepositoryInitialized { .. } => projection.initialized = true,
        TiberEvent::TaskCreated { task, .. } => {
            projection.tasks.insert(task.stem.clone(), (**task).clone());
        }
        TiberEvent::TaskTransitioned {
            stem,
            status,
            claim,
            ..
        } => {
            let task = task_mut(projection, stem)?;
            task.status.clone_from(status);
            task.claim.clone_from(claim);
        }
        TiberEvent::TaskPriorityChanged { order, .. }
        | TiberEvent::BoardReordered { order, .. } => projection.order.clone_from(order),
        TiberEvent::TaskLinksChanged {
            stem,
            blocks,
            blocked_by,
            ..
        } => {
            let task = task_mut(projection, stem)?;
            task.blocks.clone_from(blocks);
            task.blocked_by.clone_from(blocked_by);
        }
        TiberEvent::TaskSubtaskAdded { stem, subtask, .. } => {
            task_mut(projection, stem)?.subtasks.push(subtask.clone())
        }
        TiberEvent::TaskSubtaskChecked {
            stem,
            subtask_id,
            checked,
            ..
        } => {
            let item = task_mut(projection, stem)?
                .subtasks
                .iter_mut()
                .find(|item| &item.id == subtask_id)
                .ok_or_else(|| Error::Parse(format!("subtask_ref_missing ref={subtask_id}")))?;
            item.checked = *checked;
        }
        TiberEvent::TaskDetailsUpdated {
            stem,
            title,
            tags,
            summary,
            context,
            ..
        } => {
            let task = task_mut(projection, stem)?;
            task.title.clone_from(title);
            task.tags.clone_from(tags);
            task.summary.clone_from(summary);
            task.context.clone_from(context);
        }
        TiberEvent::TaskClaimChanged { stem, claim, .. } => {
            task_mut(projection, stem)?.claim.clone_from(claim)
        }
        TiberEvent::TaskPullRequestChanged {
            stem, url, status, ..
        } => {
            let task = task_mut(projection, stem)?;
            task.pr_mr_url.clone_from(url);
            task.pr_mr_status.clone_from(status);
        }
        TiberEvent::TaskAcceptanceAdded { stem, item, .. } => {
            task_mut(projection, stem)?.acceptance.push(item.clone())
        }
        TiberEvent::TaskAcceptanceChecked {
            stem,
            index,
            checked,
            ..
        } => {
            let item = task_mut(projection, stem)?
                .acceptance
                .get_mut(*index)
                .ok_or_else(|| {
                    Error::Parse(format!("acceptance_index_missing index={}", index + 1))
                })?;
            item.checked = *checked;
        }
        TiberEvent::TaskAcceptanceRemoved { stem, index, .. } => {
            let task = task_mut(projection, stem)?;
            if *index >= task.acceptance.len() {
                return Err(Error::Parse(format!(
                    "acceptance_index_missing index={}",
                    index + 1
                )));
            }
            task.acceptance.remove(*index);
        }
        TiberEvent::TaskNoteAdded { stem, note, .. } => {
            task_mut(projection, stem)?.notes.push(note.clone())
        }
        TiberEvent::TaskValidationRepaired { .. } | TiberEvent::TaskStatePublished { .. } => {}
        TiberEvent::TaskClosedFromTrailer { stem, .. } => {
            let task = task_mut(projection, stem)?;
            task.status = "done".into();
            task.claim = None;
        }
        TiberEvent::TaskRemoved { stem, .. } => {
            projection.tasks.remove(stem);
        }
        TiberEvent::CiRecoveryClaimed { state, .. }
        | TiberEvent::CiRecoveryJoined { state, .. }
        | TiberEvent::CiRecoveryTransferred { state, .. }
        | TiberEvent::CiRecoveryTakenOver { state, .. }
        | TiberEvent::CiRecoveryAssigned { state, .. }
        | TiberEvent::CiRecoveryReported { state, .. }
        | TiberEvent::CiRecoveryHeartbeatRecorded { state, .. }
        | TiberEvent::CiRecoveryDiagnosed { state, .. }
        | TiberEvent::CiRecoveryActionChosen { state, .. }
        | TiberEvent::CiRecoveryReplacementRecorded { state, .. }
        | TiberEvent::CiRecoveryResolved { state, .. }
        | TiberEvent::RecoveryStatePublished { state, .. } => {
            projection.ci_recovery =
                Some(serde_json::from_value((**state).clone()).map_err(|error| {
                    Error::Parse(format!("ci_recovery_event_invalid source={error}"))
                })?);
        }
    }
    Ok(())
}

fn append_tiber_events(
    root: &Path,
    projection: &TiberProjection,
    events: Vec<TiberEvent>,
) -> Result<(), Error> {
    if events.is_empty() {
        return Ok(());
    }
    let store = GitEventStore::open(root).map_err(event_store_error)?;
    let mut writes = StreamWrites::new();
    let mut declared = std::collections::HashSet::new();
    for event in &events {
        if declared.insert(event.stream_id().clone()) {
            let expected = projection
                .versions
                .get(event.stream_id())
                .copied()
                .unwrap_or(StreamVersion::new(0));
            writes = writes
                .register_stream(event.stream_id().clone(), expected)
                .map_err(event_store_error)?;
        }
    }
    for event in events {
        writes = writes.append(event).map_err(event_store_error)?;
    }
    match run_async(async move { store.append_events(writes).await }) {
        Ok(_) => {}
        Err(EventStoreError::VersionConflict { .. }) => {
            return Err(Error::Parse("event_version_conflict=true".into()));
        }
        Err(EventStoreError::StoreFailure { .. }) => {
            return Err(Error::Parse(
                "event_store_failed source_redacted=true event_store_authoritative_ref_retry=true"
                    .into(),
            ));
        }
        Err(error) => return Err(event_store_error(error)),
    }
    Ok(())
}

fn task_change_events(
    before: &TiberProjection,
    after: &TiberProjection,
    mutation: TaskMutation,
) -> Result<Vec<TiberEvent>, Error> {
    let mut events = Vec::new();
    let mut repairs = Vec::new();
    if !before.initialized {
        events.push(TiberEvent::RepositoryInitialized {
            stream_id: stream_id(REPOSITORY_STREAM)?,
        });
    }
    for (stem, task) in &after.tasks {
        let id = stream_id(format!("tiber:task:{stem}"))?;
        let Some(old) = before.tasks.get(stem) else {
            events.push(TiberEvent::TaskCreated {
                stream_id: id,
                task: Box::new(task.clone()),
            });
            continue;
        };
        if old.status != task.status || old.claim != task.claim {
            if matches!(mutation, TaskMutation::CloseFromTrailer) && task.status == "done" {
                events.push(TiberEvent::TaskClosedFromTrailer {
                    stream_id: id.clone(),
                    stem: stem.clone(),
                });
            } else {
                events.push(TiberEvent::TaskTransitioned {
                    stream_id: id.clone(),
                    stem: stem.clone(),
                    status: task.status.clone(),
                    claim: task.claim.clone(),
                });
            }
        }
        if old.blocks != task.blocks || old.blocked_by != task.blocked_by {
            events.push(TiberEvent::TaskLinksChanged {
                stream_id: id.clone(),
                stem: stem.clone(),
                blocks: task.blocks.clone(),
                blocked_by: task.blocked_by.clone(),
            });
            if matches!(mutation, TaskMutation::ValidateRepair) {
                for target in task
                    .blocks
                    .iter()
                    .filter(|target| !old.blocks.contains(*target))
                {
                    repairs.push(ValidationRepair::ReciprocalLinkAdded {
                        task: stem.clone(),
                        field: "blocks".into(),
                        target: target.clone(),
                    });
                }
                for target in task
                    .blocked_by
                    .iter()
                    .filter(|target| !old.blocked_by.contains(*target))
                {
                    repairs.push(ValidationRepair::ReciprocalLinkAdded {
                        task: stem.clone(),
                        field: "blocked_by".into(),
                        target: target.clone(),
                    });
                }
            }
        }
        if old.title != task.title
            || old.tags != task.tags
            || old.summary != task.summary
            || old.context != task.context
        {
            events.push(TiberEvent::TaskDetailsUpdated {
                stream_id: id.clone(),
                stem: stem.clone(),
                title: task.title.clone(),
                tags: task.tags.clone(),
                summary: task.summary.clone(),
                context: task.context.clone(),
            });
        }
        if old.pr_mr_url != task.pr_mr_url || old.pr_mr_status != task.pr_mr_status {
            events.push(TiberEvent::TaskPullRequestChanged {
                stream_id: id.clone(),
                stem: stem.clone(),
                url: task.pr_mr_url.clone(),
                status: task.pr_mr_status.clone(),
            });
        }
        if task.subtasks.len() == old.subtasks.len() + 1 && task.subtasks.starts_with(&old.subtasks)
        {
            events.push(TiberEvent::TaskSubtaskAdded {
                stream_id: id.clone(),
                stem: stem.clone(),
                subtask: task.subtasks.last().expect("one appended subtask").clone(),
            });
        } else {
            for (prior, current) in old.subtasks.iter().zip(&task.subtasks) {
                if prior.id == current.id && prior.checked != current.checked {
                    events.push(TiberEvent::TaskSubtaskChecked {
                        stream_id: id.clone(),
                        stem: stem.clone(),
                        subtask_id: current.id.clone(),
                        checked: current.checked,
                    });
                }
            }
        }
        if task.acceptance.len() == old.acceptance.len() + 1
            && task.acceptance.starts_with(&old.acceptance)
        {
            events.push(TiberEvent::TaskAcceptanceAdded {
                stream_id: id.clone(),
                stem: stem.clone(),
                item: task
                    .acceptance
                    .last()
                    .expect("one appended criterion")
                    .clone(),
            });
        } else if old.acceptance.len() == task.acceptance.len() + 1 {
            let index = (0..old.acceptance.len())
                .find(|index| old.acceptance.get(*index + 1..) == task.acceptance.get(*index..))
                .unwrap_or(old.acceptance.len() - 1);
            events.push(TiberEvent::TaskAcceptanceRemoved {
                stream_id: id.clone(),
                stem: stem.clone(),
                index,
            });
        } else {
            for (index, (prior, current)) in old.acceptance.iter().zip(&task.acceptance).enumerate()
            {
                if prior.text == current.text && prior.checked != current.checked {
                    events.push(TiberEvent::TaskAcceptanceChecked {
                        stream_id: id.clone(),
                        stem: stem.clone(),
                        index,
                        checked: current.checked,
                    });
                }
            }
        }
        for note in task.notes.iter().skip(old.notes.len()) {
            events.push(TiberEvent::TaskNoteAdded {
                stream_id: id.clone(),
                stem: stem.clone(),
                note: note.clone(),
            });
        }
    }
    for stem in before
        .tasks
        .keys()
        .filter(|stem| !after.tasks.contains_key(*stem))
    {
        events.push(TiberEvent::TaskRemoved {
            stream_id: stream_id(format!("tiber:task:{stem}"))?,
            stem: stem.clone(),
        });
    }
    if before.order != after.order {
        events.push(if matches!(mutation, TaskMutation::Prioritize) {
            TiberEvent::TaskPriorityChanged {
                stream_id: stream_id(BOARD_STREAM)?,
                order: after.order.clone(),
            }
        } else {
            TiberEvent::BoardReordered {
                stream_id: stream_id(BOARD_STREAM)?,
                order: after.order.clone(),
            }
        });
        if matches!(mutation, TaskMutation::ValidateRepair) {
            for task in after
                .order
                .iter()
                .filter(|task| !before.order.contains(*task))
            {
                repairs.push(ValidationRepair::BoardEntryAdded { task: task.clone() });
            }
            for task in before
                .order
                .iter()
                .filter(|task| !after.order.contains(*task))
            {
                repairs.push(ValidationRepair::BoardEntryRemoved { task: task.clone() });
            }
        }
    }
    if matches!(mutation, TaskMutation::ValidateRepair) && !repairs.is_empty() {
        events.push(TiberEvent::TaskValidationRepaired {
            stream_id: stream_id(BOARD_STREAM)?,
            repairs,
        });
    }
    if events
        .iter()
        .any(|event| !matches!(event, TiberEvent::RepositoryInitialized { .. }))
    {
        events.push(TiberEvent::TaskStatePublished {
            stream_id: stream_id(BOARD_STREAM)?,
        });
    }
    Ok(events)
}

fn ci_recovery_event(
    message: &str,
    stream_id: StreamId,
    state: &CiRecoveryState,
) -> Result<TiberEvent, Error> {
    let state = Box::new(
        serde_json::to_value(state)
            .map_err(|error| Error::Parse(format!("ci_recovery_event_invalid source={error}")))?,
    );
    Ok(match message {
        "Claim CI recovery" => TiberEvent::CiRecoveryClaimed { stream_id, state },
        "Join CI recovery" => TiberEvent::CiRecoveryJoined { stream_id, state },
        "Transfer CI recovery" => TiberEvent::CiRecoveryTransferred { stream_id, state },
        "Take over CI recovery" => TiberEvent::CiRecoveryTakenOver { stream_id, state },
        "Assign CI recovery helper" => TiberEvent::CiRecoveryAssigned { stream_id, state },
        "Report CI recovery helper result" => TiberEvent::CiRecoveryReported { stream_id, state },
        "Renew CI recovery lease" => TiberEvent::CiRecoveryHeartbeatRecorded { stream_id, state },
        "Diagnose CI recovery" => TiberEvent::CiRecoveryDiagnosed { stream_id, state },
        "Choose CI recovery action" => TiberEvent::CiRecoveryActionChosen { stream_id, state },
        "Record CI replacement" => TiberEvent::CiRecoveryReplacementRecorded { stream_id, state },
        "Resolve CI recovery" => TiberEvent::CiRecoveryResolved { stream_id, state },
        _ => {
            return Err(Error::Parse(format!(
                "ci_recovery_transition_unknown message={message:?}"
            )))
        }
    })
}

fn ci_recovery_state_published(
    stream_id: StreamId,
    state: &CiRecoveryState,
) -> Result<TiberEvent, Error> {
    Ok(TiberEvent::RecoveryStatePublished {
        stream_id,
        state: Box::new(
            serde_json::to_value(state).map_err(|error| {
                Error::Parse(format!("ci_recovery_event_invalid source={error}"))
            })?,
        ),
    })
}
const WORKFLOW_BLOCKER_FILE: &str = "workflow-blocker.json";

thread_local! {
    static MCP_CI_RECOVERY_SESSION: RefCell<Option<String>> = const { RefCell::new(None) };
    static COMMAND_TASK_IDS: RefCell<Option<(Vec<String>, usize)>> = const { RefCell::new(None) };
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkflowBlocker {
    schema_version: u8,
    kind: String,
    error_code: String,
    required_action: String,
    created_at: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct WorkflowBlockerData {
    pub error_code: &'static str,
    pub required_action: &'static str,
}

pub fn with_mcp_ci_recovery_session<T>(session: &str, operation: impl FnOnce() -> T) -> T {
    MCP_CI_RECOVERY_SESSION.with(|slot| {
        let previous = slot.replace(Some(session.to_string()));
        let result = operation();
        slot.replace(previous);
        result
    })
}

pub fn workflow_guard(hook_input: &str) -> Result<Option<String>, Error> {
    let input: serde_json::Value = serde_json::from_str(hook_input)
        .map_err(|error| Error::Parse(format!("workflow_hook_input_invalid source={error}")))?;
    let cwd = input
        .get("cwd")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(".");
    let repo = GitRepository::at(cwd);
    let path = repo
        .git_common_dir()?
        .join("tiber")
        .join(WORKFLOW_BLOCKER_FILE);
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(Error::Io(error)),
    };
    let blocker: WorkflowBlocker = match serde_json::from_str(&contents) {
        Ok(blocker) => blocker,
        Err(_) => return Ok(Some(
            "tiber.workflow_blocker_invalid workflow_blocked=true required_action=\"repair the Tiber workflow blocker before continuing\". Do not diagnose, edit, test, rerun, push, or perform unrelated work."
                .to_string(),
        )),
    };
    let tool_name = input
        .get("tool_name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let recovery = [
        "tiber.ci_recovery.claim",
        "tiber.ci_recovery.status",
        "tiber.sync",
    ];
    if recovery.iter().any(|allowed| tool_name.ends_with(allowed)) || exact_cli_recovery(&input) {
        return Ok(None);
    }
    Ok(Some(format!(
        "tiber.workflow_blocked workflow_blocked=true error_code={} required_action=\"{}\". Do not diagnose, edit, test, rerun, push, or perform unrelated work.",
        blocker.error_code, blocker.required_action
    )))
}

fn exact_cli_recovery(input: &serde_json::Value) -> bool {
    let command = input
        .pointer("/tool_input/command")
        .or_else(|| input.pointer("/tool_input/cmd"))
        .and_then(serde_json::Value::as_str);
    let Some(command) = command else {
        return false;
    };
    if command.contains("$(")
        || command
            .chars()
            .any(|character| matches!(character, ';' | '|' | '&' | '<' | '>' | '\n' | '\r' | '`'))
    {
        return false;
    }
    let Some(tokens) = shlex::split(command) else {
        return false;
    };
    let Some(executable) = tokens.first() else {
        return false;
    };
    if Path::new(executable).file_name() != Some(OsStr::new("tiber")) {
        return false;
    }
    match tokens.get(1..).unwrap_or_default() {
        [command] if command == "sync" => true,
        [group, command] if group == "ci-recovery" && command == "status" => true,
        [group, command, arguments @ ..] if group == "ci-recovery" && command == "claim" => {
            exact_claim_arguments(arguments)
        }
        _ => false,
    }
}

fn exact_claim_arguments(arguments: &[String]) -> bool {
    const OPTIONS: &[&str] = &[
        "--run-id",
        "--run-url",
        "--failed-sha",
        "--workflow",
        "--ref",
    ];
    let mut seen = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        let (option, has_value) = match argument.split_once('=') {
            Some((option, value)) => (option, !value.is_empty()),
            None => {
                index += 1;
                (
                    argument.as_str(),
                    arguments.get(index).is_some_and(|value| !value.is_empty()),
                )
            }
        };
        if !has_value || !OPTIONS.contains(&option) || seen.contains(&option) {
            return false;
        }
        seen.push(option);
        index += 1;
    }
    seen.len() == OPTIONS.len()
}

fn record_workflow_blocker(repo: &GitRepository, blocker: WorkflowBlocker) -> Result<(), Error> {
    let directory = repo.git_common_dir()?.join("tiber");
    fs::create_dir_all(&directory)?;
    let path = directory.join(WORKFLOW_BLOCKER_FILE);
    let temporary = directory.join(format!(".{WORKFLOW_BLOCKER_FILE}.{}", std::process::id()));
    let bytes = serde_json::to_vec(&blocker)
        .map_err(|error| Error::Parse(format!("workflow_blocker_json_invalid source={error}")))?;
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn clear_workflow_blocker(repo: &GitRepository, kind: &str) -> Result<(), Error> {
    let path = repo
        .git_common_dir()?
        .join("tiber")
        .join(WORKFLOW_BLOCKER_FILE);
    let Ok(contents) = fs::read_to_string(&path) else {
        return Ok(());
    };
    let blocker: WorkflowBlocker = serde_json::from_str(&contents)
        .map_err(|_| Error::Parse("workflow_blocker_invalid workflow_blocked=true".to_string()))?;
    if blocker.kind == kind {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Error::Io(error)),
        }
    }
    Ok(())
}

pub fn init_repository() -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.init_repository()
}

pub fn dashboard_runtime_dir() -> Result<PathBuf, Error> {
    let repo = GitRepository::discover()?;
    Ok(repo.git_common_dir()?.join("tiber"))
}

pub fn acquire_dashboard_startup_lock() -> Result<DashboardStartupLock, Error> {
    let repo = GitRepository::discover()?;
    Ok(DashboardStartupLock {
        _lock: repo.acquire_named_lock("dashboard-startup.lock")?,
    })
}

pub struct DashboardStartupLock {
    _lock: TiberLock,
}

pub fn init_repository_at(root: impl Into<PathBuf>) -> Result<(), Error> {
    let repo = GitRepository::at(root);
    repo.init_repository()
}

#[doc(hidden)]
pub fn sync_repository_at(root: impl Into<PathBuf>) -> Result<(), Error> {
    GitRepository::at(root).sync_repository()
}

pub fn claim_ci_recovery(input: CiRecoveryTrigger) -> Result<CiRecoveryClaim, Error> {
    let repo = GitRepository::discover()?;
    match repo.claim_ci_recovery(input) {
        Ok(claim) => {
            clear_workflow_blocker(&repo, "ci_claim_failed")?;
            Ok(claim)
        }
        Err(source) => {
            record_workflow_blocker(
                &repo,
                WorkflowBlocker {
                    schema_version: 1,
                    kind: "ci_claim_failed".to_string(),
                    error_code: "tiber.ci_recovery_claim_failed".to_string(),
                    required_action: "retry the shared Tiber CI-recovery claim; use status or sync only as needed to restore it".to_string(),
                    created_at: unix_timestamp()?,
                },
            )?;
            Err(Error::WorkflowBlocked {
                code: "tiber.ci_recovery_claim_failed",
                required_action: "retry the shared Tiber CI-recovery claim",
                source: Box::new(source),
            })
        }
    }
}

#[doc(hidden)]
pub fn claim_ci_recovery_at(
    root: impl Into<PathBuf>,
    input: CiRecoveryTrigger,
) -> Result<CiRecoveryClaim, Error> {
    GitRepository::at(root).claim_ci_recovery(input)
}

#[doc(hidden)]
pub fn ci_recovery_status_at(root: impl Into<PathBuf>) -> Result<CiRecoveryStatus, Error> {
    GitRepository::at(root).ci_recovery_status()
}

pub fn assert_ci_recovery_owner(
    incident_id: &str,
    epoch: u64,
) -> Result<CiRecoveryAssertion, Error> {
    let repo = GitRepository::discover()?;
    repo.assert_ci_recovery_owner(incident_id, epoch)
}

pub fn transfer_ci_recovery(
    incident_id: &str,
    epoch: u64,
    to_host: &str,
    to_session: &str,
) -> Result<CiRecoveryTransfer, Error> {
    let repo = GitRepository::discover()?;
    repo.transfer_ci_recovery(incident_id, epoch, to_host, to_session)
}

pub fn takeover_ci_recovery(incident_id: &str, epoch: u64) -> Result<CiRecoveryTransfer, Error> {
    let repo = GitRepository::discover()?;
    repo.takeover_ci_recovery(incident_id, epoch)
}

pub fn assign_ci_recovery(
    incident_id: &str,
    epoch: u64,
    input: CiRecoveryAssignmentInput,
) -> Result<CiRecoveryAssignmentResult, Error> {
    let repo = GitRepository::discover()?;
    repo.assign_ci_recovery(incident_id, epoch, input)
}

pub fn report_ci_recovery(
    incident_id: &str,
    assignment_id: &str,
    summary: &str,
    evidence: &str,
) -> Result<CiRecoveryAssignmentResult, Error> {
    let repo = GitRepository::discover()?;
    repo.report_ci_recovery(incident_id, assignment_id, summary, evidence)
}

pub fn heartbeat_ci_recovery(incident_id: &str, epoch: u64) -> Result<CiRecoveryAssertion, Error> {
    let repo = GitRepository::discover()?;
    repo.heartbeat_ci_recovery(incident_id, epoch)
}

pub fn wait_for_ci_recovery(
    incident_id: &str,
    epoch: u64,
    timeout_seconds: u64,
) -> Result<CiRecoveryWait, Error> {
    let repo = GitRepository::discover()?;
    repo.wait_for_ci_recovery(incident_id, epoch, timeout_seconds)
}

pub fn diagnose_ci_recovery(
    incident_id: &str,
    epoch: u64,
    record: CiRecoveryDiagnosisInput,
) -> Result<CiRecoveryStatus, Error> {
    let repo = GitRepository::discover()?;
    repo.diagnose_ci_recovery(incident_id, epoch, record)
}

pub fn choose_ci_recovery_action(
    incident_id: &str,
    epoch: u64,
    kind: &str,
    description: &str,
) -> Result<CiRecoveryStatus, Error> {
    let repo = GitRepository::discover()?;
    repo.choose_ci_recovery_action(incident_id, epoch, kind, description)
}

pub fn record_ci_recovery_replacement(
    incident_id: &str,
    epoch: u64,
    replacement: CiRecoveryReplacementInput,
) -> Result<CiRecoveryStatus, Error> {
    let repo = GitRepository::discover()?;
    repo.record_ci_recovery_replacement(incident_id, epoch, replacement)
}

pub fn resolve_ci_recovery(
    incident_id: &str,
    proof: CiRecoveryReleaseInput,
) -> Result<CiRecoveryStatus, Error> {
    let repo = GitRepository::discover()?;
    repo.resolve_ci_recovery(incident_id, proof)
}

pub fn ci_recovery_status() -> Result<CiRecoveryStatus, Error> {
    let repo = GitRepository::discover()?;
    repo.ci_recovery_status()
}

pub fn create_task_at(root: impl Into<PathBuf>, title: &str) -> Result<TaskPath, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_workspace(TaskMutation::Create, |repo| {
        repo.create_task(TaskTitle::parse(title)?)
    })
}

pub fn list_tasks_at(root: impl Into<PathBuf>) -> Result<Vec<TaskSummary>, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_snapshot_workspace(|repo| repo.list_tasks())
}

pub fn list_tasks_by_status_at(
    root: impl Into<PathBuf>,
    status: &str,
) -> Result<Vec<TaskSummary>, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_snapshot_workspace(|repo| repo.list_tasks_by_status(status))
}

pub fn search_tasks_at(
    root: impl Into<PathBuf>,
    query: &str,
) -> Result<Vec<TaskSearchResult>, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_snapshot_workspace(|repo| repo.search_tasks(query))
}

pub fn show_task_at(root: impl Into<PathBuf>, task_ref: &str) -> Result<String, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_snapshot_workspace(|repo| repo.show_task(task_ref))
}

pub fn task_metadata_at(root: impl Into<PathBuf>, task_ref: &str) -> Result<TaskMetadata, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_snapshot_workspace(|repo| repo.task_metadata(task_ref))
}

pub fn prioritize_before_at(
    root: impl Into<PathBuf>,
    task_ref: &str,
    before_ref: &str,
) -> Result<(), Error> {
    let repo = GitRepository::at(root);
    repo.with_task_workspace(TaskMutation::Prioritize, |repo| {
        repo.prioritize_before(task_ref, before_ref)
    })
}

#[doc(hidden)]
pub fn transition_task_at(
    root: impl Into<PathBuf>,
    task_ref: &str,
    status: &str,
) -> Result<TaskPath, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_workspace(TaskMutation::Transition, |repo| {
        repo.transition_task(task_ref, status)
    })
}

#[doc(hidden)]
pub fn link_blocks_at(root: impl Into<PathBuf>, from_ref: &str, to_ref: &str) -> Result<(), Error> {
    let repo = GitRepository::at(root);
    repo.with_task_workspace(TaskMutation::Dependencies, |repo| {
        repo.link_blocks(from_ref, to_ref)
    })
}

#[doc(hidden)]
pub fn update_task_at(
    root: impl Into<PathBuf>,
    task_ref: &str,
    update: TaskUpdate<'_>,
) -> Result<(), Error> {
    let details = update.title.is_some()
        || update.summary.is_some()
        || update.context.is_some()
        || update.tags.is_some();
    let pull_request = update.pr_mr_url.is_some() || update.pr_mr_status.is_some();
    let mutation = match (details, pull_request) {
        (true, true) => TaskMutation::UpdateDetailsAndPullRequest,
        (false, true) => TaskMutation::UpdatePullRequest,
        _ => TaskMutation::UpdateDetails,
    };
    let repo = GitRepository::at(root);
    repo.with_task_workspace(mutation, |repo| repo.update_task(task_ref, update.clone()))
}

pub fn task_documents_at(root: impl Into<PathBuf>) -> Result<Vec<TaskDocument>, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_snapshot_workspace(|repo| repo.task_documents_snapshot())
}

pub fn list_docs_at(root: impl Into<PathBuf>) -> Result<Vec<String>, Error> {
    let repo = GitRepository::at(root);
    repo.list_docs()
}

pub fn read_doc_at(root: impl Into<PathBuf>, doc_ref: &str) -> Result<String, Error> {
    let repo = GitRepository::at(root);
    repo.read_doc(doc_ref)
}

impl GitRepository {
    fn init_repository(&self) -> Result<(), Error> {
        let _lock = self.acquire_lock()?;
        let projection = load_tiber_projection(&self.root)?;
        if !projection.initialized {
            append_tiber_events(
                &self.root,
                &projection,
                vec![TiberEvent::RepositoryInitialized {
                    stream_id: stream_id(REPOSITORY_STREAM)?,
                }],
            )?;
        }
        Ok(())
    }

    fn claim_ci_recovery(&self, input: CiRecoveryTrigger) -> Result<CiRecoveryClaim, Error> {
        let _lock = self.acquire_lock()?;
        let participant = ci_recovery_participant()?;
        let input = CiRecoveryTrigger {
            run_id: required_ci_recovery_text("run_id", &input.run_id)?,
            run_url: required_ci_recovery_text("run_url", &input.run_url)?,
            failed_sha: required_ci_recovery_text("failed_sha", &input.failed_sha)?,
            workflow: required_ci_recovery_text("workflow", &input.workflow)?,
            git_ref: required_ci_recovery_text("git_ref", &input.git_ref)?,
        };
        let claim_time = unix_timestamp()?;

        for attempt in 1..=MAX_SYNC_ATTEMPTS {
            let remote_parent = self.fetch_coordination_branch()?;
            let active_state = self.read_active_ci_recovery(remote_parent.as_deref())?;
            if remote_parent.is_some() && active_state.is_none() {
                return Err(Error::Parse(
                    "ci_recovery_coordination_ref_invalid active_json=missing mutation=false"
                        .to_string(),
                ));
            }
            if let Some(mut state) = active_state {
                if state.state != "resolved" {
                    let role = if state.owner == participant {
                        CiRecoveryRole::Owner
                    } else {
                        CiRecoveryRole::Waiting
                    };
                    let mut changed = false;
                    if state.triggers.is_empty() {
                        state.triggers.push(state.trigger.clone());
                    }
                    if !state.triggers.contains(&input) {
                        let matches_failed_replacement =
                            state.replacement.as_ref().is_some_and(|replacement| {
                                replacement.status == "failed"
                                    && replacement.run_id == input.run_id
                                    && replacement.run_url == input.run_url
                                    && replacement.sha == input.failed_sha
                            });
                        if !matches_failed_replacement {
                            return Err(Error::Parse(format!(
                                "ci_recovery_distinct_trigger_requires_separate_incident active_incident_id={}",
                                state.incident_id
                            )));
                        }
                        state.trigger = input.clone();
                        state.triggers.push(input.clone());
                        changed = true;
                    }
                    if !state.participants.contains(&participant) {
                        state.participants.push(participant.clone());
                        changed = true;
                    }
                    if !changed {
                        return Ok(CiRecoveryClaim::from_state(state, role));
                    }
                    match self.push_ci_recovery_state(
                        &state,
                        remote_parent.as_deref(),
                        "Join CI recovery",
                    ) {
                        Ok(()) => {
                            return Ok(CiRecoveryClaim::from_state(state, role));
                        }
                        Err(error)
                            if is_retryable_push_failure(&error) && attempt < MAX_SYNC_ATTEMPTS =>
                        {
                            continue;
                        }
                        Err(error) => return Err(error),
                    }
                }
            }

            let state = CiRecoveryState {
                schema_version: 1,
                incident_id: ci_recovery_incident_id(&input.run_id),
                state: "diagnosing".to_string(),
                epoch: 1,
                trigger: input.clone(),
                triggers: vec![input.clone()],
                owner: participant.clone(),
                lease_expires_at: claim_time.saturating_add(CI_RECOVERY_LEASE_SECONDS),
                participants: vec![participant.clone()],
                assignments: Vec::new(),
                failure_record: None,
                diagnosis: None,
                next_action: None,
                replacement: None,
                release_proof: None,
            };
            match self.push_ci_recovery_state(&state, remote_parent.as_deref(), "Claim CI recovery")
            {
                Ok(()) => {
                    return Ok(CiRecoveryClaim::from_state(state, CiRecoveryRole::Owner));
                }
                Err(error)
                    if (is_retryable_push_failure(&error)
                        || is_coordination_branch_creation_race(&error))
                        && attempt < MAX_SYNC_ATTEMPTS =>
                {
                    continue;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("CI recovery claim attempts always return")
    }

    fn assert_ci_recovery_owner(
        &self,
        incident_id: &str,
        epoch: u64,
    ) -> Result<CiRecoveryAssertion, Error> {
        let _lock = self.acquire_lock()?;
        let participant = ci_recovery_participant()?;
        let coordination_ref = self
            .fetch_coordination_branch()?
            .ok_or_else(|| Error::Parse("ci_recovery_incident_missing active=false".to_string()))?;
        let state = self
            .read_active_ci_recovery(Some(&coordination_ref))?
            .ok_or_else(|| Error::Parse("ci_recovery_incident_missing active=false".to_string()))?;
        if state.incident_id != incident_id {
            return Err(Error::Parse(format!(
                "ci_recovery_incident_mismatch expected={} actual={incident_id}",
                state.incident_id
            )));
        }
        if state.epoch != epoch {
            return Err(Error::Parse(format!(
                "ci_recovery_stale_epoch expected={} actual={epoch}",
                state.epoch
            )));
        }
        if state.owner != participant {
            return Err(Error::Parse(format!(
                "ci_recovery_not_owner incident_id={} epoch={}",
                state.incident_id, state.epoch
            )));
        }
        ensure_ci_recovery_lease_active(&state, unix_timestamp()?)?;
        Ok(CiRecoveryAssertion {
            allowed: true,
            incident_id: state.incident_id,
            epoch: state.epoch,
            lease_expires_at: state.lease_expires_at,
        })
    }

    fn transfer_ci_recovery(
        &self,
        incident_id: &str,
        epoch: u64,
        to_host: &str,
        to_session: &str,
    ) -> Result<CiRecoveryTransfer, Error> {
        let _lock = self.acquire_lock()?;
        let caller = ci_recovery_participant()?;
        let recipient = ci_recovery_participant_from(to_host, to_session)?;
        let now = unix_timestamp()?;

        for attempt in 1..=MAX_SYNC_ATTEMPTS {
            let parent = self.fetch_coordination_branch()?.ok_or_else(|| {
                Error::Parse("ci_recovery_incident_missing active=false".to_string())
            })?;
            let mut state = self
                .read_active_ci_recovery(Some(&parent))?
                .ok_or_else(|| {
                    Error::Parse("ci_recovery_incident_missing active=false".to_string())
                })?;
            ensure_ci_recovery_owner(&state, incident_id, epoch, &caller)?;
            ensure_ci_recovery_not_resolved(&state)?;
            ensure_ci_recovery_lease_active(&state, now)?;
            state.owner = recipient.clone();
            if !state.participants.contains(&recipient) {
                state.participants.push(recipient.clone());
            }
            state.epoch = state.epoch.saturating_add(1);
            state.lease_expires_at = now.saturating_add(CI_RECOVERY_LEASE_SECONDS);

            match self.push_ci_recovery_state(&state, Some(&parent), "Transfer CI recovery") {
                Ok(()) => {
                    return Ok(CiRecoveryTransfer {
                        incident_id: state.incident_id,
                        epoch: state.epoch,
                        lease_expires_at: state.lease_expires_at,
                    });
                }
                Err(error) if is_retryable_push_failure(&error) && attempt < MAX_SYNC_ATTEMPTS => {
                    continue;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("CI recovery transfer attempts always return")
    }

    fn takeover_ci_recovery(
        &self,
        incident_id: &str,
        epoch: u64,
    ) -> Result<CiRecoveryTransfer, Error> {
        let _lock = self.acquire_lock()?;
        let successor = ci_recovery_participant()?;
        let now = unix_timestamp()?;

        for attempt in 1..=MAX_SYNC_ATTEMPTS {
            let parent = self.fetch_coordination_branch()?.ok_or_else(|| {
                Error::Parse("ci_recovery_incident_missing active=false".to_string())
            })?;
            let mut state = self
                .read_active_ci_recovery(Some(&parent))?
                .ok_or_else(|| {
                    Error::Parse("ci_recovery_incident_missing active=false".to_string())
                })?;
            ensure_ci_recovery_incident_epoch(&state, incident_id, epoch)?;
            ensure_ci_recovery_not_resolved(&state)?;
            if state.owner == successor {
                return Err(Error::Parse(format!(
                    "ci_recovery_already_owner incident_id={} epoch={}",
                    state.incident_id, state.epoch
                )));
            }
            if state.lease_expires_at > now {
                return Err(Error::Parse(format!(
                    "ci_recovery_lease_active incident_id={} epoch={} expires_at={}",
                    state.incident_id, state.epoch, state.lease_expires_at
                )));
            }
            state.owner = successor.clone();
            if !state.participants.contains(&successor) {
                state.participants.push(successor.clone());
            }
            state.epoch = state.epoch.saturating_add(1);
            state.lease_expires_at = now.saturating_add(CI_RECOVERY_LEASE_SECONDS);

            match self.push_ci_recovery_state(&state, Some(&parent), "Take over CI recovery") {
                Ok(()) => {
                    return Ok(CiRecoveryTransfer {
                        incident_id: state.incident_id,
                        epoch: state.epoch,
                        lease_expires_at: state.lease_expires_at,
                    });
                }
                Err(error) if is_retryable_push_failure(&error) && attempt < MAX_SYNC_ATTEMPTS => {
                    continue;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("CI recovery takeover attempts always return")
    }

    fn assign_ci_recovery(
        &self,
        incident_id: &str,
        epoch: u64,
        input: CiRecoveryAssignmentInput,
    ) -> Result<CiRecoveryAssignmentResult, Error> {
        let caller = ci_recovery_participant()?;
        let assignee = ci_recovery_participant_from(&input.to_host, &input.to_session)?;
        let capabilities = input
            .capabilities
            .split(',')
            .map(str::trim)
            .filter(|capability| !capability.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        if capabilities.is_empty()
            || capabilities.iter().any(|capability| {
                !matches!(
                    capability.as_str(),
                    "inspect" | "reproduce" | "edit" | "test"
                )
            })
        {
            return Err(Error::Parse(
                "ci_recovery_capability_invalid allowed=inspect,reproduce,edit,test".to_string(),
            ));
        }
        let scope = required_ci_recovery_text("assignment_scope", &input.scope)?;
        self.mutate_ci_recovery("Assign CI recovery helper", |state, now| {
            ensure_ci_recovery_owner(state, incident_id, epoch, &caller)?;
            ensure_ci_recovery_lease_active(state, now)?;
            if !state.participants.contains(&assignee) {
                return Err(Error::Parse(format!(
                    "ci_recovery_assignee_not_joined host={} session={}",
                    assignee.host, assignee.session
                )));
            }
            let assignment_id = format!("a{}", state.assignments.len().saturating_add(1));
            state.assignments.push(CiRecoveryAssignment {
                id: assignment_id.clone(),
                owner_epoch: state.epoch,
                assignee: assignee.clone(),
                capabilities: capabilities.clone(),
                scope: scope.clone(),
                report: None,
            });
            Ok(CiRecoveryAssignmentResult {
                incident_id: state.incident_id.clone(),
                assignment_id,
                epoch: state.epoch,
            })
        })
    }

    fn report_ci_recovery(
        &self,
        incident_id: &str,
        assignment_id: &str,
        summary: &str,
        evidence: &str,
    ) -> Result<CiRecoveryAssignmentResult, Error> {
        let caller = ci_recovery_participant()?;
        let summary = required_ci_recovery_text("assignment_summary", summary)?;
        let evidence = required_ci_recovery_text("assignment_evidence", evidence)?;
        self.mutate_ci_recovery("Report CI recovery helper result", |state, _now| {
            if state.incident_id != incident_id {
                return Err(Error::Parse(format!(
                    "ci_recovery_incident_mismatch expected={} actual={incident_id}",
                    state.incident_id
                )));
            }
            let assignment = state
                .assignments
                .iter_mut()
                .find(|assignment| assignment.id == assignment_id)
                .ok_or_else(|| {
                    Error::Parse(format!(
                        "ci_recovery_assignment_missing assignment_id={assignment_id}"
                    ))
                })?;
            if assignment.owner_epoch != state.epoch {
                return Err(Error::Parse(format!(
                    "ci_recovery_assignment_stale assignment_epoch={} active_epoch={}",
                    assignment.owner_epoch, state.epoch
                )));
            }
            if assignment.assignee != caller {
                return Err(Error::Parse(format!(
                    "ci_recovery_assignment_not_assignee assignment_id={assignment_id}"
                )));
            }
            assignment.report = Some(CiRecoveryReport {
                summary: summary.clone(),
                evidence: evidence.clone(),
            });
            Ok(CiRecoveryAssignmentResult {
                incident_id: state.incident_id.clone(),
                assignment_id: assignment_id.to_string(),
                epoch: state.epoch,
            })
        })
    }

    fn heartbeat_ci_recovery(
        &self,
        incident_id: &str,
        epoch: u64,
    ) -> Result<CiRecoveryAssertion, Error> {
        let caller = ci_recovery_participant()?;
        self.mutate_ci_recovery("Renew CI recovery lease", |state, now| {
            ensure_ci_recovery_owner(state, incident_id, epoch, &caller)?;
            ensure_ci_recovery_lease_active(state, now)?;
            state.lease_expires_at = now.saturating_add(CI_RECOVERY_LEASE_SECONDS);
            Ok(CiRecoveryAssertion {
                allowed: true,
                incident_id: state.incident_id.clone(),
                epoch: state.epoch,
                lease_expires_at: state.lease_expires_at,
            })
        })
    }

    fn wait_for_ci_recovery(
        &self,
        incident_id: &str,
        epoch: u64,
        timeout_seconds: u64,
    ) -> Result<CiRecoveryWait, Error> {
        if timeout_seconds > 60 {
            return Err(Error::Parse(
                "ci_recovery_wait_timeout_invalid maximum_seconds=60".to_string(),
            ));
        }
        let participant = ci_recovery_participant()?;
        if timeout_seconds == 0 {
            return Ok(CiRecoveryWait::timeout(incident_id, epoch));
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(timeout_seconds);
        let mut last_state = None;
        loop {
            if let Some(state) = last_state.as_ref() {
                if std::time::Instant::now() >= deadline {
                    return Ok(CiRecoveryWait::from_state(state, "timeout", None));
                }
            }
            let _lock = match self.try_acquire_task_lock_once() {
                Ok(lock) => lock,
                Err(error) if is_tiber_lock_busy(&error) => {
                    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                    if remaining.is_zero() {
                        return Ok(last_state
                            .as_ref()
                            .map(|state| CiRecoveryWait::from_state(state, "timeout", None))
                            .unwrap_or_else(|| CiRecoveryWait::timeout(incident_id, epoch)));
                    }
                    std::thread::sleep(Duration::from_millis(50).min(remaining));
                    continue;
                }
                Err(error) => return Err(error),
            };
            let state = {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                if remaining.is_zero() {
                    return Ok(last_state
                        .as_ref()
                        .map(|state| CiRecoveryWait::from_state(state, "timeout", None))
                        .unwrap_or_else(|| CiRecoveryWait::timeout(incident_id, epoch)));
                }
                let parent = self
                    .fetch_coordination_branch_with_timeout(remaining.min(Duration::from_secs(10)))?
                    .ok_or_else(|| {
                        Error::Parse("ci_recovery_incident_missing active=false".to_string())
                    })?;
                self.read_active_ci_recovery(Some(&parent))?
                    .ok_or_else(|| {
                        Error::Parse("ci_recovery_incident_missing active=false".to_string())
                    })?
            };
            if state.incident_id != incident_id {
                return Err(Error::Parse(format!(
                    "ci_recovery_incident_mismatch expected={} actual={incident_id}",
                    state.incident_id
                )));
            }
            if !state.participants.contains(&participant) {
                return Err(Error::Parse(format!(
                    "ci_recovery_participant_required incident_id={incident_id}"
                )));
            }
            if state.epoch != epoch {
                return Ok(CiRecoveryWait::from_state(&state, "epoch-changed", None));
            }
            if state.state == "resolved" {
                return Ok(CiRecoveryWait::from_state(&state, "resolved", None));
            }
            if let Some(assignment) = state.assignments.iter().find(|assignment| {
                assignment.owner_epoch == state.epoch && assignment.assignee == participant
            }) {
                return Ok(CiRecoveryWait::from_state(
                    &state,
                    "assignment",
                    Some(assignment.id.clone()),
                ));
            }
            if std::time::Instant::now() >= deadline {
                return Ok(CiRecoveryWait::from_state(&state, "timeout", None));
            }
            last_state = Some(state);
            drop(_lock);
            ci_recovery_signal_wait_ready()?;
            std::thread::sleep(
                Duration::from_millis(250)
                    .min(deadline.saturating_duration_since(std::time::Instant::now())),
            );
        }
    }

    fn diagnose_ci_recovery(
        &self,
        incident_id: &str,
        epoch: u64,
        record: CiRecoveryDiagnosisInput,
    ) -> Result<CiRecoveryStatus, Error> {
        let caller = ci_recovery_participant()?;
        let classification = parse_ci_recovery_choice(
            "classification",
            &record.classification,
            &["caused", "unrelated", "transient"],
        )?;
        let job = required_ci_recovery_text("job", &record.job)?;
        let step = required_ci_recovery_text("step", &record.step)?;
        let log_evidence = required_ci_recovery_text("log_evidence", &record.log_evidence)?;
        let cause = required_ci_recovery_text("cause", &record.cause)?;
        self.mutate_ci_recovery("Diagnose CI recovery", |state, now| {
            ensure_ci_recovery_owner(state, incident_id, epoch, &caller)?;
            ensure_ci_recovery_lease_active(state, now)?;
            state.failure_record = Some(CiRecoveryFailureRecord {
                job: job.clone(),
                step: step.clone(),
                log_evidence: log_evidence.clone(),
            });
            state.diagnosis = Some(CiRecoveryDiagnosis {
                cause: cause.clone(),
                classification: classification.clone(),
            });
            state.next_action = None;
            state.replacement = None;
            state.release_proof = None;
            state.state = "diagnosing".to_string();
            Ok(CiRecoveryStatus::from_state(state))
        })
    }

    fn choose_ci_recovery_action(
        &self,
        incident_id: &str,
        epoch: u64,
        kind: &str,
        description: &str,
    ) -> Result<CiRecoveryStatus, Error> {
        let caller = ci_recovery_participant()?;
        let kind = parse_ci_recovery_choice("action", kind, &["repair", "rerun"])?;
        let description = required_ci_recovery_text("description", description)?;
        self.mutate_ci_recovery("Choose CI recovery action", |state, now| {
            ensure_ci_recovery_owner(state, incident_id, epoch, &caller)?;
            ensure_ci_recovery_lease_active(state, now)?;
            let diagnosis = state
                .diagnosis
                .as_ref()
                .ok_or_else(|| Error::Parse("ci_recovery_diagnosis_required=true".to_string()))?;
            let permitted = matches!(
                (diagnosis.classification.as_str(), kind.as_str()),
                ("caused", "repair") | ("unrelated", "rerun") | ("transient", "rerun")
            );
            if !permitted {
                return Err(Error::Parse(format!(
                    "ci_recovery_action_conflicts classification={} action={kind}",
                    diagnosis.classification
                )));
            }
            state.next_action = Some(CiRecoveryAction {
                kind: kind.clone(),
                description: description.clone(),
            });
            state.state = "action-selected".to_string();
            Ok(CiRecoveryStatus::from_state(state))
        })
    }

    fn record_ci_recovery_replacement(
        &self,
        incident_id: &str,
        epoch: u64,
        replacement: CiRecoveryReplacementInput,
    ) -> Result<CiRecoveryStatus, Error> {
        let caller = ci_recovery_participant()?;
        let status = parse_ci_recovery_choice(
            "replacement_status",
            &replacement.status,
            &["queued", "running", "failed"],
        )?;
        let run_id = required_ci_recovery_text("replacement_run_id", &replacement.run_id)?;
        let run_url = required_ci_recovery_text("replacement_run_url", &replacement.run_url)?;
        let sha = required_ci_recovery_text("replacement_sha", &replacement.sha)?;
        self.mutate_ci_recovery("Record CI replacement", |state, now| {
            ensure_ci_recovery_owner(state, incident_id, epoch, &caller)?;
            ensure_ci_recovery_lease_active(state, now)?;
            if state.next_action.is_none() {
                return Err(Error::Parse(
                    "ci_recovery_next_action_required=true".to_string(),
                ));
            }
            if state
                .next_action
                .as_ref()
                .is_some_and(|action| action.kind == "rerun")
                && sha != state.trigger.failed_sha
            {
                return Err(Error::Parse(
                    "ci_recovery_rerun_sha_mismatch expected=failed_sha".to_string(),
                ));
            }
            state.replacement = Some(CiRecoveryReplacement {
                run_id: run_id.clone(),
                run_url: run_url.clone(),
                sha: sha.clone(),
                status: status.clone(),
            });
            if status == "failed" {
                state.state = "diagnosing".to_string();
                state.failure_record = None;
                state.diagnosis = None;
                state.next_action = None;
            } else {
                state.state = "waiting-ci".to_string();
            }
            Ok(CiRecoveryStatus::from_state(state))
        })
    }

    fn resolve_ci_recovery(
        &self,
        incident_id: &str,
        proof: CiRecoveryReleaseInput,
    ) -> Result<CiRecoveryStatus, Error> {
        let participant = ci_recovery_participant()?;
        if proof.terminal_status != "success" {
            return Err(Error::Parse(format!(
                "ci_recovery_terminal_success_required actual={}",
                proof.terminal_status
            )));
        }
        let replacement_run_id =
            required_ci_recovery_text("replacement_run_id", &proof.replacement_run_id)?;
        let replacement_run_url =
            required_ci_recovery_text("replacement_run_url", &proof.replacement_run_url)?;
        let sha = required_ci_recovery_text("replacement_sha", &proof.sha)?;
        self.mutate_ci_recovery("Resolve CI recovery", |state, _now| {
            if state.incident_id != incident_id {
                return Err(Error::Parse(format!(
                    "ci_recovery_incident_mismatch expected={} actual={incident_id}",
                    state.incident_id
                )));
            }
            if !state.participants.contains(&participant) {
                return Err(Error::Parse(format!(
                    "ci_recovery_participant_required incident_id={}",
                    state.incident_id
                )));
            }
            let replacement = state
                .replacement
                .as_ref()
                .ok_or_else(|| Error::Parse("ci_recovery_replacement_required=true".to_string()))?;
            if replacement.status == "failed" {
                return Err(Error::Parse(format!(
                    "ci_recovery_replacement_failed run_id={}",
                    replacement.run_id
                )));
            }
            if replacement.run_id != replacement_run_id
                || replacement.run_url != replacement_run_url
                || replacement.sha != sha
            {
                return Err(Error::Parse(
                    "ci_recovery_release_proof_mismatch=true".to_string(),
                ));
            }
            state.release_proof = Some(CiRecoveryReleaseProof {
                replacement_run_id: replacement_run_id.clone(),
                replacement_run_url: replacement_run_url.clone(),
                sha: sha.clone(),
                terminal_status: "success".to_string(),
            });
            state.state = "resolved".to_string();
            Ok(CiRecoveryStatus::from_state(state))
        })
    }

    fn ci_recovery_status(&self) -> Result<CiRecoveryStatus, Error> {
        let _lock = self.acquire_lock()?;
        let parent = self
            .fetch_coordination_branch()?
            .ok_or_else(|| Error::Parse("ci_recovery_incident_missing active=false".to_string()))?;
        let state = self
            .read_active_ci_recovery(Some(&parent))?
            .ok_or_else(|| Error::Parse("ci_recovery_incident_missing active=false".to_string()))?;
        Ok(CiRecoveryStatus::from_state(&state))
    }

    fn mutate_ci_recovery<T>(
        &self,
        message: &str,
        mut operation: impl FnMut(&mut CiRecoveryState, u64) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let _lock = self.acquire_lock()?;
        let now = unix_timestamp()?;
        for attempt in 1..=MAX_SYNC_ATTEMPTS {
            let parent = self.fetch_coordination_branch()?.ok_or_else(|| {
                Error::Parse("ci_recovery_incident_missing active=false".to_string())
            })?;
            let mut state = self
                .read_active_ci_recovery(Some(&parent))?
                .ok_or_else(|| {
                    Error::Parse("ci_recovery_incident_missing active=false".to_string())
                })?;
            ensure_ci_recovery_not_resolved(&state)?;
            let result = operation(&mut state, now)?;
            match self.push_ci_recovery_state(&state, Some(&parent), message) {
                Ok(()) => return Ok(result),
                Err(error) if is_retryable_push_failure(&error) && attempt < MAX_SYNC_ATTEMPTS => {
                    continue;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("CI recovery mutation attempts always return")
    }

    fn fetch_coordination_branch(&self) -> Result<Option<String>, Error> {
        self.fetch_coordination_branch_with_timeout(Duration::from_secs(10))
    }

    fn fetch_coordination_branch_with_timeout(
        &self,
        _timeout: Duration,
    ) -> Result<Option<String>, Error> {
        if git_status(["remote", "get-url", "origin"], Some(&self.root)).is_err() {
            return Err(Error::Parse(
                "ci_recovery_remote_required remote=origin".to_string(),
            ));
        }
        let projection = load_tiber_projection(&self.root)?;
        Ok(projection.ci_recovery.as_ref().map(|_| {
            usize::from(
                projection
                    .versions
                    .get(&stream_id(CI_RECOVERY_STREAM).expect("valid stream"))
                    .copied()
                    .unwrap_or(StreamVersion::new(0)),
            )
            .to_string()
        }))
    }

    fn read_active_ci_recovery(
        &self,
        coordination_ref: Option<&str>,
    ) -> Result<Option<CiRecoveryState>, Error> {
        let Some(_coordination_ref) = coordination_ref else {
            return Ok(None);
        };
        Ok(load_tiber_projection(&self.root)?.ci_recovery)
    }

    fn push_ci_recovery_state(
        &self,
        state: &CiRecoveryState,
        parent: Option<&str>,
        message: &str,
    ) -> Result<(), Error> {
        let projection = load_tiber_projection(&self.root)?;
        let expected = projection
            .versions
            .get(&stream_id(CI_RECOVERY_STREAM)?)
            .copied()
            .unwrap_or(StreamVersion::new(0));
        if parent
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0)
            != usize::from(expected)
        {
            return Err(Error::Parse("ci_recovery_version_conflict=true".into()));
        }
        let mut events = Vec::new();
        if !projection.initialized {
            events.push(TiberEvent::RepositoryInitialized {
                stream_id: stream_id(REPOSITORY_STREAM)?,
            });
        }
        events.push(ci_recovery_event(
            message,
            stream_id(CI_RECOVERY_STREAM)?,
            state,
        )?);
        events.push(ci_recovery_state_published(
            stream_id(CI_RECOVERY_STREAM)?,
            state,
        )?);
        append_tiber_events(&self.root, &projection, events)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CiRecoveryTrigger {
    pub run_id: String,
    pub run_url: String,
    pub failed_sha: String,
    pub workflow: String,
    pub git_ref: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CiRecoveryParticipant {
    pub host: String,
    pub session: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CiRecoveryState {
    schema_version: u32,
    incident_id: String,
    state: String,
    epoch: u64,
    trigger: CiRecoveryTrigger,
    #[serde(default)]
    triggers: Vec<CiRecoveryTrigger>,
    owner: CiRecoveryParticipant,
    lease_expires_at: u64,
    #[serde(default)]
    participants: Vec<CiRecoveryParticipant>,
    #[serde(default)]
    assignments: Vec<CiRecoveryAssignment>,
    #[serde(default)]
    failure_record: Option<CiRecoveryFailureRecord>,
    #[serde(default)]
    diagnosis: Option<CiRecoveryDiagnosis>,
    #[serde(default)]
    next_action: Option<CiRecoveryAction>,
    #[serde(default)]
    replacement: Option<CiRecoveryReplacement>,
    #[serde(default)]
    release_proof: Option<CiRecoveryReleaseProof>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiRecoveryFailureRecord {
    pub job: String,
    pub step: String,
    pub log_evidence: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiRecoveryDiagnosis {
    pub cause: String,
    pub classification: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiRecoveryAction {
    pub kind: String,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiRecoveryReplacement {
    pub run_id: String,
    pub run_url: String,
    pub sha: String,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiRecoveryReleaseProof {
    pub replacement_run_id: String,
    pub replacement_run_url: String,
    pub sha: String,
    pub terminal_status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiRecoveryAssignment {
    pub id: String,
    pub owner_epoch: u64,
    pub assignee: CiRecoveryParticipant,
    pub capabilities: Vec<String>,
    pub scope: String,
    pub report: Option<CiRecoveryReport>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CiRecoveryReport {
    pub summary: String,
    pub evidence: String,
}

#[derive(Clone, Debug)]
pub struct CiRecoveryAssignmentInput {
    pub to_host: String,
    pub to_session: String,
    pub capabilities: String,
    pub scope: String,
}

#[derive(Clone, Debug)]
pub struct CiRecoveryDiagnosisInput {
    pub job: String,
    pub step: String,
    pub log_evidence: String,
    pub cause: String,
    pub classification: String,
}

#[derive(Clone, Debug)]
pub struct CiRecoveryReplacementInput {
    pub run_id: String,
    pub run_url: String,
    pub sha: String,
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct CiRecoveryReleaseInput {
    pub replacement_run_id: String,
    pub replacement_run_url: String,
    pub sha: String,
    pub terminal_status: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CiRecoveryRole {
    Owner,
    Waiting,
}

#[derive(Clone, Debug, Serialize)]
pub struct CiRecoveryClaim {
    pub incident_id: String,
    pub state: String,
    pub role: CiRecoveryRole,
    pub epoch: u64,
    pub lease_expires_at: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CiRecoveryAssertion {
    pub allowed: bool,
    pub incident_id: String,
    pub epoch: u64,
    pub lease_expires_at: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CiRecoveryTransfer {
    pub incident_id: String,
    pub epoch: u64,
    pub lease_expires_at: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CiRecoveryAssignmentResult {
    pub incident_id: String,
    pub assignment_id: String,
    pub epoch: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CiRecoveryStatus {
    pub schema_version: u32,
    pub incident_id: String,
    pub state: String,
    pub epoch: u64,
    pub lease_expires_at: u64,
    pub hold_released: bool,
    pub trigger_count: usize,
    pub trigger: CiRecoveryTrigger,
    pub triggers: Vec<CiRecoveryTrigger>,
    pub owner: CiRecoveryParticipant,
    pub participants: Vec<CiRecoveryParticipant>,
    pub assignments: Vec<CiRecoveryAssignment>,
    pub failure_record: Option<CiRecoveryFailureRecord>,
    pub diagnosis: Option<CiRecoveryDiagnosis>,
    pub next_action: Option<CiRecoveryAction>,
    pub replacement: Option<CiRecoveryReplacement>,
    pub release_proof: Option<CiRecoveryReleaseProof>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CiRecoveryWait {
    pub incident_id: String,
    pub state: String,
    pub epoch: u64,
    pub wake_reason: String,
    pub assignment_id: Option<String>,
}

impl CiRecoveryWait {
    fn timeout(incident_id: &str, epoch: u64) -> Self {
        Self {
            incident_id: incident_id.to_string(),
            state: "unknown".to_string(),
            epoch,
            wake_reason: "timeout".to_string(),
            assignment_id: None,
        }
    }

    fn from_state(
        state: &CiRecoveryState,
        wake_reason: &str,
        assignment_id: Option<String>,
    ) -> Self {
        Self {
            incident_id: state.incident_id.clone(),
            state: state.state.clone(),
            epoch: state.epoch,
            wake_reason: wake_reason.to_string(),
            assignment_id,
        }
    }
}

impl CiRecoveryStatus {
    fn from_state(state: &CiRecoveryState) -> Self {
        Self {
            schema_version: state.schema_version,
            incident_id: state.incident_id.clone(),
            state: state.state.clone(),
            epoch: state.epoch,
            lease_expires_at: state.lease_expires_at,
            hold_released: state.state == "resolved",
            trigger_count: if state.triggers.is_empty() {
                1
            } else {
                state.triggers.len()
            },
            trigger: state.trigger.clone(),
            triggers: state.triggers.clone(),
            owner: state.owner.clone(),
            participants: state.participants.clone(),
            assignments: state.assignments.clone(),
            failure_record: state.failure_record.clone(),
            diagnosis: state.diagnosis.clone(),
            next_action: state.next_action.clone(),
            replacement: state.replacement.clone(),
            release_proof: state.release_proof.clone(),
        }
    }
}

impl CiRecoveryClaim {
    fn from_state(state: CiRecoveryState, role: CiRecoveryRole) -> Self {
        Self {
            incident_id: state.incident_id,
            state: state.state,
            role,
            epoch: state.epoch,
            lease_expires_at: state.lease_expires_at,
        }
    }
}

fn ci_recovery_incident_id(run_id: &str) -> String {
    let run_id = run_id
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .collect::<String>();
    format!("ci-{}", if run_id.is_empty() { "run" } else { &run_id })
}

fn ci_recovery_signal_wait_ready() -> Result<(), Error> {
    #[cfg(debug_assertions)]
    if let Some(path) = std::env::var_os("TIBER_CI_RECOVERY_TEST_WAIT_READY") {
        std::fs::write(path, b"ready").map_err(Error::Io)?;
    }
    Ok(())
}

fn unix_timestamp() -> Result<u64, Error> {
    #[cfg(debug_assertions)]
    if let Ok(value) = std::env::var("TIBER_CI_RECOVERY_TEST_NOW") {
        return value.parse::<u64>().map_err(|error| {
            Error::Parse(format!(
                "ci_recovery_test_clock_invalid value={value} source={error}"
            ))
        });
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| Error::Parse(format!("system_clock_before_epoch source={error}")))
}

fn ci_recovery_participant() -> Result<CiRecoveryParticipant, Error> {
    let session = std::env::var("TIBER_CLAIM_SESSION")
        .or_else(|_| std::env::var("CODEX_SESSION_ID"))
        .or_else(|_| std::env::var("CLAUDE_SESSION_ID"))
        .or_else(|_| MCP_CI_RECOVERY_SESSION.with(|slot| slot.borrow().clone().ok_or(std::env::VarError::NotPresent)))
        .map_err(|_| {
            Error::Parse(
                "ci_recovery_session_required env=TIBER_CLAIM_SESSION|CODEX_SESSION_ID|CLAUDE_SESSION_ID"
                    .to_string(),
            )
        })?;
    let session = frontmatter_scalar_value(&session);
    if session == "unknown" {
        return Err(Error::Parse(
            "ci_recovery_session_required value=non-empty".to_string(),
        ));
    }
    Ok(CiRecoveryParticipant {
        host: claim_host(),
        session,
    })
}

fn ci_recovery_participant_from(host: &str, session: &str) -> Result<CiRecoveryParticipant, Error> {
    let host = frontmatter_scalar_value(host);
    let session = frontmatter_scalar_value(session);
    if session == "unknown" {
        return Err(Error::Parse(
            "ci_recovery_recipient_session_required value=non-empty".to_string(),
        ));
    }
    Ok(CiRecoveryParticipant { host, session })
}

fn ensure_ci_recovery_owner(
    state: &CiRecoveryState,
    incident_id: &str,
    epoch: u64,
    participant: &CiRecoveryParticipant,
) -> Result<(), Error> {
    ensure_ci_recovery_incident_epoch(state, incident_id, epoch)?;
    if &state.owner != participant {
        return Err(Error::Parse(format!(
            "ci_recovery_not_owner incident_id={} epoch={}",
            state.incident_id, state.epoch
        )));
    }
    Ok(())
}

fn ensure_ci_recovery_incident_epoch(
    state: &CiRecoveryState,
    incident_id: &str,
    epoch: u64,
) -> Result<(), Error> {
    if state.incident_id != incident_id {
        return Err(Error::Parse(format!(
            "ci_recovery_incident_mismatch expected={} actual={incident_id}",
            state.incident_id
        )));
    }
    if state.epoch != epoch {
        return Err(Error::Parse(format!(
            "ci_recovery_stale_epoch expected={} actual={epoch}",
            state.epoch
        )));
    }
    Ok(())
}

fn ensure_ci_recovery_lease_active(state: &CiRecoveryState, now: u64) -> Result<(), Error> {
    if state.lease_expires_at <= now {
        return Err(Error::Parse(format!(
            "ci_recovery_lease_expired incident_id={} epoch={}",
            state.incident_id, state.epoch
        )));
    }
    Ok(())
}

fn ensure_ci_recovery_not_resolved(state: &CiRecoveryState) -> Result<(), Error> {
    if state.state == "resolved" {
        return Err(Error::Parse(format!(
            "ci_recovery_incident_resolved incident_id={} mutation=false",
            state.incident_id
        )));
    }
    Ok(())
}

fn required_ci_recovery_text(field: &str, value: &str) -> Result<String, Error> {
    let value = value.trim();
    let lower = value.to_ascii_lowercase();
    let likely_credential = [
        "ghp_",
        "github_pat_",
        "authorization:",
        "bearer ",
        "password=",
        "token=",
        "secret=",
        "-----begin private key",
        "-----begin rsa private key",
        "-----begin openssh private key",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    if value.is_empty()
        || value.len() > CI_RECOVERY_TEXT_MAX_BYTES
        || value.chars().any(char::is_control)
        || likely_credential
    {
        return Err(Error::Parse(format!(
            "ci_recovery_field_invalid field={field}"
        )));
    }
    Ok(value.to_string())
}

fn parse_ci_recovery_choice(field: &str, value: &str, allowed: &[&str]) -> Result<String, Error> {
    let value = value.trim();
    if allowed.contains(&value) {
        Ok(value.to_string())
    } else {
        Err(Error::Parse(format!(
            "ci_recovery_choice_invalid field={field} value={value} allowed={}",
            allowed.join(",")
        )))
    }
}

pub fn sync_repository() -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.sync_repository()?;
    clear_workflow_blocker(&repo, "publication_failed")
}

/// Persist a fail-closed operational hold when the authoritative Git
/// publication boundary cannot be confirmed. The Git-backed EventStore calls
/// this before returning its publication error.
pub fn record_publication_failure() -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    record_publication_failure_for(&repo)
}

pub(crate) fn record_publication_failure_at(repository: &Path) -> Result<(), Error> {
    record_publication_failure_for(&GitRepository::at(repository))
}

pub(crate) fn clear_publication_failure_at(repository: &Path) -> Result<(), Error> {
    clear_workflow_blocker(&GitRepository::at(repository), "publication_failed")
}

fn record_publication_failure_for(repo: &GitRepository) -> Result<(), Error> {
    record_workflow_blocker(
        repo,
        WorkflowBlocker {
            schema_version: 1,
            kind: "publication_failed".to_string(),
            error_code: "tiber.publication_failed".to_string(),
            required_action: "run Tiber sync until authoritative publication is resolved"
                .to_string(),
            created_at: unix_timestamp()?,
        },
    )
}

pub fn create_task(title: &str) -> Result<TaskPath, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(TaskMutation::Create, |repo| {
        repo.create_task(TaskTitle::parse(title)?)
    })
}

pub fn list_tasks() -> Result<Vec<TaskSummary>, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_snapshot_workspace(|repo| repo.list_tasks())
}

pub fn list_tasks_by_status(status: &str) -> Result<Vec<TaskSummary>, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_snapshot_workspace(|repo| repo.list_tasks_by_status(status))
}

pub fn search_tasks(query: &str) -> Result<Vec<TaskSearchResult>, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_snapshot_workspace(|repo| repo.search_tasks(query))
}

pub fn show_task(task_ref: &str) -> Result<String, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_snapshot_workspace(|repo| repo.show_task(task_ref))
}

pub fn task_metadata(task_ref: &str) -> Result<TaskMetadata, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_snapshot_workspace(|repo| repo.task_metadata(task_ref))
}

pub fn list_docs() -> Result<Vec<String>, Error> {
    let repo = GitRepository::discover()?;
    repo.list_docs()
}

pub fn read_doc(doc_ref: &str) -> Result<String, Error> {
    let repo = GitRepository::discover()?;
    repo.read_doc(doc_ref)
}

pub fn next_task() -> Result<Option<TaskSummary>, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_snapshot_workspace(|repo| repo.next_task())
}

pub fn transition_task(task_ref: &str, status: &str) -> Result<TaskPath, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(TaskMutation::Transition, |repo| {
        repo.transition_task(task_ref, status)
    })
}

pub fn prioritize_before(task_ref: &str, before_ref: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(TaskMutation::Prioritize, |repo| {
        repo.prioritize_before(task_ref, before_ref)
    })
}

pub fn link_blocks(from_ref: &str, to_ref: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(TaskMutation::Dependencies, |repo| {
        repo.link_blocks(from_ref, to_ref)
    })
}

pub fn unlink_blocks(from_ref: &str, to_ref: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(TaskMutation::Dependencies, |repo| {
        repo.unlink_blocks(from_ref, to_ref)
    })
}

pub fn add_subtask(task_ref: &str, title: &str, after_refs: &[String]) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(TaskMutation::AddSubtask, |repo| {
        repo.add_subtask(task_ref, title, after_refs)
    })
}

pub fn set_subtask_checked(task_ref: &str, index: &str, checked: bool) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(TaskMutation::CheckSubtask, |repo| {
        repo.set_subtask_checked(task_ref, index, checked)
    })
}

pub fn update_task(task_ref: &str, update: TaskUpdate<'_>) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    let details = update.title.is_some()
        || update.summary.is_some()
        || update.context.is_some()
        || update.tags.is_some();
    let pull_request = update.pr_mr_url.is_some() || update.pr_mr_status.is_some();
    let mutation = match (details, pull_request) {
        (true, true) => TaskMutation::UpdateDetailsAndPullRequest,
        (false, true) => TaskMutation::UpdatePullRequest,
        _ => TaskMutation::UpdateDetails,
    };
    repo.with_task_workspace(mutation, |repo| repo.update_task(task_ref, update.clone()))
}

pub fn add_acceptance(task_ref: &str, criterion: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(TaskMutation::AddAcceptance, |repo| {
        repo.add_acceptance(task_ref, criterion)
    })
}

pub fn set_acceptance_checked(task_ref: &str, index: &str, checked: bool) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(TaskMutation::CheckAcceptance, |repo| {
        repo.set_acceptance_checked(task_ref, index, checked)
    })
}

pub fn remove_acceptance(task_ref: &str, index: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(TaskMutation::RemoveAcceptance, |repo| {
        repo.remove_acceptance(task_ref, index)
    })
}

pub fn add_note(task_ref: &str, note: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    let date = current_date_string();
    repo.with_task_workspace(TaskMutation::AddNote, |repo| {
        repo.add_note_at(task_ref, note, &date)
    })
}

pub fn validate_fix() -> Result<Vec<ValidationMessage>, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(TaskMutation::ValidateRepair, |repo| repo.validate_fix())
}

pub fn close_from_trailers() -> Result<Vec<String>, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(TaskMutation::CloseFromTrailer, |repo| {
        repo.close_from_trailers()
    })
}

pub fn scaffold_repo(apply: bool, replace_conflicts: bool) -> Result<Vec<String>, Error> {
    let repo = GitRepository::discover()?;
    repo.scaffold_repo(apply, replace_conflicts)
}

pub fn install_bin(target_dir: &str, apply: bool) -> Result<String, Error> {
    let target_dir = expand_home(Path::new(target_dir))?;
    let launcher = tiber_launcher_path()?;
    let installed = target_dir.join("tiber");
    if apply {
        fs::create_dir_all(&target_dir)?;
        if installed.exists() || installed.symlink_metadata().is_ok() {
            return Err(Error::Parse(format!(
                "install_target_exists path={}",
                path_to_entry(&installed)?
            )));
        }
        install_launcher(&launcher, &installed)?;
    }
    Ok(format!("{} -> {}", installed.display(), launcher.display()))
}

#[derive(Debug, Eq, PartialEq)]
pub struct TaskPath {
    pub path: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct TaskSummary {
    pub path: String,
    pub title: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TaskSearchResult {
    pub id: String,
    pub status: String,
    pub title: String,
    pub summary: String,
    pub context: String,
}

impl From<&TaskSnapshot> for TaskSummary {
    fn from(snapshot: &TaskSnapshot) -> Self {
        Self {
            path: snapshot.path().to_string(),
            title: snapshot.title().to_string(),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct TaskMetadata {
    pub path: String,
    pub title: String,
    pub committed_at: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct TaskDocument {
    pub stem: String,
    pub status: String,
    pub rank: Option<usize>,
    pub contents: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ValidationMessage(String);

impl fmt::Display for ValidationMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct TaskUpdate<'a> {
    pub title: Option<&'a str>,
    pub summary: Option<&'a str>,
    pub context: Option<&'a str>,
    pub tags: Option<Vec<String>>,
    pub pr_mr_url: Option<&'a str>,
    pub pr_mr_status: Option<&'a str>,
}

#[derive(Debug)]
pub enum Error {
    CommandFailed {
        program: String,
        args: Vec<String>,
        status: String,
        stderr: String,
    },
    BacklogCapacityExceeded {
        queued: usize,
        max_queued: usize,
    },
    Io(std::io::Error),
    Parse(String),
    Core(tiber_core::CoreError),
    Usage(String),
    WorkflowBlocked {
        code: &'static str,
        required_action: &'static str,
        source: Box<Error>,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandFailed {
                program,
                args,
                status,
                stderr,
            } => write!(
                formatter,
                "tiber.command_failed program={program} args={} status={status} stderr={}",
                args.join(" "),
                stderr.trim()
            ),
            Self::BacklogCapacityExceeded {
                queued,
                max_queued,
            } => write!(
                formatter,
                "tiber.backlog_capacity_exceeded queued={queued} max_queued={max_queued} action=\"replace a lower-value queued ticket, combine genuinely overlapping tickets, or reject the candidate\""
            ),
            Self::Io(error) => write!(formatter, "tiber.io_error source={error}"),
            Self::Parse(message) => write!(formatter, "tiber.parse_error {message}"),
            Self::Core(error) => write!(formatter, "{error}"),
            Self::Usage(message) => write!(formatter, "{message}"),
            Self::WorkflowBlocked {
                code,
                required_action,
                source,
            } => write!(
                formatter,
                "{code} workflow_blocked=true required_action=\"{required_action}\" prohibited_actions=\"diagnose,edit,test,rerun,push,unrelated-work\" source={}",
                source.sanitized_workflow_blocker_source()
            ),
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    pub fn workflow_blocker_data(&self) -> Option<WorkflowBlockerData> {
        match self {
            Self::WorkflowBlocked {
                code,
                required_action,
                ..
            } => Some(WorkflowBlockerData {
                error_code: code,
                required_action,
            }),
            _ => None,
        }
    }
    fn sanitized_workflow_blocker_source(&self) -> String {
        match self {
            Self::Parse(message) => format!("tiber.parse_error {message}"),
            _ => self.sanitized_sync_source(),
        }
    }

    fn sanitized_sync_source(&self) -> String {
        match self {
            Self::CommandFailed {
                program,
                status,
                stderr,
                ..
            } => format!(
                "tiber.command_failed program={program} args_redacted=true status={status} stderr_redacted={}",
                !stderr.trim().is_empty()
            ),
            Self::BacklogCapacityExceeded {
                queued,
                max_queued,
            } => format!(
                "tiber.backlog_capacity_exceeded queued={queued} max_queued={max_queued} action=\"replace a lower-value queued ticket, combine genuinely overlapping tickets, or reject the candidate\""
            ),
            Self::Io(_) => "tiber.io_error source_redacted=true".to_string(),
            Self::Parse(message) if message.starts_with("sync_conflict ") => {
                format!("tiber.parse_error {message}")
            }
            Self::Parse(_) => "tiber.parse_error source_redacted=true".to_string(),
            Self::Core(error) => error.to_string(),
            Self::Usage(message) => message.to_string(),
            Self::WorkflowBlocked { code, .. } => format!("{code} workflow_blocked=true"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<tiber_core::CoreError> for Error {
    fn from(error: tiber_core::CoreError) -> Self {
        Self::Core(error)
    }
}

struct GitRepository {
    root: PathBuf,
    task_projection: Option<Rc<RefCell<TiberProjection>>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectConfig {
    #[serde(default)]
    backlog: BacklogConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct BacklogConfig {
    max_queued: Option<usize>,
}

impl GitRepository {
    fn at(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            task_projection: None,
        }
    }

    fn discover() -> Result<Self, Error> {
        if let Ok(root) = git_output(["rev-parse", "--show-toplevel"], None) {
            return Ok(Self::at(PathBuf::from(root.trim())));
        }

        let git_dir = git_output(["rev-parse", "--absolute-git-dir"], None).map_err(|_| {
                Error::Usage(
                    "tiber.repository_not_found action=\"run from a repository checkout or configure the integration with an explicit repository root\""
                        .to_string(),
                )
            })?;
        if let Ok(root) = git_output(["config", "--path", "--get", "core.worktree"], None) {
            let root = PathBuf::from(root.trim());
            let root = if root.is_absolute() {
                root
            } else {
                PathBuf::from(git_dir.trim()).join(root)
            };
            return Ok(Self::at(root));
        }

        Err(Error::Usage(
            "tiber.repository_root_unresolved action=\"run from a repository checkout or configure the integration with an explicit repository root\""
                .to_string(),
        ))
    }

    fn with_task_projection(&self, projection: Rc<RefCell<TiberProjection>>) -> Self {
        Self {
            root: self.root.clone(),
            task_projection: Some(projection),
        }
    }

    fn with_task_workspace<T>(
        &self,
        mutation: TaskMutation,
        mut operation: impl FnMut(&GitRepository) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let _lock = self.acquire_lock()?;
        COMMAND_TASK_IDS.with(|ids| *ids.borrow_mut() = Some((Vec::new(), 0)));
        let outcome = (|| {
            for attempt in 1..=MAX_SYNC_ATTEMPTS {
                COMMAND_TASK_IDS.with(|ids| {
                    if let Some((_, cursor)) = ids.borrow_mut().as_mut() {
                        *cursor = 0;
                    }
                });
                let projection = load_tiber_projection(&self.root)?;
                let working = Rc::new(RefCell::new(projection.clone()));
                let repo = self.with_task_projection(Rc::clone(&working));
                let result = operation(&repo)?;
                let after = working.borrow();
                let events = task_change_events(&projection, &after, mutation)?;
                match append_tiber_events(&self.root, &projection, events) {
                    Ok(()) => return Ok(result),
                    Err(Error::Parse(message))
                        if (message.starts_with("event_version_conflict=")
                            || message.contains("event_store_authoritative_ref_retry=true"))
                            && attempt < MAX_SYNC_ATTEMPTS =>
                    {
                        continue
                    }
                    Err(error) => return Err(error),
                }
            }
            unreachable!("task command retry loop returns")
        })();
        COMMAND_TASK_IDS.with(|ids| *ids.borrow_mut() = None);
        outcome
    }

    fn with_task_snapshot_workspace<T>(
        &self,
        operation: impl FnOnce(&GitRepository) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let projection = Rc::new(RefCell::new(load_tiber_projection(&self.root)?));
        let repo = self.with_task_projection(projection);
        operation(&repo)
    }

    fn current_branch(&self) -> Result<String, Error> {
        let branch = git_output(["branch", "--show-current"], Some(&self.root))?;
        let branch = branch.trim();
        if branch.is_empty() {
            return Err(Error::Parse("detached_head=true".to_string()));
        }
        Ok(branch.to_string())
    }

    fn commit_signing_enabled(&self) -> Result<bool, Error> {
        match self.git(["config", "--bool", "commit.gpgsign"]) {
            Ok(value) => Ok(value.trim() == "true"),
            Err(Error::CommandFailed { .. }) => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn sync_repository(&self) -> Result<(), Error> {
        let store = GitEventStore::open(&self.root).map_err(event_store_error)?;
        let outcome: SynchronizeOutcome =
            run_async(async move { store.synchronize().await }).map_err(event_store_error)?;
        match outcome {
            SynchronizeOutcome::Current | SynchronizeOutcome::PublishedPending => Ok(()),
            SynchronizeOutcome::DiscardedUnpublished => Err(Error::Parse(
                "event_transaction_discarded reissue_required=true workflow_blocked=true"
                    .to_string(),
            )),
        }
    }

    fn sync_repository_unlocked(&self) -> Result<(), Error> {
        Ok(())
    }

    fn sync_repository_with_admission_unlocked(
        &self,
        admits_to_backlog: bool,
    ) -> Result<(), Error> {
        if admits_to_backlog {
            self.ensure_backlog_not_over_capacity()?;
        }
        Ok(())
    }

    fn create_task(&self, title: TaskTitle) -> Result<TaskPath, Error> {
        let task_path = self.create_task_unlocked(title)?;
        self.sync_repository_with_admission_unlocked(true)?;
        Ok(task_path)
    }

    fn create_task_unlocked(&self, title: TaskTitle) -> Result<TaskPath, Error> {
        self.ensure_backlog_capacity()?;
        let nickname = self.unique_nickname(&title.file_stem())?;
        let id = new_task_id();
        let stem = format!("{id}-{nickname}");
        let projection = self.task_projection()?;
        let mut projection = projection.borrow_mut();
        projection.tasks.insert(
            stem.clone(),
            Task::new(
                stem.clone(),
                title.as_str().to_string(),
                command_recorded_at(),
            ),
        );
        projection.order.push(stem.clone());

        Ok(TaskPath { path: stem })
    }

    fn ensure_backlog_capacity(&self) -> Result<(), Error> {
        let Some(max_queued) = self.project_config()?.backlog.max_queued else {
            return Ok(());
        };
        let queued = self.backlog_count()?;
        if queued >= max_queued {
            return Err(Error::BacklogCapacityExceeded { queued, max_queued });
        }
        Ok(())
    }

    fn ensure_backlog_not_over_capacity(&self) -> Result<(), Error> {
        let Some(max_queued) = self.project_config()?.backlog.max_queued else {
            return Ok(());
        };
        let queued = self.backlog_count()?;
        if queued > max_queued {
            return Err(Error::BacklogCapacityExceeded { queued, max_queued });
        }
        Ok(())
    }

    fn backlog_count(&self) -> Result<usize, Error> {
        Ok(self
            .task_projection()?
            .borrow()
            .tasks
            .values()
            .filter(|task| task.status == "backlog")
            .count())
    }

    fn project_config(&self) -> Result<ProjectConfig, Error> {
        let path = self.root.join(CONFIG_FILE);
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ProjectConfig::default());
            }
            Err(error) => return Err(error.into()),
        };
        toml::from_str(&contents).map_err(|error| {
            Error::Parse(format!("config_invalid file={CONFIG_FILE} source={error}"))
        })
    }

    fn unique_nickname(&self, base: &str) -> Result<String, Error> {
        let mut nickname = base.to_string();
        let mut suffix = 2;
        while self.nickname_exists(&nickname)? {
            nickname = format!("{base}-{suffix}");
            suffix += 1;
        }
        Ok(nickname)
    }

    fn nickname_exists(&self, nickname: &str) -> Result<bool, Error> {
        Ok(self
            .task_projection()?
            .borrow()
            .tasks
            .keys()
            .any(|stem| stem.ends_with(&format!("-{nickname}"))))
    }

    fn list_tasks(&self) -> Result<Vec<TaskSummary>, Error> {
        Ok(self
            .board_snapshot()?
            .ordered_tasks()
            .iter()
            .map(TaskSummary::from)
            .collect())
    }

    fn list_tasks_by_status(&self, status: &str) -> Result<Vec<TaskSummary>, Error> {
        let status = parse_status(status)?;
        let projection = self.task_projection()?;
        let tasks = projection
            .borrow()
            .tasks
            .values()
            .filter(|task| task.status == status)
            .map(|task| TaskSummary {
                path: task.stem.clone(),
                title: task.title.clone(),
            })
            .collect();
        Ok(tasks)
    }

    fn search_tasks(&self, query: &str) -> Result<Vec<TaskSearchResult>, Error> {
        let query = query.to_lowercase();
        let projection = self.task_projection()?;
        let mut results = Vec::new();
        for task in projection.borrow().tasks.values() {
            if [
                task.title.as_str(),
                task.summary.as_str(),
                task.context.as_str(),
            ]
            .iter()
            .any(|field| field.to_lowercase().contains(&query))
            {
                results.push(TaskSearchResult {
                    id: task.stem.clone(),
                    status: task.status.clone(),
                    title: task.title.clone(),
                    summary: task.summary.clone(),
                    context: task.context.clone(),
                });
            }
        }
        results.sort_by(|left, right| {
            (left.status.as_str(), left.id.as_str())
                .cmp(&(right.status.as_str(), right.id.as_str()))
        });
        Ok(results)
    }

    fn board_snapshot(&self) -> Result<BoardSnapshot, Error> {
        let projection = self.task_projection()?;
        let projection = projection.borrow();
        let ordered_tasks = projection
            .order
            .iter()
            .filter_map(|stem| {
                projection
                    .tasks
                    .get(stem)
                    .map(|task| TaskSnapshot::new(stem, &task.title))
            })
            .collect();
        Ok(BoardSnapshot::from_ordered_tasks(ordered_tasks))
    }

    fn show_task(&self, task_ref: &str) -> Result<String, Error> {
        let stem = self.resolve_task_stem(task_ref)?;
        Ok(self
            .task_projection()?
            .borrow()
            .tasks
            .get(&stem)
            .expect("resolved task")
            .render_markdown())
    }

    fn task_metadata(&self, task_ref: &str) -> Result<TaskMetadata, Error> {
        let stem = self.resolve_task_stem(task_ref)?;
        let projection = self.task_projection()?;
        let projection = projection.borrow();
        let task = projection.tasks.get(&stem).expect("resolved task");
        Ok(TaskMetadata {
            path: stem,
            title: task.title.clone(),
            committed_at: Some(task.committed_at.clone()),
        })
    }

    fn task_documents_snapshot(&self) -> Result<Vec<TaskDocument>, Error> {
        let projection = self.task_projection()?;
        let projection = projection.borrow();
        let ranks = projection
            .order
            .iter()
            .enumerate()
            .map(|(index, stem)| (stem.clone(), index + 1))
            .collect::<std::collections::BTreeMap<_, _>>();
        Ok(projection
            .tasks
            .values()
            .map(|task| TaskDocument {
                rank: ranks.get(&task.stem).copied(),
                stem: task.stem.clone(),
                status: task.status.clone(),
                contents: task.render_markdown(),
            })
            .collect())
    }

    fn list_docs(&self) -> Result<Vec<String>, Error> {
        let docs_dir = self.root.join("docs");
        let mut docs = Vec::new();
        if docs_dir.exists() {
            collect_docs(&docs_dir, &docs_dir, &mut docs)?;
        }
        docs.sort();
        Ok(docs.into_iter().map(|doc| format!("docs/{doc}")).collect())
    }

    fn read_doc(&self, doc_ref: &str) -> Result<String, Error> {
        let doc_ref = parse_doc_ref(doc_ref)?;
        fs::read_to_string(self.root.join(doc_ref)).map_err(Error::Io)
    }

    fn next_task(&self) -> Result<Option<TaskSummary>, Error> {
        let projection = self.task_projection()?;
        let projection = projection.borrow();
        for stem in &projection.order {
            let Some(task) = projection.tasks.get(stem) else {
                continue;
            };
            if task.blocked_by.iter().all(|blocker| {
                projection
                    .tasks
                    .get(blocker)
                    .is_some_and(|item| item.status == "done")
            }) {
                return Ok(Some(TaskSummary {
                    path: stem.clone(),
                    title: task.title.clone(),
                }));
            }
        }
        Ok(None)
    }

    fn transition_task(&self, task_ref: &str, status: &str) -> Result<TaskPath, Error> {
        let (task_path, admits_to_backlog) = self.transition_task_unlocked(task_ref, status)?;
        self.sync_repository_with_admission_unlocked(admits_to_backlog)?;
        Ok(task_path)
    }

    fn transition_task_unlocked(
        &self,
        task_ref: &str,
        status: &str,
    ) -> Result<(TaskPath, bool), Error> {
        let stem = self.resolve_task_stem(task_ref)?;
        let status = parse_status(status)?;
        let projection = self.task_projection()?;
        let old_status = projection
            .borrow()
            .tasks
            .get(&stem)
            .expect("resolved task")
            .status
            .clone();
        let admits_to_backlog = status == "backlog" && old_status != "backlog";
        if admits_to_backlog {
            self.ensure_backlog_capacity()?;
        }
        let mut projection = projection.borrow_mut();
        let task = projection.tasks.get_mut(&stem).expect("resolved task");
        task.status = status.to_string();
        task.claim = (status == "in-progress").then(|| Claim {
            host: claim_host(),
            session: claim_session(),
        });
        if is_open_status(status) {
            if !projection.order.contains(&stem) {
                projection.order.push(stem.clone());
            }
        } else {
            projection.order.retain(|entry| entry != &stem);
        }
        Ok((TaskPath { path: stem }, admits_to_backlog))
    }

    fn prioritize_before(&self, task_ref: &str, before_ref: &str) -> Result<(), Error> {
        self.prioritize_before_unlocked(task_ref, before_ref)?;
        self.sync_repository_unlocked()
    }

    fn prioritize_before_unlocked(&self, task_ref: &str, before_ref: &str) -> Result<(), Error> {
        let task_ref = self.resolve_task_stem(task_ref)?;
        let before_ref = self.resolve_task_stem(before_ref)?;
        let projection = self.task_projection()?;
        let mut projection = projection.borrow_mut();
        let mut order = projection
            .order
            .iter()
            .filter(|&entry| entry != &task_ref)
            .cloned()
            .collect::<Vec<_>>();
        let before_index = order
            .iter()
            .position(|entry| entry == &before_ref)
            .ok_or_else(|| Error::Parse(format!("task_ref_missing ref={before_ref}")))?;
        order.insert(before_index, task_ref);
        projection.order = order;
        Ok(())
    }

    fn link_blocks(&self, from_ref: &str, to_ref: &str) -> Result<(), Error> {
        self.link_blocks_unlocked(from_ref, to_ref)?;
        self.sync_repository_unlocked()
    }

    fn link_blocks_unlocked(&self, from_ref: &str, to_ref: &str) -> Result<(), Error> {
        let from_ref = self.resolve_task_stem(from_ref)?;
        let to_ref = self.resolve_task_stem(to_ref)?;
        let projection = self.task_projection()?;
        let mut projection = projection.borrow_mut();
        add_unique(
            &mut projection
                .tasks
                .get_mut(&from_ref)
                .expect("resolved task")
                .blocks,
            &to_ref,
        );
        add_unique(
            &mut projection
                .tasks
                .get_mut(&to_ref)
                .expect("resolved task")
                .blocked_by,
            &from_ref,
        );
        Ok(())
    }

    fn unlink_blocks(&self, from_ref: &str, to_ref: &str) -> Result<(), Error> {
        self.unlink_blocks_unlocked(from_ref, to_ref)?;
        self.sync_repository_unlocked()
    }

    fn unlink_blocks_unlocked(&self, from_ref: &str, to_ref: &str) -> Result<(), Error> {
        let from_ref = self.resolve_task_stem(from_ref)?;
        let to_ref = self.resolve_task_stem(to_ref)?;
        let projection = self.task_projection()?;
        let mut projection = projection.borrow_mut();
        projection
            .tasks
            .get_mut(&from_ref)
            .expect("resolved task")
            .blocks
            .retain(|item| item != &to_ref);
        projection
            .tasks
            .get_mut(&to_ref)
            .expect("resolved task")
            .blocked_by
            .retain(|item| item != &from_ref);
        Ok(())
    }

    fn add_subtask(&self, task_ref: &str, title: &str, after_refs: &[String]) -> Result<(), Error> {
        self.add_subtask_unlocked(task_ref, title, after_refs)?;
        self.sync_repository_unlocked()
    }

    fn add_subtask_unlocked(
        &self,
        task_ref: &str,
        title: &str,
        after_refs: &[String],
    ) -> Result<(), Error> {
        let title = title.trim();
        if title.is_empty() {
            return Err(Error::Parse("subtask_title_empty=true".to_string()));
        }
        if title.chars().any(char::is_control) {
            return Err(Error::Parse("subtask_title_invalid=true".to_string()));
        }
        let after_refs = after_refs
            .iter()
            .map(|after_ref| parse_subtask_ref(after_ref))
            .collect::<Result<Vec<_>, Error>>()?;
        let stem = self.resolve_task_stem(task_ref)?;
        let projection = self.task_projection()?;
        let mut projection = projection.borrow_mut();
        let task = projection.tasks.get_mut(&stem).expect("resolved task");
        let subtask_id = task
            .subtasks
            .iter()
            .filter_map(|item| item.id.strip_prefix('s')?.parse::<usize>().ok())
            .max()
            .unwrap_or(0)
            + 1;
        task.subtasks.push(Subtask {
            id: format!("s{subtask_id}"),
            checked: false,
            title: title.to_string(),
            after: after_refs,
        });
        Ok(())
    }

    fn set_subtask_checked(&self, task_ref: &str, index: &str, checked: bool) -> Result<(), Error> {
        self.set_subtask_checked_unlocked(task_ref, index, checked)?;
        self.sync_repository_unlocked()
    }

    fn set_subtask_checked_unlocked(
        &self,
        task_ref: &str,
        index: &str,
        checked: bool,
    ) -> Result<(), Error> {
        let task_ref = self.resolve_task_stem(task_ref)?;
        let subtask_ref = parse_subtask_ref(index)?;
        let projection = self.task_projection()?;
        let mut projection = projection.borrow_mut();
        let item = projection
            .tasks
            .get_mut(&task_ref)
            .expect("resolved task")
            .subtasks
            .iter_mut()
            .find(|item| item.id == subtask_ref)
            .ok_or_else(|| Error::Parse(format!("subtask_ref_missing ref={subtask_ref}")))?;
        item.checked = checked;
        Ok(())
    }

    fn update_task(&self, task_ref: &str, update: TaskUpdate<'_>) -> Result<(), Error> {
        let task_ref = self.resolve_task_stem(task_ref)?;
        let projection = self.task_projection()?;
        let mut projection = projection.borrow_mut();
        let task = projection.tasks.get_mut(&task_ref).expect("resolved task");
        if let Some(title) = update.title {
            let title = TaskTitle::parse(title)?;
            task.title = title.as_str().to_string();
        }
        if let Some(tags) = update.tags {
            task.tags = tags.to_vec();
        }
        if let Some(pr_mr_url) = update.pr_mr_url {
            task.pr_mr_url = nonempty_option(pr_mr_url);
        }
        if let Some(pr_mr_status) = update.pr_mr_status {
            task.pr_mr_status = nonempty_option(pr_mr_status);
        }
        if let Some(summary) = update.summary {
            task.summary = parse_task_section_body(summary)?;
        }
        if let Some(context) = update.context {
            task.context = parse_task_section_body(context)?;
        }
        self.sync_repository_unlocked()
    }

    fn add_acceptance(&self, task_ref: &str, criterion: &str) -> Result<(), Error> {
        let criterion = parse_nonempty_text(criterion, "acceptance")?;
        let task_ref = self.resolve_task_stem(task_ref)?;
        self.task_projection()?
            .borrow_mut()
            .tasks
            .get_mut(&task_ref)
            .expect("resolved task")
            .acceptance
            .push(ChecklistItem {
                checked: false,
                text: criterion.to_string(),
            });
        self.sync_repository_unlocked()
    }

    fn set_acceptance_checked(
        &self,
        task_ref: &str,
        index: &str,
        checked: bool,
    ) -> Result<(), Error> {
        let index = parse_one_based_usize(index, "acceptance")? - 1;
        let task_ref = self.resolve_task_stem(task_ref)?;
        let projection = self.task_projection()?;
        let mut projection = projection.borrow_mut();
        let item = projection
            .tasks
            .get_mut(&task_ref)
            .expect("resolved task")
            .acceptance
            .get_mut(index)
            .ok_or_else(|| Error::Parse(format!("acceptance_index_missing index={}", index + 1)))?;
        item.checked = checked;
        self.sync_repository_unlocked()
    }

    fn remove_acceptance(&self, task_ref: &str, index: &str) -> Result<(), Error> {
        let index = parse_one_based_usize(index, "acceptance")? - 1;
        let task_ref = self.resolve_task_stem(task_ref)?;
        let projection = self.task_projection()?;
        let mut projection = projection.borrow_mut();
        let items = &mut projection
            .tasks
            .get_mut(&task_ref)
            .expect("resolved task")
            .acceptance;
        if index >= items.len() {
            return Err(Error::Parse(format!(
                "acceptance_index_missing index={}",
                index + 1
            )));
        }
        items.remove(index);
        self.sync_repository_unlocked()
    }

    fn add_note_at(&self, task_ref: &str, note: &str, date: &str) -> Result<(), Error> {
        let note = parse_nonempty_text(note, "note")?;
        let task_ref = self.resolve_task_stem(task_ref)?;
        self.task_projection()?
            .borrow_mut()
            .tasks
            .get_mut(&task_ref)
            .expect("resolved task")
            .notes
            .push(Note {
                date: date.to_string(),
                text: note.to_string(),
            });
        self.sync_repository_unlocked()
    }

    fn validate_fix(&self) -> Result<Vec<ValidationMessage>, Error> {
        let messages = self.validate_fix_unlocked()?;
        self.sync_repository_unlocked()?;
        Ok(messages)
    }

    fn validate_fix_unlocked(&self) -> Result<Vec<ValidationMessage>, Error> {
        let mut messages = Vec::new();
        let projection = self.task_projection()?;
        let mut projection = projection.borrow_mut();
        repair_typed_links(&mut projection.tasks, &mut messages);
        report_typed_cycles(&projection.tasks, &mut messages);
        let open = projection
            .tasks
            .values()
            .filter(|task| is_open_status(&task.status))
            .map(|task| task.stem.clone())
            .collect::<Vec<_>>();
        let reconciliation = OrderReconciliation::reconcile(projection.order.clone(), open);
        messages.extend(
            reconciliation
                .messages()
                .iter()
                .cloned()
                .map(ValidationMessage),
        );
        projection.order = reconciliation.entries().to_vec();
        Ok(messages)
    }

    fn close_from_trailers(&self) -> Result<Vec<String>, Error> {
        let log = self.git(["log", "-1", "--format=%B"])?;
        let requested = closes_trailers(&log);
        if requested.is_empty() {
            return Ok(Vec::new());
        }
        self.sync_repository_unlocked()?;
        let mut closed = Vec::new();
        for task_ref in requested {
            let resolved = self.resolve_task_stem(&task_ref)?;
            let (done, _) = self.transition_task_unlocked(&resolved, "done")?;
            closed.push(done.path);
        }
        closed.sort();
        closed.dedup();
        self.sync_repository_unlocked()?;
        Ok(closed)
    }

    fn scaffold_repo(&self, apply: bool, replace_conflicts: bool) -> Result<Vec<String>, Error> {
        let _lock = if apply {
            Some(self.acquire_lock()?)
        } else {
            None
        };
        let mut files = Vec::new();
        let mut integration_conflicts = Vec::new();
        let mut integration_messages = Vec::new();
        let equivalent_hook = self.equivalent_task_closing_hook()?;
        if equivalent_hook.is_none() {
            let active_hook = self.active_post_commit_hook()?;
            if self.hook_dispatches_tiber_snippet(&active_hook)? {
                files.push((
                    ".githooks/post-commit.tiber",
                    "#!/usr/bin/env bash\nset -euo pipefail\n\ntiber close-from-trailers\n"
                        .to_string(),
                    true,
                ));
            } else if self.explicit_hooks_path()?.is_some() {
                integration_conflicts.push(format!(
                    "hook-dispatch active={} resolution=the active hook must invoke .githooks/post-commit.tiber",
                    active_hook.display()
                ));
            } else {
                integration_messages.push(format!(
                    "skipped hook-dispatch active={} resolution=configure an active post-commit dispatcher before adding .githooks/post-commit.tiber",
                    active_hook.display()
                ));
            }
        }
        let equivalent_workflow = self.equivalent_task_closing_workflow()?;
        if equivalent_workflow.is_none() {
            if self.commit_signing_enabled()? {
                integration_conflicts.push(
                    "signed-publication generated GitHub workflow cannot access a signing key resolution=provide repository-owned signed tasks-branch automation and rerun scaffold"
                        .to_string(),
                );
            } else {
                let publication_branch = yaml_single_quoted(&self.publication_branch()?);
                files.push((
                    ".github/workflows/tiber-close-from-trailers.yml",
                    format!(
                        "name: tiber close from trailers\n\non:\n  push:\n    branches: [{publication_branch}]\n\npermissions:\n  contents: write\n\njobs:\n  close:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683\n      - name: Install Tiber\n        run: |\n          git clone --no-checkout https://github.com/jwilger/ai-plugins.git .tiber-src\n          git -C .tiber-src checkout bce89f58a2ea23e38bf508cb3800d17efba3e28e\n          cargo install --locked --path .tiber-src/plugins/development-system/components/tiber/rust/crates/tiber-cli --bin tiber --root .tiber-install\n          echo \"$PWD/.tiber-install/bin\" >> \"$GITHUB_PATH\"\n      - run: tiber close-from-trailers\n"
                    ),
                    true,
                ));
            }
        }
        let justfile_exists = self.root.join("justfile").exists();
        let planned_justfile = self.show_tasks_justfile()?;
        if let Some(justfile) = planned_justfile.as_ref() {
            files.push(("justfile", justfile.clone(), false));
        }
        let mut messages = integration_messages;
        messages.extend(
            integration_conflicts
                .iter()
                .map(|conflict| format!("conflict {conflict}")),
        );
        if justfile_exists && planned_justfile.is_none() {
            messages.push("already configured justfile".to_string());
        }
        let mut pending_files = Vec::new();
        let mut conflicts = Vec::new();
        for (path, contents, conflict_on_difference) in files {
            let destination = self.root.join(path);
            match fs::read_to_string(&destination) {
                Ok(existing) if existing == contents => {
                    messages.push(format!("already configured {path}"));
                }
                Ok(_) if conflict_on_difference => conflicts.push((path, contents)),
                Ok(_) => pending_files.push((path, contents)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    pending_files.push((path, contents));
                }
                Err(error) => return Err(error.into()),
            }
        }
        if apply
            && (!integration_conflicts.is_empty() || (!replace_conflicts && !conflicts.is_empty()))
        {
            if !integration_conflicts.is_empty() {
                return Err(Error::Parse(format!(
                    "scaffold_integration_conflicts {}",
                    integration_conflicts.join(";")
                )));
            }
            return Err(Error::Parse(format!(
                "scaffold_conflicts paths={} resolution=--replace-conflicts",
                conflicts
                    .iter()
                    .map(|(path, _contents)| *path)
                    .collect::<Vec<_>>()
                    .join(",")
            )));
        }
        if replace_conflicts {
            pending_files.extend(conflicts.iter().cloned());
        }
        if apply {
            for (path, _contents) in &pending_files {
                let destination = self.root.join(path);
                reject_symlinked_ancestors(&self.root, &destination)?;
                match fs::symlink_metadata(&destination) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        return Err(Error::Parse(format!(
                            "scaffold_destination_symlink path={path}"
                        )));
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
            }
            for (path, contents) in &pending_files {
                let destination = self.root.join(path);
                atomic_write(&destination, contents.as_bytes())?;
                messages.push(format!("wrote {path}"));
            }
        } else {
            messages.extend(
                pending_files
                    .iter()
                    .map(|(path, _contents)| format!("would write {path}")),
            );
            messages.extend(conflicts.iter().map(|(path, _contents)| {
                format!("conflict {path} resolution=--replace-conflicts")
            }));
        }
        for path in [equivalent_hook, equivalent_workflow].into_iter().flatten() {
            messages.push(format!("already configured {path}"));
        }
        Ok(messages)
    }

    fn show_tasks_justfile(&self) -> Result<Option<String>, Error> {
        let path = self.root.join("justfile");
        if !path.exists() {
            return Ok(None);
        }
        let mut contents = fs::read_to_string(path)?;
        if contents.lines().any(|line| line.trim() == "show-tasks:") {
            return Ok(None);
        }
        if !contents.ends_with('\n') {
            contents.push('\n');
        }
        contents.push_str("\nshow-tasks:\n  tiber list\n");
        Ok(Some(contents))
    }

    fn equivalent_task_closing_workflow(&self) -> Result<Option<String>, Error> {
        let workflows = self.root.join(".github").join("workflows");
        let entries = match fs::read_dir(&workflows) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut entries = entries.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let supported_extension = path
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|extension| matches!(extension, "yml" | "yaml"));
            if !entry.file_type()?.is_file() || !supported_extension {
                continue;
            }
            let relative = path
                .strip_prefix(&self.root)
                .map_err(|_| Error::Parse("scaffold_path_outside_repository".to_string()))?;
            let relative = path_to_entry(relative)?;
            let contents = fs::read(&path)?;
            let contents = std::str::from_utf8(&contents).map_err(|_| {
                Error::Parse(format!("scaffold_workflow_invalid_utf8 path={relative}"))
            })?;
            if workflow_invokes_task_closer(contents) {
                return Ok(Some(relative));
            }
        }
        Ok(None)
    }

    fn equivalent_task_closing_hook(&self) -> Result<Option<String>, Error> {
        let hooks = PathBuf::from(
            self.git(["rev-parse", "--path-format=absolute", "--git-path", "hooks"])?
                .trim(),
        );
        let hook = hooks.join("post-commit");
        if !hook.is_file() || !is_executable(&hook)? {
            return Ok(None);
        }
        let contents = fs::read(&hook)?;
        let Ok(contents) = std::str::from_utf8(&contents) else {
            return Ok(None);
        };
        if contents.lines().any(shell_line_invokes_task_closer) {
            let path = match hook.strip_prefix(&self.root) {
                Ok(relative) => path_to_entry(relative)?,
                Err(_) => hook.display().to_string(),
            };
            return Ok(Some(path));
        }
        Ok(None)
    }

    fn explicit_hooks_path(&self) -> Result<Option<PathBuf>, Error> {
        match self.git(["config", "--path", "core.hooksPath"]) {
            Ok(path) => {
                let path = PathBuf::from(path.trim());
                Ok(Some(if path.is_absolute() {
                    path
                } else {
                    self.root.join(path)
                }))
            }
            Err(Error::CommandFailed { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn active_post_commit_hook(&self) -> Result<PathBuf, Error> {
        Ok(PathBuf::from(
            self.git([
                "rev-parse",
                "--path-format=absolute",
                "--git-path",
                "hooks/post-commit",
            ])?
            .trim(),
        ))
    }

    fn publication_branch(&self) -> Result<String, Error> {
        match self.git([
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ]) {
            Ok(reference) => reference
                .trim()
                .strip_prefix("origin/")
                .filter(|branch| !branch.is_empty())
                .map(str::to_string)
                .ok_or_else(|| Error::Parse("origin_default_branch_invalid=true".to_string())),
            Err(Error::CommandFailed { .. }) => self.current_branch(),
            Err(error) => Err(error),
        }
    }

    fn hook_dispatches_tiber_snippet(&self, hook: &Path) -> Result<bool, Error> {
        if !hook.is_file() || !is_executable(hook)? {
            return Ok(false);
        }
        let contents = fs::read(hook)?;
        Ok(std::str::from_utf8(&contents).is_ok_and(hook_contents_dispatch_tiber_snippet))
    }

    fn task_file_refs(&self) -> Result<Vec<String>, Error> {
        let projection = self.task_projection()?;
        let mut refs = projection
            .borrow()
            .tasks
            .values()
            .map(|task| format!("{}/{}.md", task.status, task.stem))
            .collect::<Vec<_>>();
        refs.sort();
        Ok(refs)
    }

    fn task_projection(&self) -> Result<Rc<RefCell<TiberProjection>>, Error> {
        self.task_projection
            .clone()
            .ok_or_else(|| Error::Parse("task_projection_unavailable=true".into()))
    }

    fn resolve_task_stem(&self, task_ref: &str) -> Result<String, Error> {
        let path = self.resolve_task_ref(task_ref)?;
        task_stem(&path)
    }

    fn resolve_task_ref(&self, task_ref: &str) -> Result<PathBuf, Error> {
        if task_ref.contains('/') || task_ref.ends_with(".md") || task_ref.trim().is_empty() {
            return Err(Error::Parse(format!("invalid_task_ref ref={task_ref}")));
        }
        let mut matches = self
            .task_file_refs()?
            .into_iter()
            .filter(|candidate| {
                let stem = candidate.trim_end_matches(".md");
                let file_stem = Path::new(candidate)
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default();
                let id = file_stem
                    .split_once('-')
                    .and_then(|(date, rest)| {
                        rest.split_once('-')
                            .map(|(code, _nickname)| format!("{date}-{code}"))
                    })
                    .unwrap_or_default();
                let nickname = file_stem
                    .split_once('-')
                    .and_then(|(_date, rest)| rest.split_once('-'))
                    .map(|(_code, nickname)| nickname)
                    .unwrap_or_default();
                stem == task_ref || file_stem == task_ref || id == task_ref || nickname == task_ref
            })
            .collect::<Vec<_>>();
        matches.sort();
        match matches.as_slice() {
            [resolved] => Ok(PathBuf::from(resolved)),
            [] => Err(Error::Parse(format!("task_ref_missing ref={task_ref}"))),
            _ => Err(Error::Parse(format!(
                "ambiguous_task_ref ref={task_ref} matches={}",
                matches.join(",")
            ))),
        }
    }

    fn acquire_lock(&self) -> Result<TiberLock, Error> {
        let timeout =
            lock_retry_duration("TIBER_LOCK_RETRY_TIMEOUT_MS", DEFAULT_LOCK_RETRY_TIMEOUT);
        let interval =
            lock_retry_duration("TIBER_LOCK_RETRY_INTERVAL_MS", DEFAULT_LOCK_RETRY_INTERVAL);
        let interval = if interval.is_zero() {
            DEFAULT_LOCK_RETRY_INTERVAL
        } else {
            interval
        };
        let started_at = Instant::now();
        loop {
            match self.try_acquire_task_lock_once() {
                Ok(lock) => return Ok(lock),
                Err(error)
                    if is_tiber_lock_busy(&error) && lock_retry_remaining(started_at, timeout) =>
                {
                    thread::sleep(interval);
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn acquire_named_lock(&self, filename: &str) -> Result<TiberLock, Error> {
        let timeout =
            lock_retry_duration("TIBER_LOCK_RETRY_TIMEOUT_MS", DEFAULT_LOCK_RETRY_TIMEOUT);
        let interval =
            lock_retry_duration("TIBER_LOCK_RETRY_INTERVAL_MS", DEFAULT_LOCK_RETRY_INTERVAL);
        let interval = if interval.is_zero() {
            DEFAULT_LOCK_RETRY_INTERVAL
        } else {
            interval
        };
        let started_at = Instant::now();
        loop {
            match self.try_acquire_named_lock_once(filename) {
                Ok(lock) => return Ok(lock),
                Err(error)
                    if is_tiber_lock_busy(&error) && lock_retry_remaining(started_at, timeout) =>
                {
                    thread::sleep(interval);
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn try_acquire_named_lock_once(&self, filename: &str) -> Result<TiberLock, Error> {
        let lock_dir = self.git_common_dir()?.join("tiber");
        fs::create_dir_all(&lock_dir)?;
        let lock_path = lock_dir.join(filename);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        match file.try_lock() {
            Ok(()) => {
                file.set_len(0)?;
                file.write_all(lock_metadata().as_bytes())?;
                file.sync_data()?;
                Ok(TiberLock {
                    _file: file,
                    _legacy_sentinel: None,
                })
            }
            Err(TryLockError::WouldBlock) => Err(Error::Parse(format!(
                "tiber_lock_busy path={}",
                path_to_entry(&lock_path)?
            ))),
            Err(TryLockError::Error(error)) => Err(Error::Io(error)),
        }
    }

    fn try_acquire_task_lock_once(&self) -> Result<TiberLock, Error> {
        let lock_dir = self.git_common_dir()?.join("tiber");
        fs::create_dir_all(&lock_dir)?;
        let legacy_path = lock_dir.join("tiber.lock");
        let advisory_path = lock_dir.join("tiber.advisory.lock");
        let advisory_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&advisory_path)?;
        match advisory_file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(Error::Parse(format!(
                    "tiber_lock_busy path={}",
                    path_to_entry(&legacy_path)?
                )));
            }
            Err(TryLockError::Error(error)) => return Err(Error::Io(error)),
        }

        if let Some(stale_contents) = stale_lock_contents(&legacy_path)? {
            if fs::read_to_string(&legacy_path)
                .ok()
                .as_deref()
                .is_some_and(|contents| contents == stale_contents)
            {
                let _ = fs::remove_file(&legacy_path);
            }
        }
        let metadata = lock_metadata();
        let legacy_file = match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&legacy_path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(Error::Parse(format!(
                    "tiber_lock_busy path={}",
                    path_to_entry(&legacy_path)?
                )));
            }
            Err(error) => return Err(Error::Io(error)),
        };
        let mut legacy_sentinel = LegacySentinel {
            file: legacy_file,
            path: legacy_path,
            metadata: None,
        };
        legacy_sentinel.file.write_all(metadata.as_bytes())?;
        legacy_sentinel.file.sync_data()?;
        legacy_sentinel.metadata = Some(metadata);

        Ok(TiberLock {
            _file: advisory_file,
            _legacy_sentinel: Some(legacy_sentinel),
        })
    }

    fn git<I, S>(&self, args: I) -> Result<String, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        git_output(args, Some(&self.root))
    }

    fn git_common_dir(&self) -> Result<PathBuf, Error> {
        let git_common_dir = self.git(["rev-parse", "--git-common-dir"])?;
        let git_common_dir = PathBuf::from(git_common_dir.trim());
        if git_common_dir.is_absolute() {
            Ok(git_common_dir)
        } else {
            Ok(self.root.join(git_common_dir))
        }
    }
}

struct TiberLock {
    _file: fs::File,
    _legacy_sentinel: Option<LegacySentinel>,
}

struct LegacySentinel {
    file: fs::File,
    path: PathBuf,
    metadata: Option<String>,
}

impl Drop for LegacySentinel {
    fn drop(&mut self) {
        let still_owned = self.metadata.as_ref().is_none_or(|metadata| {
            fs::read_to_string(&self.path)
                .ok()
                .as_deref()
                .is_some_and(|contents| contents == metadata)
        });
        if still_owned {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn claim_host() -> String {
    std::env::var("TIBER_CLAIM_HOST")
        .or_else(|_| std::env::var("HOSTNAME"))
        .map(|value| frontmatter_scalar_value(&value))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn claim_session() -> String {
    std::env::var("TIBER_CLAIM_SESSION")
        .or_else(|_| std::env::var("CODEX_SESSION_ID"))
        .or_else(|_| std::env::var("CLAUDE_SESSION_ID"))
        .map(|value| frontmatter_scalar_value(&value))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn frontmatter_scalar_value(value: &str) -> String {
    let sanitized = value
        .trim()
        .chars()
        .map(|character| {
            if character.is_control() {
                '-'
            } else {
                character
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "unknown".to_string()
    } else {
        sanitized
    }
}

fn task_stem(task_path: &Path) -> Result<String, Error> {
    task_path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .ok_or_else(|| Error::Parse("task_stem_missing=true".to_string()))
}

fn is_open_status(status: &str) -> bool {
    OPEN_STATUS_DIRS.contains(&status)
}

fn nonempty_option(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn add_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|item| item == value) {
        values.push(value.to_string());
    }
}

fn repair_typed_links(
    tasks: &mut std::collections::BTreeMap<String, Task>,
    messages: &mut Vec<ValidationMessage>,
) {
    let snapshot = tasks.clone();
    for (stem, task) in &snapshot {
        for blocked in &task.blocks {
            if let Some(target) = tasks.get_mut(blocked) {
                if !target.blocked_by.contains(stem) {
                    target.blocked_by.push(stem.clone());
                    messages.push(ValidationMessage(format!(
                        "fixed reciprocal blocked_by {blocked} <- {stem}"
                    )));
                }
            }
        }
        for blocker in &task.blocked_by {
            if let Some(target) = tasks.get_mut(blocker) {
                if !target.blocks.contains(stem) {
                    target.blocks.push(stem.clone());
                    messages.push(ValidationMessage(format!(
                        "fixed reciprocal blocks {blocker} -> {stem}"
                    )));
                }
            }
        }
    }
}

fn report_typed_cycles(
    tasks: &std::collections::BTreeMap<String, Task>,
    messages: &mut Vec<ValidationMessage>,
) {
    let graph = DependencyGraph::from_tasks(
        tasks
            .values()
            .map(|task| TaskDependencies::new(task.stem.clone(), task.blocks.clone()))
            .collect(),
    );
    messages.extend(graph.cycle_messages().into_iter().map(ValidationMessage));
    for task in tasks.values() {
        let graph = DependencyGraph::from_tasks(
            task.subtasks
                .iter()
                .map(|item| TaskDependencies::new(item.id.clone(), item.after.clone()))
                .collect(),
        );
        messages.extend(
            graph
                .cycle_messages_with_label("subtask")
                .into_iter()
                .map(|message| ValidationMessage(format!("{message} task={}", task.stem))),
        );
    }
}

fn atomic_write(destination: &Path, contents: &[u8]) -> Result<(), Error> {
    static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    let parent = destination
        .parent()
        .ok_or_else(|| Error::Parse("scaffold_destination_parent_missing".to_string()))?;
    let file_name = destination
        .file_name()
        .ok_or_else(|| Error::Parse("scaffold_destination_name_missing".to_string()))?
        .to_string_lossy();
    let temporary_prefix = format!(".tiber-tmp-{file_name}-");
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        "{temporary_prefix}{}-{sequence}",
        std::process::id()
    ));
    let result = (|| -> Result<(), Error> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(contents)?;
        if let Ok(metadata) = fs::metadata(destination) {
            fs::set_permissions(&temporary, metadata.permissions())?;
        }
        file.sync_all()?;
        fs::rename(&temporary, destination)?;
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn reject_symlinked_ancestors(root: &Path, destination: &Path) -> Result<(), Error> {
    let relative = destination
        .strip_prefix(root)
        .map_err(|_| Error::Parse("scaffold_destination_outside_repository".to_string()))?;
    let mut ancestor = root.to_path_buf();
    for component in relative.parent().into_iter().flat_map(Path::components) {
        ancestor.push(component);
        match fs::symlink_metadata(&ancestor) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                let path = ancestor.strip_prefix(root).unwrap_or(&ancestor).display();
                return Err(Error::Parse(format!(
                    "scaffold_destination_ancestor_symlink path={path}"
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn workflow_invokes_task_closer(contents: &str) -> bool {
    if !workflow_has_push_trigger(contents) {
        return false;
    }
    let lines = contents.lines().collect::<Vec<_>>();
    for (jobs_index, line) in lines.iter().enumerate() {
        let jobs_trimmed = line.trim_start();
        if trim_unquoted_comment(jobs_trimmed) != "jobs:" {
            continue;
        }
        let jobs_indentation = line.len() - jobs_trimmed.len();
        for (steps_index, steps_line) in lines.iter().enumerate().skip(jobs_index + 1) {
            let steps_trimmed = steps_line.trim_start();
            if steps_trimmed.is_empty() || steps_trimmed.starts_with('#') {
                continue;
            }
            let steps_indentation = steps_line.len() - steps_trimmed.len();
            if steps_indentation <= jobs_indentation {
                break;
            }
            if trim_unquoted_comment(steps_trimmed) != "steps:" {
                continue;
            }
            if steps_invoke_task_closer(&lines, steps_index + 1, steps_indentation) {
                return true;
            }
        }
    }
    false
}

fn workflow_has_push_trigger(contents: &str) -> bool {
    let lines = contents.lines().collect::<Vec<_>>();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if line.len() != trimmed.len() {
            continue;
        }
        let Some(value) = trim_unquoted_comment(trimmed).strip_prefix("on:") else {
            continue;
        };
        let value = value.trim();
        if value == "push"
            || value
                .strip_prefix('[')
                .and_then(|value| value.strip_suffix(']'))
                .is_some_and(|events| events.split(',').any(|event| event.trim() == "push"))
        {
            return true;
        }
        if !value.is_empty() {
            return false;
        }
        for event_line in &lines[index + 1..] {
            let event = event_line.trim_start();
            let indentation = event_line.len() - event.len();
            if event.is_empty() || event.starts_with('#') {
                continue;
            }
            if indentation == 0 {
                return false;
            }
            let event = trim_unquoted_comment(event).trim();
            if event.starts_with("push:")
                || event.strip_prefix("- ").is_some_and(|item| item == "push")
            {
                return true;
            }
        }
        return false;
    }
    false
}

fn steps_invoke_task_closer(lines: &[&str], start: usize, steps_indentation: usize) -> bool {
    let mut step_indentation = None;
    let mut property_indentation = None;
    for (index, line) in lines.iter().enumerate().skip(start) {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indentation = line.len() - trimmed.len();
        if indentation <= steps_indentation {
            break;
        }
        if let Some(field) = trimmed.strip_prefix("- ") {
            step_indentation = Some(indentation);
            property_indentation = None;
            if run_field_invokes_task_closer(lines, index, indentation, field) {
                return true;
            }
            continue;
        }
        let Some(current_step_indentation) = step_indentation else {
            continue;
        };
        if indentation <= current_step_indentation {
            step_indentation = None;
            property_indentation = None;
            continue;
        }
        let current_property_indentation = *property_indentation.get_or_insert(indentation);
        if indentation == current_property_indentation
            && run_field_invokes_task_closer(lines, index, indentation, trimmed)
        {
            return true;
        }
    }
    false
}

fn run_field_invokes_task_closer(
    lines: &[&str],
    index: usize,
    indentation: usize,
    field: &str,
) -> bool {
    let Some(value) = field.strip_prefix("run:") else {
        return false;
    };
    let value = trim_unquoted_comment(value.trim());
    if value.starts_with('|') || value.starts_with('>') {
        for block_line in &lines[index + 1..] {
            let block_trimmed = block_line.trim_start();
            if block_trimmed.is_empty() {
                continue;
            }
            let block_indentation = block_line.len() - block_trimmed.len();
            if block_indentation <= indentation {
                break;
            }
            if shell_line_invokes_task_closer(block_trimmed) {
                return true;
            }
        }
        false
    } else {
        shell_line_invokes_task_closer(value)
    }
}

fn shell_line_invokes_task_closer(line: &str) -> bool {
    let line = trim_shell_comment(line.trim())
        .trim()
        .trim_matches(|character| matches!(character, '"' | '\''));
    shell_command_segments(line)
        .into_iter()
        .any(shell_command_invokes_task_closer)
}

fn trim_shell_comment(value: &str) -> &str {
    let mut quote = None;
    let mut escaped = false;
    let mut at_word_start = true;
    for (index, character) in value.char_indices() {
        if escaped {
            escaped = false;
            at_word_start = false;
            continue;
        }
        if quote != Some('\'') && character == '\\' {
            escaped = true;
            continue;
        }
        if matches!(character, '"' | '\'') {
            quote = if quote == Some(character) {
                None
            } else if quote.is_none() {
                Some(character)
            } else {
                quote
            };
            at_word_start = false;
            continue;
        }
        if character == '#' && quote.is_none() && at_word_start {
            return value[..index].trim_end();
        }
        at_word_start = quote.is_none()
            && (character.is_whitespace()
                || matches!(character, '|' | '&' | ';' | '(' | ')' | '<' | '>'));
    }
    value
}

fn shell_command_segments(line: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut escaped = false;
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let character = bytes[index] as char;
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        if quote != Some('\'') && character == '\\' {
            escaped = true;
            index += 1;
            continue;
        }
        if matches!(character, '"' | '\'') {
            quote = if quote == Some(character) {
                None
            } else if quote.is_none() {
                Some(character)
            } else {
                quote
            };
            index += 1;
            continue;
        }
        let operator_length = if quote.is_none() && character == ';' {
            1
        } else if quote.is_none()
            && index + 1 < bytes.len()
            && matches!(&bytes[index..index + 2], b"&&" | b"||")
        {
            2
        } else {
            index += 1;
            continue;
        };
        segments.push(&line[start..index]);
        index += operator_length;
        start = index;
    }
    segments.push(&line[start..]);
    segments
}

fn shell_command_invokes_task_closer(line: &str) -> bool {
    let line = line.trim();
    let line = line.strip_prefix("exec ").unwrap_or(line);
    let line = line.strip_prefix("nix develop -c ").unwrap_or(line);
    let Some(remainder) = line.strip_prefix("tiber close-from-trailers") else {
        return false;
    };
    let remainder = remainder.trim_start();
    remainder.is_empty()
        || [">", "1>", "2>"]
            .iter()
            .any(|operator| remainder.starts_with(operator))
}

fn shell_line_invokes_tiber_snippet(line: &str) -> bool {
    if line != line.trim_start() {
        return false;
    }
    shell_command_invokes_tiber_snippet(trim_shell_comment(line.trim()).trim())
}

fn hook_contents_dispatch_tiber_snippet(contents: &str) -> bool {
    let mut meaningful = contents.lines().filter_map(|line| {
        let line = trim_shell_comment(line).trim_end();
        (!line.trim().is_empty()).then_some(line)
    });
    let mut dispatched = false;
    for line in meaningful.by_ref() {
        if line.starts_with("#!") || (!dispatched && line.starts_with("set ")) {
            continue;
        }
        if dispatched || !shell_line_invokes_tiber_snippet(line) {
            return false;
        }
        dispatched = true;
    }
    dispatched
}

fn shell_command_invokes_tiber_snippet(command: &str) -> bool {
    let command = command.trim();
    let command = command.strip_prefix("exec ").unwrap_or(command);
    let command = ["source ", ". ", "bash ", "sh "]
        .iter()
        .find_map(|prefix| command.strip_prefix(prefix))
        .unwrap_or(command)
        .trim_start();
    let Some(token) = command.split_whitespace().next() else {
        return false;
    };
    let token = token.trim_matches(|character| matches!(character, '"' | '\''));
    matches!(
        token,
        ".githooks/post-commit.tiber" | "./.githooks/post-commit.tiber"
    ) || token.ends_with("/.githooks/post-commit.tiber")
}

fn yaml_single_quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn trim_unquoted_comment(value: &str) -> &str {
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quote == Some('"') && character == '\\' {
            escaped = true;
            continue;
        }
        if matches!(character, '"' | '\'') {
            quote = if quote == Some(character) {
                None
            } else if quote.is_none() {
                Some(character)
            } else {
                quote
            };
            continue;
        }
        if character == '#'
            && quote.is_none()
            && value[..index]
                .chars()
                .next_back()
                .is_none_or(char::is_whitespace)
        {
            return value[..index].trim_end();
        }
    }
    value
}

fn new_task_id() -> String {
    if let Some(value) = COMMAND_TASK_IDS.with(|ids| {
        let mut state = ids.borrow_mut();
        let (values, cursor) = state.as_mut()?;
        let value = values.get(*cursor).cloned();
        *cursor += 1;
        value
    }) {
        return value;
    }
    let value = generate_task_id();
    COMMAND_TASK_IDS.with(|ids| {
        if let Some((values, _)) = ids.borrow_mut().as_mut() {
            values.push(value.clone());
        }
    });
    value
}

fn command_recorded_at() -> String {
    std::env::var("GIT_AUTHOR_DATE")
        .ok()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(&value).ok())
        .map(|value| {
            value
                .with_timezone(&chrono::Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        })
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339())
}

fn generate_task_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let days = (now.as_secs() / 86_400) as i64;
    let (year, month, day) = civil_from_days(days);
    let mut entropy = now.as_nanos() ^ u128::from(std::process::id());
    let mut code = String::new();
    for _ in 0..4 {
        let index = (entropy % TASK_ID_ALPHABET.len() as u128) as usize;
        code.push(TASK_ID_ALPHABET[index] as char);
        entropy /= TASK_ID_ALPHABET.len() as u128;
    }
    format!("{year:04}{month:02}{day:02}-{code}")
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

fn closes_trailers(log: &str) -> Vec<String> {
    log.lines()
        .filter_map(|line| line.trim().strip_prefix("Closes:"))
        .map(str::trim)
        .filter(|task_ref| !task_ref.is_empty())
        .map(str::to_string)
        .collect()
}

fn is_retryable_push_failure(error: &Error) -> bool {
    match error {
        Error::Parse(message)
            if message.starts_with("event_version_conflict=")
                || message.contains("event_store_authoritative_ref_retry=true") =>
        {
            true
        }
        Error::CommandFailed { args, stderr, .. } => {
            args.iter().any(|arg| arg == "push")
                && (stderr.contains("non-fast-forward")
                    || stderr.contains("fetch first")
                    || stderr.contains("incorrect old value provided")
                    || stderr.contains("stale info"))
        }
        _ => false,
    }
}

fn is_coordination_branch_creation_race(error: &Error) -> bool {
    matches!(
        error,
        Error::CommandFailed { args, stderr, .. }
            if args.iter().any(|arg| arg == "push")
                && stderr.contains("cannot lock ref")
                && stderr.contains("reference already exists")
    )
}

fn parse_subtask_ref(subtask_ref: &str) -> Result<String, Error> {
    if let Some(number) = subtask_ref.strip_prefix('s') {
        if !number.is_empty() && number.chars().all(|character| character.is_ascii_digit()) {
            return Ok(subtask_ref.to_string());
        }
    }
    let index = subtask_ref
        .parse::<usize>()
        .map_err(|error| Error::Parse(format!("invalid_subtask_ref source={error}")))?;
    if index == 0 {
        return Err(Error::Parse("invalid_subtask_ref zero=true".to_string()));
    }
    Ok(format!("s{index}"))
}

fn parse_one_based_usize(input: &str, kind: &str) -> Result<usize, Error> {
    let index = input
        .parse::<usize>()
        .map_err(|error| Error::Parse(format!("invalid_{kind}_index source={error}")))?;
    if index == 0 {
        return Err(Error::Parse(format!("invalid_{kind}_index zero=true")));
    }
    Ok(index)
}

fn parse_nonempty_text<'a>(input: &'a str, kind: &str) -> Result<&'a str, Error> {
    let text = input.trim();
    if text.is_empty() {
        return Err(Error::Parse(format!("{kind}_empty=true")));
    }
    if text.chars().any(char::is_control) {
        return Err(Error::Parse(format!("{kind}_invalid=true")));
    }
    Ok(text)
}

fn parse_task_section_body(input: &str) -> Result<String, Error> {
    let text = input.trim();
    if text.is_empty() {
        return Err(Error::Parse("section_empty=true".into()));
    }
    if text
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
    {
        return Err(Error::Parse(
            "section_invalid=true recovery=\"remove control characters other than newline or tab\""
                .into(),
        ));
    }
    if text.lines().any(|line| {
        matches!(
            line,
            "## Summary"
                | "## Context / Why"
                | "## Acceptance criteria"
                | "## Subtasks"
                | "## Notes / Log"
        )
    }) {
        return Err(Error::Parse(
            "section_reserved_heading=true recovery=\"demote or rename the embedded heading\""
                .into(),
        ));
    }
    Ok(text.to_string())
}

fn current_date_string() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let days = (now.as_secs() / 86_400) as i64;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn parse_safe_relative_path(path_ref: &str, kind: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(path_ref);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(Error::Parse(format!("invalid_{kind}_ref ref={path_ref}")));
    }
    Ok(path)
}

fn parse_doc_ref(doc_ref: &str) -> Result<PathBuf, Error> {
    let path = parse_safe_relative_path(doc_ref, "doc")?;
    let mut components = path.components();
    if components
        .next()
        .and_then(|component| component.as_os_str().to_str())
        != Some("docs")
        || components.next().is_none()
    {
        return Err(Error::Parse(format!("invalid_doc_ref ref={doc_ref}")));
    }
    Ok(path)
}

fn collect_docs(root: &Path, directory: &Path, docs: &mut Vec<String>) -> Result<(), Error> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_docs(root, &entry.path(), docs)?;
        } else if file_type.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "md")
        {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .map_err(|error| Error::Parse(format!("doc_prefix source={error}")))?;
            docs.push(path_to_entry(relative)?);
        }
    }
    Ok(())
}

fn parse_status(status: &str) -> Result<&str, Error> {
    if !STATUS_DIRS.contains(&status) {
        return Err(Error::Parse(format!("invalid_status status={status}")));
    }
    Ok(status)
}

fn path_to_entry(path: &Path) -> Result<String, Error> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| Error::Parse("path_utf8=false".to_string()))
}

fn expand_home(path: &Path) -> Result<PathBuf, Error> {
    let path = path_to_entry(path)?;
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME")
            .map_err(|error| Error::Parse(format!("home_unavailable source={error}")))?;
        Ok(PathBuf::from(home).join(rest))
    } else {
        Ok(PathBuf::from(path))
    }
}

fn tiber_launcher_path() -> Result<PathBuf, Error> {
    if let Ok(path) = std::env::var("TIBER_LAUNCHER_PATH") {
        return Ok(PathBuf::from(path));
    }

    let current_exe = std::env::current_exe()?;
    if current_exe
        .components()
        .any(|component| component.as_os_str() == "dist")
    {
        if let Some(plugin_root) = current_exe
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
        {
            let launcher = plugin_root.join("bin").join("tiber");
            if launcher.exists() {
                return Ok(launcher);
            }
        }
    }

    let source_plugin_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or_else(|| Error::Parse("plugin_root_unavailable=true".to_string()))?;
    Ok(source_plugin_root.join("bin").join("tiber"))
}

fn install_launcher(launcher: &Path, installed: &Path) -> Result<(), Error> {
    static INSTALL_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    let launcher = fs::canonicalize(launcher)?;
    let launcher = path_to_entry(&launcher)?;
    let launcher = launcher.replace('\'', "'\"'\"'");
    let parent = installed
        .parent()
        .ok_or_else(|| Error::Parse("install_target_parent_missing=true".to_string()))?;
    let sequence = INSTALL_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let staged = parent.join(format!(".tiber-install-{}-{sequence}", std::process::id()));
    let result = (|| -> Result<(), Error> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)?;
        write!(file, "#!/usr/bin/env bash\nexec '{launcher}' \"$@\"\n")?;
        #[cfg(unix)]
        fs::set_permissions(&staged, fs::Permissions::from_mode(0o755))?;
        file.sync_all()?;
        fs::hard_link(&staged, installed)?;
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    let _ = fs::remove_file(&staged);
    result
}

#[cfg(unix)]
fn is_executable(path: &Path) -> Result<bool, Error> {
    Ok(fs::metadata(path)?.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> Result<bool, Error> {
    Ok(path.is_file())
}

fn git_status<I, S>(args: I, cwd: Option<&Path>) -> Result<(), Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let _ = git_output(args, cwd)?;
    Ok(())
}

fn git_output<I, S>(args: I, cwd: Option<&Path>) -> Result<String, Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect::<Vec<_>>();
    let mut command = Command::new("git");
    command.args(&args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command.env("GIT_TERMINAL_PROMPT", "0");
    command.env("LC_ALL", "C");
    command.env("LANGUAGE", "C");
    command_output("git", &args, command.output()?)
}

fn lock_metadata() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("pid={}\ntimestamp={timestamp}\n", std::process::id())
}

fn lock_retry_duration(env_name: &str, default: Duration) -> Duration {
    std::env::var(env_name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(default)
}

fn lock_retry_remaining(started_at: Instant, timeout: Duration) -> bool {
    started_at.elapsed() < timeout
}

fn is_tiber_lock_busy(error: &Error) -> bool {
    matches!(error, Error::Parse(message) if message.starts_with("tiber_lock_busy "))
}

fn stale_lock_contents(path: &Path) -> Result<Option<String>, Error> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(Error::Io(error)),
    };
    if lock_contents_are_stale(&contents) {
        Ok(Some(contents))
    } else {
        Ok(None)
    }
}

fn lock_contents_are_stale(contents: &str) -> bool {
    let pid = contents
        .lines()
        .find_map(|line| line.strip_prefix("pid="))
        .and_then(|pid| pid.parse::<u32>().ok());
    if pid.is_some_and(process_is_gone) {
        return true;
    }
    let timestamp = contents
        .lines()
        .find_map(|line| line.strip_prefix("timestamp="))
        .and_then(|timestamp| timestamp.parse::<u64>().ok());
    timestamp.is_some_and(|timestamp| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH + Duration::from_secs(timestamp))
            .unwrap_or_default()
            > Duration::from_secs(60 * 60)
    })
}

#[cfg(unix)]
fn process_is_gone(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|status| !status.success())
}

#[cfg(not(unix))]
fn process_is_gone(_pid: u32) -> bool {
    false
}

fn command_output(
    program: &str,
    args: &[std::ffi::OsString],
    output: Output,
) -> Result<String, Error> {
    if output.status.success() {
        return String::from_utf8(output.stdout)
            .map_err(|error| Error::Parse(format!("utf8=false source={error}")));
    }

    Err(Error::CommandFailed {
        program: program.to_string(),
        args: args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect(),
        status: output.status.to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

#[cfg(test)]
mod lock_tests {
    use super::*;

    fn temporary_repository(label: &str) -> PathBuf {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "tiber-git-{label}-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("create temporary repository");
        let output = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&path)
            .output()
            .expect("initialize temporary repository");
        assert!(output.status.success(), "git init should succeed");
        path
    }

    #[test]
    fn current_task_lock_excludes_a_legacy_sentinel_client_for_its_lifetime() {
        let root = temporary_repository("legacy-exclusion");
        let repository = GitRepository::at(&root);
        let lock = repository
            .acquire_lock()
            .expect("acquire current task lock");
        let legacy_path = repository
            .git_common_dir()
            .expect("resolve common directory")
            .join("tiber/tiber.lock");

        let legacy_attempt = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&legacy_path);
        assert_eq!(
            legacy_attempt
                .expect_err("legacy client must remain excluded")
                .kind(),
            std::io::ErrorKind::AlreadyExists
        );

        drop(lock);
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&legacy_path)
            .expect("legacy client may acquire after current lock release");
        fs::remove_dir_all(root).expect("remove temporary repository");
    }

    #[test]
    fn unfinished_legacy_sentinel_is_removed_on_initialization_failure() {
        let root = temporary_repository("sentinel-rollback");
        let path = root.join("tiber.lock");
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .expect("create unfinished sentinel");

        drop(LegacySentinel {
            file,
            path: path.clone(),
            metadata: None,
        });

        assert!(!path.exists(), "unfinished sentinel must roll back");
        fs::remove_dir_all(root).expect("remove temporary repository");
    }

    #[test]
    fn entering_active_work_emits_typed_transition_and_publication_events() {
        let stem = "20260805-test-claimed".to_string();
        let mut before = TiberProjection {
            initialized: true,
            ..TiberProjection::default()
        };
        before.tasks.insert(
            stem.clone(),
            Task::new(stem.clone(), "Claimed".into(), "now".into()),
        );
        before.order.push(stem.clone());
        let mut after = before.clone();
        let task = after.tasks.get_mut(&stem).unwrap();
        task.status = "in-progress".into();
        task.claim = Some(Claim {
            host: "test".into(),
            session: "test".into(),
        });
        let events = task_change_events(&before, &after, TaskMutation::Transition).unwrap();
        let names = events
            .iter()
            .map(|event| serde_json::to_value(event).unwrap()["event"].clone())
            .collect::<Vec<_>>();
        assert!(names.contains(&serde_json::Value::String("task_transitioned".into())));
        assert!(names.contains(&serde_json::Value::String("task_state_published".into())));
        let transition = events
            .iter()
            .find(|event| matches!(event, TiberEvent::TaskTransitioned { .. }))
            .unwrap();
        assert_eq!(
            serde_json::to_value(transition).unwrap()["claim"]["session"],
            "test"
        );
    }
}
