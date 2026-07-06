mod support;

use support::{assert_success, assert_success_ref, task_stem, TempRepo};

#[test]
fn create_stores_course_shaped_task_in_backlog_and_list_prints_ordered_summary() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));

    let create = repo.tiber(["create", "Write tiber docs"]);

    assert_success(create);
    let stem = task_stem(&repo, "backlog", "write-tiber-docs");
    let file_name = format!("{stem}.md");
    assert!(file_name.ends_with("-write-tiber-docs.md"));
    let (date, rest) = stem
        .split_once('-')
        .expect("task stem should contain date and random code");
    let (code, nickname) = rest
        .split_once('-')
        .expect("task stem should contain random code and nickname");
    assert_eq!(date.len(), 8, "task id date should be YYYYMMDD");
    assert!(date.chars().all(|character| character.is_ascii_digit()));
    assert_eq!(code.len(), 4, "task id random code should be four chars");
    assert!(code
        .chars()
        .all(|character| "abcdefghijkmnpqrstuvwxyz23456789".contains(character)));
    assert_eq!(nickname, "write-tiber-docs");

    let task = repo.task_file("backlog", &stem);
    assert!(task
        .starts_with("---\ntitle: Write tiber docs\nblocked_by: []\nblocks: []\ntags: []\n---\n"));
    assert!(task.contains("## Summary\n\n"));
    assert!(task.contains("## Context / Why\n\n"));
    assert!(task.contains("## Acceptance criteria\n\n"));
    assert!(task.contains("## Subtasks\n\n"));
    assert!(task.contains("## Notes / Log\n"));

    assert_eq!(repo.order_file(), format!("{stem}\n"));

    let list = repo.tiber(["list"]);

    assert_success_ref(&list);
    assert_eq!(
        String::from_utf8(list.stdout).expect("list output should be utf8"),
        format!("{stem}\tWrite tiber docs\n")
    );
}

#[test]
fn show_resolves_by_id_nickname_or_full_stem_without_storage_paths() {
    let repo = TempRepo::initialized();
    assert_success(repo.tiber(["init"]));
    assert_success(repo.tiber(["create", "Write tiber docs"]));
    let stem = task_stem(&repo, "backlog", "write-tiber-docs");
    let id = stem
        .split_once("-write-tiber-docs")
        .map(|(id, _)| id)
        .expect("stem includes nickname")
        .to_string();

    for task_ref in [id.as_str(), "write-tiber-docs", stem.as_str()] {
        let show = repo.tiber(["show", task_ref]);

        assert_success_ref(&show);
        assert!(
            String::from_utf8(show.stdout)
                .expect("show output should be utf8")
                .contains("title: Write tiber docs"),
            "show should print task for ref {task_ref}"
        );
    }
}
