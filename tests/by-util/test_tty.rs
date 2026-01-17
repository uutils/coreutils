// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::fs::File;

use uutests::new_ucmd;

#[test]
#[cfg(not(windows))]
fn test_dev_null() {
    new_ucmd!()
        .set_stdin(File::open("/dev/null").unwrap())
        .fails_with_code(1)
        .stdout_is("not a tty\n");
}

#[test]
#[cfg(not(windows))]
fn test_dev_null_silent() {
    new_ucmd!()
        .args(&["-s"])
        .set_stdin(File::open("/dev/null").unwrap())
        .fails_with_code(1)
        .stdout_is("");
}

#[test]
fn test_close_stdin() {
    let mut child = new_ucmd!().run_no_wait();
    child.close_stdin();
    child.wait().unwrap().code_is(1).stdout_is("not a tty\n");
}

#[test]
fn test_close_stdin_silent() {
    let mut child = new_ucmd!().arg("-s").run_no_wait();
    child.close_stdin();
    child.wait().unwrap().code_is(1).no_stdout();
}

#[test]
fn test_close_stdin_silent_long() {
    let mut child = new_ucmd!().arg("--silent").run_no_wait();
    child.close_stdin();
    child.wait().unwrap().code_is(1).no_stdout();
}

#[test]
fn test_close_stdin_silent_alias() {
    let mut child = new_ucmd!().arg("--quiet").run_no_wait();
    child.close_stdin();
    child.wait().unwrap().code_is(1).no_stdout();
}

#[test]
fn test_wrong_argument() {
    new_ucmd!().args(&["a"]).fails_with_code(2);
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
    use uutests::at_and_ts;

    let (_, ts) = at_and_ts!();
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

#[test]
#[cfg(all(unix, not(target_os = "freebsd")))]
fn test_newline_in_output() {
    // Test that output ends with a newline, regardless of success or failure
    let result = new_ucmd!().run();
    let stdout = result.stdout_str();
    
    // Whether it's "not a tty\n" or an actual tty path, it should end with newline
    assert!(
        stdout.ends_with('\n'),
        "Output should end with newline, got: {:?}",
        stdout
    );
}

#[test]
#[cfg(all(unix, not(target_os = "freebsd")))]
fn test_write_error_dev_full() {
    use std::process::{Command, Stdio};
    use uutests::at_and_ts;

    let (_, ts) = at_and_ts!();
    
    // Redirect stdout to /dev/full which causes write errors
    let output = Command::new(ts.bin_path.to_str().unwrap())
        .arg(&ts.util_name)
        .stdout(Stdio::from(File::create("/dev/full").unwrap()))
        .output()
        .unwrap();
    
    // Should exit with code 3 and print error to stderr
    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("write error"),
        "Should contain 'write error' in stderr, got: {:?}",
        stderr
    );
}
