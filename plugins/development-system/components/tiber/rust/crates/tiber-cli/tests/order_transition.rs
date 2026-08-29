pub mod support;

use std::fs;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use support::{assert_success, assert_success_ref, task_stem, TempRepo};

fn record_review(repo: &TempRepo, iteration: usize, outcome: &str) {
    record_review_with_scope(repo, iteration, outcome, &["README.md"]);
}

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
fn dirty_worktree_after_review_does_not_reinterpret_the_reviewed_commit() {
    let repo = TempRepo::initialized();
    enable_final_review_policy_with_minimum(&repo, 4);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=4 {
        record_review(&repo, iteration, "clean");
    }
    fs::write(repo.path().join("README.md"), "changed after review\n")
        .expect("change reviewed source");

    assert_success(repo.tiber(["transition", "reviewed-work", "done"]));
    task_stem(&repo, "done", "reviewed-work");
}

#[test]
fn whitespace_equivalent_review_identities_are_not_independent() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for (review_id, reviewer) in [
        ("review-one", "reviewer-one"),
        (" review-one", " reviewer-one"),
        ("review-one ", "reviewer-one "),
    ] {
        assert_success(repo.tiber([
            "review",
            "reviewed-work",
            "--review-id",
            review_id,
            "--reviewer",
            reviewer,
            "--reviewer-type",
            "independent-final-review",
            "--scope",
            "README.md",
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

    let completion = repo.tiber(["transition", "reviewed-work", "done"]);

    assert!(!completion.status.success());
    let stderr = String::from_utf8(completion.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("final_review_independence_incomplete")
            && stderr.contains("distinct_reviewers=1")
            && stderr.contains("distinct_reviews=1"),
        "{stderr}"
    );
}

#[test]
fn clean_review_rejects_a_non_independent_reviewer_type() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));

    let review = repo.tiber([
        "review",
        "reviewed-work",
        "--review-id",
        "self-review-1",
        "--reviewer",
        "author-1",
        "--reviewer-type",
        "author-self-review",
        "--scope",
        "README.md",
        "--commit-range",
        "HEAD..HEAD",
        "--outcome",
        "clean",
        "--evidence",
        "self review receipt",
        "--timestamp",
        "2026-08-27T12:00:00Z",
        "--source-fingerprint",
        "auto",
        "--verification-scope",
        "verification.txt",
        "--verification-fingerprint",
        "auto",
    ]);

    assert!(!review.status.success());
    let stderr = String::from_utf8(review.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("final_review_reviewer_type_invalid")
            && stderr.contains("expected=independent-final-review"),
        "{stderr}"
    );
}

#[test]
fn gitlink_scope_can_be_reviewed_and_completed() {
    let dependency = TempRepo::initialized();
    let repo = TempRepo::initialized();
    repo.git([
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        dependency
            .path()
            .to_str()
            .expect("dependency path should be utf8"),
        "dependency",
    ]);
    repo.git(["commit", "-m", "Add dependency gitlink"]);
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review_with_scope(&repo, iteration, "clean", &["dependency"]);
    }

    assert_success(repo.tiber(["transition", "reviewed-work", "done"]));
    task_stem(&repo, "done", "reviewed-work");
}

#[test]
fn changed_gitlink_checkout_does_not_reinterpret_the_parent_commit() {
    let dependency = TempRepo::initialized();
    let repo = TempRepo::initialized();
    repo.git([
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        dependency
            .path()
            .to_str()
            .expect("dependency path should be utf8"),
        "dependency",
    ]);
    repo.git(["commit", "-m", "Add dependency gitlink"]);
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review_with_scope(&repo, iteration, "clean", &["dependency"]);
    }
    repo.git(["-C", "dependency", "config", "user.name", "Tiber Test"]);
    repo.git([
        "-C",
        "dependency",
        "config",
        "user.email",
        "tiber@example.invalid",
    ]);
    repo.git(["-C", "dependency", "config", "commit.gpgsign", "false"]);
    fs::write(
        repo.path().join("dependency/README.md"),
        "changed submodule revision\n",
    )
    .expect("change submodule file");
    repo.git(["-C", "dependency", "add", "README.md"]);
    repo.git(["-C", "dependency", "commit", "-m", "Change dependency"]);

    assert_success(repo.tiber(["transition", "reviewed-work", "done"]));
    task_stem(&repo, "done", "reviewed-work");
}

