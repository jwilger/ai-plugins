mod support;

use std::io::ErrorKind;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

    let (mut child, line) = start_dashboard(&repo);
    stop_dashboard(&mut child);

    assert!(
        line.starts_with("tiber dashboard listening on http://127.0.0.1:")
            && line != "tiber dashboard listening on http://127.0.0.1:7417",
        "unexpected dashboard startup output: {line}"
    );
}

#[test]
fn dashboard_reuses_the_healthy_instance_for_the_same_repository() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    let (mut first, first_line) = start_dashboard(&repo);

    let mut second = dashboard_command(&repo)
        .spawn()
        .expect("repeat dashboard launch");
    let deadline = Instant::now() + Duration::from_secs(3);
    let second_exited = loop {
        if second
            .try_wait()
            .expect("read repeated launch status")
            .is_some()
        {
            break true;
        }
        if Instant::now() >= deadline {
            break false;
        }
        std::thread::sleep(Duration::from_millis(20));
    };

    if !second_exited {
        second.kill().ok();
    }
    let second_output = second.wait_with_output().expect("read repeated launch");
    stop_dashboard(&mut first);

    let url = first_line
        .strip_prefix("tiber dashboard listening on ")
        .expect("first launch should print URL");
    assert_eq!(
        String::from_utf8(second_output.stdout).expect("repeated launch output should be utf8"),
        format!("tiber dashboard already running on {url}\n")
    );
}

#[test]
fn dashboard_waits_for_another_same_project_startup_owner() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    let runtime_dir = repo.path().join(".git/tiber");
    std::fs::create_dir_all(&runtime_dir).expect("create dashboard runtime directory");
    let lock_path = runtime_dir.join("dashboard-startup.lock");
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_secs();
    let lock_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .expect("open dashboard startup lock");
    std::fs::write(
        &lock_path,
        format!("pid={}\ntimestamp={timestamp}\n", std::process::id()),
    )
    .expect("write dashboard startup lock");
    lock_file.lock().expect("hold dashboard startup lock");

    let mut child = dashboard_command(&repo).spawn().expect("start dashboard");
    let stdout = child.stdout.take().expect("dashboard stdout");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut lines = BufReader::new(stdout).lines();
        sender.send(lines.next().transpose()).ok();
    });

    let early_line = receiver.recv_timeout(Duration::from_millis(300));
    let started_while_owned = early_line.is_ok();
    drop(lock_file);
    let line = early_line
        .or_else(|_| receiver.recv_timeout(Duration::from_secs(5)))
        .expect("dashboard should start after lock release")
        .expect("read dashboard output")
        .expect("dashboard should print one line");
    stop_dashboard(&mut child);

    assert!(
        !started_while_owned && line.starts_with("tiber dashboard listening on http://127.0.0.1:"),
        "dashboard must wait for the same-project startup owner; output={line}"
    );
}

#[test]
fn dashboard_recovers_a_malformed_startup_lock() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    let runtime_dir = repo.path().join(".git/tiber");
    std::fs::create_dir_all(&runtime_dir).expect("create dashboard runtime directory");
    std::fs::write(runtime_dir.join("dashboard-startup.lock"), "")
        .expect("write interrupted startup lock");

    let mut child = dashboard_command(&repo)
        .env("TIBER_LOCK_RETRY_TIMEOUT_MS", "0")
        .spawn()
        .expect("start dashboard");
    let stdout = child.stdout.take().expect("dashboard stdout");
    let line = BufReader::new(stdout)
        .lines()
        .next()
        .transpose()
        .expect("read dashboard output");
    stop_dashboard(&mut child);

    assert!(
        line.is_some_and(|line| line.starts_with("tiber dashboard listening on http://127.0.0.1:")),
        "dashboard should recover interrupted startup ownership"
    );
}

fn dashboard_command(repo: &TempRepo) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tiber"));
    command
        .args(["dashboard", "serve"])
        .env_remove("TIBER_DASHBOARD_PORT")
        .current_dir(repo.path())
        .stdout(Stdio::piped());
    command
}

fn start_dashboard(repo: &TempRepo) -> (std::process::Child, String) {
    let mut child = dashboard_command(repo).spawn().expect("start dashboard");
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

    (child, line)
}

fn stop_dashboard(child: &mut std::process::Child) {
    child.kill().ok();
    child.wait().ok();
}
