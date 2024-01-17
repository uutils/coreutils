// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore winsize Openpty openpty xpixel ypixel
use crate::common::util::TestScenario;
use nix;
use std::thread::sleep;

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
    use libc::winsize;
    use nix::pty::OpenptyResult;
    use std::{
        fs::{self, File},
        io::Read,
    };

    let ts = TestScenario::new(util_name!());

    let terminal_size = winsize {
        ws_col: 80,
        ws_row: 30,
        ws_xpixel: 800,
        ws_ypixel: 300,
    };

    let OpenptyResult {
        slave: pi_slave,
        master: _pi_master,
    } = nix::pty::openpty(&terminal_size, None).unwrap();
    let OpenptyResult {
        slave: po_slave,
        master: po_master,
    } = nix::pty::openpty(&terminal_size, None).unwrap();
    let OpenptyResult {
        slave: pe_slave,
        master: pe_master,
    } = nix::pty::openpty(&terminal_size, None).unwrap();

    ts.ucmd()
        .set_stdin(pi_slave)
        .set_stdout(po_slave)
        .set_stderr(pe_slave)
        .args(&["./is_atty.sh"])
        .succeeds();

    let mut buffer_stdout = String::new();
    // Input/output error (os error 5) is returned due to pipe closes. Buffer gets content anyway.
    File::from(po_master)
        .read_to_string(&mut buffer_stdout)
        .unwrap_err();
    assert_eq!(buffer_stdout, "");

    let mut buffer_stderr = String::new();
    // Input/output error (os error 5) is returned due to pipe closes. Buffer gets content anyway.
    File::from(pe_master)
        .read_to_string(&mut buffer_stderr)
        .unwrap_err();
    assert_eq!(
        buffer_stderr.trim(),
        "nohup: ignoring input and appending output to 'nohup.out'"
    );

    sleep(std::time::Duration::from_millis(10));

    // this proves that nohup was exchanging the stdio file descriptors
    assert_eq!(
        fs::read_to_string(ts.fixtures.plus_as_string("nohup.out")).unwrap(),
        "stdin is not atty\nstdout is not atty\nstderr is not atty\n"
    );
}
