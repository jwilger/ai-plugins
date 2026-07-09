use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::fs;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

#[tokio::test]
async fn dashboard_routes_render_board_and_task_pages() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    repo.tiber(["create", "Render dashboard"]);
    let stem = repo.task_stem("backlog", "render-dashboard");

    let app = tiber_server::router_at(repo.path.clone());
    let board = app
        .clone()
        .oneshot(Request::get("/").body(Body::empty()).expect("request"))
        .await
        .expect("board response");
    assert_eq!(board.status(), StatusCode::OK);
    let board = body_text(board).await;
    assert!(board.contains("Render dashboard"));
    assert!(board.contains(&stem));
    let ticket_id = &stem[..13];
    assert!(board.contains(&format!("data-copy-task-id=\"{ticket_id}\"")));
    assert!(board.contains(&format!("Copy ticket ID {ticket_id}")));

    let task = app
        .clone()
        .oneshot(
            Request::get(format!("/tasks/{stem}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("task response");
    assert_eq!(task.status(), StatusCode::OK);
    let task = body_text(task).await;
    assert!(task.contains("title: Render dashboard"));

    let traversal = app
        .oneshot(
            Request::get("/tasks/../render-dashboard.md")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("traversal response");
    assert_eq!(traversal.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn dashboard_board_page_exposes_browser_smoke_controls() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    repo.tiber(["create", "Inspect dashboard"]);

    let app = tiber_server::router_at(repo.path.clone());
    let board = app
        .oneshot(Request::get("/").body(Body::empty()).expect("request"))
        .await
        .expect("board response");
    assert_eq!(board.status(), StatusCode::OK);
    let board = body_text(board).await;

    assert!(board.contains("data-dashboard-board"));
    assert!(board.contains("data-task-link"));
    assert!(board.contains("data-copy-task-id"));
    assert!(board.contains("data-copy-status"));
    assert!(board.contains("data-task-modal"));
    assert!(board.contains("data-modal-content"));
    assert!(board.contains("href=\"/docs\""));
    assert!(board.contains("data-external-link"));
    assert!(board.contains("data-link-intercept-status"));
    assert!(board.contains("new EventSource(\"/events\")"));
}

#[tokio::test]
async fn dashboard_copy_id_uses_full_legacy_stem() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    repo.insert_tasks_tree_file(
        "backlog/build-dashboard-task.md",
        &repo.task_document("Build dashboard task", &[], &[], &[], "Legacy summary.\n"),
    );

    let board = tiber_server::router_at(repo.path.clone())
        .oneshot(Request::get("/").body(Body::empty()).expect("request"))
        .await
        .expect("board response");

    assert_eq!(board.status(), StatusCode::OK);
    let board = body_text(board).await;
    assert!(board.contains("data-copy-task-id=\"build-dashboard-task\""));
    assert!(board.contains("Copy ticket ID build-dashboard-task"));
    assert!(!board.contains("data-copy-task-id=\"build-dashboa\""));
}

#[tokio::test]
async fn dashboard_board_renders_while_tiber_writer_lock_exists() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    repo.tiber(["create", "Visible during write"]);
    repo.write_fresh_tiber_lock();

    let response = tiber_server::router_at(repo.path.clone())
        .oneshot(Request::get("/").body(Body::empty()).expect("request"))
        .await
        .expect("board response");

    assert_eq!(response.status(), StatusCode::OK);
    let board = body_text(response).await;
    assert!(board.contains("Visible during write"));
    assert!(!board.contains("tiber_lock_busy"));
}

#[tokio::test]
async fn dashboard_board_renders_course_columns_badges_dependencies_and_modal_content() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    repo.tiber(["create", "Build API"]);
    repo.tiber(["create", "Build UI"]);
    repo.tiber(["create", "Document release"]);
    let api = repo.task_stem("backlog", "build-api");
    let ui = repo.task_stem("backlog", "build-ui");
    let docs = repo.task_stem("backlog", "document-release");
    repo.move_task("backlog", "in-progress", &ui);
    repo.move_task("backlog", "done", &docs);
    repo.insert_tasks_tree_file("order.md", &format!("{api}\n{ui}\n"));
    repo.insert_tasks_tree_file(
        &format!("backlog/{api}.md"),
        &repo.task_document(
            "Build API",
            &[],
            &[&ui],
            &["backend"],
            "API summary with `code` and [Draft](docs/missing.md).\n",
        ),
    );
    repo.insert_tasks_tree_file(
        &format!("in-progress/{ui}.md"),
        &repo.task_document("Build UI", &[&api], &[], &["frontend"], "UI summary.\n"),
    );
    repo.insert_tasks_tree_file(
        &format!("done/{docs}.md"),
        &repo.task_document("Document release", &[], &[], &["docs"], "Docs summary.\n"),
    );

    let board = tiber_server::router_at(repo.path.clone())
        .oneshot(Request::get("/").body(Body::empty()).expect("request"))
        .await
        .expect("board response");

    assert_eq!(board.status(), StatusCode::OK);
    let board = body_text(board).await;
    assert!(board.contains("data-column=\"backlog\""));
    assert!(board.contains("data-column=\"in-progress\""));
    assert!(board.contains("data-column=\"done\""));
    assert!(board.contains("data-rank-badge=\"#1\""));
    assert!(board.contains("data-recency-badge"));
    assert!(board.contains("data-dependent=\""));
    assert!(board.contains("data-dependency=\""));
    assert!(board.contains("<code>code</code>"));
    assert!(board.contains("Draft <span class=\"draft-marker\">(draft)</span>"));
    assert!(board.contains("data-modal-content"));
    assert!(board.contains("Acceptance criteria"));
    assert!(board.contains("Notes / Log"));
}

#[tokio::test]
async fn dashboard_in_progress_cards_show_pr_mr_status_badges() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    repo.tiber(["create", "Review badge"]);
    let stem = repo.task_stem("backlog", "review-badge");
    repo.move_task("backlog", "in-progress", &stem);
    let mut task = repo.task_file("in-progress", &stem);
    task = task
        .replace(
            "pr_mr_url: \n",
            "pr_mr_url: https://github.com/example/repo/pull/42\n",
        )
        .replace("pr_mr_status: \n", "pr_mr_status: checks-failing\n");
    repo.insert_tasks_tree_file(&format!("in-progress/{stem}.md"), &task);

    let response = tiber_server::router_at(repo.path.clone())
        .oneshot(Request::get("/").body(Body::empty()).expect("request"))
        .await
        .expect("board response");

    assert_eq!(response.status(), StatusCode::OK);
    let board = body_text(response).await;
    assert!(board.contains("data-pr-mr-status=\"checks-failing\""));
    assert!(board.contains("PR/MR checks failing"));
    assert!(board.contains("pr-status-checks-failing"));
}

