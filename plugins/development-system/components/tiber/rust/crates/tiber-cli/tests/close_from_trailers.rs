pub mod support;

use std::fs;
use std::process::Command;

use support::{assert_success, assert_success_ref, task_stem, TempRepo};

fn enable_final_review_policy(repo: &TempRepo) {
    enable_final_review_policy_with_minimum(repo, 3);
}

fn enable_final_review_policy_with_minimum(repo: &TempRepo, minimum: usize) {
    fs::write(
        repo.path().join(".tiber.toml"),
        format!("[final_review]\nminimum_clean_reviews = {minimum}\n"),
    )
    .expect("write tiber config");
    fs::write(repo.path().join("verification.txt"), "just ci: pass\n")
        .expect("write verification receipt");
    repo.git(["add", ".tiber.toml", "verification.txt"]);
    repo.git(["commit", "-m", "Enable final review policy"]);
}

#[test]
fn close_from_trailers_gates_an_already_done_legacy_task_after_policy_enablement() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Legacy completed work"]));
    assert_success(repo.tiber(["transition", "legacy-completed-work", "in-progress"]));
    assert_success(repo.tiber(["transition", "legacy-completed-work", "done"]));
    task_stem(&repo, "done", "legacy-completed-work");

    enable_final_review_policy(&repo);
    fs::write(repo.path().join("release.txt"), "release\n").expect("write release marker");
    repo.git(["add", "release.txt"]);
    repo.git([
        "commit",
        "-m",
        "Reference legacy work\n\nCloses: legacy-completed-work",
    ]);

    let close = repo.tiber(["close-from-trailers"]);

    assert!(!close.status.success());
    let stderr = String::from_utf8(close.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("final_review_evidence_incomplete")
            && stderr.contains("delivery_blocked=true")
            && stderr.contains("missing=3"),
        "{stderr}"
    );
    task_stem(&repo, "done", "legacy-completed-work");
}

#[test]
fn close_from_trailers_revalidates_an_already_done_reviewed_task() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Protected delivery"]));
    assert_success(repo.tiber(["transition", "protected-delivery", "in-progress"]));
    fs::write(repo.path().join("delivery.txt"), "reviewed\n").expect("write delivery file");
    for iteration in 1..=3 {
        record_delivery_review(&repo, iteration);
    }
    repo.git(["add", "delivery.txt"]);
    repo.git(["commit", "-m", "Commit reviewed delivery"]);
    assert_success(repo.tiber(["transition", "protected-delivery", "done"]));

    fs::write(
        repo.path().join("delivery.txt"),
        "changed after completion\n",
    )
    .expect("change reviewed delivery file");
    repo.git(["add", "delivery.txt"]);
    repo.git([
        "commit",
        "-m",
        "Reference changed reviewed work\n\nCloses: protected-delivery",
    ]);

    let close = repo.tiber(["close-from-trailers"]);

    assert!(!close.status.success());
    let stderr = String::from_utf8(close.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("delivery_blocked=true"), "{stderr}");
    assert!(stderr.contains("condition=source_changed"), "{stderr}");
    task_stem(&repo, "done", "protected-delivery");
}

#[test]
fn close_from_trailers_preserves_reviews_across_content_identical_staging_and_commit() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    fs::write(repo.path().join("delivery.txt"), "before\n").expect("write delivery file");
    fs::write(repo.path().join("deleted.txt"), "remove me\n").expect("write deleted file");
    repo.git(["add", "delivery.txt", "deleted.txt"]);
    repo.git(["commit", "-m", "Add reviewed delivery inputs"]);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Protected delivery"]));
    assert_success(repo.tiber(["transition", "protected-delivery", "in-progress"]));

    fs::write(repo.path().join("delivery.txt"), "after\n").expect("change delivery file");
    fs::remove_file(repo.path().join("deleted.txt")).expect("delete reviewed file");
    for iteration in 1..=3 {
        let review_id = format!("review-{iteration}");
        let reviewer = format!("reviewer-{iteration}");
        assert_success(repo.tiber([
            "review",
            "protected-delivery",
            "--review-id",
            &review_id,
            "--reviewer",
            &reviewer,
            "--reviewer-type",
            "independent-final-review",
            "--scope",
            "delivery.txt",
            "--scope",
            "deleted.txt",
            "--commit-range",
            "HEAD..HEAD",
            "--outcome",
            "clean",
            "--evidence",
            "review report receipt",
            "--timestamp",
            "2026-08-27T12:00:00Z",
            "--source-fingerprint",
            "auto",
            "--verification-scope",
            "verification.txt",
            "--verification-fingerprint",
            "auto",
        ]));
    }

    repo.git(["add", "--all"]);
    repo.git([
        "commit",
        "-m",
        "Ship reviewed content\n\nCloses: protected-delivery",
    ]);

    let close = repo.tiber(["close-from-trailers"]);

    assert_success_ref(&close);
    let done = task_stem(&repo, "done", "protected-delivery");
    let rendered = repo.task_file("done", &done);
    let commit = String::from_utf8(repo.git_output(["rev-parse", "HEAD"]).stdout)
        .expect("commit oid utf8")
        .trim()
        .to_string();
    let tree = String::from_utf8(repo.git_output(["rev-parse", "HEAD^{tree}"]).stdout)
        .expect("tree oid utf8")
        .trim()
        .to_string();
    assert!(rendered.contains(&format!("completion_snapshot commit={commit} tree={tree}")));
}

