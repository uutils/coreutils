// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::ffi::OsStr;
use std::process::ExitStatus;

use uutests::{get_tests_binary, new_ucmd};

#[cfg(unix)]
fn check_termination(result: ExitStatus) {
    // When SIGPIPE is NOT trapped, yes is killed by signal 13 (exit 141)
    // When SIGPIPE IS trapped, yes exits with code 1
    assert!(!result.success(), "yes should fail on broken pipe");
}

#[cfg(not(unix))]
fn check_termination(result: ExitStatus) {
    assert!(result.success(), "yes did not exit successfully");
}

const NO_ARGS: &[&str] = &[];

/// Run `yes`, capture some of the output, then check exit status.
fn run(args: &[impl AsRef<OsStr>], expected: &[u8]) {
    let result = new_ucmd!().args(args).run_stdout_starts_with(expected);
    check_termination(result.exit_status());
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_version() {
    new_ucmd!().arg("--version").succeeds();
}

#[test]
fn test_simple() {
    run(NO_ARGS, b"y\ny\ny\ny\n");
}

#[test]
fn test_args() {
    run(&["a", "bar", "c"], b"a bar c\na bar c\na ba");
}

#[test]
fn test_long_output() {
    run(NO_ARGS, "y\n".repeat(512 * 1024).as_bytes());
}

/// Test with an output that seems likely to get mangled in case of incomplete writes.
#[test]
fn test_long_odd_output() {
    run(&["abcdef"], "abcdef\n".repeat(1024 * 1024).as_bytes());
}

/// Test with an input that doesn't fit in the standard buffer.
#[test]
fn test_long_input() {
    #[cfg(not(windows))]
    const TIMES: usize = 14000;
    // On Windows the command line is limited to 8191 bytes.
    // This is not actually enough to fill the buffer, but it's still nice to
    // try something long.
    #[cfg(windows)]
    const TIMES: usize = 500;
    let arg = "abcdef".repeat(TIMES) + "\n";
    let expected_out = arg.repeat(30);
    run(&[&arg[..arg.len() - 1]], expected_out.as_bytes());
}

#[test]
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "netbsd"))]
fn test_piped_to_dev_full() {
    use std::fs::OpenOptions;

    for append in [true, false] {
        {
            let dev_full = OpenOptions::new()
                .write(true)
                .append(append)
                .open("/dev/full")
                .unwrap();

            new_ucmd!()
                .set_stdout(dev_full)
                .fails()
                .stderr_contains("No space left on device");
        }
    }
}

#[test]
#[cfg(any(unix, target_os = "wasi"))]
fn test_non_utf8() {
    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(target_os = "wasi")]
    use std::os::wasi::ffi::OsStrExt;

    run(
        &[
            OsStr::from_bytes(b"\xbf\xff\xee"),
            OsStr::from_bytes(b"bar"),
        ],
        &b"\xbf\xff\xee bar\n".repeat(5000),
    );
}

/// Test SIGPIPE handling in normal pipe scenario
///
/// When SIGPIPE is NOT trapped, `yes` should:
/// 1. Be killed by SIGPIPE signal (exit code 141 = 128 + 13)
/// 2. NOT print any error message to stderr
///
/// This test uses a shell command to simulate `yes | head -n 1`
/// The expected behavior matches GNU yes.
#[test]
#[cfg(unix)]
fn test_normal_pipe_sigpipe() {
    use std::process::Command;

    // Run `yes | head -n 1` via shell with pipefail to capture yes's exit code
    // In this scenario, SIGPIPE is not trapped, so yes should be killed by the signal
    let output = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "set -o pipefail; {} yes | head -n 1 > /dev/null",
            get_tests_binary!()
        ))
        .output()
        .expect("Failed to execute yes | head");

    // Extract exit code
    let exit_code = output.status.code();

    // The process should be killed by SIGPIPE (signal 13)
    // Exit code should be 141 (128 + 13) on most Unix systems
    // OR the process was terminated by signal (status.code() returns None)
    if let Some(code) = exit_code {
        assert_eq!(
            code, 141,
            "yes should exit with code 141 (killed by SIGPIPE), but got {code}"
        );
    } else {
        // Process was terminated by signal (which is also acceptable)
        use std::os::unix::process::ExitStatusExt;
        let signal = output.status.signal().unwrap();
        // Signal 13 is SIGPIPE
        assert_eq!(signal, 13, "yes should be killed by SIGPIPE (13)");
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty(),
        "yes should NOT print error message in normal pipe scenario, but got: {stderr}"
    );
}
