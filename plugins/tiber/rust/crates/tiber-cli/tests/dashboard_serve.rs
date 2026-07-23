mod support;

use std::io::ErrorKind;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
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

#[test]
fn dashboard_uses_an_explicitly_requested_available_port() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("reserve available dashboard port")
        .local_addr()
        .expect("read reserved port")
        .port();
    let port_value = port.to_string();

    let mut child =
        dashboard_command_with_args(&repo, ["dashboard", "serve", "--port", &port_value])
            .spawn()
            .expect("start dashboard");
    let stdout = child.stdout.take().expect("dashboard stdout");
    let line = BufReader::new(stdout)
        .lines()
        .next()
        .transpose()
        .expect("read dashboard output");
    stop_dashboard(&mut child);

    assert_eq!(
        line,
        Some(format!(
            "tiber dashboard listening on http://127.0.0.1:{port}"
        ))
    );
}

#[test]
fn dashboard_refuses_a_different_explicit_port_when_the_project_is_already_running() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    let first_port = available_port();
    let first_port_value = first_port.to_string();
    let (mut first, _) =
        start_dashboard_with_args(&repo, ["dashboard", "serve", "--port", &first_port_value]);
    let requested_port = available_port();
    let requested_port_value = requested_port.to_string();

    let second = dashboard_command_with_args(
        &repo,
        ["dashboard", "serve", "--port", &requested_port_value],
    )
    .output()
    .expect("repeat dashboard with different explicit port");
    stop_dashboard(&mut first);

    assert!(
        !second.status.success()
            && String::from_utf8(second.stderr)
                .expect("dashboard error should be utf8")
                .contains(&format!(
                    "dashboard_port_conflict requested={requested_port} running={first_port}"
                )),
        "a different explicit port must not be silently ignored"
    );
}

#[test]
fn dashboard_cli_port_overrides_the_environment_port() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    let requested_port = available_port();
    let requested_port_value = requested_port.to_string();
    let occupied_environment_port =
        TcpListener::bind("127.0.0.1:0").expect("occupy environment dashboard port");
    let environment_port = occupied_environment_port
        .local_addr()
        .expect("read environment dashboard port")
        .port()
        .to_string();

    let mut child = dashboard_command_with_args(
        &repo,
        ["dashboard", "serve", "--port", &requested_port_value],
    )
    .env("TIBER_DASHBOARD_PORT", environment_port)
    .spawn()
    .expect("start dashboard");
    let stdout = child.stdout.take().expect("dashboard stdout");
    let line = BufReader::new(stdout)
        .lines()
        .next()
        .transpose()
        .expect("read dashboard output");
    stop_dashboard(&mut child);

    assert_eq!(
        line,
        Some(format!(
            "tiber dashboard listening on http://127.0.0.1:{requested_port}"
        ))
    );
}

#[test]
fn dashboard_port_zero_reuses_the_automatically_selected_port() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    let (mut first, first_line) =
        start_dashboard_with_args(&repo, ["dashboard", "serve", "--port", "0"]);

    let second = dashboard_command_with_args(&repo, ["dashboard", "serve", "--port", "0"])
        .output()
        .expect("repeat automatic dashboard launch");
    stop_dashboard(&mut first);

    let url = first_line
        .strip_prefix("tiber dashboard listening on ")
        .expect("first launch should print URL");
    assert!(
        second.status.success()
            && String::from_utf8(second.stdout).expect("dashboard output should be utf8")
                == format!("tiber dashboard already running on {url}\n"),
        "port zero should retain automatic-port reuse semantics"
    );
}

#[test]
fn dashboard_opens_the_browser_only_for_a_genuinely_new_launch() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    let browser_bin = repo.path().join("browser-bin");
    std::fs::create_dir(&browser_bin).expect("create fake browser bin");
    let opener = if cfg!(target_os = "macos") {
        browser_bin.join("open")
    } else {
        browser_bin.join("xdg-open")
    };
    std::fs::write(
        &opener,
        "#!/bin/sh\nprintf '%s\\n' \"$1\" >> \"$TIBER_BROWSER_CAPTURE\"\n",
    )
    .expect("write fake browser opener");
    std::fs::set_permissions(&opener, std::fs::Permissions::from_mode(0o755))
        .expect("make fake browser opener executable");
    let capture = repo.path().join("browser-calls");
    let path = format!(
        "{}:{}",
        browser_bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let mut first = dashboard_command_with_args(&repo, ["dashboard", "serve", "--open"])
        .env("PATH", &path)
        .env("TIBER_BROWSER_CAPTURE", &capture)
        .spawn()
        .expect("start and open dashboard");
    let stdout = first.stdout.take().expect("dashboard stdout");
    let first_line = BufReader::new(stdout)
        .lines()
        .next()
        .transpose()
        .expect("read dashboard output")
        .expect("new dashboard should print URL");
    let second = dashboard_command_with_args(&repo, ["dashboard", "serve", "--open"])
        .env("PATH", path)
        .env("TIBER_BROWSER_CAPTURE", &capture)
        .output()
        .expect("repeat dashboard open");
    let deadline = Instant::now() + Duration::from_secs(3);
    while !capture.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(100));
    stop_dashboard(&mut first);

    let url = first_line
        .strip_prefix("tiber dashboard listening on ")
        .expect("first launch should print URL");
    assert!(
        second.status.success()
            && std::fs::read_to_string(capture).expect("read browser calls") == format!("{url}\n"),
        "the browser opener must run exactly once for the new dashboard"
    );
}

