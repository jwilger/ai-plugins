mod support;

use std::fs;

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
    fs::write(lock_dir.join("tiber.lock"), "held by test\n").expect("write lock file");

    let create = repo.tiber(["create", "Blocked by lock"]);

    assert!(!create.status.success(), "write command should fail");
    let stderr = String::from_utf8(create.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("tiber_lock_busy"));
    assert!(!repo
        .path()
        .join(".tasks")
        .join("todo")
        .join("blocked-by-lock.md")
        .exists());
}
