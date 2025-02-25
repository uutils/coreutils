// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore winsize Openpty openpty xpixel ypixel ptyprocess
#[cfg(not(target_os = "openbsd"))]
use std::thread::sleep;
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

// General observation: nohup.out will not be created in tests run by cargo test
// because stdin/stdout is not attached to a TTY.
// All that can be tested is the side-effects.

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(125);
}

#[test]
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_vendor = "apple"
))]
fn test_nohup_multiple_args_and_flags() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.args(&["touch", "-t", "1006161200", "file1", "file2"])
        .succeeds();
    sleep(std::time::Duration::from_millis(10));

    assert!(at.file_exists("file1"));
    assert!(at.file_exists("file2"));
}

#[test]
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_vendor = "apple"
))]
fn test_nohup_with_pseudo_terminal_emulation_on_stdin_stdout_stderr_get_replaced() {
    let ts = TestScenario::new(util_name!());
    let result = ts
        .ucmd()
        .terminal_simulation(true)
        .args(&["sh", "is_a_tty.sh"])
        .succeeds();

    assert_eq!(
        String::from_utf8_lossy(result.stderr()).trim(),
        "nohup: ignoring input and appending output to 'nohup.out'"
    );

    sleep(std::time::Duration::from_millis(10));

    // this proves that nohup was exchanging the stdio file descriptors
    assert_eq!(
        std::fs::read_to_string(ts.fixtures.plus_as_string("nohup.out")).unwrap(),
        "stdin is not a tty\nstdout is not a tty\nstderr is not a tty\n"
    );
}