#[test]
fn close_from_trailers_rejects_staged_deletion_with_same_path_untracked_replacement() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    fs::write(repo.path().join("delivery.txt"), "reviewed but omitted\n")
        .expect("write delivery file");
    repo.git(["add", "delivery.txt"]);
    repo.git(["commit", "-m", "Add delivery input"]);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Protected delivery"]));
    assert_success(repo.tiber(["transition", "protected-delivery", "in-progress"]));

    repo.git(["rm", "--cached", "delivery.txt"]);
    for iteration in 1..=3 {
        record_delivery_review(&repo, iteration);
    }
    repo.git([
        "commit",
        "-m",
        "Omit reviewed content\n\nCloses: protected-delivery",
    ]);

    let close = repo.tiber(["close-from-trailers"]);

    assert!(!close.status.success());
    let stderr = String::from_utf8(close.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("delivery_blocked=true"), "{stderr}");
    assert!(
        stderr.contains("condition=source_not_committed"),
        "{stderr}"
    );
    task_stem(&repo, "in-progress", "protected-delivery");
}

#[test]
fn close_from_trailers_rejects_staged_deletion_with_same_path_ignored_replacement() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    fs::write(repo.path().join("delivery.txt"), "reviewed but omitted\n")
        .expect("write delivery file");
    repo.git(["add", "delivery.txt"]);
    repo.git(["commit", "-m", "Add delivery input"]);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Protected delivery"]));
    assert_success(repo.tiber(["transition", "protected-delivery", "in-progress"]));

    fs::write(repo.path().join(".git/info/exclude"), "delivery.txt\n")
        .expect("ignore retained delivery file");
    repo.git(["rm", "--cached", "delivery.txt"]);
    for iteration in 1..=3 {
        record_delivery_review(&repo, iteration);
    }
    repo.git([
        "commit",
        "-m",
        "Omit ignored reviewed content\n\nCloses: protected-delivery",
    ]);

    let close = repo.tiber(["close-from-trailers"]);

    assert!(!close.status.success());
    let stderr = String::from_utf8(close.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("delivery_blocked=true"), "{stderr}");
    assert!(
        stderr.contains("condition=source_not_committed"),
        "{stderr}"
    );
    task_stem(&repo, "in-progress", "protected-delivery");
}

fn record_delivery_review(repo: &TempRepo, iteration: usize) {
    let review_id = format!("review-{iteration}");
    let reviewer = format!("reviewer-{iteration}");
    assert_success(repo.tiber([
        "review",
        "protected-delivery",
        "--review-id",
        &review_id,
        "--reviewer",
        &reviewer,
        "--reviewer-type",
        "independent-final-review",
        "--scope",
        "delivery.txt",
        "--commit-range",
        "HEAD..HEAD",
        "--outcome",
        "clean",
        "--evidence",
        "review report receipt",
        "--timestamp",
        "2026-08-27T12:00:00Z",
        "--source-fingerprint",
        "auto",
        "--verification-scope",
        "verification.txt",
        "--verification-fingerprint",
        "auto",
    ]));
}

#[test]
fn close_from_trailers_moves_closed_tasks_to_done() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Fix bug"]));
    let fix_bug = task_stem(&repo, "backlog", "fix-bug");
    fs::write(repo.path().join("fix.txt"), "fixed\n").expect("write fix file");
    repo.git(["add", "fix.txt"]);
    repo.git(["commit", "-m", "Fix bug\n\nCloses: fix-bug"]);

    let close = repo.tiber(["close-from-trailers"]);

    assert_success_ref(&close);
    assert_eq!(
        String::from_utf8(close.stdout).expect("stdout should be utf8"),
        format!("closed {fix_bug}\n")
    );
    task_stem(&repo, "done", "fix-bug");
    assert_eq!(repo.order_file(), "");
}

