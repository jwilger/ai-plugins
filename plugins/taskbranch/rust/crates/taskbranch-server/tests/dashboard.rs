use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::fs;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

#[tokio::test]
async fn dashboard_routes_render_board_and_task_pages() {
    let repo = TempRepo::initialized();
    repo.taskbranch(["init"]);
    repo.taskbranch(["create", "Render dashboard"]);

    let app = taskbranch_server::router_at(repo.path.clone());
    let board = app
        .clone()
        .oneshot(Request::get("/").body(Body::empty()).expect("request"))
        .await
        .expect("board response");
    assert_eq!(board.status(), StatusCode::OK);
    let board = body_text(board).await;
    assert!(board.contains("Render dashboard"));
    assert!(board.contains("todo/render-dashboard.md"));

    let task = app
        .oneshot(
            Request::get("/tasks/todo/render-dashboard.md")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("task response");
    assert_eq!(task.status(), StatusCode::OK);
    let task = body_text(task).await;
    assert!(task.contains("# Render dashboard"));
}

#[tokio::test]
async fn dashboard_board_page_exposes_browser_smoke_controls() {
    let repo = TempRepo::initialized();
    repo.taskbranch(["init"]);
    repo.taskbranch(["create", "Inspect dashboard"]);

    let app = taskbranch_server::router_at(repo.path.clone());
    let board = app
        .oneshot(Request::get("/").body(Body::empty()).expect("request"))
        .await
        .expect("board response");
    assert_eq!(board.status(), StatusCode::OK);
    let board = body_text(board).await;

    assert!(board.contains("data-dashboard-board"));
    assert!(board.contains("data-task-link"));
    assert!(board.contains("data-task-modal"));
    assert!(board.contains("data-modal-content"));
    assert!(board.contains("href=\"/docs\""));
    assert!(board.contains("data-external-link"));
    assert!(board.contains("data-link-intercept-status"));
}

#[tokio::test]
async fn dashboard_does_not_expose_sse_events_route() {
    let repo = TempRepo::initialized();
    repo.taskbranch(["init"]);
    repo.taskbranch(["create", "Stream dashboard"]);

    let response = taskbranch_server::router_at(repo.path.clone())
        .oneshot(
            Request::get("/events")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("events response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn dashboard_does_not_expose_http_mcp_route() {
    let repo = TempRepo::initialized();
    repo.taskbranch(["init"]);

    let response = taskbranch_server::router_at(repo.path.clone())
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
        repo.path.join("docs/guides/taskbranch.md"),
        "# Taskbranch guide\n\nDashboard docs stay read-only.\n",
    )
    .expect("write doc");

    let app = taskbranch_server::router_at(repo.path.clone());
    let docs = app
        .clone()
        .oneshot(Request::get("/docs").body(Body::empty()).expect("request"))
        .await
        .expect("docs response");
    assert_eq!(docs.status(), StatusCode::OK);
    let docs = body_text(docs).await;
    assert!(docs.contains("docs/guides/taskbranch.md"));
    assert!(docs.contains("/docs/guides/taskbranch.md"));

    let doc = app
        .clone()
        .oneshot(
            Request::get("/docs/guides/taskbranch.md")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("doc response");
    assert_eq!(doc.status(), StatusCode::OK);
    let doc = body_text(doc).await;
    assert!(doc.contains("# Taskbranch guide"));
    assert!(doc.contains("Dashboard docs stay read-only."));

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
            "taskbranch-server-test-{}-{unique}-{sequence}",
            std::process::id(),
        ));
        fs::create_dir(&path).expect("create temp repo");
        let repo = Self { path };
        repo.git(["init", "-b", "main"]);
        repo.git(["config", "user.email", "taskbranch@example.test"]);
        repo.git(["config", "user.name", "Taskbranch Test"]);
        repo.git(["config", "commit.gpgsign", "false"]);
        fs::write(repo.path.join("README.md"), "# test repo\n").expect("write readme");
        repo.git(["add", "README.md"]);
        repo.git(["commit", "-m", "Initial commit"]);
        repo
    }

    fn taskbranch<I, S>(&self, args: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let args = args
            .into_iter()
            .map(|arg| arg.as_ref().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let result = match args.as_slice() {
            [command] if command == "init" => taskbranch_git::init_repository_at(&self.path),
            [command, title] if command == "create" => {
                taskbranch_git::create_task_at(&self.path, title).map(|_| ())
            }
            _ => panic!("unsupported test taskbranch args: {args:?}"),
        };
        result.expect("taskbranch command should succeed");
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
