use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() {
    if std::env::var_os("EXPENSE_REPORT_HANGING_DESCENDANT").is_some() {
        thread::sleep(Duration::from_secs(8));
        return;
    }

    Command::new(std::env::current_exe().expect("current executable should resolve"))
        .env("EXPENSE_REPORT_HANGING_DESCENDANT", "1")
        .spawn()
        .expect("pipe-holding descendant should start");
    thread::sleep(Duration::from_secs(60));
}
