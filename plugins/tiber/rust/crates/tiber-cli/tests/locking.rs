mod support;

use std::fs;
use std::fs::OpenOptions;
use std::thread;
use std::time::Duration;

use support::{assert_success, TempRepo};

#[test]
fn write_commands_fail_when_tiber_lock_is_held() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    let git_common_dir = repo.git_output(["rev-parse", "--git-common-dir"]);
    assert_success(git_common_dir.clone());
    let git_common_dir =
        String::from_utf8(git_common_dir.stdout).expect("git common dir should be utf8");
    let lock_dir = repo.path().join(git_common_dir.trim()).join("tiber");
    fs::create_dir_all(&lock_dir).expect("create tiber lock dir");
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_dir.join("tiber.lock"))
        .expect("open lock file");
    lock_file.lock().expect("hold lock file");

    let create = repo.tiber_with_env(
        ["create", "Blocked by lock"],
        [("TIBER_LOCK_RETRY_TIMEOUT_MS", "0")],
    );

    assert!(!create.status.success(), "write command should fail");
    let stderr = String::from_utf8(create.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("tiber_lock_busy"));
    let tree = repo.git_output(["ls-tree", "-r", "--name-only", "tasks", "backlog"]);
    assert_success(tree.clone());
    assert!(!String::from_utf8(tree.stdout)
        .expect("tree output should be utf8")
        .contains("blocked-by-lock"));
    drop(lock_file);
}

#[test]
fn write_commands_retry_when_tiber_lock_is_released() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    let git_common_dir = repo.git_output(["rev-parse", "--git-common-dir"]);
    assert_success(git_common_dir.clone());
    let git_common_dir =
        String::from_utf8(git_common_dir.stdout).expect("git common dir should be utf8");
    let lock_dir = repo.path().join(git_common_dir.trim()).join("tiber");
    fs::create_dir_all(&lock_dir).expect("create tiber lock dir");
    let lock_path = lock_dir.join("tiber.lock");
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .expect("open lock file");
    lock_file.lock().expect("hold lock file");

    let releaser = {
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            drop(lock_file);
        })
    };

    let create = repo.tiber_with_env(
        ["create", "Retried after lock"],
        [
            ("TIBER_LOCK_RETRY_TIMEOUT_MS", "1000"),
            ("TIBER_LOCK_RETRY_INTERVAL_MS", "20"),
        ],
    );

    releaser.join().expect("lock releaser should finish");
    assert_success(create);
    let tree = repo.git_output(["ls-tree", "-r", "--name-only", "tasks", "backlog"]);
    assert_success(tree.clone());
    assert!(String::from_utf8(tree.stdout)
        .expect("tree output should be utf8")
        .contains("retried-after-lock"));
}
