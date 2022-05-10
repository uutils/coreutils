use std::fs::File;

use crate::common::util::*;

#[test]
#[cfg(not(windows))]
fn test_dev_null() {
    new_ucmd!()
        .set_stdin(File::open("/dev/null").unwrap())
        .fails()
        .code_is(1)
        .stdout_is("not a tty\n");
}

#[test]
#[cfg(not(windows))]
fn test_dev_null_silent() {
    new_ucmd!()
        .args(&["-s"])
        .set_stdin(File::open("/dev/null").unwrap())
        .fails()
        .code_is(1)
        .stdout_is("");
}

#[test]
fn test_close_stdin() {
    let mut child = new_ucmd!().run_no_wait();
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(std::str::from_utf8(&output.stdout), Ok("not a tty\n"));
}

#[test]
fn test_close_stdin_silent() {
    let mut child = new_ucmd!().arg("-s").run_no_wait();
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}

#[test]
fn test_close_stdin_silent_long() {
    let mut child = new_ucmd!().arg("--silent").run_no_wait();
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}

#[test]
fn test_close_stdin_silent_alias() {
    let mut child = new_ucmd!().arg("--quiet").run_no_wait();
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}

#[test]
fn test_wrong_argument() {
    new_ucmd!().args(&["a"]).fails().code_is(2);
}

#[test]
fn test_help() {
    new_ucmd!().args(&["--help"]).succeeds();
}

#[test]
// FixME: freebsd panic
#[cfg(all(unix, not(target_os = "freebsd")))]
fn test_stdout_fail() {
    use std::process::{Command, Stdio};
    let ts = TestScenario::new(util_name!());
    // Sleep inside a shell to ensure the process doesn't finish before we've
    // closed its stdout
    let mut proc = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "sleep 0.2; exec {} {}",
            ts.bin_path.to_str().unwrap(),
            ts.util_name
        ))
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    drop(proc.stdout.take());
    let status = proc.wait().unwrap();
    assert_eq!(status.code(), Some(3));
}