#[test]
fn uninitialized_gitlink_scope_is_rejected() {
    let dependency = TempRepo::initialized();
    let repo = TempRepo::initialized();
    repo.git([
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        dependency
            .path()
            .to_str()
            .expect("dependency path should be utf8"),
        "dependency",
    ]);
    repo.git(["commit", "-m", "Add dependency gitlink"]);
    repo.git(["submodule", "deinit", "-f", "dependency"]);
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));

    let review = repo.tiber([
        "review",
        "reviewed-work",
        "--review-id",
        "review-one",
        "--reviewer",
        "reviewer-one",
        "--reviewer-type",
        "independent-final-review",
        "--scope",
        "dependency",
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
    ]);

    assert!(!review.status.success());
    let stderr = String::from_utf8(review.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("final_review_gitlink_unavailable")
            && stderr.contains("initialize the scoped submodule checkout"),
        "{stderr}"
    );
}

#[test]
fn absent_uninitialized_gitlink_scope_is_rejected() {
    let dependency = TempRepo::initialized();
    let repo = TempRepo::initialized();
    repo.git([
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        dependency
            .path()
            .to_str()
            .expect("dependency path should be utf8"),
        "dependency",
    ]);
    repo.git(["commit", "-m", "Add dependency gitlink"]);
    repo.git(["submodule", "deinit", "-f", "dependency"]);
    fs::remove_dir(repo.path().join("dependency")).expect("remove empty submodule directory");
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));

    let review = repo.tiber([
        "review",
        "reviewed-work",
        "--review-id",
        "review-one",
        "--reviewer",
        "reviewer-one",
        "--reviewer-type",
        "independent-final-review",
        "--scope",
        "dependency",
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
    ]);

    assert!(!review.status.success());
    let stderr = String::from_utf8(review.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("final_review_gitlink_unavailable"),
        "{stderr}"
    );
}

#[test]
fn tracked_file_replaced_by_uncommitted_directory_is_rejected_at_completion() {
    let repo = TempRepo::initialized();
    fs::write(repo.path().join("config"), "legacy config\n").expect("write tracked config");
    repo.git(["add", "config"]);
    repo.git(["commit", "-m", "Add tracked config file"]);
    fs::remove_file(repo.path().join("config")).expect("remove tracked config file");
    fs::create_dir(repo.path().join("config")).expect("create replacement config directory");
    fs::write(repo.path().join("config/value.txt"), "replacement config\n")
        .expect("write replacement config file");
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review_with_scope(&repo, iteration, "clean", &["config"]);
    }

    let completion = repo.tiber(["transition", "reviewed-work", "done"]);
    assert!(!completion.status.success());
    let stderr = String::from_utf8(completion.stderr).expect("stderr utf8");
    assert!(
        stderr.contains("condition=source_not_committed"),
        "{stderr}"
    );
    task_stem(&repo, "in-progress", "reviewed-work");
}

#[test]
fn content_identical_file_to_directory_commit_preserves_review() {
    let repo = TempRepo::initialized();
    fs::write(repo.path().join("config"), "legacy config\n").expect("write config");
    repo.git(["add", "config"]);
    repo.git(["commit", "-m", "Add tracked config file"]);
    fs::remove_file(repo.path().join("config")).expect("remove file");
    fs::create_dir(repo.path().join("config")).expect("create directory");
    fs::write(repo.path().join("config/value.txt"), "replacement config\n")
        .expect("write replacement");
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review_with_scope(&repo, iteration, "clean", &["config"]);
    }
    repo.git(["add", "--all"]);
    repo.git(["commit", "-m", "Commit reviewed directory replacement"]);

    assert_success(repo.tiber(["transition", "reviewed-work", "done"]));
    task_stem(&repo, "done", "reviewed-work");
}

#[test]
fn literal_directory_scope_matches_the_immutable_completion_tree() {
    let repo = TempRepo::initialized();
    fs::create_dir(repo.path().join("src")).expect("create source directory");
    fs::write(repo.path().join("src/lib.rs"), "reviewed\n").expect("write reviewed source");
    repo.git(["add", "src/lib.rs"]);
    repo.git(["commit", "-m", "Add reviewed source"]);
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review_with_scope(&repo, iteration, "clean", &[":(literal)src"]);
    }

    assert_success(repo.tiber(["transition", "reviewed-work", "done"]));
    task_stem(&repo, "done", "reviewed-work");
}

fn record_review_with_scope(repo: &TempRepo, iteration: usize, outcome: &str, scope: &[&str]) {
    let review_id = format!("review-{iteration}");
    let reviewer = format!("reviewer-{iteration}");
    let mut args = vec![
        "review".to_string(),
        "reviewed-work".to_string(),
        "--review-id".to_string(),
        review_id,
        "--reviewer".to_string(),
        reviewer,
        "--reviewer-type".to_string(),
        "independent-final-review".to_string(),
    ];
    for pathspec in scope {
        args.extend(["--scope".to_string(), (*pathspec).to_string()]);
    }
    args.extend([
        "--commit-range".to_string(),
        "HEAD..HEAD".to_string(),
        "--outcome".to_string(),
        outcome.to_string(),
        "--evidence".to_string(),
        "review report receipt".to_string(),
        "--timestamp".to_string(),
        "2026-08-27T12:00:00Z".to_string(),
        "--source-fingerprint".to_string(),
        "auto".to_string(),
        "--verification-scope".to_string(),
        "verification.txt".to_string(),
        "--verification-fingerprint".to_string(),
        "auto".to_string(),
    ]);
    assert_success(repo.tiber(args));
}

