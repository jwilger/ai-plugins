mod support;

use std::process::Command;

use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn metadata_reports_task_commit_time_from_tasks_branch() {
    let repo = TempRepo::initialized();
    assert_success(repo.taskbranch(["init"]));
    assert_success(repo.taskbranch(["create", "Date stamped task"]));
    assert_success(
        Command::new(env!("CARGO_BIN_EXE_taskbranch"))
            .arg("sync")
            .current_dir(repo.path())
            .env("GIT_AUTHOR_DATE", "2024-01-02T03:04:05Z")
            .env("GIT_COMMITTER_DATE", "2024-01-02T03:04:05Z")
            .output()
            .expect("sync taskbranch"),
    );

    let metadata = repo.taskbranch(["metadata", "date-stamped-task.md"]);

    assert_success_ref(&metadata);
    assert_eq!(
        String::from_utf8(metadata.stdout).expect("metadata output should be utf8"),
        "todo/date-stamped-task.md\tDate stamped task\tcommitted_at=2024-01-02T03:04:05Z\n"
    );
}