#[test]
fn dashboard_keeps_serving_when_the_browser_opener_is_unavailable() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    let empty_path = repo.path().join("empty-path");
    std::fs::create_dir(&empty_path).expect("create empty executable path");
    let git = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|directory| directory.join("git"))
        .find(|candidate| candidate.is_file())
        .expect("find git executable");
    std::os::unix::fs::symlink(git, empty_path.join("git"))
        .expect("retain git without a browser opener");

    let mut child = dashboard_command_with_args(&repo, ["dashboard", "serve", "--open"])
        .env("PATH", empty_path)
        .stderr(Stdio::piped())
        .spawn()
        .expect("start dashboard without browser opener");
    let stdout = child.stdout.take().expect("dashboard stdout");
    let stderr = child.stderr.take().expect("dashboard stderr");
    let line = BufReader::new(stdout)
        .lines()
        .next()
        .transpose()
        .expect("read dashboard output");
    let still_running = child.try_wait().expect("read dashboard status").is_none();
    let warning = BufReader::new(stderr)
        .lines()
        .next()
        .transpose()
        .expect("read browser warning")
        .expect("browser failure should print a warning");
    stop_dashboard(&mut child);

    assert!(
        still_running
            && line.is_some_and(|line| {
                line.starts_with("tiber dashboard listening on http://127.0.0.1:")
            })
            && warning.contains("dashboard_continues=true")
            && !warning.contains("http://"),
        "browser opener failure must not stop the dashboard"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn dashboard_reaps_the_browser_opener_process() {
    let repo = TempRepo::initialized();
    repo.tiber(["init"]);
    let browser_bin = repo.path().join("browser-bin");
    std::fs::create_dir(&browser_bin).expect("create fake browser bin");
    let opener = browser_bin.join("xdg-open");
    std::fs::write(
        &opener,
        "#!/bin/sh\nprintf '%s\\n' \"$$\" > \"$TIBER_BROWSER_PID_CAPTURE\"\n",
    )
    .expect("write fake browser opener");
    std::fs::set_permissions(&opener, std::fs::Permissions::from_mode(0o755))
        .expect("make fake browser opener executable");
    let capture = repo.path().join("browser-pid");
    let path = format!(
        "{}:{}",
        browser_bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let mut dashboard = dashboard_command_with_args(&repo, ["dashboard", "serve", "--open"])
        .env("PATH", path)
        .env("TIBER_BROWSER_PID_CAPTURE", &capture)
        .spawn()
        .expect("start dashboard");
    let stdout = dashboard.stdout.take().expect("dashboard stdout");
    let _line = BufReader::new(stdout)
        .lines()
        .next()
        .transpose()
        .expect("read dashboard output")
        .expect("dashboard should print URL");
    let deadline = Instant::now() + Duration::from_secs(3);
    while !capture.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let opener_pid = std::fs::read_to_string(capture)
        .expect("read browser opener pid")
        .trim()
        .to_string();
    let process_path = std::path::PathBuf::from(format!("/proc/{opener_pid}"));
    let deadline = Instant::now() + Duration::from_secs(3);
    while process_path.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let reaped = !process_path.exists();
    let dashboard_still_running = dashboard
        .try_wait()
        .expect("read dashboard status")
        .is_none();
    stop_dashboard(&mut dashboard);

    assert!(
        reaped && dashboard_still_running,
        "dashboard must reap the browser opener without exiting"
    );
}

fn available_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("reserve available dashboard port")
        .local_addr()
        .expect("read reserved port")
        .port()
}

fn dashboard_command(repo: &TempRepo) -> Command {
    dashboard_command_with_args(repo, ["dashboard", "serve"])
}

fn dashboard_command_with_args<I, S>(repo: &TempRepo, args: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut command = Command::new(env!("CARGO_BIN_EXE_tiber"));
    command
        .args(args)
        .env_remove("TIBER_DASHBOARD_PORT")
        .current_dir(repo.path())
        .stdout(Stdio::piped());
    command
}

fn start_dashboard(repo: &TempRepo) -> (std::process::Child, String) {
    start_dashboard_with_args(repo, ["dashboard", "serve"])
}

fn start_dashboard_with_args<I, S>(repo: &TempRepo, args: I) -> (std::process::Child, String)
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut child = dashboard_command_with_args(repo, args)
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

    (child, line)
}

fn stop_dashboard(child: &mut std::process::Child) {
    child.kill().ok();
    child.wait().ok();
}