#[test]
fn next_show_transition_and_prioritize_follow_order_md() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Write docs"]));
    assert_success(repo.tiber(["create", "Review docs"]));
    let write_docs = task_stem(&repo, "backlog", "write-docs");
    let review_docs = task_stem(&repo, "backlog", "review-docs");

    let next = repo.tiber(["next"]);
    assert_success_ref(&next);
    assert_eq!(
        String::from_utf8(next.stdout).expect("next output should be utf8"),
        format!("{write_docs}\tWrite docs\n")
    );

    let show = repo.tiber(["show", "write-docs"]);
    assert_success_ref(&show);
    assert!(String::from_utf8(show.stdout)
        .expect("show output should be utf8")
        .contains("title: Write docs"));

    assert_success(repo.tiber(["transition", "write-docs", "in-progress"]));
    task_stem(&repo, "in-progress", "write-docs");
    let in_progress = repo.task_file("in-progress", &write_docs);
    assert!(in_progress.contains("claim:\n"));
    assert!(in_progress.contains("  host: "));
    assert!(in_progress.contains("  session: "));
    assert!(
        !String::from_utf8(repo.tiber(["list", "--status", "backlog"]).stdout)
            .unwrap()
            .contains(&write_docs)
    );

    assert_success(repo.tiber(["prioritize", "review-docs", "--before", "write-docs"]));

    assert_eq!(repo.order_file(), format!("{review_docs}\n{write_docs}\n"));
}

#[test]
fn transition_releases_claim_when_leaving_in_progress() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Claim lifecycle"]));
    let task = task_stem(&repo, "backlog", "claim-lifecycle");

    assert_success(repo.tiber_with_env(
        ["transition", "claim-lifecycle", "in-progress"],
        [
            ("TIBER_CLAIM_HOST", "test-host"),
            ("TIBER_CLAIM_SESSION", "test-session"),
        ],
    ));
    let in_progress = repo.task_file("in-progress", &task);
    assert!(in_progress.contains("claim:\n  host: test-host\n  session: test-session\n"));

    assert_success(repo.tiber(["transition", "claim-lifecycle", "done"]));
    let done = repo.task_file("done", &task);
    assert!(!done.contains("claim:"));
    assert!(!done.contains("test-session"));
}

#[test]
fn transition_to_done_rejects_missing_final_reviews_when_policy_is_enabled() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Protected work"]));
    assert_success(repo.tiber(["transition", "protected-work", "in-progress"]));

    let completion = repo.tiber(["transition", "protected-work", "done"]);

    assert!(!completion.status.success(), "completion should be blocked");
    let stderr = String::from_utf8(completion.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("final_review_evidence_incomplete")
            && stderr.contains("required=3")
            && stderr.contains("clean=0")
            && stderr.contains("missing=3"),
        "stderr should identify the unsatisfied review count: {stderr}"
    );
    task_stem(&repo, "in-progress", "protected-work");
}

#[test]
fn final_review_policy_rejects_a_nonzero_minimum_below_three() {
    let repo = TempRepo::initialized();
    fs::write(
        repo.path().join(".tiber.toml"),
        "[final_review]\nminimum_clean_reviews = 2\n",
    )
    .expect("write tiber config");

    assert_success(repo.tiber(["init"]));
    let create = repo.tiber(["create", "Weakly reviewed work"]);

    assert!(!create.status.success(), "weak policy should be rejected");
    let stderr = String::from_utf8(create.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("final_review.minimum_clean_reviews")
            && stderr.contains("expected=0_or_at_least_3")
            && stderr.contains("actual=2"),
        "stderr should explain the invalid policy minimum: {stderr}"
    );
}

