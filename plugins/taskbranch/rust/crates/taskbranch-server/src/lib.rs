use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use std::path::PathBuf;
use tokio::net::TcpListener;

pub fn router() -> Router {
    router_at(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

pub fn router_at(root: PathBuf) -> Router {
    Router::new()
        .route("/", get(board))
        .route("/tasks/{status}/{file}", get(task))
        .route("/docs", get(docs))
        .route("/docs/{*path}", get(doc))
        .with_state(AppState { root })
}

pub async fn serve(listener: TcpListener) -> Result<(), taskbranch_git::Error> {
    axum::serve(listener, router())
        .await
        .map_err(|error| taskbranch_git::Error::Parse(format!("dashboard_serve source={error}")))
}

#[derive(Clone)]
struct AppState {
    root: PathBuf,
}

async fn board(State(state): State<AppState>) -> Response {
    match taskbranch_git::list_tasks_at(state.root) {
        Ok(tasks) => {
            let task_items = tasks
                .into_iter()
                .map(|task| {
                    format!(
                        "<li><a data-task-link href=\"/tasks/{}\">{} {}</a></li>",
                        escape_html(&task.path),
                        escape_html(&task.path),
                        escape_html(&task.title)
                    )
                })
                .collect::<String>();
            Html(format!(
                "<!doctype html><html><head><title>taskbranch</title>{}</head><body><main data-dashboard-board><header><h1>taskbranch</h1><nav><a href=\"/docs\">Docs</a> <a data-external-link href=\"https://example.invalid/taskbranch\">External</a></nav></header><ul>{}</ul><p data-link-intercept-status aria-live=\"polite\"></p><dialog data-task-modal><article><button type=\"button\" data-modal-close>Close</button><pre data-modal-content></pre></article></dialog></main>{}</body></html>",
                dashboard_style(),
                task_items,
                dashboard_script()
            ))
            .into_response()
        }
        Err(error) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("{error}"),
        )
            .into_response(),
    }
}

async fn task(
    State(state): State<AppState>,
    Path((status, file)): Path<(String, String)>,
) -> Response {
    let task_ref = format!("{status}/{file}");
    match taskbranch_git::show_task_at(state.root, &task_ref) {
        Ok(task) => Html(format!(
            "<!doctype html><html><head><title>{}</title></head><body><main><pre>{}</pre></main></body></html>",
            escape_html(&task_ref),
            escape_html(&task)
        ))
        .into_response(),
        Err(error) => (axum::http::StatusCode::NOT_FOUND, format!("{error}")).into_response(),
    }
}

async fn docs(State(state): State<AppState>) -> Response {
    match taskbranch_git::list_docs_at(state.root) {
        Ok(docs) => Html(format!(
            "<!doctype html><html><head><title>taskbranch docs</title></head><body><main><h1>taskbranch docs</h1><ul>{}</ul></main></body></html>",
            docs
                .into_iter()
                .map(|doc| format!(
                    "<li><a href=\"/{}\">{}</a></li>",
                    escape_html(&doc),
                    escape_html(&doc)
                ))
                .collect::<String>()
        ))
        .into_response(),
        Err(error) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("{error}"),
        )
            .into_response(),
    }
}

async fn doc(State(state): State<AppState>, Path(path): Path<String>) -> Response {
    let doc_ref = format!("docs/{path}");
    match taskbranch_git::read_doc_at(state.root, &doc_ref) {
        Ok(doc) => Html(format!(
            "<!doctype html><html><head><title>{}</title></head><body><main><pre>{}</pre></main></body></html>",
            escape_html(&doc_ref),
            escape_html(&doc)
        ))
        .into_response(),
        Err(error) => (axum::http::StatusCode::NOT_FOUND, format!("{error}")).into_response(),
    }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn dashboard_style() -> &'static str {
    "<style>body{font-family:system-ui,sans-serif;margin:2rem;line-height:1.5}main{max-width:56rem}nav{display:flex;gap:1rem}ul{padding-left:1.25rem}dialog{width:min(48rem,90vw);border:1px solid #555;border-radius:8px}pre{white-space:pre-wrap}</style>"
}

fn dashboard_script() -> &'static str {
    r#"<script>
const modal = document.querySelector('[data-task-modal]');
const modalContent = document.querySelector('[data-modal-content]');
const closeButton = document.querySelector('[data-modal-close]');
const interceptStatus = document.querySelector('[data-link-intercept-status]');

document.addEventListener('click', async (event) => {
  const taskLink = event.target.closest('[data-task-link]');
  if (taskLink) {
    event.preventDefault();
    const response = await fetch(taskLink.href);
    modalContent.textContent = await response.text();
    modal.showModal();
    return;
  }

  const externalLink = event.target.closest('[data-external-link]');
  if (externalLink) {
    event.preventDefault();
    interceptStatus.textContent = `intercepted ${externalLink.href}`;
  }
});

closeButton.addEventListener('click', () => modal.close());
</script>"#
}
