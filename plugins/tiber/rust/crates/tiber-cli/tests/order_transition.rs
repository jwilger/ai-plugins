mod support;

use std::fs;

use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn next_show_transition_and_prioritize_follow_order_md() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Write docs"]));
    assert_success(repo.tiber(["create", "Review docs"]));

    let next = repo.tiber(["next"]);
    assert_success_ref(&next);
    assert_eq!(
        String::from_utf8(next.stdout).expect("next output should be utf8"),
        "todo/write-docs.md\tWrite docs\n"
    );

    let show = repo.tiber(["show", "todo/write-docs.md"]);
    assert_success_ref(&show);
    assert_eq!(
        String::from_utf8(show.stdout).expect("show output should be utf8"),
        "# Write docs\n"
    );

    assert_success(repo.tiber(["transition", "todo/write-docs.md", "doing"]));
    assert!(repo
        .path()
        .join(".tasks")
        .join("doing")
        .join("write-docs.md")
        .exists());
    assert!(!repo
        .path()
        .join(".tasks")
        .join("todo")
        .join("write-docs.md")
        .exists());

    assert_success(repo.tiber([
        "prioritize",
        "todo/review-docs.md",
        "--before",
        "doing/write-docs.md",
    ]));

    let order = fs::read_to_string(repo.path().join(".tasks").join("order.md"))
        .expect("order file should be readable");
    assert_eq!(order, "todo/review-docs.md\ndoing/write-docs.md\n");
}

#[test]
fn task_refs_can_use_unique_filename_identity_and_report_ambiguity() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Write docs"]));
    assert_success(repo.tiber(["create", "Review docs"]));

    let show = repo.tiber(["show", "write-docs.md"]);
    assert_success_ref(&show);
    assert_eq!(
        String::from_utf8(show.stdout).expect("show output should be utf8"),
        "# Write docs\n"
    );

    assert_success(repo.tiber(["transition", "write-docs.md", "doing"]));
    assert_success(repo.tiber(["prioritize", "review-docs.md", "--before", "write-docs.md"]));
    let order = fs::read_to_string(repo.path().join(".tasks").join("order.md"))
        .expect("order file should be readable");
    assert_eq!(order, "todo/review-docs.md\ndoing/write-docs.md\n");

    fs::create_dir_all(repo.path().join(".tasks").join("done")).expect("create done status");
    fs::write(
        repo.path()
            .join(".tasks")
            .join("done")
            .join("write-docs.md"),
        "# Archived duplicate\n",
    )
    .expect("write duplicate task filename");

    let ambiguous = repo.tiber(["show", "write-docs.md"]);
    assert!(!ambiguous.status.success(), "ambiguous ref should fail");
    let stderr = String::from_utf8(ambiguous.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("ambiguous_task_ref ref=write-docs.md"));
    assert!(stderr.contains("doing/write-docs.md"));
    assert!(stderr.contains("done/write-docs.md"));
}
