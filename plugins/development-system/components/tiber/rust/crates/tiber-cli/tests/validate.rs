pub mod support;

use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn validate_fix_preserves_projected_task_state() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Validated task"]));
    assert_success(repo.tiber(["acceptance", "add", "validated-task", "It works"]));

    let validate = repo.tiber(["validate", "--fix"]);
    assert_success_ref(&validate);
    let show = repo.tiber(["show", "validated-task"]);
    assert_success_ref(&show);
    assert!(String::from_utf8(show.stdout)
        .unwrap()
        .contains("- [ ] It works"));
}

#[test]
fn validate_fix_preserves_claims_on_in_progress_tasks() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Claimed task"]));
    assert_success(repo.tiber_with_env(
        ["transition", "claimed-task", "in-progress"],
        [
            ("TIBER_CLAIM_HOST", "validate-host"),
            ("TIBER_CLAIM_SESSION", "validate-session"),
        ],
    ));

    assert_success(repo.tiber(["validate", "--fix"]));
    let show = repo.tiber(["show", "claimed-task"]);
    assert_success_ref(&show);
    assert!(String::from_utf8(show.stdout)
        .unwrap()
        .contains("session: validate-session"));
}