#[test]
fn transition_to_done_accepts_three_clean_independent_final_reviews() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));

    for iteration in 1..=3 {
        record_review(&repo, iteration, "clean");
    }

    assert_success(repo.tiber(["transition", "reviewed-work", "done"]));
    let done = task_stem(&repo, "done", "reviewed-work");
    let rendered = repo.task_file("done", &done);
    assert!(rendered.contains("review-1") && rendered.contains("review-3"));
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
fn transition_to_done_revalidates_and_persists_a_new_immutable_snapshot() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review(&repo, iteration, "clean");
    }
    assert_success(repo.tiber(["transition", "reviewed-work", "done"]));
    fs::write(repo.path().join("unrelated.txt"), "unrelated\n").expect("write unrelated file");
    repo.git(["add", "unrelated.txt"]);
    repo.git(["commit", "-m", "Add unrelated file"]);
    let new_commit = String::from_utf8(repo.git_output(["rev-parse", "HEAD"]).stdout)
        .expect("commit oid utf8")
        .trim()
        .to_string();

    assert_success(repo.tiber(["transition", "reviewed-work", "done"]));

    let done = task_stem(&repo, "done", "reviewed-work");
    let rendered = repo.task_file("done", &done);
    assert!(rendered.contains(&format!("completion_snapshot commit={new_commit}")));
}

#[test]
fn transition_to_done_rejects_reviewed_uncommitted_source() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    fs::write(repo.path().join("uncommitted.rs"), "reviewed bytes\n").expect("write source");
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review_with_scope(&repo, iteration, "clean", &["uncommitted.rs"]);
    }

    let completion = repo.tiber(["transition", "reviewed-work", "done"]);

    assert!(!completion.status.success());
    let stderr = String::from_utf8(completion.stderr).expect("stderr utf8");
    assert!(
        stderr.contains("condition=source_not_committed"),
        "{stderr}"
    );
    assert!(stderr.contains("path=\"uncommitted.rs\""), "{stderr}");
    task_stem(&repo, "in-progress", "reviewed-work");
}

#[test]
fn transition_to_done_rejects_reviewed_uncommitted_verification_receipt() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    fs::write(
        repo.path().join("uncommitted-verification.txt"),
        "focused tests pass\n",
    )
    .expect("write verification receipt");
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        let review_id = format!("review-{iteration}");
        let reviewer = format!("reviewer-{iteration}");
        assert_success(repo.tiber([
            "review",
            "reviewed-work",
            "--review-id",
            &review_id,
            "--reviewer",
            &reviewer,
            "--reviewer-type",
            "independent-final-review",
            "--scope",
            "README.md",
            "--commit-range",
            "HEAD..HEAD",
            "--outcome",
            "clean",
            "--evidence",
            "review receipt",
            "--timestamp",
            "2026-08-27T12:00:00Z",
            "--source-fingerprint",
            "auto",
            "--verification-scope",
            "uncommitted-verification.txt",
            "--verification-fingerprint",
            "auto",
        ]));
    }

    let completion = repo.tiber(["transition", "reviewed-work", "done"]);

    assert!(!completion.status.success());
    let stderr = String::from_utf8(completion.stderr).expect("stderr utf8");
    assert!(
        stderr.contains("condition=verification_not_committed"),
        "{stderr}"
    );
    assert!(
        stderr.contains("path=\"uncommitted-verification.txt\""),
        "{stderr}"
    );
    task_stem(&repo, "in-progress", "reviewed-work");
}

#[test]
fn transition_to_done_rejects_a_different_committed_tree() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review(&repo, iteration, "clean");
    }
    fs::write(repo.path().join("README.md"), "different committed tree\n").expect("change source");
    repo.git(["add", "README.md"]);
    repo.git(["commit", "-m", "Change reviewed source"]);

    let completion = repo.tiber(["transition", "reviewed-work", "done"]);

    assert!(!completion.status.success());
    let stderr = String::from_utf8(completion.stderr).expect("stderr utf8");
    assert!(stderr.contains("condition=source_changed"), "{stderr}");
    assert!(stderr.contains("clean=0 missing=3"), "{stderr}");
    task_stem(&repo, "in-progress", "reviewed-work");
}

