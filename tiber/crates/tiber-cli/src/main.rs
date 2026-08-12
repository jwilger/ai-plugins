#![forbid(unsafe_code)]
#![expect(
    clippy::absolute_paths,
    clippy::exit,
    clippy::implicit_return,
    clippy::return_and_then,
    clippy::shadow_reuse,
    clippy::single_call_fn,
    clippy::std_instead_of_core,
    reason = "the thin command adapter uses process exits, OS arguments, and one-shot dispatch helpers at the imperative boundary"
)]

use std::{
    env, fs,
    io::Read as _,
    path::{Path, PathBuf},
    process,
    time::Duration,
};

use tiber_app_server::{AccountStatus, AppServerClient, AppServerConfig, inspect_protocol_schema};

/// Reviewed isolated app-server configuration template.
const ISOLATED_CONFIG: &str = include_str!("../../../config/app-server.toml");
#[expect(
    clippy::print_stderr,
    reason = "a command-line adapter intentionally writes its result and diagnostics"
)]
fn main() {
    let mut arguments = env::args_os();
    let _executable = arguments.next();
    let Some(command) = arguments.next() else {
        usage();
        process::exit(2);
    };
    match command.to_string_lossy().as_ref() {
        "app-server-probe" => run_schema_probe(arguments),
        "auth" => run_auth(arguments),
        "converse" => run_conversation(arguments),
        _ => {
            eprintln!("unknown command: {}", command.to_string_lossy());
            usage();
            process::exit(2);
        }
    }
}

#[expect(
    clippy::print_stderr,
    clippy::print_stdout,
    reason = "a command-line adapter intentionally writes its result and diagnostics"
)]
/// Runs the pinned protocol-surface checker.
fn run_schema_probe(mut arguments: impl Iterator<Item = std::ffi::OsString>) {
    let Some(schema_path) = arguments.next() else {
        usage();
        process::exit(2);
    };
    if arguments.next().is_some() {
        usage();
        process::exit(2);
    }
    let schema = fs::read_to_string(&schema_path).unwrap_or_else(|error| {
        eprintln!("app_server_schema_read_failed: {error}");
        process::exit(1);
    });
    let report = inspect_protocol_schema(&schema).unwrap_or_else(|error| {
        eprintln!("{}: {error}", error.code());
        process::exit(1);
    });
    println!(
        "app-server protocol exposes the reviewed Tiber control surface; runtime policy must cover: {}",
        report.controlled_operations().join(", ")
    );
}

#[expect(
    clippy::print_stderr,
    clippy::print_stdout,
    reason = "authentication commands intentionally print owner-facing status and handoff information"
)]
/// Runs one app-server-mediated authentication operation.
fn run_auth(mut arguments: impl Iterator<Item = std::ffi::OsString>) {
    let Some(operation) = arguments.next() else {
        usage();
        process::exit(2);
    };
    let operation = operation.to_string_lossy().into_owned();
    if !matches!(
        operation.as_str(),
        "status" | "login" | "login-api-key" | "logout"
    ) || arguments.next().is_some()
    {
        usage();
        process::exit(2);
    }
    let api_key = (operation == "login-api-key").then(|| {
        let mut value = String::new();
        std::io::stdin()
            .read_to_string(&mut value)
            .unwrap_or_else(|error| {
                eprintln!("app_server_api_key_read_failed: {error}");
                process::exit(1);
            });
        value.trim().to_owned()
    });
    let mut client = start_default_client();
    let result = match operation.as_str() {
        "status" => client.account_status().map(|status| match status {
            AccountStatus::ApiKey => println!("authenticated: api-key"),
            AccountStatus::ChatGpt { email } => println!(
                "authenticated: chatgpt{}",
                email.map_or_else(String::new, |email| format!(" ({email})"))
            ),
            AccountStatus::SignedOut => println!("signed out"),
        }),
        "login" => client.start_chatgpt_login().and_then(|handoff| {
            println!("open {}", handoff.auth_url);
            println!("waiting for login id: {}", handoff.login_id);
            client.await_chatgpt_login(&handoff.login_id)
        }),
        "login-api-key" => client
            .login_with_api_key(api_key.as_deref().unwrap_or_default())
            .map(|()| println!("authenticated: api-key")),
        _ => client.logout().map(|()| println!("signed out")),
    };
    result.unwrap_or_else(|error| {
        eprintln!("{}: {error}", error.code());
        process::exit(1);
    });
}

#[expect(
    clippy::print_stderr,
    clippy::print_stdout,
    reason = "the conversation CLI streams its final observation and inert tool requests"
)]
/// Runs one minimal streamed conversation.
fn run_conversation(arguments: impl Iterator<Item = std::ffi::OsString>) {
    let prompt = arguments
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    if prompt.is_empty() {
        usage();
        process::exit(2);
    }
    let mut client = start_default_client();
    let result = client.converse(&prompt).unwrap_or_else(|error| {
        eprintln!("{}: {error}", error.code());
        process::exit(1);
    });
    print!("{}", result.text);
    for request in result.inert_tool_requests {
        eprintln!(
            "inert tool request: {} {} {}",
            request.tool, request.call_id, request.arguments
        );
    }
}

#[expect(
    clippy::print_stderr,
    reason = "startup failures are emitted as stable CLI diagnostics"
)]
/// Starts the default isolated app-server client.
fn start_default_client() -> AppServerClient {
    let executable = resolve_executable("codex").unwrap_or_else(|| {
        eprintln!("app_server_executable_not_found: codex is not on PATH");
        process::exit(1);
    });
    let codex_home = tiber_codex_home().unwrap_or_else(|| {
        eprintln!("app_server_state_home_unavailable: HOME and XDG_STATE_HOME are unset");
        process::exit(1);
    });
    let workspace = env::current_dir().unwrap_or_else(|error| {
        eprintln!("app_server_workspace_unavailable: {error}");
        process::exit(1);
    });
    let config = AppServerConfig::new(
        executable,
        vec![
            "app-server".to_owned(),
            "--stdio".to_owned(),
            "--strict-config".to_owned(),
        ],
        codex_home,
        workspace,
        Duration::from_mins(10),
    )
    .unwrap_or_else(|error| {
        eprintln!("{}: {error}", error.code());
        process::exit(1);
    });
    AppServerClient::start(config, ISOLATED_CONFIG).unwrap_or_else(|error| {
        eprintln!("{}: {error}", error.code());
        process::exit(1);
    })
}

/// Resolves one executable from `PATH` without invoking a shell.
fn resolve_executable(name: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|path| {
        env::split_paths(&path)
            .map(|directory| directory.join(name))
            .find(|candidate| candidate.is_file())
    })
}

/// Resolves Tiber's persistent isolated Codex home.
fn tiber_codex_home() -> Option<PathBuf> {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| Path::new(&home).join(".local/state")))
        .map(|state| state.join("tiber/codex"))
}

#[expect(
    clippy::print_stderr,
    reason = "usage belongs on stderr for invalid command invocations"
)]
/// Prints the supported command grammar.
fn usage() {
    eprintln!(
        "usage: tiber app-server-probe <authority-surface.json> | auth <status|login|login-api-key|logout> | converse <prompt>"
    );
}
