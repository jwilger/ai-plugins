use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::fs::OpenOptions;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use taskbranch_core::{
    BoardSnapshot, DependencyGraph, OrderReconciliation, TaskDependencies, TaskSnapshot, TaskTitle,
};

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
    repo.create_task(TaskTitle::parse(title)?)
}

pub fn list_tasks_at(root: impl Into<PathBuf>) -> Result<Vec<TaskSummary>, Error> {
    let repo = GitRepository::at(root);
    repo.list_tasks()
}

pub fn show_task_at(root: impl Into<PathBuf>, task_ref: &str) -> Result<String, Error> {
    let repo = GitRepository::at(root);
    repo.show_task(task_ref)
}

pub fn task_metadata_at(root: impl Into<PathBuf>, task_ref: &str) -> Result<TaskMetadata, Error> {
    let repo = GitRepository::at(root);
    repo.task_metadata(task_ref)
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
        let worktree_name = self.current_branch()?;
        self.ensure_tasks_branch(&worktree_name)?;
        self.ensure_local_tasks_link(&worktree_name)?;
        Ok(())
    }
}

pub fn sync_repository() -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.sync_repository()
}

pub fn create_task(title: &str) -> Result<TaskPath, Error> {
    let repo = GitRepository::discover()?;
    repo.create_task(TaskTitle::parse(title)?)
}

pub fn list_tasks() -> Result<Vec<TaskSummary>, Error> {
    let repo = GitRepository::discover()?;
    repo.list_tasks()
}

pub fn show_task(task_ref: &str) -> Result<String, Error> {
    let repo = GitRepository::discover()?;
    repo.show_task(task_ref)
}

pub fn task_metadata(task_ref: &str) -> Result<TaskMetadata, Error> {
    let repo = GitRepository::discover()?;
    repo.task_metadata(task_ref)
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
    repo.next_task()
}

pub fn transition_task(task_ref: &str, status: &str) -> Result<TaskPath, Error> {
    let repo = GitRepository::discover()?;
    repo.transition_task(task_ref, status)
}

pub fn prioritize_before(task_ref: &str, before_ref: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.prioritize_before(task_ref, before_ref)
}

pub fn link_blocks(from_ref: &str, to_ref: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.link_blocks(from_ref, to_ref)
}

pub fn unlink_blocks(from_ref: &str, to_ref: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.unlink_blocks(from_ref, to_ref)
}

pub fn add_subtask(task_ref: &str, title: &str) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.add_subtask(task_ref, title)
}

pub fn set_subtask_checked(task_ref: &str, index: &str, checked: bool) -> Result<(), Error> {
    let repo = GitRepository::discover()?;
    repo.set_subtask_checked(task_ref, index, checked)
}

pub fn validate_fix() -> Result<Vec<ValidationMessage>, Error> {
    let repo = GitRepository::discover()?;
    repo.validate_fix()
}

pub fn close_from_trailers() -> Result<Vec<String>, Error> {
    let repo = GitRepository::discover()?;
    repo.close_from_trailers()
}

