//! Unified EventCore backend contract suite for Tiber's Git adapter.

use eventcore_fs::{FileCheckpointStore, FileProjectorCoordinator};
use eventcore_testing::contract::{backend_contract_tests, ContractTestEvent};
use eventcore_types::{
    CheckpointStore, EventStore, ProjectorCoordinator, StreamId, StreamPosition, StreamVersion,
    StreamWrites,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use tiber_git::git_event_store::GitEventStore;

struct TempGitStore {
    _dir: TempDir,
    repository: PathBuf,
    origin: PathBuf,
    inner: GitEventStore,
}

impl eventcore_types::EventStore for TempGitStore {
    async fn read_stream<E: eventcore_types::Event>(
        &self,
        stream_id: eventcore_types::StreamId,
    ) -> Result<eventcore_types::EventStream<E>, eventcore_types::EventStoreError> {
        self.inner.read_stream(stream_id).await
    }

    async fn append_events(
        &self,
        writes: eventcore_types::StreamWrites,
    ) -> Result<eventcore_types::EventStreamSlice, eventcore_types::EventStoreError> {
        self.inner.append_events(writes).await
    }
}

impl eventcore_types::EventReader for TempGitStore {
    type Error = eventcore_types::EventStoreError;

    async fn read_events<E: eventcore_types::Event>(
        &self,
        filter: eventcore_types::EventFilter,
        page: eventcore_types::EventPage,
    ) -> Result<Vec<(E, StreamPosition)>, Self::Error> {
        self.inner.read_events(filter, page).await
    }
}

struct TempCheckpoint {
    _dir: TempDir,
    inner: FileCheckpointStore,
}

impl CheckpointStore for TempCheckpoint {
    type Error = <FileCheckpointStore as CheckpointStore>::Error;

    async fn load(&self, name: &str) -> Result<Option<StreamPosition>, Self::Error> {
        self.inner.load(name).await
    }

    async fn save(&self, name: &str, position: StreamPosition) -> Result<(), Self::Error> {
        self.inner.save(name, position).await
    }
}

struct TempCoordinator {
    _dir: TempDir,
    inner: FileProjectorCoordinator,
}

impl ProjectorCoordinator for TempCoordinator {
    type Error = <FileProjectorCoordinator as ProjectorCoordinator>::Error;
    type Guard = <FileProjectorCoordinator as ProjectorCoordinator>::Guard;

    async fn try_acquire(&self, subscription_name: &str) -> Result<Self::Guard, Self::Error> {
        self.inner.try_acquire(subscription_name).await
    }
}

fn make_store() -> TempGitStore {
    let dir = TempDir::new().expect("create temporary Git event store");
    let repository = dir.path().join("repository");
    let origin = dir.path().join("origin.git");
    let signing_key = dir.path().join("signing-key");
    run(
        dir.path(),
        ["init", "--bare", origin.to_str().expect("UTF-8 origin")],
    );
    run(
        dir.path(),
        ["init", repository.to_str().expect("UTF-8 repository")],
    );
    let key_status = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-f"])
        .arg(&signing_key)
        .status()
        .expect("run ssh-keygen");
    assert!(
        key_status.success(),
        "generate deterministic test signing setup"
    );
    run(&repository, ["config", "user.name", "Tiber Contract Test"]);
    run(
        &repository,
        ["config", "user.email", "tiber-contract@example.invalid"],
    );
    run(&repository, ["config", "gpg.format", "ssh"]);
    run(
        &repository,
        [
            "config",
            "user.signingkey",
            signing_key.to_str().expect("UTF-8 signing key"),
        ],
    );
    run(
        &repository,
        [
            "remote",
            "add",
            "origin",
            origin.to_str().expect("UTF-8 origin"),
        ],
    );
    let inner = GitEventStore::open(&repository).expect("open Git event store");
    TempGitStore {
        _dir: dir,
        repository,
        origin,
        inner,
    }
}

fn run<const N: usize>(repository: &Path, arguments: [&str; N]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(arguments)
        .output()
        .expect("run Git command");
    assert!(
        output.status.success(),
        "Git command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn make_checkpoint_store() -> TempCheckpoint {
    let dir = TempDir::new().expect("create temporary checkpoint store");
    let inner = FileCheckpointStore::open(dir.path()).expect("open checkpoint store");
    TempCheckpoint { _dir: dir, inner }
}

fn make_coordinator() -> TempCoordinator {
    let dir = TempDir::new().expect("create temporary projector coordinator");
    let inner = FileProjectorCoordinator::open(dir.path()).expect("open projector coordinator");
    TempCoordinator { _dir: dir, inner }
}

backend_contract_tests! {
    suite = git_event_store,
    make_store = || { crate::make_store() },
    make_checkpoint_store = || { crate::make_checkpoint_store() },
    make_coordinator = || { crate::make_coordinator() },
}

#[tokio::test]
async fn first_append_creates_one_signed_authoritative_branch() {
    let fixture = make_store();
    let stream_id = StreamId::try_new("tiber:repository").expect("valid stream id");
    let writes = StreamWrites::new()
        .register_stream(stream_id.clone(), StreamVersion::new(0))
        .expect("register stream")
        .append(ContractTestEvent::new(stream_id))
        .expect("append event");

    fixture
        .append_events(writes)
        .await
        .expect("publish first event transaction");

    let candidate = git_output(
        fixture.origin.parent().expect("origin parent"),
        [
            "--git-dir",
            fixture.origin.to_str().expect("UTF-8 origin"),
            "rev-parse",
            "refs/heads/tiber",
        ],
    );
    let commit = git_output(
        &fixture.repository,
        ["cat-file", "commit", candidate.trim()],
    );
    assert!(commit.contains("gpgsig -----BEGIN SSH SIGNATURE-----"));

    let refs = git_output(
        fixture.origin.parent().expect("origin parent"),
        [
            "--git-dir",
            fixture.origin.to_str().expect("UTF-8 origin"),
            "for-each-ref",
            "--format=%(refname)",
            "refs/heads",
        ],
    );
    assert_eq!(refs.trim(), "refs/heads/tiber");
}

#[tokio::test]
async fn two_writers_publish_one_candidate_and_report_one_version_conflict() {
    let fixture = make_store();
    let left = fixture.inner.clone();
    let right = GitEventStore::open(&fixture.repository).expect("open second writer");
    let stream_id = StreamId::try_new("tiber:board").expect("valid stream id");
    let make_writes = || {
        StreamWrites::new()
            .register_stream(stream_id.clone(), StreamVersion::new(0))
            .expect("register stream")
            .append(ContractTestEvent::new(stream_id.clone()))
            .expect("append event")
    };

    let (left_result, right_result) = tokio::join!(
        left.append_events(make_writes()),
        right.append_events(make_writes())
    );
    let results = [left_result, right_result];

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(
                result,
                Err(eventcore_types::EventStoreError::VersionConflict { .. })
            ))
            .count(),
        1
    );
}

fn git_output<const N: usize>(repository: &Path, arguments: [&str; N]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(arguments)
        .output()
        .expect("run Git command");
    assert!(
        output.status.success(),
        "Git command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}
