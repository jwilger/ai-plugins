#![forbid(unsafe_code)]

use std::{env, fs, process};

use tiber_app_server::inspect_protocol_schema;

#[expect(
    clippy::print_stderr,
    clippy::print_stdout,
    reason = "a command-line adapter intentionally writes its result and diagnostics"
)]
fn main() {
    let mut arguments = env::args_os();
    let _executable = arguments.next();
    let Some(command) = arguments.next() else {
        eprintln!("usage: tiber app-server-probe <authority-surface.json>");
        process::exit(2);
    };
    let Some(schema_path) = arguments.next() else {
        eprintln!("usage: tiber app-server-probe <authority-surface.json>");
        process::exit(2);
    };
    if command != "app-server-probe" {
        eprintln!("unknown command: {}", command.to_string_lossy());
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
    if report.is_compatible() {
        println!("app-server protocol preserves the Tiber authority contract");
        return;
    }

    eprintln!(
        "app_server_tool_isolation_unverified: the verified app-server schema has operation item types but no reviewed isolation proof: {}",
        report.unverified_operations().join(", ")
    );
    process::exit(1);
}