#[tokio::test]
async fn dashboard_marks_agent_unresolvable_blocked_tasks() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    repo.tiber(["create", "Blocked externally"]);
    repo.tiber(["create", "Blocked with a long reason"]);
    repo.tiber(["create", "Previously blocked done"]);
    let stem = repo.task_stem("backlog", "blocked-externally");
    let long_stem = repo.task_stem("backlog", "blocked-with-a-long-reason");
    let done_stem = repo.task_stem("backlog", "previously-blocked-done");
    let task = repo.task_file("backlog", &stem).replace(
        "agent_blocked_reason: \n",
        "agent_blocked_reason: Waiting on account access that the agent cannot grant.\n",
    );
    repo.insert_tasks_tree_file(&format!("backlog/{stem}.md"), &task);
    let long_reason = "Waiting on a production account owner to grant access after confirming the requester has completed the required approval workflow and audit checklist.";
    let long_task = repo.task_file("backlog", &long_stem).replace(
        "agent_blocked_reason: \n",
        &format!("agent_blocked_reason: {long_reason}\n"),
    );
    repo.insert_tasks_tree_file(&format!("backlog/{long_stem}.md"), &long_task);
    repo.move_task("backlog", "done", &done_stem);
    let done_task = repo.task_file("done", &done_stem).replace(
        "agent_blocked_reason: \n",
        "agent_blocked_reason: Stale external blocker.\n",
    );
    repo.insert_tasks_tree_file(&format!("done/{done_stem}.md"), &done_task);

    let response = tiber_server::router_at(repo.path.clone())
        .oneshot(Request::get("/").body(Body::empty()).expect("request"))
        .await
        .expect("board response");

    assert_eq!(response.status(), StatusCode::OK);
    let board = body_text(response).await;
    assert!(board.contains("is-agent-blocked"));
    assert!(board.contains("data-agent-blocked"));
    assert!(board.contains(
        "aria-label=\"Agent-unresolvable blocked: Waiting on account access that the agent cannot grant.\""
    ));
    assert!(
        board.contains(">Blocked: Waiting on account access that the agent cannot grant.</span>")
    );
    assert!(board.contains(
        ">Blocked: Waiting on a production account owner to grant access after confirming the requester has comp...</span>"
    ));
    assert!(board.contains(&format!("title=\"{long_reason}\"")));
    assert!(board.contains("data-agent-blocked-reason"));
    assert!(board.contains("Blocked reason"));
    assert!(board.contains("Waiting on account access that the agent cannot grant."));
    assert!(board.contains(long_reason));
    assert!(!board.contains("Stale external blocker."));
    assert!(!board.contains("PR/MR blocked"));
}