#[cfg(unix)]
#[test]
fn immutable_tree_fingerprint_rejects_mode_symlink_and_deletion_changes() {
    for variant in ["mode", "symlink", "deletion"] {
        let repo = TempRepo::initialized();
        enable_final_review_policy(&repo);
        let reviewed_path = match variant {
            "mode" => {
                fs::write(repo.path().join("script.sh"), "#!/bin/sh\nexit 0\n")
                    .expect("write script");
                "script.sh"
            }
            "symlink" => {
                fs::write(repo.path().join("target-a"), "a\n").expect("write target a");
                fs::write(repo.path().join("target-b"), "b\n").expect("write target b");
                std::os::unix::fs::symlink("target-a", repo.path().join("selected"))
                    .expect("create symlink");
                "selected"
            }
            "deletion" => {
                fs::write(repo.path().join("removed.txt"), "reviewed\n").expect("write file");
                "removed.txt"
            }
            _ => unreachable!(),
        };
        repo.git(["add", "--all"]);
        repo.git(["commit", "-m", "Add reviewed tree entry"]);
        assert_success(repo.tiber(["init"]));
        assert_success(repo.tiber(["create", "Reviewed work"]));
        assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
        for iteration in 1..=3 {
            record_review_with_scope(&repo, iteration, "clean", &[reviewed_path]);
        }
        match variant {
            "mode" => repo.git(["update-index", "--chmod=+x", "script.sh"]),
            "symlink" => {
                fs::remove_file(repo.path().join("selected")).expect("remove symlink");
                std::os::unix::fs::symlink("target-b", repo.path().join("selected"))
                    .expect("replace symlink");
                repo.git(["add", "selected"]);
            }
            "deletion" => repo.git(["rm", "removed.txt"]),
            _ => unreachable!(),
        }
        repo.git(["commit", "-m", "Change reviewed tree entry"]);

        let completion = repo.tiber(["transition", "reviewed-work", "done"]);
        assert!(!completion.status.success(), "{variant} change must fail");
        let stderr = String::from_utf8(completion.stderr).expect("stderr utf8");
        assert!(
            stderr.contains("condition=source_changed")
                || stderr.contains("condition=source_not_committed"),
            "{variant}: {stderr}"
        );
        task_stem(&repo, "in-progress", "reviewed-work");
    }
}

#[test]
fn completion_race_persists_the_resolved_commit_not_later_worktree_bytes() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review(&repo, iteration, "clean");
    }
    let reviewed_commit = String::from_utf8(repo.git_output(["rev-parse", "HEAD"]).stdout)
        .expect("commit oid utf8")
        .trim()
        .to_string();
    let barrier = repo.path().join("completion-barrier");
    let mut child = Command::new(env!("CARGO_BIN_EXE_tiber"))
        .args(["transition", "reviewed-work", "done"])
        .env("TIBER_TEST_COMPLETION_SNAPSHOT_BARRIER", &barrier)
        .current_dir(repo.path())
        .spawn()
        .expect("spawn completion");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !barrier.join("ready").exists() {
        assert!(
            Instant::now() < deadline,
            "completion did not reach snapshot barrier"
        );
        thread::sleep(Duration::from_millis(10));
    }
    fs::write(
        repo.path().join("README.md"),
        "changed after immutable snapshot\n",
    )
    .expect("mutate worktree");
    fs::write(barrier.join("release"), "continue\n").expect("release barrier");
    let status = child.wait().expect("wait completion");
    assert!(status.success(), "immutable completion should succeed");

    let done = task_stem(&repo, "done", "reviewed-work");
    let rendered = repo.task_file("done", &done);
    assert!(rendered.contains(&format!("completion_snapshot commit={reviewed_commit}")));
    assert!(
        String::from_utf8(repo.git_output(["status", "--short", "README.md"]).stdout)
            .expect("status utf8")
            .contains("README.md")
    );
}

#[test]
fn uncommitted_implementation_change_does_not_reinterpret_head_completion() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review(&repo, iteration, "clean");
    }
    fs::write(repo.path().join("README.md"), "# changed after review\n")
        .expect("change reviewed source");

    assert_success(repo.tiber(["transition", "reviewed-work", "done"]));
    task_stem(&repo, "done", "reviewed-work");
}

#[test]
fn newly_staged_file_is_not_included_in_immutable_head_completion() {
    let repo = TempRepo::initialized();
    fs::create_dir(repo.path().join("src")).expect("create source directory");
    fs::write(repo.path().join("src/existing.rs"), "reviewed\n").expect("write reviewed source");
    repo.git(["add", "src/existing.rs"]);
    repo.git(["commit", "-m", "Add reviewed source"]);
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review_with_scope(&repo, iteration, "clean", &["src/**"]);
    }
    fs::write(repo.path().join("src/new.rs"), "new implementation\n")
        .expect("write newly matching source");
    repo.git(["add", "src/new.rs"]);

    assert_success(repo.tiber(["transition", "reviewed-work", "done"]));
    task_stem(&repo, "done", "reviewed-work");
}

#[test]
fn newly_matching_committed_file_invalidates_reviewed_scope() {
    let repo = TempRepo::initialized();
    fs::create_dir(repo.path().join("src")).expect("create source directory");
    fs::write(repo.path().join("src/existing.rs"), "reviewed\n").expect("write source");
    repo.git(["add", "src/existing.rs"]);
    repo.git(["commit", "-m", "Add reviewed source"]);
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review_with_scope(&repo, iteration, "clean", &["src/**"]);
    }
    fs::write(
        repo.path().join("src/new.rs"),
        "unreviewed committed source\n",
    )
    .expect("write matching source");
    repo.git(["add", "src/new.rs"]);
    repo.git(["commit", "-m", "Expand reviewed scope"]);

    let completion = repo.tiber(["transition", "reviewed-work", "done"]);

    assert!(!completion.status.success());
    let stderr = String::from_utf8(completion.stderr).expect("stderr utf8");
    assert!(stderr.contains("condition=source_changed"), "{stderr}");
    assert!(stderr.contains("clean=0 missing=3"), "{stderr}");
    task_stem(&repo, "in-progress", "reviewed-work");
}

