use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::fs::OpenOptions;
use std::io::Read;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tiber_core::{
    BoardSnapshot, DependencyGraph, OrderReconciliation, TaskDependencies, TaskSnapshot, TaskTitle,
};

const STATUS_DIRS: &[&str] = &["backlog", "in-progress", "done", "abandoned"];
const OPEN_STATUS_DIRS: &[&str] = &["backlog", "in-progress"];
const TASK_ID_ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789";
const TASK_ID_GENERATION_ATTEMPTS: usize = 32;
const DEFAULT_LOCK_RETRY_TIMEOUT: Duration = Duration::from_secs(3);
const REMOTE_IO_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(50);
const MAX_CONFLICT_SNAPSHOT_SIDE_BYTES: usize = 64 * 1024;
const MAX_TASK_BLOB_BYTES: u64 = 1024 * 1024;

pub fn init_repository() -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.init_repository()
}

pub fn init_repository_at(root: impl Into<PathBuf>) -> Result<(), Error> {
    let repo = GitRepository::at(root);
    repo.init_repository()
}

pub fn create_task_at(root: impl Into<PathBuf>, title: &str) -> Result<TaskPath, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_workspace(|repo| repo.create_task(TaskTitle::parse(title)?))
}

pub fn list_tasks_at(root: impl Into<PathBuf>) -> Result<Vec<TaskSummary>, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_workspace(|repo| repo.list_tasks())
}

pub fn show_task_at(root: impl Into<PathBuf>, task_ref: &str) -> Result<String, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_workspace(|repo| repo.show_task(task_ref))
}

pub fn task_metadata_at(root: impl Into<PathBuf>, task_ref: &str) -> Result<TaskMetadata, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_workspace(|repo| repo.task_metadata(task_ref))
}

pub fn conflict_snapshot_at(
    root: impl Into<PathBuf>,
    path: &str,
) -> Result<ConflictSnapshot, Error> {
    let repo = GitRepository::at(root);
    repo.conflict_snapshot(path)
}

pub fn resolve_conflict_at(
    root: impl Into<PathBuf>,
    path: &str,
    side: &str,
) -> Result<ConflictResolution, Error> {
    let repo = GitRepository::at(root);
    repo.resolve_conflict(path, side)
}

pub fn resolve_conflicts_at(
    root: impl Into<PathBuf>,
    resolutions: &[ConflictResolutionRequest],
) -> Result<Vec<ConflictResolution>, Error> {
    let repo = GitRepository::at(root);
    repo.resolve_conflicts(resolutions)
}

pub fn task_documents_at(root: impl Into<PathBuf>) -> Result<Vec<TaskDocument>, Error> {
    let repo = GitRepository::at(root);
    repo.with_task_snapshot_workspace(|repo| repo.task_documents_snapshot())
}

pub fn task_documents_local_at(root: impl Into<PathBuf>) -> Result<Vec<TaskDocument>, Error> {
    let repo = GitRepository::at(root);
    repo.with_local_task_snapshot_workspace(|repo| repo.task_documents_snapshot())
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
        self.ensure_tasks_branch()?;
        Ok(())
    }
}

pub fn sync_repository() -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.sync_repository())
}

pub fn create_task(title: &str) -> Result<TaskPath, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.create_task(TaskTitle::parse(title)?))
}

pub fn list_tasks() -> Result<Vec<TaskSummary>, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.list_tasks())
}

pub fn show_task(task_ref: &str) -> Result<String, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.show_task(task_ref))
}

pub fn task_metadata(task_ref: &str) -> Result<TaskMetadata, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.task_metadata(task_ref))
}

pub fn conflict_snapshot(path: &str) -> Result<ConflictSnapshot, Error> {
    let repo = GitRepository::discover()?;
    repo.conflict_snapshot(path)
}

pub fn resolve_conflict(path: &str, side: &str) -> Result<ConflictResolution, Error> {
    let repo = GitRepository::discover()?;
    repo.resolve_conflict(path, side)
}

pub fn resolve_conflicts(
    resolutions: &[ConflictResolutionRequest],
) -> Result<Vec<ConflictResolution>, Error> {
    let repo = GitRepository::discover()?;
    repo.resolve_conflicts(resolutions)
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
    repo.with_task_workspace(|repo| repo.next_task())
}

pub fn transition_task(task_ref: &str, status: &str) -> Result<TaskPath, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.transition_task(task_ref, status))
}

pub fn prioritize_before(task_ref: &str, before_ref: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.prioritize_before(task_ref, before_ref))
}

pub fn link_blocks(from_ref: &str, to_ref: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.link_blocks(from_ref, to_ref))
}

pub fn unlink_blocks(from_ref: &str, to_ref: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.unlink_blocks(from_ref, to_ref))
}

pub fn add_subtask(task_ref: &str, title: &str, after_refs: &[String]) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.add_subtask(task_ref, title, after_refs))
}

pub fn set_subtask_checked(task_ref: &str, index: &str, checked: bool) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.set_subtask_checked(task_ref, index, checked))
}

pub fn update_task(task_ref: &str, update: TaskUpdate<'_>) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.update_task(task_ref, update.clone()))
}

pub fn add_acceptance(task_ref: &str, criterion: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.add_acceptance(task_ref, criterion))
}

pub fn set_acceptance_checked(task_ref: &str, index: &str, checked: bool) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.set_acceptance_checked(task_ref, index, checked))
}

pub fn remove_acceptance(task_ref: &str, index: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.remove_acceptance(task_ref, index))
}

pub fn add_note(task_ref: &str, note: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.add_note(task_ref, note))
}

pub fn validate_fix() -> Result<Vec<ValidationMessage>, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.validate_fix())
}

pub fn close_from_trailers() -> Result<Vec<String>, Error> {
    let repo = GitRepository::discover()?;
    repo.with_task_workspace(|repo| repo.close_from_trailers())
}

