mod support;

use std::io::ErrorKind;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use support::TempRepo;

#[test]
fn dashboard_without_a_fixed_port_selects_an_available_port_and_prints_its_url() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    let _legacy_port = match TcpListener::bind("127.0.0.1:7417") {
        Ok(listener) => Some(listener),
        Err(error) if error.kind() == ErrorKind::AddrInUse => None,
        Err(error) => panic!("occupy legacy dashboard port: {error}"),
    };

    let mut child = Command::new(env!("CARGO_BIN_EXE_tiber"))
        .args(["dashboard", "serve"])
        .env_remove("TIBER_DASHBOARD_PORT")
        .current_dir(repo.path())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start dashboard");
    let stdout = child.stdout.take().expect("dashboard stdout");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut lines = BufReader::new(stdout).lines();
        sender.send(lines.next().transpose()).ok();
    });

    let line = receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("dashboard should report its URL")
        .expect("read dashboard output")
        .expect("dashboard should print one line");

    child.kill().ok();
    child.wait().ok();

    assert!(
        line.starts_with("tiber dashboard listening on http://127.0.0.1:")
            && line != "tiber dashboard listening on http://127.0.0.1:7417",
        "unexpected dashboard startup output: {line}"
    );
}