#[test]
fn new_untracked_file_is_not_included_in_immutable_head_completion() {
    let repo = TempRepo::initialized();
    fs::create_dir(repo.path().join("src")).expect("create source directory");
    fs::write(repo.path().join("src/existing.rs"), "reviewed\n").expect("write reviewed source");
    repo.git(["add", "src/existing.rs"]);
    repo.git(["commit", "-m", "Add reviewed source"]);
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review_with_scope(&repo, iteration, "clean", &["src/**"]);
    }
    fs::write(repo.path().join("src/new.rs"), "new implementation\n")
        .expect("write newly matching source");

    assert_success(repo.tiber(["transition", "reviewed-work", "done"]));
    task_stem(&repo, "done", "reviewed-work");
}

#[test]
fn staged_unreviewed_bytes_are_not_included_in_immutable_head_completion() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review(&repo, iteration, "clean");
    }
    let reviewed = fs::read_to_string(repo.path().join("README.md"))
        .expect("read reviewed working-tree content");
    fs::write(repo.path().join("README.md"), "staged but unreviewed\n")
        .expect("write staged content");
    repo.git(["add", "README.md"]);
    fs::write(repo.path().join("README.md"), reviewed)
        .expect("restore reviewed working-tree content");

    let reviewed_head = String::from_utf8(repo.git_output(["rev-parse", "HEAD"]).stdout)
        .expect("head utf8")
        .trim()
        .to_string();
    assert_success(repo.tiber(["transition", "reviewed-work", "done"]));
    let done = task_stem(&repo, "done", "reviewed-work");
    assert!(repo
        .task_file("done", &done)
        .contains(&format!("completion_snapshot commit={reviewed_head}")));
    assert!(
        String::from_utf8(repo.git_output(["diff", "--cached", "--name-only"]).stdout)
            .expect("diff utf8")
            .contains("README.md")
    );
}

#[test]
fn substantive_finding_resets_the_clean_review_sequence() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    record_review(&repo, 1, "clean");
    record_review(&repo, 2, "clean");
    record_review(&repo, 3, "finding");
    record_review(&repo, 4, "clean");
    record_review(&repo, 5, "clean");

    let completion = repo.tiber(["transition", "reviewed-work", "done"]);

    assert!(!completion.status.success());
    let stderr = String::from_utf8(completion.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("final_review_evidence_incomplete")
            && stderr.contains("clean=2")
            && stderr.contains("missing=1"),
        "stderr should explain the finding reset: {stderr}"
    );
}

#[test]
fn expanded_scope_resets_the_clean_review_sequence() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    fs::write(repo.path().join("scope.txt"), "expanded scope\n").expect("write scoped file");
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review(&repo, iteration, "clean");
    }
    record_review_with_scope(&repo, 4, "clean", &["README.md", "scope.txt"]);

    let completion = repo.tiber(["transition", "reviewed-work", "done"]);

    assert!(!completion.status.success());
    let stderr = String::from_utf8(completion.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("clean=1") && stderr.contains("missing=2"));
}

#[test]
fn changed_verification_evidence_resets_the_clean_review_sequence() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review(&repo, iteration, "clean");
    }
    fs::write(
        repo.path().join("verification.txt"),
        "just ci: rerun pass\n",
    )
    .expect("change verification receipt");
    record_review_with_scope(&repo, 4, "clean", &["README.md"]);

    let completion = repo.tiber(["transition", "reviewed-work", "done"]);

    assert!(!completion.status.success());
    let stderr = String::from_utf8(completion.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("clean=1") && stderr.contains("missing=2"));
}

#[test]
fn transition_to_done_rejects_an_uncommitted_policy_opt_out() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    fs::write(
        repo.path().join(".tiber.toml"),
        "[final_review]\nminimum_clean_reviews = 0\n",
    )
    .expect("write temporary opt out");

    let completion = repo.tiber(["transition", "reviewed-work", "done"]);

    assert!(!completion.status.success());
    let stderr = String::from_utf8(completion.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("final_review_config_uncommitted"),
        "{stderr}"
    );
    task_stem(&repo, "in-progress", "reviewed-work");
}