#[tokio::test]
async fn dashboard_exposes_read_only_sse_events_route() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    repo.tiber(["create", "Stream dashboard"]);

    let response = tiber_server::router_at(repo.path.clone())
        .oneshot(
            Request::get("/events")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("events response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .expect("content type"),
        "text/event-stream"
    );
    let mut body = response.into_body();
    let body = next_body_frame(&mut body).await;
    assert!(body.starts_with("data: "));
    assert!(body.contains("Stream dashboard"));
}

#[tokio::test]
async fn dashboard_events_error_frames_are_valid_json() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    repo.insert_tasks_tree_file("order.md", "backlog/not-a-stem.md\n");

    let response = tiber_server::router_at(repo.path.clone())
        .oneshot(
            Request::get("/events")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("events response");
    assert_eq!(response.status(), StatusCode::OK);
    let mut body = response.into_body();
    let frame = next_body_frame(&mut body).await;
    let event = assert_json_event(&frame);
    assert!(
        event["error"].is_string(),
        "error event should carry a JSON string error: {event}"
    );
}

#[tokio::test]
async fn dashboard_events_stream_board_changes_without_reconnecting() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    repo.tiber(["create", "Initial stream task"]);

    let response = tiber_server::router_at(repo.path.clone())
        .oneshot(
            Request::get("/events")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("events response");
    assert_eq!(response.status(), StatusCode::OK);
    let mut body = response.into_body();

    let initial = next_body_frame(&mut body).await;
    assert!(initial.contains("Initial stream task"));
    assert_json_event(&initial);

    repo.tiber(["create", "Second stream task"]);

    let changed = next_body_frame(&mut body).await;
    assert!(changed.contains("Second stream task"));
    assert_json_event(&changed);
}

#[tokio::test]
async fn dashboard_events_stream_agent_blocked_reason_changes() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    repo.tiber(["create", "Blocked stream task"]);
    repo.tiber(["create", "Done stale event task"]);
    let stem = repo.task_stem("backlog", "blocked-stream-task");
    let done_stem = repo.task_stem("backlog", "done-stale-event-task");
    let done_task = repo.task_file("backlog", &done_stem).replace(
        "agent_blocked_reason: \n",
        "agent_blocked_reason: Hidden stale done reason.\n",
    );
    repo.insert_tasks_tree_file(&format!("backlog/{done_stem}.md"), &done_task);
    repo.move_task("backlog", "done", &done_stem);

    let response = tiber_server::router_at(repo.path.clone())
        .oneshot(
            Request::get("/events")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("events response");
    assert_eq!(response.status(), StatusCode::OK);
    let mut body = response.into_body();

    let initial = next_body_frame(&mut body).await;
    assert!(initial.contains(&stem));
    assert!(!initial.contains("Waiting on external account access."));
    assert!(!initial.contains("Hidden stale done reason."));
    assert_json_event(&initial);

    let task = repo.task_file("backlog", &stem).replace(
        "agent_blocked_reason: \n",
        "agent_blocked_reason: Waiting on external account access.\n",
    );
    repo.insert_tasks_tree_file(&format!("backlog/{stem}.md"), &task);

    let changed = next_body_frame(&mut body).await;
    assert!(changed.contains(&stem));
    assert!(changed.contains("Waiting on external account access."));
    assert!(!changed.contains("Hidden stale done reason."));
    let changed_json = assert_json_event(&changed);
    assert_eq!(
        changed_json["tasks"]
            .as_array()
            .expect("tasks array")
            .iter()
            .find(|task| task["stem"] == stem)
            .expect("blocked task")["agent_blocked_reason"],
        "Waiting on external account access."
    );
}