#[test]
fn close_from_trailers_blocks_delivery_without_required_final_reviews() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Protected delivery"]));
    assert_success(repo.tiber(["transition", "protected-delivery", "in-progress"]));
    fs::write(repo.path().join("delivery.txt"), "delivered\n").expect("write delivery file");
    repo.git(["add", "delivery.txt"]);
    repo.git([
        "commit",
        "-m",
        "Protected delivery\n\nCloses: protected-delivery",
    ]);

    let close = repo.tiber(["close-from-trailers"]);

    assert!(
        !close.status.success(),
        "delivery closure should be blocked"
    );
    let stderr = String::from_utf8(close.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("final_review_evidence_incomplete")
            && stderr.contains("delivery_blocked=true")
            && stderr.contains("missing=3"),
        "stderr should identify the delivery gate: {stderr}"
    );
    task_stem(&repo, "in-progress", "protected-delivery");
}

#[test]
fn close_from_trailers_rejects_reused_review_identity() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Protected delivery"]));
    assert_success(repo.tiber(["transition", "protected-delivery", "in-progress"]));
    fs::write(repo.path().join("delivery.txt"), "delivered\n").expect("write delivery file");
    for iteration in 1..=3 {
        let reviewer = format!("reviewer-{iteration}");
        assert_success(repo.tiber([
            "review",
            "protected-delivery",
            "--review-id",
            "reused-review-id",
            "--reviewer",
            &reviewer,
            "--reviewer-type",
            "independent-final-review",
            "--scope",
            "delivery.txt",
            "--commit-range",
            "HEAD..HEAD",
            "--outcome",
            "clean",
            "--evidence",
            "review report receipt",
            "--timestamp",
            "2026-08-27T12:00:00Z",
            "--source-fingerprint",
            "auto",
            "--verification-scope",
            "verification.txt",
            "--verification-fingerprint",
            "auto",
        ]));
    }
    repo.git(["add", "delivery.txt"]);
    repo.git([
        "commit",
        "-m",
        "Protected delivery\n\nCloses: protected-delivery",
    ]);

    let close = repo.tiber(["close-from-trailers"]);

    assert!(
        !close.status.success(),
        "reused review identity should block delivery"
    );
    let stderr = String::from_utf8(close.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("final_review_independence_incomplete")
            && stderr.contains("delivery_blocked=true")
            && stderr.contains("distinct_reviews=1"),
        "stderr should identify the reused review identity: {stderr}"
    );
}

#[test]
fn close_from_trailers_rejects_an_uncommitted_policy_opt_out() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Protected delivery"]));
    assert_success(repo.tiber(["transition", "protected-delivery", "in-progress"]));
    fs::write(repo.path().join("delivery.txt"), "delivered\n").expect("write delivery file");
    repo.git(["add", "delivery.txt"]);
    repo.git([
        "commit",
        "-m",
        "Protected delivery\n\nCloses: protected-delivery",
    ]);
    fs::write(
        repo.path().join(".tiber.toml"),
        "[final_review]\nminimum_clean_reviews = 0\n",
    )
    .expect("write temporary opt out");

    let close = repo.tiber(["close-from-trailers"]);

    assert!(!close.status.success());
    let stderr = String::from_utf8(close.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("final_review_config_uncommitted"),
        "{stderr}"
    );
    task_stem(&repo, "in-progress", "protected-delivery");
}

#[test]
fn close_from_trailers_ignores_uncommitted_verification_bytes_outside_immutable_head() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Protected delivery"]));
    assert_success(repo.tiber(["transition", "protected-delivery", "in-progress"]));
    fs::write(repo.path().join("delivery.txt"), "delivered\n").expect("write delivery file");
    repo.git(["add", "delivery.txt"]);
    for iteration in 1..=3 {
        record_delivery_review(&repo, iteration);
    }
    repo.git([
        "commit",
        "-m",
        "Protected delivery\n\nCloses: protected-delivery",
    ]);
    fs::write(repo.path().join("verification.txt"), "just ci: failed\n")
        .expect("change verification evidence");

    let close = repo.tiber(["close-from-trailers"]);

    assert_success_ref(&close);
    task_stem(&repo, "done", "protected-delivery");
    assert!(String::from_utf8(
        repo.git_output(["status", "--short", "verification.txt"])
            .stdout
    )
    .expect("status utf8")
    .contains("verification.txt"));
}

