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

    let missing = app
        .clone()
        .oneshot(
            Request::get("/tasks/missing")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("missing task response");
    let missing_status = missing.status();
    let missing = body_text(missing).await;
    assert_eq!(missing_status, StatusCode::NOT_FOUND, "{missing}");

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
async fn dashboard_board_fetches_remote_tasks_in_fresh_clone() {
    let origin = TempRepo::bare();
    let seed = TempRepo::initialized();
    seed.git([
        "remote",
        "add",
        "origin",
        origin.path.to_str().expect("origin path utf8"),
    ]);
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);
    seed.tiber(["init"]);
    seed.tiber(["create", "Remote dashboard task"]);
    seed.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);

    let clone = TempRepo::clone_from(&origin);
    let app = tiber_server::router_at(clone.path.clone());
    let board = app
        .oneshot(Request::get("/").body(Body::empty()).expect("request"))
        .await
        .expect("board response");

    assert_eq!(board.status(), StatusCode::OK);
    let board = body_text(board).await;
    assert!(board.contains("Remote dashboard task"));
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
async fn dashboard_board_renders_local_snapshot_while_origin_repo_is_locked() {
    let origin = TempRepo::bare();
    let repo = TempRepo::initialized();
    repo.git([
        "remote",
        "add",
        "origin",
        origin.path.to_str().expect("origin path utf8"),
    ]);
    repo.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);
    repo.tiber(["init"]);
    repo.tiber(["create", "Visible during origin lock"]);
    repo.write_fresh_tiber_lock();

    let response = tiber_server::router_at(repo.path.clone())
        .oneshot(Request::get("/").body(Body::empty()).expect("request"))
        .await
        .expect("board response");

    assert_eq!(response.status(), StatusCode::OK);
    let board = body_text(response).await;
    assert!(board.contains("Visible during origin lock"));
    assert!(!board.contains("tiber_lock_busy"));
}

#[tokio::test]
async fn dashboard_redacts_task_sync_errors_from_board_and_events() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    repo.git([
        "remote",
        "add",
        "origin",
        "https://user:secret-token@example.invalid/private/repo.git",
    ]);

    let app = tiber_server::router_at(repo.path.clone());
    let board = app
        .clone()
        .oneshot(Request::get("/").body(Body::empty()).expect("request"))
        .await
        .expect("board response");
    assert_eq!(board.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let board = body_text(board).await;
    assert!(board.contains("dashboard_task_load_failed"));
    assert!(board.contains("args_redacted=true"));
    assert!(board.contains("stderr_redacted=true"));
    assert!(!board.contains("secret-token"));
    assert!(!board.contains("private/repo.git"));

    let response = app
        .clone()
        .oneshot(
            Request::get("/events")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("events response");
    assert_eq!(response.status(), StatusCode::OK);
    let mut body = response.into_body();
    let event = next_body_frame(&mut body).await;
    assert!(event.contains("dashboard_task_load_failed"));
    assert!(event.contains("args_redacted=true"));
    assert!(event.contains("stderr_redacted=true"));
    assert!(!event.contains("secret-token"));
    assert!(!event.contains("private/repo.git"));

    let task = app
        .oneshot(
            Request::get("/tasks/missing")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("task response");
    assert_eq!(task.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let task = body_text(task).await;
    assert!(task.contains("dashboard_task_load_failed"));
    assert!(task.contains("args_redacted=true"));
    assert!(task.contains("stderr_redacted=true"));
    assert!(!task.contains("secret-token"));
    assert!(!task.contains("private/repo.git"));
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

    repo.tiber(["create", "Second stream task"]);

    let changed = next_body_frame(&mut body).await;
    assert!(changed.contains("Second stream task"));
}

#[tokio::test]
async fn dashboard_events_retry_remote_sync_after_failure() {
    let origin = TempRepo::bare();
    let seed = TempRepo::initialized();
    seed.git([
        "remote",
        "add",
        "origin",
        origin.path.to_str().expect("origin path utf8"),
    ]);
    seed.git(["push", "origin", "main"]);
    origin.git(["symbolic-ref", "HEAD", "refs/heads/main"]);
    seed.tiber(["init"]);
    seed.tiber(["create", "Remote stream task"]);
    seed.git(["push", "origin", "refs/heads/tasks:refs/heads/tasks"]);

    let clone = TempRepo::clone_from(&origin);
    let missing_origin = clone.path.join("missing-origin.git");
    clone.git([
        "remote",
        "set-url",
        "origin",
        missing_origin
            .to_str()
            .expect("missing origin path should be utf8"),
    ]);

    let response = tiber_server::router_at(clone.path.clone())
        .oneshot(
            Request::get("/events")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("events response");
    assert_eq!(response.status(), StatusCode::OK);
    let mut body = response.into_body();

    let failed = next_body_frame(&mut body).await;
    assert!(failed.contains("dashboard_task_load_failed"));
    assert!(failed.contains("args_redacted=true"));

    clone.git([
        "remote",
        "set-url",
        "origin",
        origin.path.to_str().expect("origin path utf8"),
    ]);
    let recovered = next_body_frame(&mut body).await;
    assert!(recovered.contains("Remote stream task"));
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
    fn new() -> Self {
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
        Self { path }
    }

    fn initialized() -> Self {
        let repo = Self::new();
        repo.git(["init", "-b", "main"]);
        repo.git(["config", "user.email", "tiber@example.test"]);
        repo.git(["config", "user.name", "Tiber Test"]);
        repo.git(["config", "commit.gpgsign", "false"]);
        fs::write(repo.path.join("README.md"), "# test repo\n").expect("write readme");
        repo.git(["add", "README.md"]);
        repo.git(["commit", "-m", "Initial commit"]);
        repo
    }

    fn bare() -> Self {
        let repo = Self::new();
        repo.git(["init", "--bare"]);
        repo
    }

    fn clone_from(origin: &Self) -> Self {
        let clone = Self::new();
        assert_success(
            Command::new("git")
                .args(["clone", origin.path.to_str().expect("origin path utf8")])
                .arg(&clone.path)
                .output()
                .expect("clone repository"),
        );
        clone.git(["config", "user.email", "tiber@example.test"]);
        clone.git(["config", "user.name", "Tiber Test"]);
        clone.git(["config", "commit.gpgsign", "false"]);
        clone
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