#[tokio::test]
async fn dashboard_does_not_expose_http_mcp_route() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);

    let response = tiber_server::router_at(repo.path.clone())
        .oneshot(Request::get("/mcp").body(Body::empty()).expect("request"))
        .await
        .expect("mcp response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn dashboard_routes_render_repo_docs_with_relative_paths() {
    let repo = TempRepo::initialized();
    fs::create_dir_all(repo.path.join("docs/guides")).expect("create docs directory");
    fs::write(
        repo.path.join("docs/guides/tiber.md"),
        "# Tiber guide\n\nDashboard docs stay read-only.\n\n[Draft](missing.md)\n",
    )
    .expect("write doc");

    let app = tiber_server::router_at(repo.path.clone());
    let docs = app
        .clone()
        .oneshot(Request::get("/docs").body(Body::empty()).expect("request"))
        .await
        .expect("docs response");
    assert_eq!(docs.status(), StatusCode::OK);
    let docs = body_text(docs).await;
    assert!(docs.contains("docs/guides/tiber.md"));
    assert!(docs.contains("/docs/guides/tiber.md"));

    let doc = app
        .clone()
        .oneshot(
            Request::get("/docs/guides/tiber.md")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("doc response");
    assert_eq!(doc.status(), StatusCode::OK);
    let doc = body_text(doc).await;
    assert!(doc.contains("<h1>Tiber guide</h1>"));
    assert!(doc.contains("Dashboard docs stay read-only."));
    assert!(doc.contains("Draft (draft)"));

    let traversal = app
        .oneshot(
            Request::get("/docs/../README.md")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("traversal response");
    assert_eq!(traversal.status(), StatusCode::NOT_FOUND);
}

async fn next_body_frame(body: &mut Body) -> String {
    let frame = tokio::time::timeout(Duration::from_secs(4), body.frame())
        .await
        .expect("timed out waiting for dashboard event")
        .expect("dashboard event stream ended")
        .expect("dashboard event frame should be readable");
    let bytes = frame
        .into_data()
        .expect("dashboard event frame should contain data");
    String::from_utf8(bytes.to_vec()).expect("dashboard event should be utf8")
}

fn assert_json_event(frame: &str) -> serde_json::Value {
    let data = frame
        .strip_prefix("data: ")
        .expect("dashboard event frame should start with data prefix")
        .trim();
    serde_json::from_str(data).expect("dashboard event data should be valid json")
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    String::from_utf8(bytes.to_vec()).expect("body should be utf8")
}

struct TempRepo {
    path: std::path::PathBuf,
}

impl TempRepo {
    fn initialized() -> Self {
        static TEMP_REPO_SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        let sequence = TEMP_REPO_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "tiber-server-test-{}-{unique}-{sequence}",
            std::process::id(),
        ));
        fs::create_dir(&path).expect("create temp repo");
        let repo = Self { path };
        repo.git(["init", "-b", "main"]);
        repo.git(["config", "user.email", "tiber@example.test"]);
        repo.git(["config", "user.name", "Tiber Test"]);
        repo.git(["config", "commit.gpgsign", "false"]);
        fs::write(repo.path.join("README.md"), "# test repo\n").expect("write readme");
        repo.git(["add", "README.md"]);
        repo.git(["commit", "-m", "Initial commit"]);
        repo
    }

    fn tiber<I, S>(&self, args: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let args = args
            .into_iter()
            .map(|arg| arg.as_ref().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let result = match args.as_slice() {
            [command] if command == "init" => tiber_git::init_repository_at(&self.path),
            [command, title] if command == "create" => {
                tiber_git::create_task_at(&self.path, title).map(|_| ())
            }
            _ => panic!("unsupported test tiber args: {args:?}"),
        };
        result.expect("tiber command should succeed");
    }

    fn git<I, S>(&self, args: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        assert_success(
            Command::new("git")
                .args(args)
                .current_dir(&self.path)
                .output()
                .expect("run git"),
        );
    }

    fn move_task(&self, from_status: &str, to_status: &str, stem: &str) {
        let contents = self.task_file(from_status, stem);
        self.insert_tasks_tree_file(&format!("{to_status}/{stem}.md"), &contents);
        self.remove_tasks_tree_file(&format!("{from_status}/{stem}.md"));
    }

    fn task_file(&self, status: &str, stem: &str) -> String {
        let output = Command::new("git")
            .args(["show", &format!("tasks:{status}/{stem}.md")])
            .current_dir(&self.path)
            .output()
            .expect("read task");
        assert_success(output.clone());
        String::from_utf8(output.stdout).expect("task should be utf8")
    }

    fn task_document(
        &self,
        title: &str,
        blocked_by: &[&str],
        blocks: &[&str],
        tags: &[&str],
        summary: &str,
    ) -> String {
        format!(
            "---\ntitle: {title}\nblocked_by: [{}]\nblocks: [{}]\ntags: [{}]\n---\n\n## Summary\n\n{summary}\n## Context / Why\n\nContext.\n\n## Acceptance criteria\n\n- [ ] Done condition\n\n## Subtasks\n\n- [ ] (s1) First step\n\n## Notes / Log\n\n- 2026-07-06: Note.\n",
            blocked_by.join(", "),
            blocks.join(", "),
            tags.join(", ")
        )
    }

    fn task_stem(&self, status: &str, nickname: &str) -> String {
        let output = Command::new("git")
            .args(["ls-tree", "-r", "--name-only", "tasks", status])
            .current_dir(&self.path)
            .output()
            .expect("list tasks tree");
        assert_success(output.clone());
        let mut matches = String::from_utf8(output.stdout)
            .expect("tree should be utf8")
            .lines()
            .filter_map(|path| {
                path.strip_prefix(&format!("{status}/"))
                    .and_then(|name| name.strip_suffix(".md"))
                    .filter(|stem| stem.ends_with(&format!("-{nickname}")))
                    .map(str::to_string)
            })
            .collect::<Vec<_>>();
        matches.sort();
        assert_eq!(matches.len(), 1, "expected one task matching {nickname}");
        matches.remove(0)
    }

    fn write_fresh_tiber_lock(&self) {
        let lock_dir = self.path.join(".git").join("tiber");
        fs::create_dir_all(&lock_dir).expect("create tiber lock directory");
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_secs();
        fs::write(
            lock_dir.join("tiber.lock"),
            format!("pid={}\ntimestamp={timestamp}\n", std::process::id()),
        )
        .expect("write tiber lock");
    }

    fn insert_tasks_tree_file(&self, path: &str, contents: &str) {
        let blob = Command::new("git")
            .args(["hash-object", "-w", "--stdin"])
            .current_dir(&self.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .as_mut()
                    .expect("hash-object stdin")
                    .write_all(contents.as_bytes())?;
                child.wait_with_output()
            })
            .expect("write task blob");
        assert_success(blob.clone());
        let blob = String::from_utf8(blob.stdout)
            .expect("blob should be utf8")
            .trim()
            .to_string();
        self.with_tasks_index(|index| {
            self.git_env(
                [
                    "update-index",
                    "--add",
                    "--cacheinfo",
                    "100644",
                    &blob,
                    path,
                ],
                index,
            );
        });
    }

    fn remove_tasks_tree_file(&self, path: &str) {
        self.with_tasks_index(|index| {
            self.git_env(["update-index", "--force-remove", path], index);
        });
    }

    fn with_tasks_index(&self, update: impl FnOnce(&std::path::Path)) {
        let index = self.path.join(".git").join("tiber-server-test-index");
        self.git_env(["read-tree", "tasks"], &index);
        update(&index);
        let tree = self.git_env_output(["write-tree"], &index);
        let tree = String::from_utf8(tree.stdout)
            .expect("tree should be utf8")
            .trim()
            .to_string();
        let commit = Command::new("git")
            .args([
                "commit-tree",
                &tree,
                "-p",
                "tasks",
                "-m",
                "Update test tasks",
            ])
            .current_dir(&self.path)
            .output()
            .expect("commit test tree");
        assert_success(commit.clone());
        let commit = String::from_utf8(commit.stdout)
            .expect("commit should be utf8")
            .trim()
            .to_string();
        self.git(["update-ref", "refs/heads/tasks", &commit]);
        let _ = fs::remove_file(index);
    }

    fn git_env<I, S>(&self, args: I, index: &std::path::Path)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        assert_success(self.git_env_output(args, index));
    }

    fn git_env_output<I, S>(&self, args: I, index: &std::path::Path) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        Command::new("git")
            .args(args)
            .env("GIT_INDEX_FILE", index)
            .current_dir(&self.path)
            .output()
            .expect("run git with index")
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn assert_success(output: Output) {
    assert!(
        output.status.success(),
        "command failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