#[test]
fn close_from_trailers_is_a_noop_without_current_head_closures() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    let tasks_before = repo.git_output(["rev-parse", "tiber"]).stdout;
    fs::write(repo.path().join("ordinary.txt"), "ordinary\n").expect("write ordinary change");
    repo.git(["add", "ordinary.txt"]);
    repo.git(["commit", "-m", "Ordinary main change"]);

    let close = repo.tiber_with_env(
        ["close-from-trailers"],
        [
            ("GIT_AUTHOR_DATE", "2001-01-01T00:00:00Z"),
            ("GIT_COMMITTER_DATE", "2001-01-01T00:00:00Z"),
        ],
    );

    assert_success_ref(&close);
    assert!(close.stdout.is_empty());
    assert_eq!(repo.git_output(["rev-parse", "tiber"]).stdout, tasks_before);
}

#[test]
fn close_from_trailers_fetches_remote_tasks_before_resolving_closures() {
    let origin = TempRepo::new();
    origin.git(["init", "--bare"]);

    let seed = TempRepo::initialized();
    assert_success(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(origin.path())
            .current_dir(seed.path())
            .output()
            .expect("add origin remote"),
    );
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);
    assert_success(seed.tiber(["init"]));
    assert_success(seed.tiber(["create", "Publish architecture records"]));
    let architecture = task_stem(&seed, "backlog", "publish-architecture-records");
    assert_success(seed.tiber(["create", "Publish release notes"]));
    let release_notes = task_stem(&seed, "backlog", "publish-release-notes");
    assert_success(seed.tiber(["transition", &architecture, "in-progress"]));
    assert_success(seed.tiber(["transition", &release_notes, "in-progress"]));
    fs::write(seed.path().join("architecture.md"), "published\n").expect("write completed work");
    seed.git(["add", "architecture.md"]);
    seed.git([
        "commit",
        "-m",
        &format!("Publish architecture records\n\nCloses: {architecture}\nCloses: {release_notes}"),
    ]);
    seed.git(["push", "origin", "main"]);

    let automation = TempRepo::new();
    assert_success(
        Command::new("git")
            .args(["clone", origin.path().to_str().expect("origin path utf8")])
            .arg(automation.path())
            .output()
            .expect("clone automation checkout"),
    );
    automation.git(["config", "user.email", "tiber@example.test"]);
    automation.git(["config", "user.name", "Tiber Test"]);
    automation.git(["config", "commit.gpgsign", "false"]);

    let close = automation.tiber(["close-from-trailers"]);

    assert_success_ref(&close);
    let mut expected_closed = [architecture.as_str(), release_notes.as_str()];
    expected_closed.sort();
    assert_eq!(
        String::from_utf8(close.stdout).expect("stdout should be utf8"),
        expected_closed
            .into_iter()
            .map(|task| format!("closed {task}\n"))
            .collect::<String>()
    );
    for task in [architecture, release_notes] {
        let shown = automation.tiber(["show", &task]);
        assert_success_ref(&shown);
        task_stem(&automation, "done", task.splitn(3, '-').nth(2).unwrap());
    }
}

#[test]
fn close_from_trailers_fails_when_a_requested_task_is_missing() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    fs::write(repo.path().join("fix.txt"), "fixed\n").expect("write completed work");
    repo.git(["add", "fix.txt"]);
    repo.git(["commit", "-m", "Fix missing task\n\nCloses: 20260721-miss"]);

    let close = repo.tiber(["close-from-trailers"]);

    assert!(!close.status.success());
    assert_eq!(
        String::from_utf8(close.stderr).expect("stderr should be utf8"),
        "tiber.parse_error task_ref_missing ref=20260721-miss\n"
    );
}

#[test]
fn close_from_trailers_ignores_closures_from_older_commits() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    fs::write(repo.path().join("old.txt"), "old\n").expect("write historical work");
    repo.git(["add", "old.txt"]);
    repo.git(["commit", "-m", "Historical work\n\nCloses: 20260720-gone"]);
    assert_success(repo.tiber(["create", "Current delivery"]));
    let current = task_stem(&repo, "backlog", "current-delivery");
    fs::write(repo.path().join("current.txt"), "current\n").expect("write current work");
    repo.git(["add", "current.txt"]);
    repo.git([
        "commit",
        "-m",
        &format!("Current delivery\n\nCloses: {current}"),
    ]);

    let close = repo.tiber(["close-from-trailers"]);

    assert_success(close);
    task_stem(&repo, "done", "current-delivery");
}