#[test]
fn transition_to_done_allows_a_committed_policy_opt_out() {
    let repo = TempRepo::initialized();
    fs::write(
        repo.path().join(".tiber.toml"),
        "[final_review]\nminimum_clean_reviews = 0\n",
    )
    .expect("write committed opt out");
    repo.git(["add", ".tiber.toml"]);
    repo.git(["commit", "-m", "Opt out of final review policy"]);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));

    assert_success(repo.tiber(["transition", "reviewed-work", "done"]));
}

#[test]
fn review_rejects_a_scope_that_matches_no_files() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));

    let review = repo.tiber([
        "review",
        "reviewed-work",
        "--review-id",
        "review-1",
        "--reviewer",
        "reviewer-1",
        "--reviewer-type",
        "independent-final-review",
        "--scope",
        "does-not-exist",
        "--commit-range",
        "HEAD..HEAD",
        "--outcome",
        "clean",
        "--evidence",
        "review receipt",
        "--timestamp",
        "2026-08-27T12:00:00Z",
        "--source-fingerprint",
        "auto",
        "--verification-scope",
        "verification.txt",
        "--verification-fingerprint",
        "auto",
    ]);

    assert!(!review.status.success());
    let stderr = String::from_utf8(review.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("final_review_source_scope_empty"),
        "{stderr}"
    );
}

#[test]
fn review_supports_a_pathspec_containing_spaces() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    fs::write(repo.path().join("path with spaces.txt"), "reviewed\n").expect("write spaced path");
    repo.git(["add", "path with spaces.txt"]);
    repo.git(["commit", "-m", "Add spaced path"]);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review_with_scope(&repo, iteration, "clean", &["path with spaces.txt"]);
    }

    assert_success(repo.tiber(["transition", "reviewed-work", "done"]));
}

#[test]
fn unstaged_deletion_is_not_included_in_immutable_head_completion() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    fs::write(repo.path().join("deleted.txt"), "reviewed\n").expect("write source");
    repo.git(["add", "deleted.txt"]);
    repo.git(["commit", "-m", "Add reviewed source"]);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review_with_scope(&repo, iteration, "clean", &["deleted.txt"]);
    }
    fs::remove_file(repo.path().join("deleted.txt")).expect("delete reviewed source");

    assert_success(repo.tiber(["transition", "reviewed-work", "done"]));
    task_stem(&repo, "done", "reviewed-work");
}

#[test]
fn review_can_fingerprint_a_staged_deletion() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    fs::write(repo.path().join("deleted.txt"), "reviewed\n").expect("write source");
    repo.git(["add", "deleted.txt"]);
    repo.git(["commit", "-m", "Add source to delete"]);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    fs::remove_file(repo.path().join("deleted.txt")).expect("delete source");
    repo.git(["add", "deleted.txt"]);

    record_review_with_scope(&repo, 1, "clean", &["deleted.txt"]);

    let shown = repo.tiber(["show", "reviewed-work"]);
    assert_success_ref(&shown);
    assert!(String::from_utf8(shown.stdout)
        .expect("show output utf8")
        .contains("review-1"));
}

#[test]
fn invalid_commit_range_is_rejected_before_review_is_recorded() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));

    let review = repo.tiber([
        "review",
        "reviewed-work",
        "--review-id",
        "review-1",
        "--reviewer",
        "reviewer-1",
        "--reviewer-type",
        "independent-final-review",
        "--scope",
        "README.md",
        "--commit-range",
        "HEAD",
        "--outcome",
        "clean",
        "--evidence",
        "review receipt",
        "--timestamp",
        "2026-08-27T12:00:00Z",
        "--source-fingerprint",
        "auto",
        "--verification-scope",
        "verification.txt",
        "--verification-fingerprint",
        "auto",
    ]);

    assert!(!review.status.success());
    let stderr = String::from_utf8(review.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("final_review_commit_range_invalid"),
        "{stderr}"
    );
}

#[test]
fn changed_uncommitted_verification_does_not_reinterpret_head_completion() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        record_review(&repo, iteration, "clean");
    }
    fs::write(repo.path().join("verification.txt"), "just ci: failed\n")
        .expect("change verification receipt");

    assert_success(repo.tiber(["transition", "reviewed-work", "done"]));
    task_stem(&repo, "done", "reviewed-work");
}

