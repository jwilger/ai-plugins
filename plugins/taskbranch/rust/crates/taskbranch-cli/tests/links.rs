mod support;

use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn link_and_unlink_maintain_reciprocal_dependencies() {
    let repo = TempRepo::initialized();
    assert_success(repo.taskbranch(["init"]));
    assert_success(repo.taskbranch(["create", "Build API"]));
    assert_success(repo.taskbranch(["create", "Build UI"]));

    let link = repo.taskbranch(["link", "todo/build-api.md", "blocks", "todo/build-ui.md"]);

    assert_success(link);
    let api = repo.taskbranch(["show", "todo/build-api.md"]);
    let ui = repo.taskbranch(["show", "todo/build-ui.md"]);
    assert_success_ref(&api);
    assert_success_ref(&ui);
    assert_eq!(
        String::from_utf8(api.stdout).expect("api task should be utf8"),
        "# Build API\n\n## Blocks\n- todo/build-ui.md\n"
    );
    assert_eq!(
        String::from_utf8(ui.stdout).expect("ui task should be utf8"),
        "# Build UI\n\n## Blocked By\n- todo/build-api.md\n"
    );

    let unlink = repo.taskbranch(["unlink", "todo/build-api.md", "blocks", "todo/build-ui.md"]);

    assert_success(unlink);
    let api = repo.taskbranch(["show", "todo/build-api.md"]);
    let ui = repo.taskbranch(["show", "todo/build-ui.md"]);
    assert_success_ref(&api);
    assert_success_ref(&ui);
    assert_eq!(
        String::from_utf8(api.stdout).expect("api task should be utf8"),
        "# Build API\n"
    );
    assert_eq!(
        String::from_utf8(ui.stdout).expect("ui task should be utf8"),
        "# Build UI\n"
    );
}