pub fn scaffold_repo(apply: bool) -> Result<Vec<String>, Error> {
    let repo = GitRepository::discover()?;
    repo.scaffold_repo(apply)
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
pub struct ValidationMessage(String);

impl fmt::Display for ValidationMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug)]
pub enum Error {
    CommandFailed {
        program: String,
        args: Vec<String>,
        status: String,
        stderr: String,
    },
    Io(std::io::Error),
    Parse(String),
    Core(taskbranch_core::CoreError),
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
                "taskbranch.command_failed program={program} args={} status={status} stderr={}",
                args.join(" "),
                stderr.trim()
            ),
            Self::Io(error) => write!(formatter, "taskbranch.io_error source={error}"),
            Self::Parse(message) => write!(formatter, "taskbranch.parse_error {message}"),
            Self::Core(error) => write!(formatter, "{error}"),
            Self::Usage(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<taskbranch_core::CoreError> for Error {
    fn from(error: taskbranch_core::CoreError) -> Self {
        Self::Core(error)
    }
}

struct GitRepository {
    root: PathBuf,
}

impl GitRepository {
    fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn discover() -> Result<Self, Error> {
        let root = git_output(["rev-parse", "--show-toplevel"], None)?;
        let root_path = PathBuf::from(root.trim());
        Ok(Self::at(root_path))
    }

    fn current_branch(&self) -> Result<String, Error> {
        let branch = git_output(["branch", "--show-current"], Some(&self.root))?;
        let branch = branch.trim();
        if branch.is_empty() {
            return Err(Error::Parse("detached_head=true".to_string()));
        }
        Ok(branch.to_string())
    }

    fn ensure_tasks_branch(&self, worktree_name: &str) -> Result<(), Error> {
        if git_status(
            ["show-ref", "--verify", "refs/heads/tasks"],
            Some(&self.root),
        )
        .is_ok()
        {
            return Ok(());
        }

        let order_blob = self.git_with_stdin(["hash-object", "-w", "--stdin"], "")?;
        let tasks_tree = self.git_with_stdin(
            ["mktree"],
            &format!("100644 blob {}\torder.md\n", order_blob.trim()),
        )?;
        let worktree_tree = self.git_with_stdin(
            ["mktree"],
            &format!("040000 tree {}\t.tasks\n", tasks_tree.trim()),
        )?;
        let root_tree = self.git_with_stdin(
            ["mktree"],
            &format!("040000 tree {}\t{worktree_name}\n", worktree_tree.trim()),
        )?;
        let commit = self.git([
            "commit-tree",
            root_tree.trim(),
            "-m",
            "Initialize taskbranch",
        ])?;
        self.git([
            "update-ref",
            "refs/heads/tasks",
            commit.trim(),
            "0000000000000000000000000000000000000000",
        ])?;
        Ok(())
    }

    fn ensure_local_tasks_link(&self, worktree_name: &str) -> Result<(), Error> {
        let target = PathBuf::from(format!(".git/taskbranch/{worktree_name}/.tasks"));
        let canonical_tasks = self.root.join(&target);
        fs::create_dir_all(&canonical_tasks)?;
        let order = canonical_tasks.join("order.md");
        if !order.exists() {
            fs::write(order, "")?;
        }

        let link = self.root.join(".tasks");
        if link.exists() || link.symlink_metadata().is_ok() {
            return Ok(());
        }

        create_symlink(&target, &link)?;
        Ok(())
    }

    fn sync_repository(&self) -> Result<(), Error> {
        let _lock = self.acquire_lock()?;
        let worktree_name = self.current_branch()?;
        match self.sync_repository_once(&worktree_name) {
            Ok(()) => Ok(()),
            Err(error) if is_retryable_push_failure(&error) => {
                self.sync_repository_once(&worktree_name)
            }
            Err(error) => Err(error),
        }
    }

    fn sync_repository_once(&self, worktree_name: &str) -> Result<(), Error> {
        let remote_parent = self.fetch_origin_tasks()?;
        if remote_parent.is_some() {
            self.merge_remote_tasks(worktree_name)?;
        }
        let tasks_tree = self.write_directory_tree(&self.tasks_dir())?;
        let worktree_tree = self.git_with_stdin(
            ["mktree"],
            &format!("040000 tree {}\t.tasks\n", tasks_tree.trim()),
        )?;
        let root_tree = self.git_with_stdin(
            ["mktree"],
            &format!("040000 tree {}\t{worktree_name}\n", worktree_tree.trim()),
        )?;
        let parent = match remote_parent {
            Some(parent) => parent,
            None => self.git(["rev-parse", "--verify", "refs/heads/tasks"])?,
        };
        let commit = self.git([
            "commit-tree",
            root_tree.trim(),
            "-p",
            parent.trim(),
            "-m",
            "Sync taskbranch state",
        ])?;
        self.git(["update-ref", "refs/heads/tasks", commit.trim()])?;
        self.push_tasks_branch_if_origin_exists()?;
        Ok(())
    }

    fn fetch_origin_tasks(&self) -> Result<Option<String>, Error> {
        if git_status(["remote", "get-url", "origin"], Some(&self.root)).is_err() {
            return Ok(None);
        }
        match self.git(["fetch", "origin", "tasks:refs/remotes/origin/tasks"]) {
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

    fn merge_remote_tasks(&self, worktree_name: &str) -> Result<(), Error> {
        let prefix = format!("{worktree_name}/.tasks/");
        let listing = self.git([
            "ls-tree",
            "-r",
            "--name-only",
            "refs/remotes/origin/tasks",
            "--",
            format!("{worktree_name}/.tasks").as_str(),
        ])?;
        let mut remote_order = Vec::new();
        for path in listing.lines().filter(|line| !line.trim().is_empty()) {
            let Some(relative) = path.strip_prefix(&prefix) else {
                continue;
            };
            let contents = self.git(["show", &format!("refs/remotes/origin/tasks:{path}")])?;
            if relative == "order.md" {
                remote_order = contents
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(str::to_string)
                    .collect();
                continue;
            }
            let destination = self.tasks_dir().join(parse_task_ref(relative)?);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            if destination.exists() {
                let local_contents = fs::read_to_string(&destination)?;
                if local_contents != contents {
                    return Err(Error::Parse(format!(
                        "sync_conflict path={}",
                        path_to_entry(Path::new(relative))?
                    )));
                }
            } else {
                fs::write(destination, contents)?;
            }
        }
        if !remote_order.is_empty() {
            let mut merged_order = remote_order;
            for local_entry in self.order_entries()? {
                if !merged_order.contains(&local_entry) {
                    merged_order.push(local_entry);
                }
            }
            self.write_order(&merged_order)?;
        }
        Ok(())
    }

    fn push_tasks_branch_if_origin_exists(&self) -> Result<(), Error> {
        if git_status(["remote", "get-url", "origin"], Some(&self.root)).is_err() {
            return Ok(());
        }
        self.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"])?;
        Ok(())
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
        let _lock = self.acquire_lock()?;
        self.create_task_unlocked(title)
    }

    fn create_task_unlocked(&self, title: TaskTitle) -> Result<TaskPath, Error> {
        let tasks_dir = self.root.join(".tasks");
        let todo_dir = tasks_dir.join("todo");
        fs::create_dir_all(&todo_dir)?;

        let task_path = format!("todo/{}.md", title.file_stem());
        let absolute_task_path = tasks_dir.join(&task_path);
        fs::write(&absolute_task_path, format!("# {}\n", title.as_str()))?;

        let order_path = tasks_dir.join("order.md");
        let mut order = if order_path.exists() {
            fs::read_to_string(&order_path)?
        } else {
            String::new()
        };
        if !order.lines().any(|line| line == task_path) {
            order.push_str(&task_path);
            order.push('\n');
            fs::write(order_path, order)?;
        }

        Ok(TaskPath { path: task_path })
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
        self.soft_read_sync();
        let ordered_tasks = self
            .order_entries()?
            .into_iter()
            .map(|path| {
                let task = fs::read_to_string(self.tasks_dir().join(&path))?;
                let title = parse_title(&task)?;
                Ok(TaskSnapshot::new(path, title))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(BoardSnapshot::from_ordered_tasks(ordered_tasks))
    }

    fn show_task(&self, task_ref: &str) -> Result<String, Error> {
        self.soft_read_sync();
        fs::read_to_string(self.tasks_dir().join(self.resolve_task_ref(task_ref)?))
            .map_err(Error::Io)
    }

    fn task_metadata(&self, task_ref: &str) -> Result<TaskMetadata, Error> {
        self.soft_read_sync();
        let task_ref = self.resolve_task_ref(task_ref)?;
        let path = path_to_entry(&task_ref)?;
        let task = fs::read_to_string(self.tasks_dir().join(&task_ref))?;
        let title = parse_title(&task)?;
        let committed_at = self.task_committed_at(&path)?;
        Ok(TaskMetadata {
            path,
            title,
            committed_at,
        })
    }

    fn task_committed_at(&self, task_ref: &str) -> Result<Option<String>, Error> {
        let worktree_name = self.current_branch()?;
        let branch_path = format!("{worktree_name}/.tasks/{task_ref}");
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
        Ok(self.board_snapshot()?.next_task().map(TaskSummary::from))
    }

    fn soft_read_sync(&self) {
        let _ =
            self.current_branch()
                .and_then(|worktree_name| match self.fetch_origin_tasks()? {
                    Some(_) => self.merge_remote_tasks(&worktree_name),
                    None => Ok(()),
                });
    }

    fn transition_task(&self, task_ref: &str, status: &str) -> Result<TaskPath, Error> {
        let _lock = self.acquire_lock()?;
        self.transition_task_unlocked(task_ref, status)
    }

    fn transition_task_unlocked(&self, task_ref: &str, status: &str) -> Result<TaskPath, Error> {
        let task_ref = self.resolve_task_ref(task_ref)?;
        let status = parse_status(status)?;
        let file_name = task_ref
            .file_name()
            .ok_or_else(|| Error::Parse("task_ref_filename_missing=true".to_string()))?;
        let new_ref = PathBuf::from(status).join(file_name);

        let tasks_dir = self.root.join(".tasks");
        let from = tasks_dir.join(&task_ref);
        let to = tasks_dir.join(&new_ref);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(from, &to)?;

        let old_entry = path_to_entry(&task_ref)?;
        let new_entry = path_to_entry(&new_ref)?;
        let order = self
            .order_entries()?
            .into_iter()
            .map(|entry| {
                if entry == old_entry {
                    new_entry.clone()
                } else {
                    entry
                }
            })
            .collect::<Vec<_>>();
        self.write_order(&order)?;
        Ok(TaskPath { path: new_entry })
    }

    fn prioritize_before(&self, task_ref: &str, before_ref: &str) -> Result<(), Error> {
        let _lock = self.acquire_lock()?;
        self.prioritize_before_unlocked(task_ref, before_ref)
    }

    fn prioritize_before_unlocked(&self, task_ref: &str, before_ref: &str) -> Result<(), Error> {
        let task_ref = path_to_entry(&self.resolve_task_ref(task_ref)?)?;
        let before_ref = path_to_entry(&self.resolve_task_ref(before_ref)?)?;
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
        let _lock = self.acquire_lock()?;
        self.link_blocks_unlocked(from_ref, to_ref)
    }

    fn link_blocks_unlocked(&self, from_ref: &str, to_ref: &str) -> Result<(), Error> {
        let from_ref = path_to_entry(&self.resolve_task_ref(from_ref)?)?;
        let to_ref = path_to_entry(&self.resolve_task_ref(to_ref)?)?;
        self.update_task_section(&from_ref, "Blocks", &to_ref, SectionOperation::Add)?;
        self.update_task_section(&to_ref, "Blocked By", &from_ref, SectionOperation::Add)
    }

    fn unlink_blocks(&self, from_ref: &str, to_ref: &str) -> Result<(), Error> {
        let _lock = self.acquire_lock()?;
        self.unlink_blocks_unlocked(from_ref, to_ref)
    }

    fn unlink_blocks_unlocked(&self, from_ref: &str, to_ref: &str) -> Result<(), Error> {
        let from_ref = path_to_entry(&self.resolve_task_ref(from_ref)?)?;
        let to_ref = path_to_entry(&self.resolve_task_ref(to_ref)?)?;
        self.update_task_section(&from_ref, "Blocks", &to_ref, SectionOperation::Remove)?;
        self.update_task_section(&to_ref, "Blocked By", &from_ref, SectionOperation::Remove)
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

    fn add_subtask(&self, task_ref: &str, title: &str) -> Result<(), Error> {
        let _lock = self.acquire_lock()?;
        self.add_subtask_unlocked(task_ref, title)
    }

    fn add_subtask_unlocked(&self, task_ref: &str, title: &str) -> Result<(), Error> {
        let title = title.trim();
        if title.is_empty() {
            return Err(Error::Parse("subtask_title_empty=true".to_string()));
        }
        self.update_task_section(
            &path_to_entry(&self.resolve_task_ref(task_ref)?)?,
            "Subtasks",
            &format!("[ ] {title}"),
            SectionOperation::Add,
        )
    }

    fn set_subtask_checked(&self, task_ref: &str, index: &str, checked: bool) -> Result<(), Error> {
        let _lock = self.acquire_lock()?;
        self.set_subtask_checked_unlocked(task_ref, index, checked)
    }

    fn set_subtask_checked_unlocked(
        &self,
        task_ref: &str,
        index: &str,
        checked: bool,
    ) -> Result<(), Error> {
        let task_ref = self.resolve_task_ref(task_ref)?;
        let index = parse_one_based_index(index)?;
        let path = self.tasks_dir().join(&task_ref);
        let task = fs::read_to_string(&path)?;
        fs::write(path, update_subtask_check_state(&task, index, checked)?)?;
        Ok(())
    }

    fn validate_fix(&self) -> Result<Vec<ValidationMessage>, Error> {
        let _lock = self.acquire_lock()?;
        self.validate_fix_unlocked()
    }

    fn validate_fix_unlocked(&self) -> Result<Vec<ValidationMessage>, Error> {
        let mut messages = Vec::new();
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
        let _lock = self.acquire_lock()?;
        let log = self.git(["log", "--format=%B%x00"])?;
        let mut closed = Vec::new();
        for task_ref in closes_trailers(&log) {
            let task_path = parse_task_ref(&task_ref)?;
            if !self.tasks_dir().join(&task_path).exists() {
                continue;
            }
            let done = self.transition_task_unlocked(&task_ref, "done")?;
            closed.push(done.path);
        }
        closed.sort();
        closed.dedup();
        Ok(closed)
    }

    fn scaffold_repo(&self, apply: bool) -> Result<Vec<String>, Error> {
        let _lock = if apply {
            Some(self.acquire_lock()?)
        } else {
            None
        };
        let mut files = vec![
            (
                ".gitignore",
                "# taskbranch local working copy\n.tasks\n".to_string(),
            ),
            (
                ".githooks/post-commit.taskbranch",
                "#!/usr/bin/env bash\nset -euo pipefail\n\ntaskbranch close-from-trailers\n"
                    .to_string(),
            ),
            (
                ".github/workflows/taskbranch-close-from-trailers.yml",
                "name: taskbranch close from trailers\n\non:\n  push:\n    branches: [main]\n\njobs:\n  close:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - run: taskbranch close-from-trailers\n".to_string(),
            ),
        ];
        if let Some(justfile) = self.show_tasks_justfile()? {
            files.push(("justfile", justfile));
        }
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
        contents.push_str("\nshow-tasks:\n  taskbranch list\n");
        Ok(Some(contents))
    }

    fn report_schema_errors(
        &self,
        task_refs: &[String],
        messages: &mut Vec<ValidationMessage>,
    ) -> Result<(), Error> {
        for task_ref in task_refs {
            let task = fs::read_to_string(self.tasks_dir().join(task_ref))?;
            if parse_title(&task).is_err() {
                messages.push(ValidationMessage(format!(
                    "schema title-missing {task_ref}"
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
            if task_ref.starts_with("doing/") {
                continue;
            }
            let path = self.tasks_dir().join(task_ref);
            let task = fs::read_to_string(&path)?;
            let repaired = remove_markdown_section(&task, "Claims");
            if repaired != task {
                fs::write(path, repaired)?;
                messages.push(ValidationMessage(format!(
                    "fixed misplaced-claim {task_ref}"
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
            for blocked_ref in markdown_section_items(&task, "Blocks") {
                if !task_refs.contains(&blocked_ref) {
                    messages.push(ValidationMessage(format!(
                        "dangling link {task_ref} blocks {blocked_ref}"
                    )));
                    continue;
                }
                let blocked_task = fs::read_to_string(self.tasks_dir().join(&blocked_ref))?;
                if !markdown_section_items(&blocked_task, "Blocked By").contains(task_ref) {
                    self.update_task_section(
                        &blocked_ref,
                        "Blocked By",
                        task_ref,
                        SectionOperation::Add,
                    )?;
                    messages.push(ValidationMessage(format!(
                        "fixed reciprocal-link {blocked_ref} blocked-by {task_ref}"
                    )));
                }
            }
            for blocker_ref in markdown_section_items(&task, "Blocked By") {
                if !task_refs.contains(&blocker_ref) {
                    messages.push(ValidationMessage(format!(
                        "dangling link {task_ref} blocked-by {blocker_ref}"
                    )));
                    continue;
                }
                let blocker_task = fs::read_to_string(self.tasks_dir().join(&blocker_ref))?;
                if !markdown_section_items(&blocker_task, "Blocks").contains(task_ref) {
                    self.update_task_section(
                        &blocker_ref,
                        "Blocks",
                        task_ref,
                        SectionOperation::Add,
                    )?;
                    messages.push(ValidationMessage(format!(
                        "fixed reciprocal-link {blocker_ref} blocks {task_ref}"
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
                    Ok(TaskDependencies::new(
                        task_ref,
                        markdown_section_items(&task, "Blocks"),
                    ))
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
        let graph = DependencyGraph::from_tasks(
            task_refs
                .iter()
                .map(|task_ref| {
                    let task = fs::read_to_string(self.tasks_dir().join(task_ref))?;
                    Ok(TaskDependencies::new(
                        task_ref,
                        subtask_refs(&task, task_refs),
                    ))
                })
                .collect::<Result<Vec<_>, Error>>()?,
        );
        messages.extend(
            graph
                .cycle_messages_with_label("subtask")
                .into_iter()
                .map(ValidationMessage),
        );
        Ok(())
    }

    fn reconcile_order(
        &self,
        task_refs: &[String],
        messages: &mut Vec<ValidationMessage>,
    ) -> Result<(), Error> {
        let reconciliation =
            OrderReconciliation::reconcile(self.order_entries()?, task_refs.to_vec());
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
        for status in fs::read_dir(&tasks_dir)? {
            let status = status?;
            if !status.file_type()?.is_dir() {
                continue;
            }
            let status_name = status.file_name().to_string_lossy().into_owned();
            for task in fs::read_dir(status.path())? {
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
        let path = parse_safe_relative_path(task_ref, "task")?;
        match path.components().count() {
            1 => {
                let file_name = path_to_entry(&path)?;
                let mut matches = self
                    .task_file_refs()?
                    .into_iter()
                    .filter(|candidate| {
                        Path::new(candidate)
                            .file_name()
                            .is_some_and(|candidate_file| candidate_file == file_name.as_str())
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
            2 => Ok(path),
            _ => Err(Error::Parse(format!("invalid_task_ref ref={task_ref}"))),
        }
    }

    fn order_entries(&self) -> Result<Vec<String>, Error> {
        let order_path = self.tasks_dir().join("order.md");
        if !order_path.exists() {
            return Ok(Vec::new());
        }

        fs::read_to_string(order_path)?
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| path_to_entry(&parse_task_ref(line)?))
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
        self.root.join(".tasks")
    }

    fn acquire_lock(&self) -> Result<TaskbranchLock, Error> {
        let git_common_dir = self.git(["rev-parse", "--git-common-dir"])?;
        let git_common_dir = PathBuf::from(git_common_dir.trim());
        let git_common_dir = if git_common_dir.is_absolute() {
            git_common_dir
        } else {
            self.root.join(git_common_dir)
        };
        let lock_dir = git_common_dir.join("taskbranch");
        fs::create_dir_all(&lock_dir)?;
        let lock_path = lock_dir.join("taskbranch.lock");
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(_) => Ok(TaskbranchLock { path: lock_path }),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Err(Error::Parse(
                format!("taskbranch_lock_busy path={}", path_to_entry(&lock_path)?),
            )),
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

struct TaskbranchLock {
    path: PathBuf,
}

impl Drop for TaskbranchLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[derive(Clone, Copy)]
enum SectionOperation {
    Add,
    Remove,
}

fn parse_title(task: &str) -> Result<String, Error> {
    task.lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(str::to_string)
        .ok_or_else(|| Error::Parse("task_title_missing=true".to_string()))
}

fn update_markdown_section(
    document: &str,
    heading: &str,
    item: &str,
    operation: SectionOperation,
) -> String {
    let heading_line = format!("## {heading}");
    let item_line = format!("- {item}");
    let mut before = Vec::new();
    let mut section = Vec::new();
    let mut after = Vec::new();
    let mut split = SectionSplit::Before;

    for line in document.lines() {
        match split {
            SectionSplit::Before if line == heading_line => split = SectionSplit::Section,
            SectionSplit::Before => before.push(line.to_string()),
            SectionSplit::Section if line.starts_with("## ") => {
                split = SectionSplit::After;
                after.push(line.to_string());
            }
            SectionSplit::Section => section.push(line.to_string()),
            SectionSplit::After => after.push(line.to_string()),
        }
    }

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
        sections.push(format!("{heading_line}\n{}", section.join("\n")));
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

fn subtask_refs(document: &str, task_refs: &[String]) -> Vec<String> {
    markdown_section_items(document, "Subtasks")
        .into_iter()
        .filter_map(|item| {
            item.strip_prefix("[ ] ")
                .or_else(|| item.strip_prefix("[x] "))
                .map(str::to_string)
        })
        .filter(|item| task_refs.contains(item))
        .collect()
}

fn remove_markdown_section(document: &str, heading: &str) -> String {
    let heading_line = format!("## {heading}");
    let mut before = Vec::new();
    let mut after = Vec::new();
    let mut split = SectionSplit::Before;

    for line in document.lines() {
        match split {
            SectionSplit::Before if line == heading_line => split = SectionSplit::Section,
            SectionSplit::Before => before.push(line.to_string()),
            SectionSplit::Section if line.starts_with("## ") => {
                split = SectionSplit::After;
                after.push(line.to_string());
            }
            SectionSplit::Section => {}
            SectionSplit::After => after.push(line.to_string()),
        }
    }

    if matches!(split, SectionSplit::Before) {
        return document.to_string();
    }

    trim_blank_edges(&mut before);
    trim_blank_edges(&mut after);
    let mut sections = Vec::new();
    let before = before.join("\n");
    if !before.is_empty() {
        sections.push(before);
    }
    let after = after.join("\n");
    if !after.is_empty() {
        sections.push(after);
    }
    format!("{}\n", sections.join("\n\n"))
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
            args.first().is_some_and(|arg| arg == "push")
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
    target_index: usize,
    checked: bool,
) -> Result<String, Error> {
    let mut current_index = 0;
    let mut lines = Vec::new();
    for line in document.lines() {
        if let Some(title) = line.strip_prefix("- [ ] ") {
            current_index += 1;
            if current_index == target_index {
                lines.push(format!("- [{}] {title}", if checked { "x" } else { " " }));
            } else {
                lines.push(line.to_string());
            }
        } else if let Some(title) = line.strip_prefix("- [x] ") {
            current_index += 1;
            if current_index == target_index {
                lines.push(format!("- [{}] {title}", if checked { "x" } else { " " }));
            } else {
                lines.push(line.to_string());
            }
        } else {
            lines.push(line.to_string());
        }
    }

    if current_index < target_index {
        return Err(Error::Parse(format!(
            "subtask_missing index={target_index}"
        )));
    }

    Ok(format!("{}\n", lines.join("\n")))
}

fn parse_one_based_index(index: &str) -> Result<usize, Error> {
    let index = index
        .parse::<usize>()
        .map_err(|error| Error::Parse(format!("invalid_subtask_index source={error}")))?;
    if index == 0 {
        return Err(Error::Parse("invalid_subtask_index zero=true".to_string()));
    }
    Ok(index)
}

fn trim_blank_edges(lines: &mut Vec<String>) {
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
}

fn parse_task_ref(task_ref: &str) -> Result<PathBuf, Error> {
    let path = parse_safe_relative_path(task_ref, "task")?;
    if path.components().count() != 2 {
        return Err(Error::Parse(format!("invalid_task_ref ref={task_ref}")));
    }
    Ok(path)
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
    if status.is_empty()
        || status
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || character == '-'))
    {
        return Err(Error::Parse(format!("invalid_status status={status}")));
    }
    Ok(status)
}

fn path_to_entry(path: &Path) -> Result<String, Error> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| Error::Parse("path_utf8=false".to_string()))
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
    command_output("git", &args, command.output()?)
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