#[test]
fn deleted_untracked_verification_receipt_reports_not_committed() {
    let repo = TempRepo::initialized();
    enable_final_review_policy(&repo);
    fs::write(
        repo.path().join("volatile-verification.txt"),
        "isolated provider evaluation: pass\n",
    )
    .expect("write untracked verification receipt");
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Reviewed work"]));
    assert_success(repo.tiber(["transition", "reviewed-work", "in-progress"]));
    for iteration in 1..=3 {
        let review_id = format!("review-{iteration}");
        let reviewer = format!("reviewer-{iteration}");
        assert_success(repo.tiber([
            "review",
            "reviewed-work",
            "--review-id",
            &review_id,
            "--reviewer",
            &reviewer,
            "--reviewer-type",
            "independent-final-review",
            "--scope",
            "README.md",
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
            "volatile-verification.txt",
            "--verification-fingerprint",
            "auto",
        ]));
    }
    fs::remove_file(repo.path().join("volatile-verification.txt"))
        .expect("delete verification receipt");

    let completion = repo.tiber(["transition", "reviewed-work", "done"]);

    assert!(!completion.status.success());
    let stderr = String::from_utf8(completion.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("condition=verification_not_committed"),
        "{stderr}"
    );
}

#[test]
fn transition_refuses_reopening_into_a_full_backlog() {
    let repo = TempRepo::initialized();
    fs::write(
        repo.path().join(".tiber.toml"),
        "[backlog]\nmax_queued = 1\n",
    )
    .expect("write tiber config");
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Completed work"]));
    assert_success(repo.tiber(["transition", "completed-work", "done"]));
    assert_success(repo.tiber(["create", "Queued work"]));

    let reopen = repo.tiber(["transition", "completed-work", "backlog"]);

    assert!(!reopen.status.success(), "reopen should refuse overflow");
    let stderr = String::from_utf8(reopen.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("backlog_capacity_exceeded")
            && stderr.contains("queued=1")
            && stderr.contains("max_queued=1"),
        "stderr should explain the full backlog: {stderr}"
    );
    task_stem(&repo, "done", "completed-work");
}

#[test]
fn transition_refuses_moving_active_work_into_a_full_backlog() {
    let repo = TempRepo::initialized();
    fs::write(
        repo.path().join(".tiber.toml"),
        "[backlog]\nmax_queued = 1\n",
    )
    .expect("write tiber config");
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Active work"]));
    assert_success(repo.tiber(["transition", "active-work", "in-progress"]));
    assert_success(repo.tiber(["create", "Queued work"]));

    let move_back = repo.tiber(["transition", "active-work", "backlog"]);

    assert!(
        !move_back.status.success(),
        "move into backlog should refuse overflow"
    );
    let stderr = String::from_utf8(move_back.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("backlog_capacity_exceeded"),
        "stderr should explain the full backlog: {stderr}"
    );
    task_stem(&repo, "in-progress", "active-work");
}

#[test]
fn over_capacity_projects_can_move_work_out_of_the_backlog() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "First queued work"]));
    assert_success(repo.tiber(["create", "Second queued work"]));
    fs::write(
        repo.path().join(".tiber.toml"),
        "[backlog]\nmax_queued = 1\n",
    )
    .expect("write tiber config");

    assert_success(repo.tiber(["transition", "first-queued-work", "in-progress"]));

    task_stem(&repo, "in-progress", "first-queued-work");
    task_stem(&repo, "backlog", "second-queued-work");
}

#[test]
fn next_skips_tasks_blocked_by_open_dependencies() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Blocked task"]));
    assert_success(repo.tiber(["create", "Dependency task"]));
    let blocked = task_stem(&repo, "backlog", "blocked-task");
    let dependency = task_stem(&repo, "backlog", "dependency-task");
    assert_success(repo.tiber(["link", "dependency-task", "blocks", "blocked-task"]));
    assert_success(repo.tiber(["prioritize", "blocked-task", "--before", "dependency-task"]));

    let next = repo.tiber(["next"]);

    assert_success_ref(&next);
    assert_eq!(
        String::from_utf8(next.stdout).expect("next output should be utf8"),
        format!("{dependency}\tDependency task\n")
    );

    assert_success(repo.tiber(["transition", "dependency-task", "done"]));
    let next_after_dependency_done = repo.tiber(["next"]);

    assert_success_ref(&next_after_dependency_done);
    assert_eq!(
        String::from_utf8(next_after_dependency_done.stdout).expect("next output should be utf8"),
        format!("{blocked}\tBlocked task\n")
    );
}

#[test]
fn task_refs_can_use_unique_filename_identity() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Write docs"]));
    assert_success(repo.tiber(["create", "Review docs"]));
    let write_docs = task_stem(&repo, "backlog", "write-docs");
    let review_docs = task_stem(&repo, "backlog", "review-docs");

    let show = repo.tiber(["show", "write-docs"]);
    assert_success_ref(&show);
    assert!(String::from_utf8(show.stdout)
        .expect("show output should be utf8")
        .contains("title: Write docs"));

    assert_success(repo.tiber(["transition", "write-docs", "in-progress"]));
    assert_success(repo.tiber(["prioritize", "review-docs", "--before", "write-docs"]));
    assert_eq!(repo.order_file(), format!("{review_docs}\n{write_docs}\n"));
}
