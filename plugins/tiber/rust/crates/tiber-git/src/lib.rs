use serde::Deserialize;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::fs::{OpenOptions, TryLockError};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
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
const DEFAULT_LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(50);
const CONFIG_FILE: &str = ".tiber.toml";
const MAX_SYNC_ATTEMPTS: usize = 8;

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

pub fn prioritize_before_at(
    root: impl Into<PathBuf>,
    task_ref: &str,
    before_ref: &str,
) -> Result<(), Error> {
    let repo = GitRepository::at(root);
    repo.with_task_workspace(|repo| repo.prioritize_before(task_ref, before_ref))
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
        self.ensure_tasks_branch()?;
        self.ignore_local_tasks_in_source_gitignore()?;
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
    repo.with_task_workspace(|repo| repo.update_task(task_ref, update))
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

#[derive(Debug)]
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
    BacklogCapacityExceeded {
        queued: usize,
        max_queued: usize,
    },
    Io(std::io::Error),
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
        }
    }
}

impl std::error::Error for Error {}

impl Error {
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
            Self::TaskCreatedSyncFailed { .. } | Self::TaskCreateSyncFailed { .. } => {
                "tiber.create_sync_failed nested=true".to_string()
            }
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
    tasks_dir: Option<PathBuf>,
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
        operation: impl FnOnce(&GitRepository) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let _lock = self.acquire_lock()?;
        self.ensure_tasks_branch()?;
        let workspace = TaskWorkspace::create()?;
        self.materialize_tasks_ref("refs/heads/tasks", workspace.path())?;
        let repo = self.with_tasks_dir(workspace.path().to_path_buf());
        operation(&repo)
    }

    fn with_task_snapshot_workspace<T>(
        &self,
        operation: impl FnOnce(&GitRepository) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let workspace = TaskWorkspace::create()?;
        if git_status(
            ["show-ref", "--verify", "refs/heads/tasks"],
            Some(&self.root),
        )
        .is_err()
        {
            return operation(&self.with_tasks_dir(workspace.path().to_path_buf()));
        }
        self.materialize_tasks_ref("refs/heads/tasks", workspace.path())?;
        let repo = self.with_tasks_dir(workspace.path().to_path_buf());
        operation(&repo)
    }

    fn materialize_tasks_ref(&self, task_ref: &str, destination: &Path) -> Result<(), Error> {
        for status in STATUS_DIRS {
            fs::create_dir_all(destination.join(status))?;
        }
        let order = destination.join("order.md");
        if !order.exists() {
            fs::write(&order, "")?;
        }

        let listing = self.git(["ls-tree", "-r", "--name-only", task_ref])?;
        for path in listing.lines().filter(|line| !line.trim().is_empty()) {
            if path == "order.md" || is_course_task_path(path) || path.ends_with("/.gitkeep") {
                let contents = self.git(["show", &format!("{task_ref}:{path}")])?;
                let destination = destination.join(path);
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(destination, contents)?;
            }
        }
        Ok(())
    }

    fn current_branch(&self) -> Result<String, Error> {
        let branch = git_output(["branch", "--show-current"], Some(&self.root))?;
        let branch = branch.trim();
        if branch.is_empty() {
            return Err(Error::Parse("detached_head=true".to_string()));
        }
        Ok(branch.to_string())
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
        let commit = self.commit_tree(root_tree.trim(), None, "Initialize tiber")?;
        self.git([
            "update-ref",
            "refs/heads/tasks",
            commit.trim(),
            "0000000000000000000000000000000000000000",
        ])?;
        Ok(())
    }

    fn ignore_local_tasks_in_source_gitignore(&self) -> Result<(), Error> {
        let gitignore = self.root.join(".gitignore");
        let mut contents = fs::read_to_string(&gitignore).unwrap_or_default();
        if !contents.lines().any(|line| line.trim() == ".tasks") {
            if !contents.ends_with('\n') && !contents.is_empty() {
                contents.push('\n');
            }
            contents.push_str(".tasks\n");
            fs::write(gitignore, contents)?;
        }
        Ok(())
    }

    fn sync_repository(&self) -> Result<(), Error> {
        self.sync_repository_unlocked()
    }

    fn sync_repository_unlocked(&self) -> Result<(), Error> {
        self.sync_repository_with_admission_unlocked(false)
    }

    fn sync_repository_with_admission_unlocked(
        &self,
        admits_to_backlog: bool,
    ) -> Result<(), Error> {
        let worktree_name = self.current_branch()?;
        let admission_baseline = if admits_to_backlog {
            Some(self.git(["rev-parse", "--verify", "refs/heads/tasks"])?)
        } else {
            None
        };
        for attempt in 1..=MAX_SYNC_ATTEMPTS {
            match self.sync_repository_once(
                &worktree_name,
                admits_to_backlog,
                admission_baseline.as_deref(),
            ) {
                Ok(()) => return Ok(()),
                Err(error) if is_retryable_push_failure(&error) => {
                    if attempt < MAX_SYNC_ATTEMPTS {
                        continue;
                    }
                    if let Some(baseline) = admission_baseline.as_deref() {
                        self.rollback_admission(baseline)?;
                    }
                    return Err(error);
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("sync attempts loop always returns")
    }

    fn rollback_admission(&self, baseline: &str) -> Result<(), Error> {
        let current = self.git(["rev-parse", "--verify", "refs/heads/tasks"])?;
        self.git([
            "update-ref",
            "refs/heads/tasks",
            baseline.trim(),
            current.trim(),
        ])?;
        Ok(())
    }

    fn sync_repository_once(
        &self,
        worktree_name: &str,
        admits_to_backlog: bool,
        admission_baseline: Option<&str>,
    ) -> Result<(), Error> {
        let local_parent = self.git(["rev-parse", "--verify", "refs/heads/tasks"])?;
        let remote_parent = self.fetch_origin_tasks()?;
        if remote_parent
            .as_deref()
            .is_some_and(|parent| parent.trim() != local_parent.trim())
        {
            self.merge_remote_tasks(worktree_name)?;
        }
        if admits_to_backlog {
            if let Err(error) = self.ensure_backlog_not_over_capacity() {
                if let Some(baseline) = admission_baseline {
                    self.rollback_admission(baseline)?;
                }
                return Err(error);
            }
        }
        let tasks_tree = self.write_directory_tree(&self.tasks_dir())?;
        let root_tree = tasks_tree;
        let parent = match remote_parent {
            Some(parent) => parent,
            None => local_parent.clone(),
        };
        let commit = self.commit_tree(root_tree.trim(), Some(parent.trim()), "Sync tiber state")?;
        self.git([
            "update-ref",
            "refs/heads/tasks",
            commit.trim(),
            local_parent.trim(),
        ])?;
        self.push_tasks_branch_if_origin_exists()?;
        Ok(())
    }

    fn fetch_origin_tasks(&self) -> Result<Option<String>, Error> {
        if git_status(["remote", "get-url", "origin"], Some(&self.root)).is_err() {
            return Ok(None);
        }
        match self.git_with_timeout(
            ["fetch", "origin", "tasks:refs/remotes/origin/tasks"],
            Duration::from_secs(10),
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
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    fn merge_remote_tasks(&self, _worktree_name: &str) -> Result<(), Error> {
        let listing = self.git(["ls-tree", "-r", "--name-only", "refs/remotes/origin/tasks"])?;
        let mut remote_order = Vec::new();
        for path in listing.lines().filter(|line| !line.trim().is_empty()) {
            let contents = self.git(["show", &format!("refs/remotes/origin/tasks:{path}")])?;
            if path == "order.md" {
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
            let destination = self.tasks_dir().join(path);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            if destination.exists() {
                let local_contents = fs::read_to_string(&destination)?;
                if local_contents != contents {
                    return Err(Error::Parse(format!(
                        "sync_conflict path={}",
                        path_to_entry(Path::new(path))?
                    )));
                }
            } else if self.local_task_with_same_stem_path(path)?.is_some() {
                return Err(Error::Parse(format!(
                    "sync_conflict path={}",
                    path_to_entry(Path::new(path))?
                )));
            } else {
                fs::write(destination, contents)?;
            }
        }
        if !remote_order.is_empty() {
            let local_order = self.order_entries()?;
            if order_conflicts(&remote_order, &local_order) {
                return Err(Error::Parse("sync_conflict path=order.md".to_string()));
            }
            let mut merged_order = remote_order;
            for local_entry in local_order {
                if !merged_order.contains(&local_entry) {
                    merged_order.push(local_entry);
                }
            }
            self.write_order(&merged_order)?;
        }
        Ok(())
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

    fn push_tasks_branch_if_origin_exists(&self) -> Result<(), Error> {
        if git_status(["remote", "get-url", "origin"], Some(&self.root)).is_err() {
            return Ok(());
        }
        self.git([
            "-c",
            "core.hooksPath=/dev/null",
            "push",
            "origin",
            "refs/heads/tasks:refs/heads/tasks",
        ])?;
        Ok(())
    }

    fn commit_tree(
        &self,
        tree: &str,
        parent: Option<&str>,
        message: &str,
    ) -> Result<String, Error> {
        let mut args = vec!["commit-tree".to_string()];
        if self.commit_signing_enabled()? {
            args.push("-S".to_string());
        }
        args.push(tree.to_string());
        if let Some(parent) = parent {
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
                let blob =
                    self.git(["hash-object", "-w", entry.path().to_string_lossy().as_ref()])?;
                entries.push(format!("100644 blob {}\t{name}\n", blob.trim()));
            }
        }
        entries.sort();
        self.git_with_stdin(["mktree"], &entries.concat())
    }

    fn create_task(&self, title: TaskTitle) -> Result<TaskPath, Error> {
        let task_path = self.create_task_unlocked(title)?;
        if let Err(error) = self.sync_repository_with_admission_unlocked(true) {
            if matches!(error, Error::BacklogCapacityExceeded { .. }) {
                return Err(error);
            }
            let committed = self.task_committed_to_tasks_ref(&task_path).unwrap_or(true);
            if !committed {
                return Err(Error::TaskCreateSyncFailed {
                    source: Box::new(error),
                });
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
        self.ensure_backlog_capacity(&backlog_dir)?;

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

    fn ensure_backlog_capacity(&self, backlog_dir: &Path) -> Result<(), Error> {
        let Some(max_queued) = self.project_config()?.backlog.max_queued else {
            return Ok(());
        };
        let queued = Self::backlog_count(backlog_dir)?;
        if queued >= max_queued {
            return Err(Error::BacklogCapacityExceeded { queued, max_queued });
        }
        Ok(())
    }

    fn ensure_backlog_not_over_capacity(&self) -> Result<(), Error> {
        let Some(max_queued) = self.project_config()?.backlog.max_queued else {
            return Ok(());
        };
        let queued = Self::backlog_count(&self.tasks_dir().join("backlog"))?;
        if queued > max_queued {
            return Err(Error::BacklogCapacityExceeded { queued, max_queued });
        }
        Ok(())
    }

    fn backlog_count(backlog_dir: &Path) -> Result<usize, Error> {
        let mut queued = 0;
        for entry in fs::read_dir(backlog_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() && entry.path().extension() == Some(OsStr::new("md")) {
                queued += 1;
            }
        }
        Ok(queued)
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
        let committed_at = self.git(vec![
            "log".to_string(),
            "-1".to_string(),
            "--format=%cI".to_string(),
            "refs/heads/tasks".to_string(),
            "--".to_string(),
            branch_path,
        ])?;
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
        let worktree_name = self.current_branch()?;
        if self.fetch_origin_tasks()?.is_some() {
            self.merge_remote_tasks(&worktree_name)?;
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
        let (task_path, admits_to_backlog) = self.transition_task_unlocked(task_ref, status)?;
        self.sync_repository_with_admission_unlocked(admits_to_backlog)?;
        Ok(task_path)
    }

    fn transition_task_unlocked(
        &self,
        task_ref: &str,
        status: &str,
    ) -> Result<(TaskPath, bool), Error> {
        let task_ref = self.resolve_task_ref(task_ref)?;
        let status = parse_status(status)?;
        let tasks_dir = self.tasks_dir();
        let admits_to_backlog = status == "backlog" && !task_ref.starts_with("backlog");
        if admits_to_backlog {
            let backlog_dir = tasks_dir.join("backlog");
            fs::create_dir_all(&backlog_dir)?;
            self.ensure_backlog_capacity(&backlog_dir)?;
        }
        let file_name = task_ref
            .file_name()
            .ok_or_else(|| Error::Parse("task_ref_filename_missing=true".to_string()))?;
        let new_ref = PathBuf::from(status).join(file_name);

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
        Ok((TaskPath { path: new_entry }, admits_to_backlog))
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
        let log = self.git(["log", "-1", "--format=%B"])?;
        let requested = closes_trailers(&log);
        if requested.is_empty() {
            return Ok(Vec::new());
        }
        self.sync_repository_unlocked()?;
        let mut closed = Vec::new();
        for task_ref in requested {
            let resolved = task_stem(&self.resolve_task_ref(&task_ref)?)?;
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
        let gitignore_path = self.root.join(".gitignore");
        let mut gitignore = match fs::read_to_string(&gitignore_path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error.into()),
        };
        if !gitignore.lines().any(|line| line.trim() == ".tasks") {
            if !gitignore.ends_with('\n') && !gitignore.is_empty() {
                gitignore.push('\n');
            }
            if !gitignore.is_empty() {
                gitignore.push('\n');
            }
            gitignore.push_str("# tiber local working copy\n.tasks\n");
        }
        let mut files = vec![(".gitignore", gitignore, false)];
        let equivalent_hook = self.equivalent_task_closing_hook()?;
        if equivalent_hook.is_none() {
            files.push((
                ".githooks/post-commit.tiber",
                "#!/usr/bin/env bash\nset -euo pipefail\n\ntiber close-from-trailers\n".to_string(),
                true,
            ));
        }
        let equivalent_workflow = self.equivalent_task_closing_workflow()?;
        if equivalent_workflow.is_none() {
            files.push((
                ".github/workflows/tiber-close-from-trailers.yml",
                "name: tiber close from trailers\n\non:\n  push:\n    branches: [main]\n\njobs:\n  close:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - name: Install Tiber\n        run: |\n          git clone --depth 1 https://github.com/jwilger/ai-plugins.git .tiber-src\n          cargo install --path .tiber-src/plugins/tiber/rust/crates/tiber-cli --bin tiber --root .tiber-install\n          echo \"$PWD/.tiber-install/bin\" >> \"$GITHUB_PATH\"\n      - run: tiber close-from-trailers\n".to_string(),
                true,
            ));
        }
        let justfile_exists = self.root.join("justfile").exists();
        let planned_justfile = self.show_tasks_justfile()?;
        if let Some(justfile) = planned_justfile.as_ref() {
            files.push(("justfile", justfile.clone(), false));
        }
        let mut messages = Vec::new();
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
        if apply && !replace_conflicts && !conflicts.is_empty() {
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
        self.acquire_named_lock("tiber.lock")
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
                Ok(TiberLock { _file: file })
            }
            Err(TryLockError::WouldBlock) => Err(Error::Parse(format!(
                "tiber_lock_busy path={}",
                path_to_entry(&lock_path)?
            ))),
            Err(TryLockError::Error(error)) => Err(Error::Io(error)),
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
    _file: fs::File,
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
        _ => Err(Error::Parse(format!(
            "ambiguous_task_ref ref={task_ref} matches={}",
            matches.join(",")
        ))),
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
