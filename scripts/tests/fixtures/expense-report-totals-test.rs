use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn totals_aggregates_sorts_and_filters_inclusively() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_expense-report"))
        .args(["totals", "--minimum-cents", "150"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("expense-report should start");
    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(b"travel,200\nfood,100\nfood,50\n")
        .expect("fixture input should be written");

    let output = child.wait_with_output().expect("process should finish");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "food,150\ntravel,200\n"
    );
}