pub fn scaffold_repo(apply: bool) -> Result<Vec<String>, Error> {
    let repo = GitRepository::discover()?;
    repo.scaffold_repo(apply)
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
pub struct ConflictSnapshot {
    pub path: String,
    pub local_path: Option<String>,
    pub remote_path: Option<String>,
    pub local: Option<String>,
    pub remote: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ConflictResolution {
    pub path: String,
    pub side: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConflictSideChoice {
    Local,
    Remote,
}

impl ConflictSideChoice {
    pub fn parse(side: &str) -> Result<Self, Error> {
        match side {
            "local" => Ok(Self::Local),
            "remote" => Ok(Self::Remote),
            _ => Err(Error::Parse(format!(
                "invalid_conflict_side side={}",
                quoted_string(side)
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConflictResolutionRequest {
    pub path: String,
    pub side: ConflictSideChoice,
}

impl ConflictResolutionRequest {
    pub fn parse(path: impl Into<String>, side: &str) -> Result<Self, Error> {
        Ok(Self {
            path: path.into(),
            side: ConflictSideChoice::parse(side)?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ConflictSide {
    path: String,
    contents: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ConflictPath {
    value: String,
    key: String,
}

impl ConflictPath {
    fn parse(path: &str) -> Result<Self, Error> {
        let trimmed = path.trim();
        if trimmed.chars().any(char::is_control) {
            return Err(invalid_conflict_path(trimmed));
        }
        if trimmed != "order.md" && !is_course_task_path(trimmed) {
            return Err(invalid_conflict_path(trimmed));
        }
        let path = Path::new(trimmed);
        if path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(invalid_conflict_path(trimmed));
        }
        let key = if trimmed == "order.md" {
            trimmed.to_string()
        } else {
            format!("task:{}", task_stem(path)?)
        };
        Ok(Self {
            value: trimmed.to_string(),
            key,
        })
    }

    fn as_str(&self) -> &str {
        &self.value
    }

    fn key(&self) -> &str {
        &self.key
    }

    fn into_string(self) -> String {
        self.value
    }

    fn matches_storage_path(&self, path: &str) -> bool {
        if self.value == path {
            return true;
        }
        if self.value == "order.md" || path == "order.md" {
            return false;
        }
        is_course_task_path(path)
            && task_stem(Path::new(path))
                .ok()
                .is_some_and(|path_stem| self.key == format!("task:{path_stem}"))
    }
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
    TaskCreatedSyncFailed {
        task_path: String,
        source: Box<Error>,
    },
    TaskCreateSyncFailed {
        source: Box<Error>,
    },
    Io(std::io::Error),
    SyncConflict {
        path: String,
    },
    TaskRefInvalid {
        reference: String,
    },
    TaskRefMissing {
        reference: String,
    },
    TaskRefAmbiguous {
        reference: String,
        matches: Vec<String>,
    },
    Parse(String),
    Core(tiber_core::CoreError),
    Usage(String),
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
            Self::TaskCreatedSyncFailed { task_path, source } => write!(
                formatter,
                "tiber.create_sync_failed created={task_path} recovery=\"run tiber sync after resolving the sync error\" source={}",
                source.sanitized_sync_source()
            ),
            Self::TaskCreateSyncFailed { source } => write!(
                formatter,
                "tiber.create_sync_failed recovery=\"resolve the sync error before retrying create\" source={}",
                source.sanitized_sync_source()
            ),
            Self::Io(error) => write!(formatter, "tiber.io_error source={error}"),
            Self::SyncConflict { path } => {
                write!(
                    formatter,
                    "tiber.parse_error {}",
                    sync_conflict_guidance(path)
                )
            }
            Self::TaskRefInvalid { reference } => write!(
                formatter,
                "tiber.parse_error invalid_task_ref ref={reference}"
            ),
            Self::TaskRefMissing { reference } => write!(
                formatter,
                "tiber.parse_error task_ref_missing ref={reference}"
            ),
            Self::TaskRefAmbiguous { reference, matches } => write!(
                formatter,
                "tiber.parse_error ambiguous_task_ref ref={reference} matches={}",
                matches.join(",")
            ),
            Self::Parse(message) => write!(formatter, "tiber.parse_error {message}"),
            Self::Core(error) => write!(formatter, "{error}"),
            Self::Usage(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    pub fn is_tiber_lock_busy(&self) -> bool {
        is_tiber_lock_busy(self)
    }

    pub fn sanitized_dashboard_source(&self) -> String {
        let source = match self {
            Self::CommandFailed { .. } | Self::Io(_) => self.sanitized_sync_source(),
            _ => self.sanitized_agent_source(),
        };
        format!("dashboard_task_load_failed source={}", source)
    }

    pub fn sanitized_agent_source(&self) -> String {
        match self {
            Self::SyncConflict { path } => {
                format!("tiber.parse_error {}", sync_conflict_guidance(path))
            }
            Self::TaskRefInvalid { .. }
            | Self::TaskRefMissing { .. }
            | Self::TaskRefAmbiguous { .. } => self.to_string(),
            Self::Parse(message) => format!("tiber.parse_error {message}"),
            Self::CommandFailed {
                program,
                args,
                status,
                stderr,
            } if command_failure_needs_redaction(args) => self.sanitized_sync_source(),
            Self::CommandFailed {
                program,
                status,
                stderr,
                ..
            } => format!(
                "tiber.command_failed program={program} args_redacted=true status={status} stderr={}",
                quoted_string(&sanitized_command_stderr(stderr))
            ),
            Self::Io(error) => format!("tiber.io_error source={}", quoted_string(&error.to_string())),
            Self::TaskCreatedSyncFailed { .. } | Self::TaskCreateSyncFailed { .. } => {
                self.to_string()
            }
            Self::Core(error) => error.to_string(),
            Self::Usage(message) => message.to_string(),
        }
    }

    pub fn is_task_ref_resolution_error(&self) -> bool {
        matches!(
            self,
            Self::TaskRefInvalid { .. }
                | Self::TaskRefMissing { .. }
                | Self::TaskRefAmbiguous { .. }
        )
    }

    fn is_task_ref_missing_error(&self) -> bool {
        matches!(self, Self::TaskRefMissing { .. })
    }

    fn sanitized_sync_source(&self) -> String {
        match self {
            Self::CommandFailed {
                program,
                status,
                stderr,
                ..
            } => {
                let category = git_stderr_category(stderr);
                format!(
                    "tiber.command_failed program={program} args_redacted=true status={status} stderr_redacted={} stderr_category={category}",
                    !stderr.trim().is_empty()
                )
            }
            Self::TaskCreatedSyncFailed { .. } | Self::TaskCreateSyncFailed { .. } => {
                "tiber.create_sync_failed nested=true".to_string()
            }
            Self::Io(_) => "tiber.io_error source_redacted=true".to_string(),
            Self::SyncConflict { path } => {
                format!("tiber.parse_error {}", sync_conflict_guidance(path))
            }
            Self::TaskRefInvalid { .. }
            | Self::TaskRefMissing { .. }
            | Self::TaskRefAmbiguous { .. } => self.to_string(),
            Self::Parse(_) => "tiber.parse_error source_redacted=true".to_string(),
            Self::Core(error) => error.to_string(),
            Self::Usage(message) => message.to_string(),
        }
    }
}

fn git_stderr_category(stderr: &str) -> &'static str {
    let stderr = stderr.to_ascii_lowercase();
    if stderr.trim().is_empty() {
        "none"
    } else if stderr.contains("permission denied")
        || stderr.contains("authentication failed")
        || stderr.contains("could not read from remote repository")
        || stderr.contains("publickey")
    {
        "auth_or_permission"
    } else if stderr.contains("could not resolve hostname")
        || stderr.contains("name or service not known")
        || stderr.contains("temporary failure in name resolution")
    {
        "network_name_resolution"
    } else if stderr.contains("repository not found")
        || stderr.contains("not appear to be a git repository")
        || stderr.contains("couldn't find remote ref")
        || stderr.contains("could not find remote ref")
    {
        "remote_missing_or_unavailable"
    } else if stderr.contains("non-fast-forward") || stderr.contains("fetch first") {
        "remote_rejected_non_fast_forward"
    } else {
        "other"
    }
}

fn sync_conflict_guidance(path: &str) -> String {
    format!(
        "sync_conflict path={} recovery=\"run tiber conflict show <path>, preserve both versions, choose tiber conflict resolve <path> --local or --remote, then rerun tiber sync\" mcp_tool=tiber.conflict_show mcp_resolve_tool=tiber.conflict_resolve",
        quoted_string(path)
    )
}

fn sync_conflict_error(path: &str) -> Error {
    Error::SyncConflict {
        path: path_to_entry(Path::new(path)).unwrap_or_else(|_| path.to_string()),
    }
}

fn task_ref_invalid(reference: &str) -> Error {
    Error::TaskRefInvalid {
        reference: reference.to_string(),
    }
}

fn task_ref_missing(reference: &str) -> Error {
    Error::TaskRefMissing {
        reference: reference.to_string(),
    }
}

fn task_ref_ambiguous(reference: &str, matches: Vec<String>) -> Error {
    Error::TaskRefAmbiguous {
        reference: reference.to_string(),
        matches,
    }
}

fn sanitized_command_stderr(stderr: &str) -> String {
    let mut sanitized = stderr.trim().replace('\n', "\\n");
    for marker in ["https://", "http://", "ssh://"] {
        while let Some(start) = sanitized.find(marker) {
            let end = sanitized[start..]
                .find(char::is_whitespace)
                .map(|offset| start + offset)
                .unwrap_or(sanitized.len());
            sanitized.replace_range(start..end, "<redacted-url>");
        }
    }
    if sanitized.len() > 512 {
        sanitized.truncate(512);
        sanitized.push_str("...");
    }
    sanitized
}

fn command_failure_needs_redaction(args: &[String]) -> bool {
    matches!(
        git_subcommand(args),
        Some("fetch" | "push" | "ls-remote" | "remote")
    )
}

fn git_subcommand(args: &[String]) -> Option<&str> {
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        if arg == "-c" {
            index += 2;
            continue;
        }
        if arg.starts_with("-c") {
            index += 1;
            continue;
        }
        if arg.starts_with('-') {
            index += 1;
            continue;
        }
        return Some(arg);
    }
    None
}

fn quoted_string(value: &str) -> String {
    format!("{value:?}")
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
    tasks_dir: Option<PathBuf>,
}

impl GitRepository {
    fn at(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            tasks_dir: None,
        }
    }

    fn discover() -> Result<Self, Error> {
        let root = git_output(["rev-parse", "--show-toplevel"], None)?;
        let root_path = PathBuf::from(root.trim());
        Ok(Self::at(root_path))
    }

    fn with_tasks_dir(&self, tasks_dir: PathBuf) -> Self {
        Self {
            root: self.root.clone(),
            tasks_dir: Some(tasks_dir),
        }
    }

    fn with_task_workspace<T>(
        &self,
        mut operation: impl FnMut(&GitRepository) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let _lock = self.acquire_lock()?;
        self.ensure_tasks_branch_from_origin()?;
        self.fast_forward_local_tasks_ref_from_origin(false)?;
        let workspace = TaskWorkspace::create()?;
        self.materialize_tasks_ref("refs/heads/tasks", workspace.path())?;
        let repo = self.with_tasks_dir(workspace.path().to_path_buf());
        match operation(&repo) {
            Err(error) if error.is_task_ref_missing_error() => {
                let workspace = TaskWorkspace::create()?;
                self.materialize_tasks_ref("refs/heads/tasks", workspace.path())?;
                let repo = self.with_tasks_dir(workspace.path().to_path_buf());
                repo.read_sync().map_err(implicit_remote_refresh_error)?;
                repo.sync_repository_unlocked()?;
                let workspace = TaskWorkspace::create()?;
                self.materialize_tasks_ref("refs/heads/tasks", workspace.path())?;
                let repo = self.with_tasks_dir(workspace.path().to_path_buf());
                operation(&repo)
            }
            result => result,
        }
    }

    fn with_task_snapshot_workspace<T>(
        &self,
        operation: impl FnOnce(&GitRepository) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let lock = if self.has_origin_remote() {
            Some(self.acquire_lock()?)
        } else {
            None
        };
        let workspace = TaskWorkspace::create()?;
        if git_status(
            ["show-ref", "--verify", "refs/heads/tasks"],
            Some(&self.root),
        )
        .is_ok()
        {
            self.materialize_tasks_ref("refs/heads/tasks", workspace.path())?;
        }
        let repo = self.with_tasks_dir(workspace.path().to_path_buf());
        if lock.is_some() {
            repo.read_sync()?;
        }
        operation(&repo)
    }

    fn with_local_task_snapshot_workspace<T>(
        &self,
        operation: impl FnOnce(&GitRepository) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let workspace = TaskWorkspace::create()?;
        if git_status(
            ["show-ref", "--verify", "refs/heads/tasks"],
            Some(&self.root),
        )
        .is_ok()
        {
            self.materialize_tasks_ref("refs/heads/tasks", workspace.path())?;
        }
        operation(&self.with_tasks_dir(workspace.path().to_path_buf()))
    }

    fn materialize_tasks_ref(&self, task_ref: &str, destination: &Path) -> Result<(), Error> {
        for status in STATUS_DIRS {
            fs::create_dir_all(destination.join(status))?;
        }
        let order = destination.join("order.md");
        if !order.exists() {
            fs::write(&order, "")?;
        }

        let resolved_task_ref = self.git(["rev-parse", "--verify", task_ref])?;
        let resolved_task_ref = resolved_task_ref.trim();
        let listing = self.git(["ls-tree", "-r", "--name-only", resolved_task_ref])?;
        for path in listing.lines().filter(|line| !line.trim().is_empty()) {
            if path == "order.md" || is_course_task_path(path) || is_status_gitkeep_path(path) {
                self.ensure_git_blob_size(resolved_task_ref, path, MAX_TASK_BLOB_BYTES)?;
                let contents = self.git(["show", &format!("{resolved_task_ref}:{path}")])?;
                let destination = destination.join(path);
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(destination, contents)?;
            }
        }
        Ok(())
    }

    fn ensure_tasks_branch(&self) -> Result<(), Error> {
        if git_status(
            ["show-ref", "--verify", "refs/heads/tasks"],
            Some(&self.root),
        )
        .is_ok()
        {
            return Ok(());
        }

        let empty_blob = self.git_with_stdin(["hash-object", "-w", "--stdin"], "")?;
        let status_tree = self.git_with_stdin(
            ["mktree"],
            &format!("100644 blob {}\t.gitkeep\n", empty_blob.trim()),
        )?;
        let mut entries = vec![format!("100644 blob {}\torder.md\n", empty_blob.trim())];
        for status in STATUS_DIRS {
            entries.push(format!("040000 tree {}\t{status}\n", status_tree.trim()));
        }
        entries.sort();
        let root_tree = self.git_with_stdin(["mktree"], &entries.concat())?;
        let commit = self.commit_tree(root_tree.trim(), &[], "Initialize tiber")?;
        self.git([
            "update-ref",
            "refs/heads/tasks",
            commit.trim(),
            "0000000000000000000000000000000000000000",
        ])?;
        Ok(())
    }

    fn ensure_tasks_branch_from_origin(&self) -> Result<(), Error> {
        if git_status(
            ["show-ref", "--verify", "refs/heads/tasks"],
            Some(&self.root),
        )
        .is_ok()
        {
            return Ok(());
        }
        let remote_parent = match self.fetch_origin_tasks(true) {
            Ok(remote_parent) => remote_parent,
            Err(error) => return Err(implicit_remote_refresh_error(error)),
        };
        if let Some(remote_parent) = remote_parent {
            self.git([
                "update-ref",
                "refs/heads/tasks",
                remote_parent.trim(),
                "0000000000000000000000000000000000000000",
            ])?;
            return Ok(());
        }
        self.ensure_tasks_branch()
    }

    fn sync_repository(&self) -> Result<(), Error> {
        self.sync_repository_unlocked()
    }

    fn sync_repository_unlocked(&self) -> Result<(), Error> {
        let allow_missing_remote_tasks = self.partial_create_marker_allows_missing_remote()?;
        match self.sync_repository_unlocked_allowing_missing_remote(allow_missing_remote_tasks) {
            Ok(()) => {
                if allow_missing_remote_tasks {
                    self.clear_partial_create_marker()?;
                }
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    fn sync_repository_unlocked_allowing_missing_remote(
        &self,
        allow_missing_remote_tasks: bool,
    ) -> Result<(), Error> {
        match self.sync_repository_once(allow_missing_remote_tasks) {
            Ok(()) => Ok(()),
            Err(error) if is_retryable_push_failure(&error) => {
                let workspace = TaskWorkspace::create()?;
                self.materialize_tasks_ref("refs/heads/tasks", workspace.path())?;
                let repo = self.with_tasks_dir(workspace.path().to_path_buf());
                repo.sync_repository_once(allow_missing_remote_tasks)
            }
            Err(error) => Err(error),
        }
    }

    fn sync_repository_once(&self, allow_missing_remote_tasks: bool) -> Result<(), Error> {
        let local_parent = self.git(["rev-parse", "--verify", "refs/heads/tasks"])?;
        self.ensure_local_task_sizes(&self.tasks_dir())?;
        let remote_parent = self.fetch_origin_tasks(allow_missing_remote_tasks)?;
        let remote_advanced = remote_parent
            .as_deref()
            .is_some_and(|parent| parent.trim() != local_parent.trim());
        let local_parent = if remote_advanced {
            self.commit_tasks_workspace_if_changed(
                local_parent.trim(),
                "Record tiber local state before remote merge",
            )?
        } else {
            local_parent.trim().to_string()
        };
        if remote_advanced {
            let merge_base = self.task_merge_base(&local_parent, remote_parent.as_deref())?;
            self.merge_remote_tasks(merge_base.as_deref())?;
        }
        let tasks_tree = self.write_directory_tree(&self.tasks_dir())?;
        let root_tree = tasks_tree;
        let parent = match &remote_parent {
            Some(parent) => parent.trim(),
            None => local_parent.trim(),
        };
        let parents = if remote_advanced {
            vec![parent, local_parent.as_str()]
        } else {
            vec![parent]
        };
        let commit = self.commit_tree(root_tree.trim(), &parents, "Sync tiber state")?;
        self.git([
            "update-ref",
            "refs/heads/tasks",
            commit.trim(),
            local_parent.as_str(),
        ])?;
        self.push_tasks_branch_if_origin_exists()?;
        Ok(())
    }

    fn commit_tasks_workspace_if_changed(
        &self,
        parent: &str,
        message: &str,
    ) -> Result<String, Error> {
        let workspace_tree = self.write_directory_tree(&self.tasks_dir())?;
        let parent_tree = self.git(["rev-parse", &format!("{parent}^{{tree}}")])?;
        if workspace_tree.trim() == parent_tree.trim() {
            return Ok(parent.to_string());
        }
        let commit = self.commit_tree(workspace_tree.trim(), &[parent], message)?;
        self.git(["update-ref", "refs/heads/tasks", commit.trim(), parent])?;
        Ok(commit.trim().to_string())
    }

    fn fetch_origin_tasks(
        &self,
        allow_missing_remote_tasks: bool,
    ) -> Result<Option<String>, Error> {
        if !self.has_origin_remote() {
            return Ok(None);
        }
        let had_tracking_ref = git_status(
            ["show-ref", "--verify", "refs/remotes/origin/tasks"],
            Some(&self.root),
        )
        .is_ok();
        let had_local_tasks_ref = git_status(
            ["show-ref", "--verify", "refs/heads/tasks"],
            Some(&self.root),
        )
        .is_ok();
        let had_local_task_state = had_local_tasks_ref && self.local_tasks_ref_has_tasks()?;
        match self.git_with_timeout(
            ["fetch", "origin", "tasks:refs/remotes/origin/tasks"],
            REMOTE_IO_TIMEOUT,
        ) {
            Ok(_) => Ok(Some(self.git([
                "rev-parse",
                "--verify",
                "refs/remotes/origin/tasks",
            ])?)),
            Err(Error::CommandFailed { stderr, .. })
                if stderr.contains("couldn't find remote ref tasks")
                    || stderr.contains("could not find remote ref tasks") =>
            {
                if had_tracking_ref || (!allow_missing_remote_tasks && had_local_task_state) {
                    return Err(tasks_remote_rewritten_error());
                }
                self.git(["update-ref", "-d", "refs/remotes/origin/tasks"])?;
                Ok(None)
            }
            Err(Error::CommandFailed { stderr, .. })
                if is_non_fast_forward_fetch_rejection(&stderr) =>
            {
                Err(tasks_remote_rewritten_error())
            }
            Err(error) => Err(error),
        }
    }

    fn has_origin_remote(&self) -> bool {
        git_status(["remote", "get-url", "origin"], Some(&self.root)).is_ok()
    }

    fn fast_forward_local_tasks_ref_from_origin(&self, allow_populated: bool) -> Result<(), Error> {
        let local_parent = self.git(["rev-parse", "--verify", "refs/heads/tasks"])?;
        if !allow_populated && self.local_tasks_ref_has_tasks()? {
            return Ok(());
        }
        let remote_parent = match self.fetch_origin_tasks(true) {
            Ok(remote_parent) => remote_parent,
            Err(error) => return Err(implicit_remote_refresh_error(error)),
        };
        let Some(remote_parent) = remote_parent else {
            return Ok(());
        };
        if remote_parent.trim() == local_parent.trim() {
            return Ok(());
        }
        if git_status(
            [
                "merge-base",
                "--is-ancestor",
                local_parent.trim(),
                remote_parent.trim(),
            ],
            Some(&self.root),
        )
        .is_ok()
        {
            self.git([
                "update-ref",
                "refs/heads/tasks",
                remote_parent.trim(),
                local_parent.trim(),
            ])?;
        }
        Ok(())
    }

    fn local_tasks_ref_has_tasks(&self) -> Result<bool, Error> {
        if git_status(
            ["show-ref", "--verify", "refs/heads/tasks"],
            Some(&self.root),
        )
        .is_err()
        {
            return Ok(false);
        }
        let listing = self.git(["ls-tree", "-r", "--name-only", "refs/heads/tasks"])?;
        Ok(listing.lines().any(is_course_task_path))
    }

    fn task_merge_base(
        &self,
        local_ref: &str,
        remote_ref: Option<&str>,
    ) -> Result<Option<String>, Error> {
        let Some(remote_ref) = remote_ref else {
            return Ok(None);
        };
        match self.git(["merge-base", local_ref.trim(), remote_ref.trim()]) {
            Ok(merge_base) => Ok(Some(merge_base.trim().to_string())),
            Err(Error::CommandFailed { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn merge_remote_tasks(&self, base_ref: Option<&str>) -> Result<(), Error> {
        self.merge_remote_tasks_except(base_ref, &[])
    }

    fn merge_remote_tasks_except(
        &self,
        base_ref: Option<&str>,
        ignored_conflict_paths: &[ConflictPath],
    ) -> Result<(), Error> {
        let listing = self.git(["ls-tree", "-r", "--name-only", "refs/remotes/origin/tasks"])?;
        let mut remote_task_paths = std::collections::BTreeSet::new();
        let mut remote_task_stems = std::collections::BTreeSet::new();
        let mut remote_order = Vec::new();
        for path in listing.lines().filter(|line| !line.trim().is_empty()) {
            self.ensure_git_blob_size("refs/remotes/origin/tasks", path, MAX_TASK_BLOB_BYTES)?;
            let contents = self.git(["show", &format!("refs/remotes/origin/tasks:{path}")])?;
            if path == "order.md" {
                if conflict_path_is_selected(ignored_conflict_paths, path) {
                    continue;
                }
                remote_order = contents
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(str::to_string)
                    .collect();
                continue;
            }
            if !is_course_task_path(path) {
                continue;
            }
            remote_task_paths.insert(path.to_string());
            remote_task_stems.insert(task_stem(Path::new(path))?);
            if conflict_path_is_selected(ignored_conflict_paths, path) {
                continue;
            }
            let destination = self.tasks_dir().join(path);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            if destination.exists() {
                let local_contents = fs::read_to_string(&destination)?;
                if local_contents != contents {
                    match self.base_task_contents(base_ref, path)? {
                        Some(base_contents) if local_contents == base_contents => {
                            fs::write(destination, contents)?;
                        }
                        Some(base_contents) if contents == base_contents => {}
                        _ => {
                            return Err(sync_conflict_error(path));
                        }
                    }
                }
            } else if self.local_task_with_same_stem_path(path)?.is_some() {
                return Err(sync_conflict_error(path));
            } else {
                match self.base_task_contents(base_ref, path)? {
                    Some(base_contents) if contents == base_contents => {}
                    Some(_) => return Err(sync_conflict_error(path)),
                    None => fs::write(destination, contents)?,
                }
            }
        }
        if let Some(base_ref) = base_ref {
            self.apply_remote_deletions(
                base_ref,
                &remote_task_paths,
                &remote_task_stems,
                ignored_conflict_paths,
            )?;
        }
        if !remote_order.is_empty() {
            let local_order = self.order_entries()?;
            if order_conflicts(&remote_order, &local_order) {
                return Err(sync_conflict_error("order.md"));
            }
            let mut merged_order = remote_order;
            for local_entry in local_order {
                if !merged_order.contains(&local_entry) {
                    merged_order.push(local_entry);
                }
            }
            self.write_order(&merged_order)?;
        }
        self.reconcile_open_order()?;
        Ok(())
    }

    fn base_task_contents(
        &self,
        base_ref: Option<&str>,
        path: &str,
    ) -> Result<Option<String>, Error> {
        let Some(base_ref) = base_ref else {
            return Ok(None);
        };
        if git_status(
            ["cat-file", "-e", &format!("{base_ref}:{path}")],
            Some(&self.root),
        )
        .is_err()
        {
            return Ok(None);
        }
        Ok(Some(self.git(["show", &format!("{base_ref}:{path}")])?))
    }

    fn apply_remote_deletions(
        &self,
        base_ref: &str,
        remote_task_paths: &std::collections::BTreeSet<String>,
        remote_task_stems: &std::collections::BTreeSet<String>,
        ignored_conflict_paths: &[ConflictPath],
    ) -> Result<(), Error> {
        for local_path in self.task_file_refs()? {
            if remote_task_paths.contains(&local_path) {
                continue;
            }
            if conflict_path_is_selected(ignored_conflict_paths, &local_path) {
                continue;
            }
            let local_stem = task_stem(Path::new(&local_path))?;
            if remote_task_stems.contains(&local_stem) {
                continue;
            }
            let Some(base_path) = self.base_task_path_with_stem(base_ref, &local_stem)? else {
                continue;
            };
            if base_path != local_path {
                return Err(sync_conflict_error(&local_path));
            }
            let base_contents = self.git(["show", &format!("{}:{base_path}", base_ref.trim())])?;
            let local_file = self.tasks_dir().join(&local_path);
            let local_contents = fs::read_to_string(&local_file)?;
            if local_contents == base_contents {
                fs::remove_file(local_file)?;
            } else {
                return Err(sync_conflict_error(&local_path));
            }
        }
        Ok(())
    }

    fn base_task_path_with_stem(
        &self,
        base_ref: &str,
        stem: &str,
    ) -> Result<Option<String>, Error> {
        let listing = self.git(["ls-tree", "-r", "--name-only", base_ref.trim()])?;
        Ok(listing
            .lines()
            .filter(|path| is_course_task_path(path))
            .find(|path| task_stem(Path::new(path)).is_ok_and(|candidate| candidate == stem))
            .map(str::to_string))
    }

    fn local_task_with_same_stem_path(&self, remote_path: &str) -> Result<Option<String>, Error> {
        let Some(remote_stem) = Path::new(remote_path)
            .file_stem()
            .and_then(|stem| stem.to_str())
        else {
            return Ok(None);
        };
        Ok(self.task_file_refs()?.into_iter().find(|local_path| {
            local_path.as_str() != remote_path
                && Path::new(local_path)
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|local_stem| local_stem == remote_stem)
        }))
    }

    fn local_task_with_matching_conflict_path(
        &self,
        path: &ConflictPath,
    ) -> Result<Option<String>, Error> {
        Ok(self.task_file_refs()?.into_iter().find(|local_path| {
            local_path.as_str() != path.as_str() && path.matches_storage_path(local_path)
        }))
    }

    fn push_tasks_branch_if_origin_exists(&self) -> Result<(), Error> {
        if git_status(["remote", "get-url", "origin"], Some(&self.root)).is_err() {
            return Ok(());
        }
        self.git_with_timeout(
            [
                "-c",
                "core.hooksPath=/dev/null",
                "push",
                "origin",
                "refs/heads/tasks:refs/heads/tasks",
            ],
            REMOTE_IO_TIMEOUT,
        )?;
        Ok(())
    }

    fn commit_tree(&self, tree: &str, parents: &[&str], message: &str) -> Result<String, Error> {
        let mut args = vec!["commit-tree".to_string()];
        if self.commit_signing_enabled()? {
            args.push("-S".to_string());
        }
        args.push(tree.to_string());
        for parent in parents {
            args.push("-p".to_string());
            args.push(parent.to_string());
        }
        args.push("-m".to_string());
        args.push(message.to_string());
        self.git(args)
    }

    fn commit_signing_enabled(&self) -> Result<bool, Error> {
        match self.git(["config", "--bool", "commit.gpgsign"]) {
            Ok(value) => Ok(value.trim() == "true"),
            Err(Error::CommandFailed { .. }) => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn write_directory_tree(&self, directory: &Path) -> Result<String, Error> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                let tree = self.write_directory_tree(&entry.path())?;
                entries.push(format!("040000 tree {}\t{name}\n", tree.trim()));
            } else if file_type.is_file() {
                let entry_path = entry.path();
                let display_path = entry_path
                    .strip_prefix(self.tasks_dir())
                    .unwrap_or(entry_path.as_path())
                    .to_string_lossy()
                    .into_owned();
                ensure_local_file_size(&entry_path, &display_path, MAX_TASK_BLOB_BYTES)?;
                let blob =
                    self.git(["hash-object", "-w", entry_path.to_string_lossy().as_ref()])?;
                entries.push(format!("100644 blob {}\t{name}\n", blob.trim()));
            }
        }
        entries.sort();
        self.git_with_stdin(["mktree"], &entries.concat())
    }

    fn ensure_local_task_sizes(&self, directory: &Path) -> Result<(), Error> {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                self.ensure_local_task_sizes(&entry.path())?;
            } else if file_type.is_file() {
                let entry_path = entry.path();
                let display_path = entry_path
                    .strip_prefix(self.tasks_dir())
                    .unwrap_or(entry_path.as_path())
                    .to_string_lossy()
                    .into_owned();
                ensure_local_file_size(&entry_path, &display_path, MAX_TASK_BLOB_BYTES)?;
            }
        }
        Ok(())
    }

    fn create_task(&self, title: TaskTitle) -> Result<TaskPath, Error> {
        let allow_missing_remote_tasks = !self.local_tasks_ref_has_tasks()?;
        let task_path = self.create_task_unlocked(title)?;
        if let Err(error) =
            self.sync_repository_unlocked_allowing_missing_remote(allow_missing_remote_tasks)
        {
            let committed = self.task_committed_to_tasks_ref(&task_path).unwrap_or(true);
            if !committed {
                return Err(Error::TaskCreateSyncFailed {
                    source: Box::new(error),
                });
            }
            if allow_missing_remote_tasks {
                self.write_partial_create_marker(&task_path.path)?;
            }
            return Err(Error::TaskCreatedSyncFailed {
                task_path: task_path.path,
                source: Box::new(error),
            });
        }
        Ok(task_path)
    }

    fn task_committed_to_tasks_ref(&self, task_path: &TaskPath) -> Result<bool, Error> {
        match git_status(
            [
                "cat-file",
                "-e",
                &format!("refs/heads/tasks:backlog/{}.md", task_path.path),
            ],
            Some(&self.root),
        ) {
            Ok(()) => Ok(true),
            Err(Error::CommandFailed { .. }) => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn create_task_unlocked(&self, title: TaskTitle) -> Result<TaskPath, Error> {
        let tasks_dir = self.tasks_dir();
        let backlog_dir = tasks_dir.join("backlog");
        fs::create_dir_all(&backlog_dir)?;

        let nickname = self.unique_nickname(&title.file_stem())?;
        let id = new_task_id();
        let stem = format!("{id}-{nickname}");
        let task_path = format!("backlog/{stem}.md");
        let absolute_task_path = tasks_dir.join(&task_path);
        fs::write(&absolute_task_path, new_task_document(title.as_str()))?;

        let order_path = tasks_dir.join("order.md");
        let mut order = if order_path.exists() {
            fs::read_to_string(&order_path)?
        } else {
            String::new()
        };
        if !order.lines().any(|line| line == stem) {
            order.push_str(&stem);
            order.push('\n');
            fs::write(order_path, order)?;
        }

        Ok(TaskPath { path: stem })
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
            .task_file_refs()?
            .iter()
            .any(|task_ref| task_ref.ends_with(&format!("-{nickname}.md"))))
    }

    fn list_tasks(&self) -> Result<Vec<TaskSummary>, Error> {
        Ok(self
            .board_snapshot()?
            .ordered_tasks()
            .iter()
            .map(TaskSummary::from)
            .collect())
    }

    fn board_snapshot(&self) -> Result<BoardSnapshot, Error> {
        self.read_sync()?;
        let ordered_tasks = self
            .order_entries()?
            .into_iter()
            .map(|stem| {
                let path = self.resolve_task_ref(&stem)?;
                let task = fs::read_to_string(self.tasks_dir().join(&path))?;
                let title = parse_title(&task)?;
                Ok(TaskSnapshot::new(stem, title))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(BoardSnapshot::from_ordered_tasks(ordered_tasks))
    }

    fn show_task(&self, task_ref: &str) -> Result<String, Error> {
        self.read_sync()?;
        fs::read_to_string(self.tasks_dir().join(self.resolve_task_ref(task_ref)?))
            .map_err(Error::Io)
    }

    fn task_metadata(&self, task_ref: &str) -> Result<TaskMetadata, Error> {
        self.read_sync()?;
        let task_ref = self.resolve_task_ref(task_ref)?;
        let path = task_stem(&task_ref)?;
        let task = fs::read_to_string(self.tasks_dir().join(&task_ref))?;
        let title = parse_title(&task)?;
        let committed_at = self.task_committed_at(&path_to_entry(&task_ref)?)?;
        Ok(TaskMetadata {
            path,
            title,
            committed_at,
        })
    }

    fn conflict_snapshot(&self, path: &str) -> Result<ConflictSnapshot, Error> {
        let _lock = self.acquire_lock()?;
        let path = ConflictPath::parse(path)?;
        if git_status(
            ["show-ref", "--verify", "refs/heads/tasks"],
            Some(&self.root),
        )
        .is_err()
        {
            return Err(Error::Parse("tiber_not_initialized=true".to_string()));
        }
        self.fetch_origin_tasks(false)?;
        let local_side = self.task_ref_conflict_snapshot_side("refs/heads/tasks", &path)?;
        let remote_path = self.remote_conflict_path(&path)?;
        let remote = if let Some(path) = &remote_path {
            Some(self.capped_git_blob(["show", &format!("refs/remotes/origin/tasks:{path}")])?)
        } else {
            None
        };
        Ok(ConflictSnapshot {
            path: path.as_str().to_string(),
            local_path: local_side.as_ref().map(|side| side.path.clone()),
            remote_path,
            local: local_side.map(|side| side.contents),
            remote,
        })
    }

    fn resolve_conflict(&self, path: &str, side: &str) -> Result<ConflictResolution, Error> {
        Ok(self
            .resolve_conflicts(&[ConflictResolutionRequest::parse(path, side)?])?
            .remove(0))
    }

    fn resolve_conflicts(
        &self,
        resolutions: &[ConflictResolutionRequest],
    ) -> Result<Vec<ConflictResolution>, Error> {
        if resolutions.is_empty() {
            return Err(Error::Usage(
                "conflict resolve requires at least one path and side".to_string(),
            ));
        }
        let _lock = self.acquire_lock()?;
        let resolutions = resolutions
            .iter()
            .map(|resolution| Ok((ConflictPath::parse(&resolution.path)?, resolution.side)))
            .collect::<Result<Vec<_>, Error>>()?;
        let mut selected_keys = std::collections::BTreeSet::new();
        for (path, _side) in &resolutions {
            if !selected_keys.insert(path.key().to_string()) {
                return Err(Error::Parse(format!(
                    "duplicate_conflict_resolution path={}",
                    quoted_string(path.as_str())
                )));
            }
        }
        let selected_paths = resolutions
            .iter()
            .map(|(path, _side)| path.clone())
            .collect::<Vec<_>>();
        if git_status(
            ["show-ref", "--verify", "refs/heads/tasks"],
            Some(&self.root),
        )
        .is_err()
        {
            return Err(Error::Parse("tiber_not_initialized=true".to_string()));
        }
        let initial_workspace = TaskWorkspace::create()?;
        self.materialize_tasks_ref("refs/heads/tasks", initial_workspace.path())?;
        let initial_repo = self.with_tasks_dir(initial_workspace.path().to_path_buf());
        let original_selected_local_sides =
            initial_repo.selected_local_conflict_sides(&selected_paths)?;
        let original_local_parent = self.git(["rev-parse", "--verify", "refs/heads/tasks"])?;
        let mut previous_selected_remote_sides = None;
        for attempt in 0..2 {
            let local_parent = self.git(["rev-parse", "--verify", "refs/heads/tasks"])?;
            let remote_parent = self
                .fetch_origin_tasks(false)?
                .ok_or_else(|| Error::Parse("remote_tasks_missing=true".to_string()))?;
            let merge_base = self.task_merge_base(&local_parent, Some(&remote_parent))?;
            let selected_remote_sides = self.selected_remote_sides(&selected_paths)?;
            if let Some(previous) = &previous_selected_remote_sides {
                if previous != &selected_remote_sides {
                    self.git([
                        "update-ref",
                        "refs/heads/tasks",
                        original_local_parent.trim(),
                        local_parent.trim(),
                    ])?;
                    return Err(sync_conflict_error(
                        selected_paths
                            .first()
                            .map_or("order.md", ConflictPath::as_str),
                    ));
                }
            }
            let workspace = TaskWorkspace::create()?;
            self.materialize_tasks_ref("refs/heads/tasks", workspace.path())?;
            let repo = self.with_tasks_dir(workspace.path().to_path_buf());
            repo.restore_selected_local_conflict_sides(&original_selected_local_sides)?;
            repo.ensure_selected_conflict_sides(&original_selected_local_sides)?;
            repo.merge_remote_tasks_except(merge_base.as_deref(), &selected_paths)?;
            for (path, side) in &resolutions {
                match side {
                    ConflictSideChoice::Local => {
                        if let Some(local_side) = repo.local_conflict_side(path)? {
                            let local_path = repo.tasks_dir().join(&local_side.path);
                            ensure_file_size(&local_path, &local_side.path, MAX_TASK_BLOB_BYTES)?;
                        } else {
                            return Err(Error::Parse(format!(
                                "local_conflict_side_missing path={}",
                                quoted_string(path.as_str())
                            )));
                        }
                    }
                    ConflictSideChoice::Remote => {
                        repo.apply_remote_conflict_side(path)?;
                    }
                }
            }
            repo.reconcile_open_order()?;
            let root_tree = repo.write_directory_tree(&repo.tasks_dir())?;
            let commit = repo.commit_tree(
                root_tree.trim(),
                &[remote_parent.trim(), local_parent.trim()],
                "Resolve tiber conflict",
            )?;
            self.git([
                "update-ref",
                "refs/heads/tasks",
                commit.trim(),
                local_parent.trim(),
            ])?;
            match self.push_tasks_branch_if_origin_exists() {
                Ok(()) => break,
                Err(error) if attempt == 0 && is_retryable_push_failure(&error) => {
                    self.git([
                        "update-ref",
                        "refs/heads/tasks",
                        local_parent.trim(),
                        commit.trim(),
                    ])?;
                    previous_selected_remote_sides = Some(selected_remote_sides);
                }
                Err(error) => {
                    self.git([
                        "update-ref",
                        "refs/heads/tasks",
                        local_parent.trim(),
                        commit.trim(),
                    ])?;
                    return Err(error);
                }
            }
        }
        Ok(resolutions
            .into_iter()
            .map(|(path, side)| ConflictResolution {
                path: path.into_string(),
                side: side.as_str().to_string(),
            })
            .collect())
    }

    fn selected_remote_sides(
        &self,
        paths: &[ConflictPath],
    ) -> Result<std::collections::BTreeMap<String, Option<ConflictSide>>, Error> {
        paths
            .iter()
            .map(|path| {
                let side = if let Some(remote_path) = self.remote_conflict_path(path)? {
                    self.ensure_git_blob_size(
                        "refs/remotes/origin/tasks",
                        &remote_path,
                        MAX_TASK_BLOB_BYTES,
                    )?;
                    Some(ConflictSide {
                        contents: self
                            .git(["show", &format!("refs/remotes/origin/tasks:{remote_path}")])?,
                        path: remote_path,
                    })
                } else {
                    None
                };
                Ok((path.as_str().to_string(), side))
            })
            .collect()
    }

    fn selected_local_conflict_sides(
        &self,
        paths: &[ConflictPath],
    ) -> Result<Vec<(ConflictPath, ConflictSide)>, Error> {
        paths
            .iter()
            .map(|path| {
                Ok((
                    path.clone(),
                    self.local_conflict_side(path)?.ok_or_else(|| {
                        Error::Parse(format!(
                            "local_conflict_side_missing path={}",
                            quoted_string(path.as_str())
                        ))
                    })?,
                ))
            })
            .collect()
    }

    fn restore_selected_local_conflict_sides(
        &self,
        sides: &[(ConflictPath, ConflictSide)],
    ) -> Result<(), Error> {
        for (_selected_path, side) in sides {
            if is_course_task_path(&side.path) {
                if let Some(local_path) = self.local_task_with_same_stem_path(&side.path)? {
                    if local_path != side.path {
                        let local = self.tasks_dir().join(local_path);
                        if local.exists() {
                            fs::remove_file(local)?;
                        }
                    }
                }
            }
            let destination = self.tasks_dir().join(&side.path);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(destination, &side.contents)?;
        }
        Ok(())
    }

    fn ensure_selected_conflict_sides(
        &self,
        sides: &[(ConflictPath, ConflictSide)],
    ) -> Result<(), Error> {
        for (selected_path, local) in sides {
            self.ensure_selected_conflict_side(selected_path, local)?;
        }
        Ok(())
    }

    fn ensure_selected_conflict_side(
        &self,
        selected_path: &ConflictPath,
        local: &ConflictSide,
    ) -> Result<(), Error> {
        let Some(remote_path) = self.remote_conflict_path(selected_path)? else {
            if self.local_deletion_conflict_path(selected_path)?.is_some() {
                return Ok(());
            }
            return Err(Error::Parse(format!(
                "remote_conflict_side_missing path={}",
                quoted_string(selected_path.as_str())
            )));
        };
        let remote_ref = format!("refs/remotes/origin/tasks:{remote_path}");
        let remote = self
            .git(["show", &remote_ref])
            .map_err(|error| match error {
                Error::CommandFailed { .. } => Error::Parse(format!(
                    "remote_conflict_side_missing path={}",
                    quoted_string(selected_path.as_str())
                )),
                other => other,
            })?;
        if local.path == remote_path && local.contents == remote {
            return Err(Error::Parse(format!(
                "conflict_side_not_in_conflict path={}",
                quoted_string(selected_path.as_str())
            )));
        }
        Ok(())
    }

    fn remote_conflict_path(&self, path: &ConflictPath) -> Result<Option<String>, Error> {
        if git_status(
            ["show-ref", "--verify", "refs/remotes/origin/tasks"],
            Some(&self.root),
        )
        .is_err()
        {
            return Ok(None);
        }
        let remote_ref = format!("refs/remotes/origin/tasks:{}", path.as_str());
        if git_status(["cat-file", "-e", &remote_ref], Some(&self.root)).is_ok() {
            return Ok(Some(path.as_str().to_string()));
        }
        if path.as_str() == "order.md" {
            return Ok(None);
        }
        let listing = self.git(["ls-tree", "-r", "--name-only", "refs/remotes/origin/tasks"])?;
        Ok(listing
            .lines()
            .find(|remote_path| path.matches_storage_path(remote_path))
            .map(str::to_string))
    }

    fn local_conflict_side(&self, path: &ConflictPath) -> Result<Option<ConflictSide>, Error> {
        let direct = self.tasks_dir().join(path.as_str());
        if direct.exists() {
            return Ok(Some(ConflictSide {
                path: path.as_str().to_string(),
                contents: fs::read_to_string(direct)?,
            }));
        }
        if path.as_str() != "order.md" {
            if let Some(local_path) = self.local_task_with_matching_conflict_path(path)? {
                return Ok(Some(ConflictSide {
                    contents: fs::read_to_string(self.tasks_dir().join(&local_path))?,
                    path: local_path,
                }));
            }
        }
        Ok(None)
    }

    fn capped_git_blob<I, S>(&self, args: I) -> Result<String, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        git_output_capped(args, Some(&self.root), MAX_CONFLICT_SNAPSHOT_SIDE_BYTES)
    }

    fn task_ref_conflict_snapshot_side(
        &self,
        task_ref: &str,
        path: &ConflictPath,
    ) -> Result<Option<ConflictSide>, Error> {
        let direct_ref = format!("{task_ref}:{}", path.as_str());
        if git_status(["cat-file", "-e", &direct_ref], Some(&self.root)).is_ok() {
            return Ok(Some(ConflictSide {
                path: path.as_str().to_string(),
                contents: self.capped_git_blob(["show", &direct_ref])?,
            }));
        }
        if path.as_str() != "order.md" {
            if let Some(local_path) = self.task_ref_task_with_same_stem_path(task_ref, path)? {
                let refspec = format!("{task_ref}:{local_path}");
                return Ok(Some(ConflictSide {
                    contents: self.capped_git_blob(["show", &refspec])?,
                    path: local_path,
                }));
            }
        }
        Ok(None)
    }

    fn task_ref_task_with_same_stem_path(
        &self,
        task_ref: &str,
        path: &ConflictPath,
    ) -> Result<Option<String>, Error> {
        let listing = self.git(["ls-tree", "-r", "--name-only", task_ref])?;
        Ok(listing
            .lines()
            .find(|candidate| *candidate != path.as_str() && path.matches_storage_path(candidate))
            .map(str::to_string))
    }

    fn ensure_git_blob_size(&self, task_ref: &str, path: &str, limit: u64) -> Result<(), Error> {
        let object = format!("{task_ref}:{path}");
        let size = self.git(["cat-file", "-s", &object])?;
        let size = size.trim().parse::<u64>().map_err(|_| {
            Error::Parse(format!(
                "invalid_task_blob_size path={}",
                quoted_string(path)
            ))
        })?;
        if size > limit {
            return Err(Error::Parse(format!(
                "task_blob_too_large path={} bytes={} max_bytes={} recovery=\"stop; coordinate to shrink or remove the oversized Tiber task blob in refs/heads/tasks or origin/tasks without force-pushing or overwriting shared task state\"",
                quoted_string(path),
                size,
                limit
            )));
        }
        Ok(())
    }

    fn apply_remote_conflict_side(&self, path: &ConflictPath) -> Result<(), Error> {
        let Some(remote_path) = self.remote_conflict_path(path)? else {
            if let Some(local_path) = self.local_deletion_conflict_path(path)? {
                let local = self.tasks_dir().join(local_path);
                if local.exists() {
                    fs::remove_file(local)?;
                }
                return Ok(());
            }
            return Err(Error::Parse(format!(
                "remote_conflict_side_missing path={}",
                quoted_string(path.as_str())
            )));
        };
        let remote_ref = format!("refs/remotes/origin/tasks:{remote_path}");
        self.ensure_git_blob_size(
            "refs/remotes/origin/tasks",
            &remote_path,
            MAX_TASK_BLOB_BYTES,
        )?;
        let contents = self.git(["show", &remote_ref])?;
        if remote_path == "order.md" {
            fs::write(self.tasks_dir().join(remote_path), contents)?;
            return Ok(());
        }
        if let Some(local_path) = self.local_task_with_same_stem_path(&remote_path)? {
            if local_path != remote_path {
                let local = self.tasks_dir().join(local_path);
                if local.exists() {
                    fs::remove_file(local)?;
                }
            }
        }
        let destination = self.tasks_dir().join(remote_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(destination, contents)?;
        Ok(())
    }

    fn local_deletion_conflict_path(&self, path: &ConflictPath) -> Result<Option<String>, Error> {
        let Some(local_path) = self.local_conflict_side(path)?.map(|side| side.path) else {
            return Ok(None);
        };
        let remote_ref = format!("refs/remotes/origin/tasks:{local_path}");
        match git_status(["cat-file", "-e", &remote_ref], Some(&self.root)) {
            Ok(()) => Ok(None),
            Err(Error::CommandFailed { .. }) => Ok(Some(local_path)),
            Err(error) => Err(error),
        }
    }

    fn reconcile_open_order(&self) -> Result<(), Error> {
        let open_tasks = self
            .task_file_refs()?
            .iter()
            .filter(|task_ref| {
                OPEN_STATUS_DIRS
                    .iter()
                    .any(|status| task_ref.starts_with(&format!("{status}/")))
            })
            .map(|task_ref| task_stem(Path::new(task_ref)))
            .collect::<Result<Vec<_>, Error>>()?;
        let reconciliation = OrderReconciliation::reconcile(self.order_entries()?, open_tasks);
        self.write_order(reconciliation.entries())
    }

    fn task_documents_snapshot(&self) -> Result<Vec<TaskDocument>, Error> {
        let ranks = self
            .order_entries()?
            .into_iter()
            .enumerate()
            .map(|(index, stem)| (stem, index + 1))
            .collect::<std::collections::BTreeMap<_, _>>();
        self.task_file_refs()?
            .into_iter()
            .map(|task_ref| {
                let stem = task_stem(Path::new(&task_ref))?;
                let status = task_ref
                    .split_once('/')
                    .map(|(status, _)| status.to_string())
                    .ok_or_else(|| Error::Parse(format!("task_status_missing ref={task_ref}")))?;
                let contents = fs::read_to_string(self.tasks_dir().join(&task_ref))?;
                Ok(TaskDocument {
                    rank: ranks.get(&stem).copied(),
                    stem,
                    status,
                    contents,
                })
            })
            .collect()
    }

    fn task_committed_at(&self, task_ref: &str) -> Result<Option<String>, Error> {
        let branch_path = task_ref.to_string();
        let mut args = vec![
            "log".to_string(),
            "-1".to_string(),
            "--format=%cI".to_string(),
            "refs/heads/tasks".to_string(),
        ];
        if git_status(
            ["show-ref", "--verify", "refs/remotes/origin/tasks"],
            Some(&self.root),
        )
        .is_ok()
        {
            args.push("refs/remotes/origin/tasks".to_string());
        }
        args.push("--".to_string());
        args.push(branch_path);
        let committed_at = self.git(args)?;
        let committed_at = committed_at.trim();
        if committed_at.is_empty() {
            Ok(None)
        } else {
            Ok(Some(committed_at.to_string()))
        }
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
        self.read_sync()?;
        for stem in self.order_entries()? {
            let path = self.resolve_task_ref(&stem)?;
            let task = fs::read_to_string(self.tasks_dir().join(&path))?;
            if self.task_is_ready(&task)? {
                let title = parse_title(&task)?;
                return Ok(Some(TaskSummary { path: stem, title }));
            }
        }
        Ok(None)
    }

    fn read_sync(&self) -> Result<(), Error> {
        let local_parent = match self.git(["rev-parse", "--verify", "refs/heads/tasks"]) {
            Ok(local_parent) => Some(local_parent),
            Err(Error::CommandFailed { .. }) => None,
            Err(error) => return Err(error),
        };
        let remote_parent = self.fetch_origin_tasks(false)?;
        if let Some(remote_parent) = remote_parent.as_deref() {
            let merge_base = match local_parent.as_deref() {
                Some(local_parent) => self.task_merge_base(local_parent, Some(remote_parent))?,
                None => None,
            };
            self.merge_remote_tasks(merge_base.as_deref())?;
        }
        Ok(())
    }

    fn task_is_ready(&self, task: &str) -> Result<bool, Error> {
        for blocker_ref in frontmatter_array(task, "blocked_by")? {
            let Ok(blocker_path) = self.resolve_task_ref(&blocker_ref) else {
                return Ok(false);
            };
            if !blocker_path.starts_with("done") {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn transition_task(&self, task_ref: &str, status: &str) -> Result<TaskPath, Error> {
        let task_path = self.transition_task_unlocked(task_ref, status)?;
        self.sync_repository_unlocked()?;
        Ok(task_path)
    }

    fn transition_task_unlocked(&self, task_ref: &str, status: &str) -> Result<TaskPath, Error> {
        let task_ref = self.resolve_task_ref(task_ref)?;
        let status = parse_status(status)?;
        let file_name = task_ref
            .file_name()
            .ok_or_else(|| Error::Parse("task_ref_filename_missing=true".to_string()))?;
        let new_ref = PathBuf::from(status).join(file_name);

        let tasks_dir = self.tasks_dir();
        let from = tasks_dir.join(&task_ref);
        let to = tasks_dir.join(&new_ref);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(from, &to)?;
        let task = fs::read_to_string(&to)?;
        let task = if status == "in-progress" {
            upsert_frontmatter_claim(&task)?
        } else {
            remove_frontmatter_claim(&task)?
        };
        fs::write(&to, task)?;

        let old_entry = task_stem(&task_ref)?;
        let new_entry = task_stem(&new_ref)?;
        let mut order = self.order_entries()?;
        if is_open_status(status) {
            if !order
                .iter()
                .any(|entry| entry == &old_entry || entry == &new_entry)
            {
                order.push(new_entry.clone());
            }
            for entry in &mut order {
                if entry == &old_entry {
                    *entry = new_entry.clone();
                }
            }
        } else {
            order.retain(|entry| entry != &old_entry && entry != &new_entry);
        }
        self.write_order(&order)?;
        Ok(TaskPath { path: new_entry })
    }

    fn prioritize_before(&self, task_ref: &str, before_ref: &str) -> Result<(), Error> {
        self.prioritize_before_unlocked(task_ref, before_ref)?;
        self.sync_repository_unlocked()
    }

    fn prioritize_before_unlocked(&self, task_ref: &str, before_ref: &str) -> Result<(), Error> {
        let task_ref = task_stem(&self.resolve_task_ref(task_ref)?)?;
        let before_ref = task_stem(&self.resolve_task_ref(before_ref)?)?;
        let mut order = self
            .order_entries()?
            .into_iter()
            .filter(|entry| entry != &task_ref)
            .collect::<Vec<_>>();
        let before_index = order
            .iter()
            .position(|entry| entry == &before_ref)
            .ok_or_else(|| Error::Parse(format!("task_ref_missing ref={before_ref}")))?;
        order.insert(before_index, task_ref);
        self.write_order(&order)
    }

    fn link_blocks(&self, from_ref: &str, to_ref: &str) -> Result<(), Error> {
        self.link_blocks_unlocked(from_ref, to_ref)?;
        self.sync_repository_unlocked()
    }

    fn link_blocks_unlocked(&self, from_ref: &str, to_ref: &str) -> Result<(), Error> {
        let from_path = self.resolve_task_ref(from_ref)?;
        let to_path = self.resolve_task_ref(to_ref)?;
        let from_ref = task_stem(&from_path)?;
        let to_ref = task_stem(&to_path)?;
        self.update_task_frontmatter_array(&from_ref, "blocks", &to_ref, SectionOperation::Add)?;
        self.update_task_frontmatter_array(&to_ref, "blocked_by", &from_ref, SectionOperation::Add)
    }

    fn unlink_blocks(&self, from_ref: &str, to_ref: &str) -> Result<(), Error> {
        self.unlink_blocks_unlocked(from_ref, to_ref)?;
        self.sync_repository_unlocked()
    }

    fn unlink_blocks_unlocked(&self, from_ref: &str, to_ref: &str) -> Result<(), Error> {
        let from_path = self.resolve_task_ref(from_ref)?;
        let to_path = self.resolve_task_ref(to_ref)?;
        let from_ref = task_stem(&from_path)?;
        let to_ref = task_stem(&to_path)?;
        self.update_task_frontmatter_array(&from_ref, "blocks", &to_ref, SectionOperation::Remove)?;
        self.update_task_frontmatter_array(
            &to_ref,
            "blocked_by",
            &from_ref,
            SectionOperation::Remove,
        )
    }

    fn update_task_frontmatter_array(
        &self,
        task_ref: &str,
        key: &str,
        item: &str,
        operation: SectionOperation,
    ) -> Result<(), Error> {
        let path = self.tasks_dir().join(self.resolve_task_ref(task_ref)?);
        let task = fs::read_to_string(&path)?;
        fs::write(path, update_frontmatter_array(&task, key, item, operation)?)?;
        Ok(())
    }

    fn update_task_section(
        &self,
        task_ref: &str,
        heading: &str,
        item: &str,
        operation: SectionOperation,
    ) -> Result<(), Error> {
        let path = self.tasks_dir().join(self.resolve_task_ref(task_ref)?);
        let task = fs::read_to_string(&path)?;
        fs::write(
            path,
            update_markdown_section(&task, heading, item, operation),
        )?;
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
        let task_path = self.resolve_task_ref(task_ref)?;
        let task = fs::read_to_string(self.tasks_dir().join(&task_path))?;
        let subtask_id = next_subtask_id(&task);
        let after_suffix = if after_refs.is_empty() {
            String::new()
        } else {
            format!(" — after: {}", after_refs.join(", "))
        };
        self.update_task_section(
            &task_stem(&task_path)?,
            "Subtasks",
            &format!("[ ] ({subtask_id}) {title}{after_suffix}"),
            SectionOperation::Add,
        )
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
        let task_ref = self.resolve_task_ref(task_ref)?;
        let subtask_ref = parse_subtask_ref(index)?;
        let path = self.tasks_dir().join(&task_ref);
        let task = fs::read_to_string(&path)?;
        fs::write(
            path,
            update_subtask_check_state(&task, &subtask_ref, checked)?,
        )?;
        Ok(())
    }

    fn update_task(&self, task_ref: &str, update: TaskUpdate<'_>) -> Result<(), Error> {
        let task_ref = self.resolve_task_ref(task_ref)?;
        let path = self.tasks_dir().join(&task_ref);
        let mut task = fs::read_to_string(&path)?;
        if let Some(title) = update.title {
            let title = TaskTitle::parse(title)?;
            task = update_frontmatter_scalar(&task, "title", title.as_str())?;
        }
        if let Some(tags) = update.tags {
            task = update_frontmatter_array_values(&task, "tags", tags)?;
        }
        if let Some(pr_mr_url) = update.pr_mr_url {
            task = upsert_frontmatter_optional_scalar(&task, "pr_mr_url", pr_mr_url)?;
        }
        if let Some(pr_mr_status) = update.pr_mr_status {
            task = upsert_frontmatter_optional_scalar(&task, "pr_mr_status", pr_mr_status)?;
        }
        if let Some(summary) = update.summary {
            task = replace_markdown_section_body(&task, "Summary", summary)?;
        }
        if let Some(context) = update.context {
            task = replace_markdown_section_body(&task, "Context / Why", context)?;
        }
        fs::write(path, task)?;
        self.sync_repository_unlocked()
    }

    fn add_acceptance(&self, task_ref: &str, criterion: &str) -> Result<(), Error> {
        let criterion = parse_nonempty_text(criterion, "acceptance")?;
        let task_ref = self.resolve_task_ref(task_ref)?;
        let path = self.tasks_dir().join(&task_ref);
        let task = fs::read_to_string(&path)?;
        fs::write(
            path,
            update_markdown_section(
                &task,
                "Acceptance criteria",
                &format!("[ ] {criterion}"),
                SectionOperation::Add,
            ),
        )?;
        self.sync_repository_unlocked()
    }

    fn set_acceptance_checked(
        &self,
        task_ref: &str,
        index: &str,
        checked: bool,
    ) -> Result<(), Error> {
        let index = parse_one_based_usize(index, "acceptance")?;
        let task_ref = self.resolve_task_ref(task_ref)?;
        let path = self.tasks_dir().join(&task_ref);
        let task = fs::read_to_string(&path)?;
        fs::write(
            path,
            update_checklist_item(
                &task,
                "Acceptance criteria",
                index,
                ChecklistOperation::Set(checked),
            )?,
        )?;
        self.sync_repository_unlocked()
    }

    fn remove_acceptance(&self, task_ref: &str, index: &str) -> Result<(), Error> {
        let index = parse_one_based_usize(index, "acceptance")?;
        let task_ref = self.resolve_task_ref(task_ref)?;
        let path = self.tasks_dir().join(&task_ref);
        let task = fs::read_to_string(&path)?;
        fs::write(
            path,
            update_checklist_item(
                &task,
                "Acceptance criteria",
                index,
                ChecklistOperation::Remove,
            )?,
        )?;
        self.sync_repository_unlocked()
    }

    fn add_note(&self, task_ref: &str, note: &str) -> Result<(), Error> {
        let note = parse_nonempty_text(note, "note")?;
        let task_ref = self.resolve_task_ref(task_ref)?;
        let path = self.tasks_dir().join(&task_ref);
        let task = fs::read_to_string(&path)?;
        fs::write(
            path,
            update_markdown_section(
                &task,
                "Notes / Log",
                &format!("{}: {note}", current_date_string()),
                SectionOperation::Add,
            ),
        )?;
        self.sync_repository_unlocked()
    }

    fn validate_fix(&self) -> Result<Vec<ValidationMessage>, Error> {
        let messages = self.validate_fix_unlocked()?;
        self.sync_repository_unlocked()?;
        Ok(messages)
    }

    fn validate_fix_unlocked(&self) -> Result<Vec<ValidationMessage>, Error> {
        let mut messages = Vec::new();
        self.repair_legacy_task_ids(&mut messages)?;
        let task_refs = self.task_file_refs()?;
        self.report_schema_errors(&task_refs, &mut messages)?;
        self.repair_misplaced_claims(&task_refs, &mut messages)?;
        self.repair_reciprocal_links(&task_refs, &mut messages)?;
        self.report_dependency_cycles(&task_refs, &mut messages)?;
        self.report_subtask_cycles(&task_refs, &mut messages)?;
        self.reconcile_order(&task_refs, &mut messages)?;
        Ok(messages)
    }

    fn close_from_trailers(&self) -> Result<Vec<String>, Error> {
        let log = self.git(["log", "--format=%B%x00"])?;
        let mut closed = Vec::new();
        for task_ref in closes_trailers(&log) {
            let resolved = match self.resolve_task_ref(&task_ref) {
                Ok(resolved) => task_stem(&resolved)?,
                Err(_) => continue,
            };
            let done = self.transition_task_unlocked(&resolved, "done")?;
            closed.push(done.path);
        }
        closed.sort();
        closed.dedup();
        self.sync_repository_unlocked()?;
        Ok(closed)
    }

    fn scaffold_repo(&self, apply: bool) -> Result<Vec<String>, Error> {
        let _lock = if apply {
            Some(self.acquire_lock()?)
        } else {
            None
        };
        let files = vec![
            (
                ".githooks/post-commit.tiber",
                "#!/usr/bin/env bash\nset -euo pipefail\n\ntiber close-from-trailers\n"
                    .to_string(),
            ),
            (
                ".github/workflows/tiber-close-from-trailers.yml",
                "name: tiber close from trailers\n\non:\n  push:\n    branches: [main]\n\njobs:\n  close:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - name: Install Tiber\n        run: |\n          git clone --depth 1 https://github.com/jwilger/ai-plugins.git .tiber-src\n          cargo install --path .tiber-src/plugins/tiber/rust/crates/tiber-cli --bin tiber --root .tiber-install\n          echo \"$PWD/.tiber-install/bin\" >> \"$GITHUB_PATH\"\n      - run: tiber close-from-trailers\n".to_string(),
            ),
        ];
        if apply {
            for (path, contents) in &files {
                let destination = self.root.join(path);
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(destination, contents)?;
            }
        }
        Ok(files
            .into_iter()
            .map(|(path, _contents)| path.to_string())
            .collect())
    }

    fn repair_legacy_task_ids(&self, messages: &mut Vec<ValidationMessage>) -> Result<(), Error> {
        let task_refs = self.task_file_refs()?;
        let mut migrations = std::collections::BTreeMap::new();
        for task_ref in &task_refs {
            let legacy_stem = task_stem(Path::new(task_ref))?;
            if stem_parts(&legacy_stem).is_some() {
                continue;
            }
            let status = task_ref
                .split_once('/')
                .map(|(status, _name)| status)
                .ok_or_else(|| Error::Parse(format!("task_status_missing ref={task_ref}")))?;
            let new_stem = self.unique_migration_stem(&legacy_stem, &migrations)?;
            let from = self.tasks_dir().join(task_ref);
            let to = self.tasks_dir().join(status).join(format!("{new_stem}.md"));
            fs::rename(from, to)?;
            migrations.insert(legacy_stem, new_stem);
        }

        if migrations.is_empty() {
            return Ok(());
        }

        let order = self
            .order_entries()?
            .into_iter()
            .map(|entry| migrations.get(&entry).cloned().unwrap_or(entry))
            .collect::<Vec<_>>();
        self.write_order(&order)?;

        for task_ref in self.task_file_refs()? {
            let path = self.tasks_dir().join(&task_ref);
            let mut task = fs::read_to_string(&path)?;
            for key in ["blocked_by", "blocks"] {
                let values = frontmatter_array(&task, key)?
                    .into_iter()
                    .map(|value| migrations.get(&value).cloned().unwrap_or(value))
                    .collect::<Vec<_>>();
                task = update_frontmatter_array_values(&task, key, values)?;
            }
            fs::write(path, task)?;
        }

        messages.extend(
            migrations
                .into_iter()
                .map(|(old, new)| ValidationMessage(format!("fixed task-id {old} {new}"))),
        );
        Ok(())
    }

    fn unique_migration_stem(
        &self,
        legacy_stem: &str,
        migrations: &std::collections::BTreeMap<String, String>,
    ) -> Result<String, Error> {
        let existing = self
            .task_file_refs()?
            .into_iter()
            .map(|task_ref| task_stem(Path::new(&task_ref)))
            .collect::<Result<Vec<_>, Error>>()?;
        for _attempt in 0..TASK_ID_GENERATION_ATTEMPTS {
            let candidate = format!("{}-{legacy_stem}", new_task_id());
            if !existing.iter().any(|stem| stem == &candidate)
                && !migrations.values().any(|stem| stem == &candidate)
            {
                return Ok(candidate);
            }
        }
        Err(Error::Parse(format!(
            "task_id_collision legacy_stem={legacy_stem}"
        )))
    }

    fn report_schema_errors(
        &self,
        task_refs: &[String],
        messages: &mut Vec<ValidationMessage>,
    ) -> Result<(), Error> {
        for task_ref in task_refs {
            let task = fs::read_to_string(self.tasks_dir().join(task_ref))?;
            let stem = task_stem(Path::new(task_ref))?;
            if parse_title(&task).is_err() {
                messages.push(ValidationMessage(format!("schema title-missing {stem}")));
            }
            for forbidden_key in forbidden_frontmatter_keys(&task) {
                messages.push(ValidationMessage(format!(
                    "schema forbidden-key {stem} {forbidden_key}"
                )));
            }
        }
        Ok(())
    }

    fn repair_misplaced_claims(
        &self,
        task_refs: &[String],
        messages: &mut Vec<ValidationMessage>,
    ) -> Result<(), Error> {
        for task_ref in task_refs {
            if task_ref.starts_with("in-progress/") {
                continue;
            }
            let path = self.tasks_dir().join(task_ref);
            let task = fs::read_to_string(&path)?;
            let repaired = remove_frontmatter_claim(&task)?;
            if repaired != task {
                fs::write(path, repaired)?;
                messages.push(ValidationMessage(format!(
                    "fixed misplaced-claim {}",
                    task_stem(Path::new(task_ref))?
                )));
            }
        }
        Ok(())
    }

    fn repair_reciprocal_links(
        &self,
        task_refs: &[String],
        messages: &mut Vec<ValidationMessage>,
    ) -> Result<(), Error> {
        for task_ref in task_refs {
            let task = fs::read_to_string(self.tasks_dir().join(task_ref))?;
            let task_stem = task_stem(Path::new(task_ref))?;
            for blocked_ref in frontmatter_array(&task, "blocks")? {
                let Some(blocked_stem) = resolve_task_ref_to_stem(task_refs, &blocked_ref)? else {
                    messages.push(ValidationMessage(format!(
                        "dangling link {task_stem} blocks {blocked_ref}"
                    )));
                    continue;
                };
                let blocked_path = self.resolve_task_ref(&blocked_stem)?;
                let blocked_task = fs::read_to_string(self.tasks_dir().join(&blocked_path))?;
                if !frontmatter_array(&blocked_task, "blocked_by")?
                    .iter()
                    .any(|candidate| resolves_to(candidate, &task_stem))
                {
                    self.update_task_frontmatter_array(
                        &blocked_stem,
                        "blocked_by",
                        &task_stem,
                        SectionOperation::Add,
                    )?;
                    messages.push(ValidationMessage(format!(
                        "fixed reciprocal-link {blocked_stem} blocked-by {task_stem}"
                    )));
                }
            }
            for blocker_ref in frontmatter_array(&task, "blocked_by")? {
                let Some(blocker_stem) = resolve_task_ref_to_stem(task_refs, &blocker_ref)? else {
                    messages.push(ValidationMessage(format!(
                        "dangling link {task_stem} blocked-by {blocker_ref}"
                    )));
                    continue;
                };
                let blocker_path = self.resolve_task_ref(&blocker_stem)?;
                let blocker_task = fs::read_to_string(self.tasks_dir().join(&blocker_path))?;
                if !frontmatter_array(&blocker_task, "blocks")?
                    .iter()
                    .any(|candidate| resolves_to(candidate, &task_stem))
                {
                    self.update_task_frontmatter_array(
                        &blocker_stem,
                        "blocks",
                        &task_stem,
                        SectionOperation::Add,
                    )?;
                    messages.push(ValidationMessage(format!(
                        "fixed reciprocal-link {blocker_stem} blocks {task_stem}"
                    )));
                }
            }
        }
        Ok(())
    }

    fn report_dependency_cycles(
        &self,
        task_refs: &[String],
        messages: &mut Vec<ValidationMessage>,
    ) -> Result<(), Error> {
        let graph = DependencyGraph::from_tasks(
            task_refs
                .iter()
                .map(|task_ref| {
                    let task = fs::read_to_string(self.tasks_dir().join(task_ref))?;
                    let task_stem = task_stem(Path::new(task_ref))?;
                    let blocks = frontmatter_array(&task, "blocks")?
                        .into_iter()
                        .filter_map(|blocked_ref| {
                            resolve_task_ref_to_stem(task_refs, &blocked_ref)
                                .ok()
                                .flatten()
                        })
                        .collect::<Vec<_>>();
                    Ok(TaskDependencies::new(task_stem, blocks))
                })
                .collect::<Result<Vec<_>, Error>>()?,
        );
        messages.extend(graph.cycle_messages().into_iter().map(ValidationMessage));
        Ok(())
    }

    fn report_subtask_cycles(
        &self,
        task_refs: &[String],
        messages: &mut Vec<ValidationMessage>,
    ) -> Result<(), Error> {
        for task_ref in task_refs {
            let task = fs::read_to_string(self.tasks_dir().join(task_ref))?;
            let stem = task_stem(Path::new(task_ref))?;
            messages.extend(
                subtask_cycle_messages(&stem, &task)
                    .into_iter()
                    .map(ValidationMessage),
            );
        }
        Ok(())
    }

    fn reconcile_order(
        &self,
        task_refs: &[String],
        messages: &mut Vec<ValidationMessage>,
    ) -> Result<(), Error> {
        let open_tasks = task_refs
            .iter()
            .filter(|task_ref| {
                OPEN_STATUS_DIRS
                    .iter()
                    .any(|status| task_ref.starts_with(&format!("{status}/")))
            })
            .map(|task_ref| task_stem(Path::new(task_ref)))
            .collect::<Result<Vec<_>, Error>>()?;
        let reconciliation = OrderReconciliation::reconcile(self.order_entries()?, open_tasks);
        messages.extend(
            reconciliation
                .messages()
                .iter()
                .cloned()
                .map(ValidationMessage),
        );
        self.write_order(reconciliation.entries())
    }

    fn task_file_refs(&self) -> Result<Vec<String>, Error> {
        let mut refs = Vec::new();
        let tasks_dir = self.tasks_dir();
        if !tasks_dir.exists() {
            return Ok(refs);
        }
        for status_name in STATUS_DIRS {
            let status_dir = tasks_dir.join(status_name);
            if !status_dir.is_dir() {
                continue;
            }
            for task in fs::read_dir(status_dir)? {
                let task = task?;
                if task.file_type()?.is_file()
                    && task
                        .path()
                        .extension()
                        .is_some_and(|extension| extension == "md")
                {
                    refs.push(format!(
                        "{status_name}/{}",
                        task.file_name().to_string_lossy()
                    ));
                }
            }
        }
        refs.sort();
        Ok(refs)
    }

    fn resolve_task_ref(&self, task_ref: &str) -> Result<PathBuf, Error> {
        if task_ref.contains('/') || task_ref.ends_with(".md") || task_ref.trim().is_empty() {
            return Err(task_ref_invalid(task_ref));
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
            [] => Err(task_ref_missing(task_ref)),
            _ => Err(task_ref_ambiguous(task_ref, matches)),
        }
    }

    fn order_entries(&self) -> Result<Vec<String>, Error> {
        let order_path = self.tasks_dir().join("order.md");
        if !order_path.exists() {
            return Ok(Vec::new());
        }

        fs::read_to_string(order_path)?
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                if line.contains('/') || line.ends_with(".md") {
                    return Err(Error::Parse(format!("invalid_order_entry ref={line}")));
                }
                Ok(line.to_string())
            })
            .collect::<Result<Vec<_>, Error>>()
    }

    fn write_order(&self, entries: &[String]) -> Result<(), Error> {
        fs::write(
            self.tasks_dir().join("order.md"),
            entries
                .iter()
                .map(|entry| format!("{entry}\n"))
                .collect::<String>(),
        )?;
        Ok(())
    }

    fn tasks_dir(&self) -> PathBuf {
        self.tasks_dir
            .clone()
            .expect("task operations require a materialized Git tree workspace")
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
            match self.try_acquire_lock_once() {
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

    fn try_acquire_lock_once(&self) -> Result<TiberLock, Error> {
        let lock_dir = self.git_common_dir()?.join("tiber");
        fs::create_dir_all(&lock_dir)?;
        let lock_path = lock_dir.join("tiber.lock");
        if let Some(stale_contents) = stale_lock_contents(&lock_path)? {
            if fs::read_to_string(&lock_path)
                .ok()
                .as_deref()
                .is_some_and(|current_contents| current_contents == stale_contents)
            {
                let _ = fs::remove_file(&lock_path);
            }
        }
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(_) => {
                fs::write(&lock_path, lock_metadata())?;
                Ok(TiberLock { path: lock_path })
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Err(Error::Parse(
                format!("tiber_lock_busy path={}", path_to_entry(&lock_path)?),
            )),
            Err(error) => Err(Error::Io(error)),
        }
    }

    fn partial_create_marker_path(&self) -> Result<PathBuf, Error> {
        Ok(self
            .git_common_dir()?
            .join("tiber")
            .join("partial-create-sync"))
    }

    fn write_partial_create_marker(&self, task_path: &str) -> Result<(), Error> {
        let marker_path = self.partial_create_marker_path()?;
        if let Some(parent) = marker_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let local_parent = self.git(["rev-parse", "--verify", "refs/heads/tasks"])?;
        fs::write(
            marker_path,
            format!("task_path={task_path}\ntasks_ref={}", local_parent.trim()),
        )?;
        Ok(())
    }

    fn partial_create_marker_allows_missing_remote(&self) -> Result<bool, Error> {
        let marker_path = self.partial_create_marker_path()?;
        let marker = match fs::read_to_string(marker_path) {
            Ok(marker) => marker,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(Error::Io(error)),
        };
        let Some(marker_tasks_ref) = marker
            .lines()
            .find_map(|line| line.strip_prefix("tasks_ref="))
        else {
            return Ok(false);
        };
        let local_parent = self.git(["rev-parse", "--verify", "refs/heads/tasks"])?;
        Ok(local_parent.trim() == marker_tasks_ref.trim())
    }

    fn clear_partial_create_marker(&self) -> Result<(), Error> {
        let marker_path = self.partial_create_marker_path()?;
        match fs::remove_file(marker_path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(Error::Io(error)),
        }
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

    fn git_with_timeout<I, S>(&self, args: I, timeout: Duration) -> Result<String, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        git_output_with_timeout(args, Some(&self.root), timeout)
    }

    fn git_with_stdin<I, S>(&self, args: I, stdin: &str) -> Result<String, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args = args
            .into_iter()
            .map(|arg| arg.as_ref().to_owned())
            .collect::<Vec<_>>();
        let mut child = Command::new("git")
            .args(&args)
            .current_dir(&self.root)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C")
            .env("LANGUAGE", "C")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        {
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .ok_or_else(|| Error::Parse("stdin_unavailable=true".to_string()))?
                .write_all(stdin.as_bytes())?;
        }
        command_output("git", &args, child.wait_with_output()?)
    }
}

struct TiberLock {
    path: PathBuf,
}

impl Drop for TiberLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

struct TaskWorkspace {
    path: PathBuf,
}

impl TaskWorkspace {
    fn create() -> Result<Self, Error> {
        static TASK_WORKSPACE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let sequence = TASK_WORKSPACE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "tiber-task-tree-{}-{unique}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TaskWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Clone, Copy)]
enum SectionOperation {
    Add,
    Remove,
}

enum ChecklistOperation {
    Set(bool),
    Remove,
}

fn parse_title(task: &str) -> Result<String, Error> {
    if let Some(frontmatter) = task.strip_prefix("---\n") {
        for line in frontmatter.lines() {
            if line == "---" {
                break;
            }
            if let Some(title) = line.strip_prefix("title: ") {
                let title = title.trim();
                if !title.is_empty() {
                    return Ok(title.to_string());
                }
            }
        }
    }
    task.lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(str::to_string)
        .ok_or_else(|| Error::Parse("task_title_missing=true".to_string()))
}

fn frontmatter(document: &str) -> Result<&str, Error> {
    let Some(rest) = document.strip_prefix("---\n") else {
        return Err(Error::Parse("frontmatter_missing=true".to_string()));
    };
    rest.split_once("\n---\n")
        .map(|(frontmatter, _body)| frontmatter)
        .ok_or_else(|| Error::Parse("frontmatter_unclosed=true".to_string()))
}

fn frontmatter_array(document: &str, key: &str) -> Result<Vec<String>, Error> {
    let prefix = format!("{key}: ");
    for line in frontmatter(document)?.lines() {
        if let Some(value) = line.strip_prefix(&prefix) {
            return parse_inline_array(value);
        }
    }
    Ok(Vec::new())
}

fn forbidden_frontmatter_keys(document: &str) -> Vec<String> {
    let forbidden = ["id", "nickname", "status", "created", "updated"];
    let Ok(frontmatter) = frontmatter(document) else {
        return Vec::new();
    };
    frontmatter
        .lines()
        .filter_map(|line| line.split_once(':').map(|(key, _value)| key.trim()))
        .filter(|key| forbidden.contains(key))
        .map(str::to_string)
        .collect()
}

fn remove_frontmatter_claim(document: &str) -> Result<String, Error> {
    let Some(rest) = document.strip_prefix("---\n") else {
        return Ok(document.to_string());
    };
    let Some((frontmatter, body)) = rest.split_once("\n---\n") else {
        return Ok(document.to_string());
    };
    let mut lines = Vec::new();
    let mut skipping_claim = false;
    let mut removed = false;
    for line in frontmatter.lines() {
        if line == "claim:" {
            skipping_claim = true;
            removed = true;
            continue;
        }
        if skipping_claim {
            if line.starts_with(' ') || line.starts_with('\t') || line.trim().is_empty() {
                continue;
            }
            skipping_claim = false;
        }
        lines.push(line.to_string());
    }
    if removed {
        Ok(format!("---\n{}\n---\n{body}", lines.join("\n")))
    } else {
        Ok(document.to_string())
    }
}

fn upsert_frontmatter_claim(document: &str) -> Result<String, Error> {
    let document = remove_frontmatter_claim(document)?;
    let Some(rest) = document.strip_prefix("---\n") else {
        return Err(Error::Parse("frontmatter_missing=true".to_string()));
    };
    let Some((frontmatter, body)) = rest.split_once("\n---\n") else {
        return Err(Error::Parse("frontmatter_unclosed=true".to_string()));
    };
    let mut frontmatter = frontmatter.to_string();
    if !frontmatter.is_empty() && !frontmatter.ends_with('\n') {
        frontmatter.push('\n');
    }
    frontmatter.push_str("claim:\n");
    frontmatter.push_str(&format!("  host: {}\n", claim_host()));
    frontmatter.push_str(&format!("  session: {}\n", claim_session()));
    Ok(format!("---\n{frontmatter}---\n{body}"))
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

fn new_task_document(title: &str) -> String {
    format!(
        "---\ntitle: {title}\nblocked_by: []\nblocks: []\ntags: []\npr_mr_url: \npr_mr_status: \n---\n\n## Summary\n\n## Context / Why\n\n## Acceptance criteria\n\n## Subtasks\n\n## Notes / Log\n"
    )
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

fn is_course_task_path(path: &str) -> bool {
    let path = Path::new(path);
    let mut components = path.components();
    let Some(status) = components
        .next()
        .and_then(|component| component.as_os_str().to_str())
    else {
        return false;
    };
    STATUS_DIRS.contains(&status)
        && components.next().is_some()
        && components.next().is_none()
        && path.extension().is_some_and(|extension| extension == "md")
}

fn is_status_gitkeep_path(path: &str) -> bool {
    let path = Path::new(path);
    let mut components = path.components();
    let Some(status) = components
        .next()
        .and_then(|component| component.as_os_str().to_str())
    else {
        return false;
    };
    STATUS_DIRS.contains(&status)
        && components
            .next()
            .is_some_and(|component| component.as_os_str() == ".gitkeep")
        && components.next().is_none()
}

fn invalid_conflict_path(path: &str) -> Error {
    Error::Parse(format!(
        "invalid_conflict_path path={}",
        quoted_string(path)
    ))
}

fn conflict_path_is_selected(selected_paths: &[ConflictPath], path: &str) -> bool {
    selected_paths
        .iter()
        .any(|selected| selected.matches_storage_path(path))
}

fn tasks_remote_rewritten_error() -> Error {
    Error::Parse(
        "tasks_remote_rewritten recovery=\"stop and inspect origin/tasks; do not force-push or overwrite shared task state without human coordination\""
            .to_string(),
    )
}

fn implicit_remote_refresh_error(error: Error) -> Error {
    Error::Parse(format!(
        "tasks_remote_refresh_failed source={}",
        error.sanitized_sync_source()
    ))
}

fn stem_parts(stem: &str) -> Option<(&str, &str)> {
    let (date, rest) = stem.split_once('-')?;
    let (code, nickname) = rest.split_once('-')?;
    if date.len() == 8 && code.len() == 4 && !nickname.is_empty() {
        Some((&stem[..13], nickname))
    } else {
        None
    }
}

fn resolves_to(candidate: &str, target_stem: &str) -> bool {
    if candidate == target_stem {
        return true;
    }
    let Some((id, nickname)) = stem_parts(target_stem) else {
        return false;
    };
    candidate == id || candidate == nickname
}

fn resolve_task_ref_to_stem(task_refs: &[String], task_ref: &str) -> Result<Option<String>, Error> {
    let mut matches = task_refs
        .iter()
        .filter_map(|candidate| task_stem(Path::new(candidate)).ok())
        .filter(|stem| resolves_to(task_ref, stem))
        .collect::<Vec<_>>();
    matches.sort();
    matches.dedup();
    match matches.as_slice() {
        [stem] => Ok(Some(stem.clone())),
        [] => Ok(None),
        _ => Err(task_ref_ambiguous(task_ref, matches)),
    }
}

fn new_task_id() -> String {
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

fn update_markdown_section(
    document: &str,
    heading: &str,
    item: &str,
    operation: SectionOperation,
) -> String {
    let heading_line = format!("## {heading}");
    let item_line = format!("- {item}");
    let (mut before, mut section, mut after, _) = split_markdown_section(document, heading);

    section.retain(|line| !line.trim().is_empty() && line != &item_line);
    if matches!(operation, SectionOperation::Add) {
        section.push(item_line);
    }
    trim_blank_edges(&mut before);
    trim_blank_edges(&mut after);

    let mut sections = Vec::new();
    let before = before.join("\n");
    if !before.is_empty() {
        sections.push(before);
    }
    if !section.is_empty() {
        sections.push(format!("{heading_line}\n\n{}", section.join("\n")));
    }
    let after = after.join("\n");
    if !after.is_empty() {
        sections.push(after);
    }

    format!("{}\n", sections.join("\n\n"))
}

fn markdown_section_items(document: &str, heading: &str) -> Vec<String> {
    let heading_line = format!("## {heading}");
    let mut in_section = false;
    let mut items = Vec::new();

    for line in document.lines() {
        if line == heading_line {
            in_section = true;
            continue;
        }
        if in_section && line.starts_with("## ") {
            break;
        }
        if in_section {
            if let Some(item) = line.strip_prefix("- ") {
                items.push(item.to_string());
            }
        }
    }

    items
}

fn update_frontmatter_array(
    document: &str,
    key: &str,
    item: &str,
    operation: SectionOperation,
) -> Result<String, Error> {
    let Some(rest) = document.strip_prefix("---\n") else {
        return Err(Error::Parse("frontmatter_missing=true".to_string()));
    };
    let Some((frontmatter, body)) = rest.split_once("\n---\n") else {
        return Err(Error::Parse("frontmatter_unclosed=true".to_string()));
    };
    let prefix = format!("{key}: ");
    let mut found = false;
    let mut lines = Vec::new();
    for line in frontmatter.lines() {
        if let Some(raw_array) = line.strip_prefix(&prefix) {
            found = true;
            let mut values = parse_inline_array(raw_array)?;
            values.retain(|value| value != item);
            if matches!(operation, SectionOperation::Add) {
                values.push(item.to_string());
                values.sort();
                values.dedup();
            }
            lines.push(format!("{key}: [{}]", values.join(", ")));
        } else {
            lines.push(line.to_string());
        }
    }
    if !found {
        return Err(Error::Parse(format!("frontmatter_key_missing key={key}")));
    }
    Ok(format!("---\n{}\n---\n{body}", lines.join("\n")))
}

fn update_frontmatter_scalar(document: &str, key: &str, value: &str) -> Result<String, Error> {
    update_frontmatter_line(document, key, &format!("{key}: {value}"))
}

fn upsert_frontmatter_optional_scalar(
    document: &str,
    key: &str,
    value: &str,
) -> Result<String, Error> {
    let value = frontmatter_optional_scalar_value(value);
    match update_frontmatter_scalar(document, key, &value) {
        Ok(updated) => Ok(updated),
        Err(Error::Parse(message)) if message == format!("frontmatter_key_missing key={key}") => {
            insert_frontmatter_line(document, &format!("{key}: {value}"))
        }
        Err(error) => Err(error),
    }
}

fn frontmatter_optional_scalar_value(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|character| {
            if character.is_control() {
                '-'
            } else {
                character
            }
        })
        .collect::<String>()
}

fn update_frontmatter_array_values(
    document: &str,
    key: &str,
    values: Vec<String>,
) -> Result<String, Error> {
    update_frontmatter_line(document, key, &format!("{key}: [{}]", values.join(", ")))
}

fn update_frontmatter_line(document: &str, key: &str, replacement: &str) -> Result<String, Error> {
    let Some(rest) = document.strip_prefix("---\n") else {
        return Err(Error::Parse("frontmatter_missing=true".to_string()));
    };
    let Some((frontmatter, body)) = rest.split_once("\n---\n") else {
        return Err(Error::Parse("frontmatter_unclosed=true".to_string()));
    };
    let prefix = format!("{key}:");
    let mut found = false;
    let lines = frontmatter
        .lines()
        .map(|line| {
            if line.starts_with(&prefix) {
                found = true;
                replacement.to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>();
    if !found {
        return Err(Error::Parse(format!("frontmatter_key_missing key={key}")));
    }
    Ok(format!("---\n{}\n---\n{body}", lines.join("\n")))
}

fn insert_frontmatter_line(document: &str, line: &str) -> Result<String, Error> {
    let Some(rest) = document.strip_prefix("---\n") else {
        return Err(Error::Parse("frontmatter_missing=true".to_string()));
    };
    let Some((frontmatter, body)) = rest.split_once("\n---\n") else {
        return Err(Error::Parse("frontmatter_unclosed=true".to_string()));
    };
    let mut frontmatter = frontmatter.to_string();
    if !frontmatter.is_empty() && !frontmatter.ends_with('\n') {
        frontmatter.push('\n');
    }
    frontmatter.push_str(line);
    frontmatter.push('\n');
    Ok(format!("---\n{frontmatter}---\n{body}"))
}

fn replace_markdown_section_body(
    document: &str,
    heading: &str,
    body: &str,
) -> Result<String, Error> {
    let body = parse_nonempty_text(body, "section")?;
    let heading_line = format!("## {heading}");
    let (mut before, _section, mut after, found) = split_markdown_section(document, heading);
    if !found {
        return Err(Error::Parse(format!("section_missing heading={heading}")));
    }
    trim_blank_edges(&mut before);
    trim_blank_edges(&mut after);
    let mut sections = Vec::new();
    let before = before.join("\n");
    if !before.is_empty() {
        sections.push(before);
    }
    sections.push(format!("{heading_line}\n\n{body}"));
    let after = after.join("\n");
    if !after.is_empty() {
        sections.push(after);
    }
    Ok(format!("{}\n", sections.join("\n\n")))
}

fn update_checklist_item(
    document: &str,
    heading: &str,
    target_index: usize,
    operation: ChecklistOperation,
) -> Result<String, Error> {
    let heading_line = format!("## {heading}");
    let (mut before, mut section, mut after, found) = split_markdown_section(document, heading);
    if !found {
        return Err(Error::Parse(format!("section_missing heading={heading}")));
    }
    let mut current_index = 0;
    let mut changed = false;
    let mut updated = Vec::new();
    for line in section.drain(..) {
        let item = line
            .strip_prefix("- [ ] ")
            .or_else(|| line.strip_prefix("- [x] "));
        if let Some(item) = item {
            current_index += 1;
            if current_index == target_index {
                changed = true;
                match operation {
                    ChecklistOperation::Set(checked) => {
                        updated.push(format!("- [{}] {item}", if checked { "x" } else { " " }));
                    }
                    ChecklistOperation::Remove => {}
                }
                continue;
            }
        }
        updated.push(line);
    }
    if !changed {
        return Err(Error::Parse(format!(
            "checklist_item_missing index={target_index}"
        )));
    }
    trim_blank_edges(&mut before);
    trim_blank_edges(&mut after);
    updated.retain(|line| !line.trim().is_empty());
    let mut sections = Vec::new();
    let before = before.join("\n");
    if !before.is_empty() {
        sections.push(before);
    }
    if updated.is_empty() {
        sections.push(heading_line);
    } else {
        sections.push(format!("{heading_line}\n\n{}", updated.join("\n")));
    }
    let after = after.join("\n");
    if !after.is_empty() {
        sections.push(after);
    }
    Ok(format!("{}\n", sections.join("\n\n")))
}

fn parse_inline_array(value: &str) -> Result<Vec<String>, Error> {
    let value = value.trim();
    let Some(inner) = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    else {
        return Err(Error::Parse(format!("invalid_inline_array value={value}")));
    };
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(inner
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect())
}

fn subtask_cycle_messages(task_stem: &str, document: &str) -> Vec<String> {
    let dependencies = parse_subtask_dependencies(document);
    let graph = DependencyGraph::from_tasks(
        dependencies
            .iter()
            .map(|(subtask_id, after)| TaskDependencies::new(subtask_id.clone(), after.clone()))
            .collect(),
    );
    graph
        .cycle_messages_with_label("subtask")
        .into_iter()
        .map(|message| {
            message
                .strip_prefix("cycle subtask ")
                .map(|cycle| format!("cycle subtask {task_stem}:{cycle}"))
                .unwrap_or(message)
        })
        .collect()
}

fn parse_subtask_dependencies(document: &str) -> Vec<(String, Vec<String>)> {
    markdown_section_items(document, "Subtasks")
        .into_iter()
        .filter_map(|item| {
            let item = item
                .strip_prefix("[ ] ")
                .or_else(|| item.strip_prefix("[x] "))?;
            let id_start = item.find("(s")?;
            let after_id_start = id_start + 1;
            let after_id_end = item[after_id_start..].find(')')? + after_id_start;
            let subtask_id = item[after_id_start..after_id_end].to_string();
            let after = item
                .find("after:")
                .map(|after_index| {
                    item[after_index + "after:".len()..]
                        .split(',')
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Some((subtask_id, after))
        })
        .collect()
}

fn next_subtask_id(document: &str) -> String {
    let next = document
        .lines()
        .filter_map(|line| {
            line.find("(s").and_then(|start| {
                let rest = &line[start + 2..];
                rest.find(')')
                    .and_then(|end| rest[..end].parse::<usize>().ok())
            })
        })
        .max()
        .unwrap_or(0)
        + 1;
    format!("s{next}")
}

fn split_markdown_section(
    document: &str,
    heading: &str,
) -> (Vec<String>, Vec<String>, Vec<String>, bool) {
    let heading_line = format!("## {heading}");
    let mut before = Vec::new();
    let mut section = Vec::new();
    let mut after = Vec::new();
    let mut split = SectionSplit::Before;
    let mut found = false;

    for line in document.lines() {
        match split {
            SectionSplit::Before if line == heading_line => {
                found = true;
                split = SectionSplit::Section;
            }
            SectionSplit::Before => before.push(line.to_string()),
            SectionSplit::Section if line.starts_with("## ") => {
                split = SectionSplit::After;
                after.push(line.to_string());
            }
            SectionSplit::Section => section.push(line.to_string()),
            SectionSplit::After => after.push(line.to_string()),
        }
    }

    (before, section, after, found)
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

fn is_non_fast_forward_fetch_rejection(stderr: &str) -> bool {
    stderr.contains("non-fast-forward") || stderr.contains("would clobber existing")
}

enum SectionSplit {
    Before,
    Section,
    After,
}

fn update_subtask_check_state(
    document: &str,
    target_ref: &str,
    checked: bool,
) -> Result<String, Error> {
    let heading = "Subtasks";
    let heading_line = format!("## {heading}");
    let (mut before, mut section, mut after, section_found) =
        split_markdown_section(document, heading);
    if !section_found {
        return Err(Error::Parse(format!("section_missing heading={heading}")));
    }

    let target_marker = format!("({target_ref})");
    let mut found = false;
    let mut updated = Vec::new();
    for line in section.drain(..) {
        let title = line
            .strip_prefix("- [ ] ")
            .or_else(|| line.strip_prefix("- [x] "));
        if let Some(title) = title {
            if title.starts_with(&target_marker) {
                found = true;
                updated.push(format!("- [{}] {title}", if checked { "x" } else { " " }));
                continue;
            }
        }
        updated.push(line);
    }

    if !found {
        return Err(Error::Parse(format!("subtask_missing ref={target_ref}")));
    }

    trim_blank_edges(&mut before);
    trim_blank_edges(&mut after);
    updated.retain(|line| !line.trim().is_empty());
    let mut sections = Vec::new();
    let before = before.join("\n");
    if !before.is_empty() {
        sections.push(before);
    }
    if updated.is_empty() {
        sections.push(heading_line);
    } else {
        sections.push(format!("{heading_line}\n\n{}", updated.join("\n")));
    }
    let after = after.join("\n");
    if !after.is_empty() {
        sections.push(after);
    }
    Ok(format!("{}\n", sections.join("\n\n")))
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

fn current_date_string() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let days = (now.as_secs() / 86_400) as i64;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn trim_blank_edges(lines: &mut Vec<String>) {
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
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

#[cfg(unix)]
fn install_launcher(launcher: &Path, installed: &Path) -> Result<(), Error> {
    create_symlink(launcher, installed)
}

#[cfg(not(unix))]
fn install_launcher(launcher: &Path, installed: &Path) -> Result<(), Error> {
    fs::copy(launcher, installed)?;
    Ok(())
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> Result<(), Error> {
    symlink(target, link)?;
    Ok(())
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

fn git_output_with_timeout<I, S>(
    args: I,
    cwd: Option<&Path>,
    timeout: Duration,
) -> Result<String, Error>
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
    command.env("GIT_TERMINAL_PROMPT", "0");
    command.env("LC_ALL", "C");
    command.env("LANGUAGE", "C");
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let started = SystemTime::now();
    loop {
        if let Some(_status) = child.try_wait()? {
            return command_output("git", &args, child.wait_with_output()?);
        }
        if started.elapsed().unwrap_or_default() >= timeout {
            let _ = child.kill();
            let output = child.wait_with_output()?;
            return Err(Error::CommandFailed {
                program: "git".to_string(),
                args: args
                    .iter()
                    .map(|arg| arg.to_string_lossy().into_owned())
                    .collect(),
                status: "timeout".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn git_output_capped<I, S>(args: I, cwd: Option<&Path>, limit: usize) -> Result<String, Error>
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
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let mut stdout = child.stdout.take().expect("stdout should be piped");
    let mut bytes = Vec::new();
    stdout
        .by_ref()
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)?;
    drop(stdout);
    let truncated = bytes.len() > limit;
    if truncated {
        let _ = child.kill();
        let _ = child.wait();
        bytes.truncate(limit);
        return Ok(capped_utf8_text(bytes, limit, true));
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(Error::CommandFailed {
            program: "git".to_string(),
            args: args
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(capped_utf8_text(bytes, limit, false))
}

fn ensure_file_size(path: &Path, display_path: &str, limit: u64) -> Result<(), Error> {
    let size = fs::metadata(path)?.len();
    if size > limit {
        return Err(Error::Parse(format!(
            "task_blob_too_large path={} bytes={} max_bytes={} recovery=\"stop; coordinate to shrink or remove the oversized Tiber task blob in refs/heads/tasks or origin/tasks without force-pushing or overwriting shared task state\"",
            quoted_string(display_path),
            size,
            limit
        )));
    }
    Ok(())
}

fn ensure_local_file_size(path: &Path, display_path: &str, limit: u64) -> Result<(), Error> {
    let size = fs::metadata(path)?.len();
    if size > limit {
        return Err(Error::Parse(format!(
            "task_blob_too_large path={} bytes={} max_bytes={} recovery=\"reduce this local Tiber task below the size limit, rerun the command, and do not publish the oversized task blob to shared refs\"",
            quoted_string(display_path),
            size,
            limit
        )));
    }
    Ok(())
}

fn capped_utf8_text(mut bytes: Vec<u8>, limit: usize, truncated: bool) -> String {
    if !truncated {
        return String::from_utf8_lossy(&bytes).into_owned();
    }
    while std::str::from_utf8(&bytes).is_err() {
        bytes.pop();
    }
    format!(
        "{}\n[truncated: conflict side exceeded {limit} bytes]",
        String::from_utf8_lossy(&bytes)
    )
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

fn order_conflicts(remote_order: &[String], local_order: &[String]) -> bool {
    let local_common = local_order
        .iter()
        .filter(|entry| remote_order.contains(entry))
        .collect::<Vec<_>>();
    let remote_common = remote_order
        .iter()
        .filter(|entry| local_order.contains(entry))
        .collect::<Vec<_>>();
    local_common != remote_common
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
mod tests {
    use super::{command_failure_needs_redaction, is_status_gitkeep_path};

    #[test]
    fn status_gitkeep_path_rejects_parent_components_and_non_status_paths() {
        assert!(is_status_gitkeep_path("backlog/.gitkeep"));
        assert!(is_status_gitkeep_path("in-progress/.gitkeep"));
        assert!(!is_status_gitkeep_path("../.gitkeep"));
        assert!(!is_status_gitkeep_path("backlog/../.gitkeep"));
        assert!(!is_status_gitkeep_path("backlog/nested/.gitkeep"));
        assert!(!is_status_gitkeep_path("unknown/.gitkeep"));
    }

    #[test]
    fn remote_command_redaction_detects_git_config_prefixes() {
        let args = vec![
            "-c".to_string(),
            "core.hooksPath=/dev/null".to_string(),
            "push".to_string(),
            "origin".to_string(),
        ];
        assert!(command_failure_needs_redaction(&args));
        assert!(!command_failure_needs_redaction(&[
            "commit-tree".to_string(),
            "HEAD".to_string()
        ]));
    }
}
