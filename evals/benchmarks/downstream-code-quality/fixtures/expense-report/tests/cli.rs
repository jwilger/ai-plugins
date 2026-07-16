use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn validate_reports_the_number_of_records() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_expense-report"))
        .arg("validate")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("expense-report should start");
    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(b"food,125\ntravel,400\n")
        .expect("fixture input should be written");

    let output = child.wait_with_output().expect("process should finish");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "valid,2\n");
}
