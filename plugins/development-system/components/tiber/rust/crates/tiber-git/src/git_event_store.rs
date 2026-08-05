//! EventCore adapter whose transaction boundary is a confirmed Git ref.

use eventcore_fs::{FileEventStore, FsEventStoreError};
use eventcore_types::{
    Event, EventFilter, EventPage, EventReader, EventStore, EventStoreError, EventStream,
    EventStreamSlice, Operation, StreamId, StreamPosition, StreamVersion, StreamWrites,
};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Mutex;

/// The single authoritative ref used for every new Tiber event stream.
pub const TIBER_BRANCH: &str = "tiber";
const LOCAL_REF: &str = "refs/heads/tiber";
const REMOTE_REF: &str = "refs/remotes/origin/tiber";
const REMOTE_HEAD: &str = "refs/heads/tiber";
const STORE_DIRECTORY: &str = "eventstore";
const PUBLICATION_RETRIES: usize = 3;

/// Failure to open or refresh the Git-backed store.
#[derive(Debug, thiserror::Error)]
pub enum GitEventStoreOpenError {
    #[error(transparent)]
    FileStore(#[from] FsEventStoreError),
    #[error("Git event store operation failed: {0}")]
    Git(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
struct Stage {
    _directory: TempDir,
    work_tree: PathBuf,
    store: FileEventStore,
    base: Option<String>,
    has_origin: bool,
}

/// EventCore store whose successful append means the candidate is confirmed
/// on the authoritative `tiber` ref.
#[derive(Clone, Debug)]
pub struct GitEventStore {
    repository: PathBuf,
    common_directory: PathBuf,
    stage: Arc<Mutex<Stage>>,
}

impl GitEventStore {
    /// Opens a repository-backed store. An absent branch is an empty store.
    pub fn open(repository: impl AsRef<Path>) -> Result<Self, GitEventStoreOpenError> {
        let repository = repository.as_ref().to_path_buf();
        let common_directory = git_path(&repository, ["rev-parse", "--git-common-dir"])?;
        let common_directory = if common_directory.is_absolute() {
            common_directory
        } else {
            repository.join(common_directory)
        };
        let stage = load_stage(&repository)?;
        Ok(Self {
            repository,
            common_directory,
            stage: Arc::new(Mutex::new(stage)),
        })
    }

    fn pending_publication_path(&self) -> PathBuf {
        self.common_directory
            .join("tiber")
            .join("pending-publication")
    }

    async fn refresh(&self) -> Result<(), EventStoreError> {
        let refreshed =
            load_stage(&self.repository).map_err(|_| store_failure(Operation::ReadStream))?;
        *self.stage.lock().await = refreshed;
        Ok(())
    }
}

impl EventStore for GitEventStore {
    async fn read_stream<E: Event>(
        &self,
        stream_id: StreamId,
    ) -> Result<EventStream<E>, EventStoreError> {
        self.refresh().await?;
        self.stage.lock().await.store.read_stream(stream_id).await
    }

    async fn append_events(
        &self,
        writes: StreamWrites,
    ) -> Result<EventStreamSlice, EventStoreError> {
        if self.pending_publication_path().exists() {
            return Err(store_failure(Operation::AppendEvents));
        }

        let conflict = writes
            .expected_versions()
            .iter()
            .next()
            .map(|(stream_id, version)| (stream_id.clone(), *version));
        let stage =
            load_stage(&self.repository).map_err(|_| store_failure(Operation::AppendEvents))?;
        let appended = stage.store.append_events(writes).await?;
        let candidate = create_candidate(&self.repository, &stage)
            .map_err(|_| store_failure(Operation::AppendEvents))?;

        let publication = if stage.has_origin {
            publish_remote(&self.repository, &candidate, stage.base.as_deref())
        } else {
            publish_local(&self.repository, &candidate, stage.base.as_deref())
        };

        match publication {
            Ok(Publication::Confirmed) => {
                *self.stage.lock().await = load_stage(&self.repository)
                    .map_err(|_| store_failure(Operation::AppendEvents))?;
                Ok(appended)
            }
            Ok(Publication::Conflict) => {
                *self.stage.lock().await = load_stage(&self.repository)
                    .map_err(|_| store_failure(Operation::AppendEvents))?;
                let (stream_id, expected) =
                    conflict.ok_or_else(|| store_failure(Operation::AppendEvents))?;
                Err(EventStoreError::VersionConflict {
                    stream_id,
                    expected,
                    actual: StreamVersion::new(usize::from(expected).saturating_add(1)),
                })
            }
            Err(_) => {
                persist_pending(&self.pending_publication_path(), &candidate)
                    .map_err(|_| store_failure(Operation::AppendEvents))?;
                *self.stage.lock().await = stage;
                Err(store_failure(Operation::AppendEvents))
            }
        }
    }
}

impl EventReader for GitEventStore {
    type Error = EventStoreError;

    async fn read_events<E: Event>(
        &self,
        filter: EventFilter,
        page: EventPage,
    ) -> Result<Vec<(E, StreamPosition)>, Self::Error> {
        self.refresh().await?;
        self.stage
            .lock()
            .await
            .store
            .read_events(filter, page)
            .await
    }
}

#[derive(Debug, Eq, PartialEq)]
enum Publication {
    Confirmed,
    Conflict,
}

fn load_stage(repository: &Path) -> Result<Stage, GitEventStoreOpenError> {
    let has_origin = git(repository, ["remote", "get-url", "origin"])
        .map(|output| output.status.success())
        .unwrap_or(false);
    let base = if has_origin {
        refresh_remote(repository)?
    } else {
        resolve_optional_ref(repository, LOCAL_REF)?
    };

    let directory = TempDir::new()?;
    let work_tree = directory.path().join("work-tree");
    fs::create_dir_all(&work_tree)?;
    if let Some(commit) = &base {
        checkout_tree(repository, commit, &work_tree)?;
    }
    let store = FileEventStore::open(work_tree.join(STORE_DIRECTORY))?;
    Ok(Stage {
        _directory: directory,
        work_tree,
        store,
        base,
        has_origin,
    })
}

fn refresh_remote(repository: &Path) -> Result<Option<String>, GitEventStoreOpenError> {
    let advertised = git(
        repository,
        ["ls-remote", "--exit-code", "origin", REMOTE_HEAD],
    )?;
    match advertised.status.code() {
        Some(0) => {
            require_success(git(
                repository,
                [
                    "fetch",
                    "--no-tags",
                    "origin",
                    &format!("{REMOTE_HEAD}:{REMOTE_REF}"),
                ],
            )?)?;
            resolve_optional_ref(repository, REMOTE_REF)
        }
        Some(2) => Ok(None),
        _ => Err(git_error("refresh authoritative tiber ref", &advertised)),
    }
}

fn checkout_tree(
    repository: &Path,
    commit: &str,
    work_tree: &Path,
) -> Result<(), GitEventStoreOpenError> {
    let index = work_tree.join("git-index");
    require_success(git_with(
        repository,
        None,
        [("GIT_INDEX_FILE", index.as_os_str())],
        ["read-tree", commit],
    )?)?;
    require_success(git_with(
        repository,
        Some(work_tree),
        [("GIT_INDEX_FILE", index.as_os_str())],
        ["checkout-index", "--all", "--force"],
    )?)?;
    let _ = fs::remove_file(index);
    Ok(())
}

fn create_candidate(repository: &Path, stage: &Stage) -> Result<String, GitEventStoreOpenError> {
    let index = stage.work_tree.join("candidate-index");
    let index_env = [("GIT_INDEX_FILE", index.as_os_str())];
    if let Some(base) = &stage.base {
        require_success(git_with(repository, None, index_env, ["read-tree", base])?)?;
    } else {
        require_success(git_with(
            repository,
            None,
            index_env,
            ["read-tree", "--empty"],
        )?)?;
    }
    require_success(git_with(
        repository,
        Some(&stage.work_tree),
        index_env,
        ["add", "--all", "--", STORE_DIRECTORY],
    )?)?;
    let tree = output_text(require_success(git_with(
        repository,
        None,
        index_env,
        ["write-tree"],
    )?)?);
    let mut arguments = vec![
        "commit-tree",
        tree.as_str(),
        "-S",
        "-m",
        "tiber event transaction",
    ];
    if let Some(base) = &stage.base {
        arguments.extend(["-p", base.as_str()]);
    }
    let candidate = output_text(require_success(git_with(
        repository, None, index_env, arguments,
    )?)?);
    let _ = fs::remove_file(index);
    Ok(candidate)
}

fn publish_remote(
    repository: &Path,
    candidate: &str,
    base: Option<&str>,
) -> Result<Publication, GitEventStoreOpenError> {
    for _ in 0..PUBLICATION_RETRIES {
        let push = git(
            repository,
            ["push", "origin", &format!("{candidate}:{REMOTE_HEAD}")],
        )?;
        let remote = refresh_remote(repository)?;
        if remote
            .as_deref()
            .is_some_and(|head| is_ancestor(repository, candidate, head))
        {
            return Ok(Publication::Confirmed);
        }
        if remote.as_deref() != base {
            return Ok(Publication::Conflict);
        }
        if push.status.success() {
            break;
        }
    }
    Err(GitEventStoreOpenError::Git(
        "publication outcome remained indeterminate".to_owned(),
    ))
}

fn publish_local(
    repository: &Path,
    candidate: &str,
    base: Option<&str>,
) -> Result<Publication, GitEventStoreOpenError> {
    let expected = base.unwrap_or("0000000000000000000000000000000000000000");
    let update = git(repository, ["update-ref", LOCAL_REF, candidate, expected])?;
    if update.status.success() {
        return Ok(Publication::Confirmed);
    }
    let current = resolve_optional_ref(repository, LOCAL_REF)?;
    if current
        .as_deref()
        .is_some_and(|head| is_ancestor(repository, candidate, head))
    {
        Ok(Publication::Confirmed)
    } else {
        Ok(Publication::Conflict)
    }
}

fn is_ancestor(repository: &Path, ancestor: &str, descendant: &str) -> bool {
    git(
        repository,
        ["merge-base", "--is-ancestor", ancestor, descendant],
    )
    .is_ok_and(|output| output.status.success())
}

fn resolve_optional_ref(
    repository: &Path,
    reference: &str,
) -> Result<Option<String>, GitEventStoreOpenError> {
    let output = git(repository, ["rev-parse", "--verify", reference])?;
    if output.status.success() {
        Ok(Some(output_text(output)))
    } else {
        Ok(None)
    }
}

fn persist_pending(path: &Path, candidate: &str) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, format!("{candidate}\n"))?;
    fs::rename(temporary, path)
}

fn git_path<const N: usize>(
    repository: &Path,
    arguments: [&str; N],
) -> Result<PathBuf, GitEventStoreOpenError> {
    Ok(PathBuf::from(output_text(require_success(git(
        repository, arguments,
    )?)?)))
}

fn git<I, S>(repository: &Path, arguments: I) -> Result<Output, GitEventStoreOpenError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    git_with(
        repository,
        None,
        std::iter::empty::<(&str, &OsStr)>(),
        arguments,
    )
}

fn git_with<I, S, E, K, V>(
    repository: &Path,
    work_tree: Option<&Path>,
    environment: E,
    arguments: I,
) -> Result<Output, GitEventStoreOpenError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    E: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    let mut command = Command::new("git");
    command.arg("-C").arg(repository);
    if let Some(work_tree) = work_tree {
        command.arg(format!("--work-tree={}", work_tree.display()));
    }
    command.envs(environment).args(arguments);
    command.output().map_err(GitEventStoreOpenError::Io)
}

fn require_success(output: Output) -> Result<Output, GitEventStoreOpenError> {
    if output.status.success() {
        Ok(output)
    } else {
        Err(git_error("execute Git command", &output))
    }
}

fn git_error(operation: &str, output: &Output) -> GitEventStoreOpenError {
    GitEventStoreOpenError::Git(format!(
        "{operation}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

fn output_text(output: Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn store_failure(operation: Operation) -> EventStoreError {
    EventStoreError::StoreFailure { operation }
}
