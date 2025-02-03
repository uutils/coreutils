// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) abcdefghijklmnopqrstuvwxyz efghijklmnopqrstuvwxyz vwxyz emptyfile file siette ocho nueve diez MULT
// spell-checker:ignore (libs) kqueue
// spell-checker:ignore (jargon) tailable untailable datasame runneradmin tmpi
// spell-checker:ignore (cmd) taskkill
#![allow(
    clippy::unicode_not_nfc,
    clippy::cast_lossless,
    clippy::cast_possible_truncation
)]

use pretty_assertions::assert_eq;
use rand::distr::Alphanumeric;
use rstest::rstest;
use std::char::from_digit;
use std::fs::File;
use std::io::Write;
#[cfg(not(target_vendor = "apple"))]
use std::io::{Seek, SeekFrom};
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "android"),
    not(target_os = "windows"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))]
use std::path::Path;
use std::process::Stdio;
use tail::chunks::BUFFER_SIZE as CHUNK_BUFFER_SIZE;
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "android"),
    not(target_os = "windows"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))]
use tail::text;
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::random::{AlphanumericNewline, RandomizedString};
#[cfg(unix)]
use uutests::unwrap_or_return;
#[cfg(unix)]
use uutests::util::expected_result;
#[cfg(unix)]
#[cfg(not(windows))]
use uutests::util::is_ci;
use uutests::util::TestScenario;
use uutests::util_name;

const FOOBAR_TXT: &str = "foobar.txt";
const FOOBAR_2_TXT: &str = "foobar2.txt";
const FOOBAR_WITH_NULL_TXT: &str = "foobar_with_null.txt";
#[allow(dead_code)]
const FOLLOW_NAME_TXT: &str = "follow_name.txt";
#[allow(dead_code)]
const FOLLOW_NAME_SHORT_EXP: &str = "follow_name_short.expected";
#[allow(dead_code)]
const FOLLOW_NAME_EXP: &str = "follow_name.expected";

const DEFAULT_SLEEP_INTERVAL_MILLIS: u64 = 1000;

// The binary integer "10000000" is *not* a valid UTF-8 encoding
// of a character: https://en.wikipedia.org/wiki/UTF-8#Encoding
#[cfg(unix)]
const INVALID_UTF8: u8 = 0x80;
#[cfg(windows)]
const INVALID_UTF16: u16 = 0xD800;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_stdin_default() {
    new_ucmd!()
        .pipe_in_fixture(FOOBAR_TXT)
        .run()
        .stdout_is_fixture("foobar_stdin_default.expected")
        .no_stderr();
}

#[test]
fn test_stdin_explicit() {
    new_ucmd!()
        .pipe_in_fixture(FOOBAR_TXT)
        .arg("-")
        .run()
        .stdout_is_fixture("foobar_stdin_default.expected")
        .no_stderr();
}

#[test]
// FIXME: the -f test fails with: Assertion failed. Expected 'tail' to be running but exited with status=exit status: 0
#[ignore = "disabled until fixed"]
#[cfg(not(target_vendor = "apple"))] // FIXME: for currently not working platforms
fn test_stdin_redirect_file() {
    // $ echo foo > f

    // $ tail < f
    // foo

    // $ tail -f < f
    // foo
    //

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.write("f", "foo");

    ts.ucmd()
        .set_stdin(File::open(at.plus("f")).unwrap())
        .run()
        .stdout_is("foo")
        .succeeded();
    ts.ucmd()
        .set_stdin(File::open(at.plus("f")).unwrap())
        .arg("-v")
        .run()
        .no_stderr()
        .stdout_is("==> standard input <==\nfoo")
        .succeeded();

    let mut p = ts
        .ucmd()
        .arg("-f")
        .set_stdin(File::open(at.plus("f")).unwrap())
        .run_no_wait();

    p.make_assertion_with_delay(500).is_alive();
    p.kill()
        .make_assertion()
        .with_all_output()
        .stdout_only("foo");
}

#[test]
#[cfg(not(target_vendor = "apple"))] // FIXME: for currently not working platforms
fn test_stdin_redirect_offset() {
    // inspired by: "gnu/tests/tail-2/start-middle.sh"

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.write("k", "1\n2\n");
    let mut fh = File::open(at.plus("k")).unwrap();
    fh.seek(SeekFrom::Start(2)).unwrap();

    ts.ucmd()
        .set_stdin(fh)
        .run()
        .no_stderr()
        .stdout_is("2\n")
        .succeeded();
}

#[test]
#[cfg(not(target_vendor = "apple"))] // FIXME: for currently not working platforms
fn test_stdin_redirect_offset2() {
    // like test_stdin_redirect_offset but with multiple files

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.write("k", "1\n2\n");
    at.write("l", "3\n4\n");
    at.write("m", "5\n6\n");
    let mut fh = File::open(at.plus("k")).unwrap();
    fh.seek(SeekFrom::Start(2)).unwrap();

    ts.ucmd()
        .set_stdin(fh)
        .args(&["k", "-", "l", "m"])
        .run()
        .no_stderr()
        .stdout_is(
            "==> k <==\n1\n2\n\n==> standard input <==\n2\n\n==> l <==\n3\n4\n\n==> m <==\n5\n6\n",
        )
        .succeeded();
}

#[test]
fn test_nc_0_wo_follow() {
    // verify that -[nc]0 without -f, exit without reading

    let ts = TestScenario::new(util_name!());
    ts.ucmd()
        .args(&["-n0", "missing"])
        .run()
        .no_stderr()
        .no_stdout()
        .succeeded();
    ts.ucmd()
        .args(&["-c0", "missing"])
        .run()
        .no_stderr()
        .no_stdout()
        .succeeded();
}

#[test]
#[cfg(all(unix, not(target_os = "freebsd")))]
fn test_nc_0_wo_follow2() {
    use std::os::unix::fs::PermissionsExt;
    // verify that -[nc]0 without -f, exit without reading

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.make_file("unreadable")
        .set_permissions(PermissionsExt::from_mode(0o000))
        .unwrap();

    ts.ucmd()
        .args(&["-n0", "unreadable"])
        .run()
        .no_stderr()
        .no_stdout()
        .succeeded();
    ts.ucmd()
        .args(&["-c0", "unreadable"])
        .run()
        .no_stderr()
        .no_stdout()
        .succeeded();
}

// TODO: Add similar test for windows
#[test]
#[cfg(unix)]
fn test_permission_denied() {
    use std::os::unix::fs::PermissionsExt;

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.make_file("unreadable")
        .set_permissions(PermissionsExt::from_mode(0o000))
        .unwrap();

    ts.ucmd()
        .arg("unreadable")
        .fails()
        .stderr_is("tail: cannot open 'unreadable' for reading: Permission denied\n")
        .no_stdout()
        .code_is(1);
}

// TODO: Add similar test for windows
#[test]
#[cfg(unix)]
fn test_permission_denied_multiple() {
    use std::os::unix::fs::PermissionsExt;

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.touch("file1");
    at.touch("file2");

    at.make_file("unreadable")
        .set_permissions(PermissionsExt::from_mode(0o000))
        .unwrap();

    ts.ucmd()
        .args(&["file1", "unreadable", "file2"])
        .fails()
        .stderr_is("tail: cannot open 'unreadable' for reading: Permission denied\n")
        .stdout_is("==> file1 <==\n\n==> file2 <==\n")
        .code_is(1);
}

#[test]
fn test_follow_redirect_stdin_name_retry() {
    // $ touch f && tail -F - < f
    // tail: cannot follow '-' by name
    // NOTE: Not sure why GNU's tail doesn't just follow `f` in this case.

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("f");

    let mut args = vec!["-F", "-"];
    for _ in 0..2 {
        ts.ucmd()
            .set_stdin(File::open(at.plus("f")).unwrap())
            .args(&args)
            .fails()
            .no_stdout()
            .stderr_is("tail: cannot follow '-' by name\n")
            .code_is(1);
        args.pop();
    }
}

#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))] // FIXME: for currently not working platforms
fn test_stdin_redirect_dir() {
    // $ mkdir dir
    // $ tail < dir, $ tail - < dir
    // tail: error reading 'standard input': Is a directory

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkdir("dir");

    ts.ucmd()
        .set_stdin(File::open(at.plus("dir")).unwrap())
        .fails()
        .no_stdout()
        .stderr_is("tail: error reading 'standard input': Is a directory\n")
        .code_is(1);
    ts.ucmd()
        .set_stdin(File::open(at.plus("dir")).unwrap())
        .arg("-")
        .fails()
        .no_stdout()
        .stderr_is("tail: error reading 'standard input': Is a directory\n")
        .code_is(1);
}

// On macOS path.is_dir() can be false for directories if it was a redirect,
// e.g. `$ tail < DIR. The library feature to detect the
// std::io::ErrorKind::IsADirectory isn't stable so we currently show the a wrong
// error message.
// FIXME: If `std::io::ErrorKind::IsADirectory` becomes stable or macos handles
//  redirected directories like linux show the correct message like in
//  `test_stdin_redirect_dir`
#[test]
#[cfg(target_vendor = "apple")]
fn test_stdin_redirect_dir_when_target_os_is_macos() {
    // $ mkdir dir
    // $ tail < dir, $ tail - < dir
    // tail: error reading 'standard input': Is a directory

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkdir("dir");

    ts.ucmd()
        .set_stdin(File::open(at.plus("dir")).unwrap())
        .fails()
        .no_stdout()
        .stderr_is("tail: cannot open 'standard input' for reading: No such file or directory\n")
        .code_is(1);
    ts.ucmd()
        .set_stdin(File::open(at.plus("dir")).unwrap())
        .arg("-")
        .fails()
        .no_stdout()
        .stderr_is("tail: cannot open 'standard input' for reading: No such file or directory\n")
        .code_is(1);
}

#[test]
fn test_follow_stdin_descriptor() {
    let ts = TestScenario::new(util_name!());

    let mut args = vec!["-f", "-"];
    for _ in 0..2 {
        let mut p = ts
            .ucmd()
            .set_stdin(Stdio::piped())
            .args(&args)
            .run_no_wait();
        p.make_assertion_with_delay(500).is_alive();
        p.kill()
            .make_assertion()
            .with_all_output()
            .no_stderr()
            .no_stdout();

        args.pop();
    }
}

#[test]
fn test_follow_stdin_name_retry() {
    // $ tail -F -
    // tail: cannot follow '-' by name
    let mut args = vec!["-F", "-"];
    for _ in 0..2 {
        new_ucmd!()
            .args(&args)
            .run()
            .no_stdout()
            .stderr_is("tail: cannot follow '-' by name\n")
            .code_is(1);
        args.pop();
    }
}

#[test]
fn test_follow_bad_fd() {
    // Provoke a "bad file descriptor" error by closing the fd
    // inspired by: "gnu/tests/tail-2/follow-stdin.sh"

    // `$ tail -f <&-` OR `$ tail -f - <&-`
    // tail: cannot fstat 'standard input': Bad file descriptor
    // tail: error reading 'standard input': Bad file descriptor
    // tail: no files remaining
    // tail: -: Bad file descriptor
    //
    // $ `tail <&-`
    // tail: cannot fstat 'standard input': Bad file descriptor
    // tail: -: Bad file descriptor

    // WONT-FIX:
    // see also: https://github.com/uutils/coreutils/issues/2873
}

#[test]
fn test_single_default() {
    new_ucmd!()
        .arg(FOOBAR_TXT)
        .run()
        .stdout_is_fixture("foobar_single_default.expected");
}

#[test]
fn test_n_greater_than_number_of_lines() {
    new_ucmd!()
        .arg("-n")
        .arg("99999999")
        .arg(FOOBAR_TXT)
        .run()
        .stdout_is_fixture(FOOBAR_TXT);
}

#[test]
fn test_null_default() {
    new_ucmd!()
        .arg("-z")
        .arg(FOOBAR_WITH_NULL_TXT)
        .run()
        .stdout_is_fixture("foobar_with_null_default.expected");
}

#[test]
#[cfg(not(target_os = "windows"))] // FIXME: test times out
fn test_follow_single() {
    let (at, mut ucmd) = at_and_ucmd!();

    let mut child = ucmd.arg("-f").arg(FOOBAR_TXT).run_no_wait();

    let expected_fixture = "foobar_single_default.expected";

    child
        .make_assertion_with_delay(200)
        .is_alive()
        .with_current_output()
        .stdout_only_fixture(expected_fixture);

    // We write in a temporary copy of foobar.txt
    let expected = "line1\nline2\n";
    at.append(FOOBAR_TXT, expected);

    child
        .make_assertion_with_delay(DEFAULT_SLEEP_INTERVAL_MILLIS)
        .is_alive();
    child
        .kill()
        .make_assertion()
        .with_current_output()
        .stdout_only(expected);
}

/// Test for following when bytes are written that are not valid UTF-8.
#[test]
#[cfg(not(target_os = "windows"))] // FIXME: test times out
fn test_follow_non_utf8_bytes() {
    // Tail the test file and start following it.
    let (at, mut ucmd) = at_and_ucmd!();
    let mut child = ucmd.arg("-f").arg(FOOBAR_TXT).run_no_wait();

    child
        .make_assertion_with_delay(500)
        .is_alive()
        .with_current_output()
        .stdout_only_fixture("foobar_single_default.expected");

    // Now append some bytes that are not valid UTF-8.
    //
    // We also write the newline character because our implementation
    // of `tail` is attempting to read a line of input, so the
    // presence of a newline character will force the `follow()`
    // function to conclude reading input bytes and start writing them
    // to output. The newline character is not fundamental to this
    // test, it is just a requirement of the current implementation.
    let expected = [INVALID_UTF8, b'\n'];
    at.append_bytes(FOOBAR_TXT, &expected);

    child
        .make_assertion_with_delay(DEFAULT_SLEEP_INTERVAL_MILLIS)
        .with_current_output()
        .stdout_only_bytes(expected);

    child.make_assertion().is_alive();
    child.kill();
}

#[test]
#[cfg(not(target_os = "windows"))] // FIXME: test times out
fn test_follow_multiple() {
    let (at, mut ucmd) = at_and_ucmd!();
    let mut child = ucmd
        .arg("-f")
        .arg(FOOBAR_TXT)
        .arg(FOOBAR_2_TXT)
        .run_no_wait();

    child
        .make_assertion_with_delay(500)
        .is_alive()
        .with_current_output()
        .stdout_only_fixture("foobar_follow_multiple.expected");

    let first_append = "trois\n";
    at.append(FOOBAR_2_TXT, first_append);

    child
        .make_assertion_with_delay(DEFAULT_SLEEP_INTERVAL_MILLIS)
        .with_current_output()
        .stdout_only(first_append);

    let second_append = "twenty\nthirty\n";
    at.append(FOOBAR_TXT, second_append);

    child
        .make_assertion_with_delay(DEFAULT_SLEEP_INTERVAL_MILLIS)
        .with_current_output()
        .stdout_only_fixture("foobar_follow_multiple_appended.expected");

    child.make_assertion().is_alive();
    child.kill();
}

#[test]
#[cfg(not(target_os = "windows"))] // FIXME: test times out
fn test_follow_name_multiple() {
    // spell-checker:disable-next-line
    for argument in ["--follow=name", "--follo=nam", "--f=n"] {
        let (at, mut ucmd) = at_and_ucmd!();
        let mut child = ucmd
            .arg(argument)
            .arg(FOOBAR_TXT)
            .arg(FOOBAR_2_TXT)
            .run_no_wait();

        child
            .make_assertion_with_delay(500)
            .is_alive()
            .with_current_output()
            .stdout_only_fixture("foobar_follow_multiple.expected");

        let first_append = "trois\n";
        at.append(FOOBAR_2_TXT, first_append);

        child
            .make_assertion_with_delay(DEFAULT_SLEEP_INTERVAL_MILLIS)
            .with_current_output()
            .stdout_only(first_append);

        let second_append = "twenty\nthirty\n";
        at.append(FOOBAR_TXT, second_append);

        child
            .make_assertion_with_delay(DEFAULT_SLEEP_INTERVAL_MILLIS)
            .with_current_output()
            .stdout_only_fixture("foobar_follow_multiple_appended.expected");

        child.make_assertion().is_alive();
        child.kill();
    }
}

#[test]
fn test_follow_multiple_untailable() {
    // $ tail -f DIR1 DIR2
    // ==> DIR1 <==
    // tail: error reading 'DIR1': Is a directory
    // tail: DIR1: cannot follow end of this type of file; giving up on this name
    //
    // ==> DIR2 <==
    // tail: error reading 'DIR2': Is a directory
    // tail: DIR2: cannot follow end of this type of file; giving up on this name
    // tail: no files remaining

    let expected_stdout = "==> DIR1 <==\n\n==> DIR2 <==\n";
    let expected_stderr = "tail: error reading 'DIR1': Is a directory\n\
        tail: DIR1: cannot follow end of this type of file; giving up on this name\n\
        tail: error reading 'DIR2': Is a directory\n\
        tail: DIR2: cannot follow end of this type of file; giving up on this name\n\
        tail: no files remaining\n";

    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("DIR1");
    at.mkdir("DIR2");
    ucmd.arg("-f")
        .arg("DIR1")
        .arg("DIR2")
        .fails()
        .stderr_is(expected_stderr)
        .stdout_is(expected_stdout)
        .code_is(1);
}

#[test]
fn test_follow_stdin_pipe() {
    new_ucmd!()
        .arg("-f")
        .pipe_in_fixture(FOOBAR_TXT)
        .run()
        .stdout_is_fixture("follow_stdin.expected")
        .no_stderr();
}

#[test]
#[cfg(not(target_os = "windows"))] // FIXME: for currently not working platforms
fn test_follow_invalid_pid() {
    new_ucmd!()
        .args(&["-f", "--pid=-1234"])
        .fails()
        .no_stdout()
        .stderr_is("tail: invalid PID: '-1234'\n");
    new_ucmd!()
        .args(&["-f", "--pid=abc"])
        .fails()
        .no_stdout()
        .stderr_is("tail: invalid PID: 'abc': invalid digit found in string\n");
    let max_pid = (i32::MAX as i64 + 1).to_string();
    new_ucmd!()
        .args(&["-f", "--pid", &max_pid])
        .fails()
        .no_stdout()
        .stderr_is(format!(
            "tail: invalid PID: '{max_pid}': number too large to fit in target type\n"
        ));
}

// FixME: test PASSES for usual windows builds, but fails for coverage testing builds (likely related to the specific RUSTFLAGS '-Zpanic_abort_tests -Cpanic=abort')  This test also breaks tty settings under bash requiring a 'stty sane' or reset. // spell-checker:disable-line
// FIXME: FreeBSD: See issue https://github.com/uutils/coreutils/issues/4306
//        Fails intermittently in the CI, but couldn't reproduce the failure locally.
#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "android"),
    not(target_os = "freebsd")
))] // FIXME: for currently not working platforms
fn test_follow_with_pid() {
    use std::process::Command;

    let (at, mut ucmd) = at_and_ucmd!();

    #[cfg(unix)]
    let dummy_cmd = "sh";

    #[cfg(windows)]
    let dummy_cmd = "cmd";

    let mut dummy = Command::new(dummy_cmd).spawn().unwrap();
    let pid = dummy.id();

    let mut child = ucmd
        .arg("-f")
        .arg(format!("--pid={pid}"))
        .arg(FOOBAR_TXT)
        .arg(FOOBAR_2_TXT)
        .run_no_wait();

    child
        .make_assertion_with_delay(500)
        .is_alive()
        .with_current_output()
        .stdout_only_fixture("foobar_follow_multiple.expected");

    let first_append = "trois\n";
    at.append(FOOBAR_2_TXT, first_append);

    child
        .make_assertion_with_delay(DEFAULT_SLEEP_INTERVAL_MILLIS)
        .with_current_output()
        .stdout_only(first_append);

    let second_append = "twenty\nthirty\n";
    at.append(FOOBAR_TXT, second_append);

    child
        .make_assertion_with_delay(DEFAULT_SLEEP_INTERVAL_MILLIS)
        .is_alive()
        .with_current_output()
        .stdout_only_fixture("foobar_follow_multiple_appended.expected");

    // kill the dummy process and give tail time to notice this
    dummy.kill().unwrap();
    let _ = dummy.wait();

    child.delay(DEFAULT_SLEEP_INTERVAL_MILLIS);

    let third_append = "should\nbe\nignored\n";
    at.append(FOOBAR_TXT, third_append);

    child
        .make_assertion_with_delay(DEFAULT_SLEEP_INTERVAL_MILLIS)
        .is_not_alive()
        .with_current_output()
        .no_stderr()
        .no_stdout()
        .success();
}

#[test]
fn test_single_big_args() {
    const FILE: &str = "single_big_args.txt";
    const EXPECTED_FILE: &str = "single_big_args_expected.txt";
    const LINES: usize = 1_000_000;
    const N_ARG: usize = 100_000;

    let (at, mut ucmd) = at_and_ucmd!();

    let mut big_input = at.make_file(FILE);
    for i in 0..LINES {
        writeln!(big_input, "Line {i}").expect("Could not write to FILE");
    }
    big_input.flush().expect("Could not flush FILE");

    let mut big_expected = at.make_file(EXPECTED_FILE);
    for i in (LINES - N_ARG)..LINES {
        writeln!(big_expected, "Line {i}").expect("Could not write to EXPECTED_FILE");
    }
    big_expected.flush().expect("Could not flush EXPECTED_FILE");

    ucmd.arg(FILE).arg("-n").arg(format!("{N_ARG}")).run();
    // .stdout_is(at.read(EXPECTED_FILE));
}

#[test]
fn test_bytes_single() {
    new_ucmd!()
        .arg("-c")
        .arg("10")
        .arg(FOOBAR_TXT)
        .run()
        .stdout_is_fixture("foobar_bytes_single.expected");
}

#[test]
fn test_bytes_stdin() {
    new_ucmd!()
        .pipe_in_fixture(FOOBAR_TXT)
        .arg("-c")
        .arg("13")
        .run()
        .stdout_is_fixture("foobar_bytes_stdin.expected")
        .no_stderr();
}

#[test]
fn test_bytes_big() {
    const FILE: &str = "test_bytes_big.txt";
    const EXPECTED_FILE: &str = "test_bytes_big_expected.txt";
    const BYTES: usize = 1_000_000;
    const N_ARG: usize = 100_000;

    let (at, mut ucmd) = at_and_ucmd!();

    let mut big_input = at.make_file(FILE);
    for i in 0..BYTES {
        let digit = from_digit((i % 10) as u32, 10).unwrap();
        write!(big_input, "{digit}").expect("Could not write to FILE");
    }
    big_input.flush().expect("Could not flush FILE");

    let mut big_expected = at.make_file(EXPECTED_FILE);
    for i in (BYTES - N_ARG)..BYTES {
        let digit = from_digit((i % 10) as u32, 10).unwrap();
        write!(big_expected, "{digit}").expect("Could not write to EXPECTED_FILE");
    }
    big_expected.flush().expect("Could not flush EXPECTED_FILE");

    let result = ucmd
        .arg(FILE)
        .arg("-c")
        .arg(format!("{N_ARG}"))
        .succeeds()
        .stdout_move_str();
    let expected = at.read(EXPECTED_FILE);

    assert_eq!(result.len(), expected.len());
    for (actual_char, expected_char) in result.chars().zip(expected.chars()) {
        assert_eq!(actual_char, expected_char);
    }
}

#[test]
fn test_lines_with_size_suffix() {
    const FILE: &str = "test_lines_with_size_suffix.txt";
    const EXPECTED_FILE: &str = "test_lines_with_size_suffix_expected.txt";
    const LINES: usize = 3_000;
    const N_ARG: usize = 2 * 1024;

    let (at, mut ucmd) = at_and_ucmd!();

    let mut big_input = at.make_file(FILE);
    for i in 0..LINES {
        writeln!(big_input, "Line {i}").expect("Could not write to FILE");
    }
    big_input.flush().expect("Could not flush FILE");

    let mut big_expected = at.make_file(EXPECTED_FILE);
    for i in (LINES - N_ARG)..LINES {
        writeln!(big_expected, "Line {i}").expect("Could not write to EXPECTED_FILE");
    }
    big_expected.flush().expect("Could not flush EXPECTED_FILE");

    ucmd.arg(FILE)
        .arg("-n")
        .arg("2K")
        .run()
        .stdout_is_fixture(EXPECTED_FILE);
}

#[test]
fn test_multiple_input_files() {
    new_ucmd!()
        .arg(FOOBAR_TXT)
        .arg(FOOBAR_2_TXT)
        .run()
        .no_stderr()
        .stdout_is_fixture("foobar_follow_multiple.expected");
}

#[test]
fn test_multiple_input_files_missing() {
    new_ucmd!()
        .arg(FOOBAR_TXT)
        .arg("missing1")
        .arg(FOOBAR_2_TXT)
        .arg("missing2")
        .run()
        .stdout_is_fixture("foobar_follow_multiple.expected")
        .stderr_is(
            "tail: cannot open 'missing1' for reading: No such file or directory\n\
                tail: cannot open 'missing2' for reading: No such file or directory\n",
        )
        .code_is(1);
}

#[test]
fn test_follow_missing() {
    // Ensure that --follow=name does not imply --retry.
    // Ensure that --follow={descriptor,name} (without --retry) does *not wait* for the
    // file to appear.
    for follow_mode in &["--follow=descriptor", "--follow=name", "--fo=d", "--fo=n"] {
        new_ucmd!()
            .arg(follow_mode)
            .arg("missing")
            .run()
            .no_stdout()
            .stderr_is(
                "tail: cannot open 'missing' for reading: No such file or directory\n\
                    tail: no files remaining\n",
            )
            .code_is(1);
    }
}

#[test]
fn test_follow_name_stdin() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("FILE1");
    at.touch("FILE2");
    ts.ucmd()
        .arg("--follow=name")
        .arg("-")
        .run()
        .stderr_is("tail: cannot follow '-' by name\n")
        .code_is(1);
    ts.ucmd()
        .arg("--follow=name")
        .arg("FILE1")
        .arg("-")
        .arg("FILE2")
        .run()
        .stderr_is("tail: cannot follow '-' by name\n")
        .code_is(1);
}

#[test]
fn test_multiple_input_files_with_suppressed_headers() {
    new_ucmd!()
        .arg(FOOBAR_TXT)
        .arg(FOOBAR_2_TXT)
        .arg("-q")
        .run()
        .stdout_is_fixture("foobar_multiple_quiet.expected");
}

#[test]
fn test_multiple_input_quiet_flag_overrides_verbose_flag_for_suppressing_headers() {
    new_ucmd!()
        .arg(FOOBAR_TXT)
        .arg(FOOBAR_2_TXT)
        .arg("-v")
        .arg("-q")
        .run()
        .stdout_is_fixture("foobar_multiple_quiet.expected");
}

#[test]
fn test_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("DIR");
    ucmd.arg("DIR")
        .run()
        .stderr_is("tail: error reading 'DIR': Is a directory\n")
        .code_is(1);
}

#[test]
fn test_dir_follow() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkdir("DIR");
    for mode in &["--follow=descriptor", "--follow=name"] {
        ts.ucmd()
            .arg(mode)
            .arg("DIR")
            .run()
            .no_stdout()
            .stderr_is(
                "tail: error reading 'DIR': Is a directory\n\
                    tail: DIR: cannot follow end of this type of file; giving up on this name\n\
                    tail: no files remaining\n",
            )
            .code_is(1);
    }
}

#[test]
fn test_dir_follow_retry() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkdir("DIR");
    ts.ucmd()
        .arg("--follow=descriptor")
        .arg("--retry")
        .arg("DIR")
        .run()
        .stderr_is(
            "tail: warning: --retry only effective for the initial open\n\
                tail: error reading 'DIR': Is a directory\n\
                tail: DIR: cannot follow end of this type of file\n\
                tail: no files remaining\n",
        )
        .code_is(1);
}

#[test]
fn test_negative_indexing() {
    let positive_lines_index = new_ucmd!().arg("-n").arg("5").arg(FOOBAR_TXT).run();

    let negative_lines_index = new_ucmd!().arg("-n").arg("-5").arg(FOOBAR_TXT).run();

    let positive_bytes_index = new_ucmd!().arg("-c").arg("20").arg(FOOBAR_TXT).run();

    let negative_bytes_index = new_ucmd!().arg("-c").arg("-20").arg(FOOBAR_TXT).run();

    assert_eq!(positive_lines_index.stdout(), negative_lines_index.stdout());
    assert_eq!(positive_bytes_index.stdout(), negative_bytes_index.stdout());
}

#[test]
fn test_sleep_interval() {
    new_ucmd!().arg("-s").arg("10").arg(FOOBAR_TXT).succeeds();
    new_ucmd!().arg("-s").arg(".1").arg(FOOBAR_TXT).succeeds();
    new_ucmd!().arg("-s.1").arg(FOOBAR_TXT).succeeds();
    new_ucmd!().arg("-s").arg("-1").arg(FOOBAR_TXT).fails();
    new_ucmd!()
        .arg("-s")
        .arg("1..1")
        .arg(FOOBAR_TXT)
        .fails()
        .stderr_contains("invalid number of seconds: '1..1'")
        .code_is(1);
}

/// Test for reading all but the first NUM bytes: `tail -c +3`.
#[test]
fn test_positive_bytes() {
    new_ucmd!()
        .args(&["-c", "+3"])
        .pipe_in("abcde")
        .succeeds()
        .stdout_is("cde");
}

/// Test for reading all bytes, specified by `tail -c +0`.
#[test]
fn test_positive_zero_bytes() {
    let ts = TestScenario::new(util_name!());
    ts.ucmd()
        .args(&["-c", "+0"])
        .pipe_in("abcde")
        .succeeds()
        .stdout_is("abcde");
    ts.ucmd()
        .args(&["-c", "0"])
        .pipe_in("abcde")
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();
}

/// Test for reading all but the first NUM lines: `tail -n +3`.
#[test]
fn test_positive_lines() {
    new_ucmd!()
        .args(&["-n", "+3"])
        .pipe_in("a\nb\nc\nd\ne\n")
        .succeeds()
        .stdout_is("c\nd\ne\n");
}

/// Test for reading all but the first NUM lines of a file: `tail -n +3 infile`.
#[test]
fn test_positive_lines_file() {
    new_ucmd!()
        .args(&["-n", "+7", "foobar.txt"])
        .succeeds()
        .stdout_is(
            "siette
ocho
nueve
diez
once
",
        );
}

/// Test for reading all but the first NUM bytes of a file: `tail -c +3 infile`.
#[test]
fn test_positive_bytes_file() {
    new_ucmd!()
        .args(&["-c", "+42", "foobar.txt"])
        .succeeds()
        .stdout_is(
            "ho
nueve
diez
once
",
        );
}

/// Test for reading all but the first NUM lines: `tail -3`.
#[test]
fn test_obsolete_syntax_positive_lines() {
    new_ucmd!()
        .args(&["-3"])
        .pipe_in("a\nb\nc\nd\ne\n")
        .succeeds()
        .stdout_is("c\nd\ne\n");
}

/// Test for reading all but the first NUM lines: `tail -n -10`.
#[test]
fn test_small_file() {
    new_ucmd!()
        .args(&["-n -10"])
        .pipe_in("a\nb\nc\nd\ne\n")
        .succeeds()
        .stdout_is("a\nb\nc\nd\ne\n");
}

/// Test for reading all but the first NUM lines: `tail -10`.
#[test]
fn test_obsolete_syntax_small_file() {
    new_ucmd!()
        .args(&["-10"])
        .pipe_in("a\nb\nc\nd\ne\n")
        .succeeds()
        .stdout_is("a\nb\nc\nd\ne\n");
}

/// Test for reading all lines, specified by `tail -n +0`.
#[test]
fn test_positive_zero_lines() {
    let ts = TestScenario::new(util_name!());
    ts.ucmd()
        .args(&["-n", "+0"])
        .pipe_in("a\nb\nc\nd\ne\n")
        .succeeds()
        .stdout_is("a\nb\nc\nd\ne\n");
    ts.ucmd()
        .args(&["-n", "0"])
        .pipe_in("a\nb\nc\nd\ne\n")
        .ignore_stdin_write_error()
        .succeeds()
        .no_stderr()
        .no_stdout();
}

#[test]
fn test_invalid_num() {
    new_ucmd!()
        .args(&["-c", "1024R", "emptyfile.txt"])
        .fails()
        .stderr_str()
        .starts_with("tail: invalid number of bytes: '1024R'");
    new_ucmd!()
        .args(&["-n", "1024R", "emptyfile.txt"])
        .fails()
        .stderr_str()
        .starts_with("tail: invalid number of lines: '1024R'");
    new_ucmd!()
        .args(&["-c", "1Y", "emptyfile.txt"])
        .fails()
        .stderr_str()
        .starts_with("tail: invalid number of bytes: '1Y': Value too large for defined data type");
    new_ucmd!()
        .args(&["-n", "1Y", "emptyfile.txt"])
        .fails()
        .stderr_str()
        .starts_with("tail: invalid number of lines: '1Y': Value too large for defined data type");
    new_ucmd!()
        .args(&["-c", "-³"])
        .fails()
        .stderr_str()
        .starts_with("tail: invalid number of bytes: '³'");
}

#[test]
fn test_num_with_undocumented_sign_bytes() {
    // tail: '-' is not documented (8.32 man pages)
    // head: '+' is not documented (8.32 man pages)
    const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyz";
    new_ucmd!()
        .args(&["-c", "5"])
        .pipe_in(ALPHABET)
        .succeeds()
        .stdout_is("vwxyz");
    new_ucmd!()
        .args(&["-c", "-5"])
        .pipe_in(ALPHABET)
        .succeeds()
        .stdout_is("vwxyz");
    new_ucmd!()
        .args(&["-c", "+5"])
        .pipe_in(ALPHABET)
        .succeeds()
        .stdout_is("efghijklmnopqrstuvwxyz");
}

#[test]
#[cfg(unix)]
fn test_bytes_for_funny_unix_files() {
    // inspired by: gnu/tests/tail-2/tail-c.sh
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    for file in ["/proc/version", "/sys/kernel/profiling"] {
        if !at.file_exists(file) {
            continue;
        }
        let args = ["--bytes", "1", file];
        let result = ts.ucmd().args(&args).run();
        let exp_result = unwrap_or_return!(expected_result(&ts, &args));
        result
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str())
            .code_is(exp_result.code());
    }
}

#[test]
fn test_retry1() {
    // inspired by: gnu/tests/tail-2/retry.sh
    // Ensure --retry without --follow results in a warning.

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let file_name = "FILE";
    at.touch(file_name);

    let result = ts.ucmd().arg(file_name).arg("--retry").run();
    result
        .stderr_is("tail: warning: --retry ignored; --retry is useful only when following\n")
        .code_is(0);
}

#[test]
fn test_retry2() {
    // inspired by: gnu/tests/tail-2/retry.sh
    // The same as test_retry2 with a missing file: expect error message and exit 1.

    let ts = TestScenario::new(util_name!());
    let missing = "missing";

    let result = ts.ucmd().arg(missing).arg("--retry").run();
    result
        .stderr_is(
            "tail: warning: --retry ignored; --retry is useful only when following\n\
                tail: cannot open 'missing' for reading: No such file or directory\n",
        )
        .code_is(1);
}

#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))] // FIXME: for currently not working platforms
fn test_retry3() {
    // inspired by: gnu/tests/tail-2/retry.sh
    // Ensure that `tail --retry --follow=name` waits for the file to appear.

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let missing = "missing";

    let expected_stderr = "tail: cannot open 'missing' for reading: No such file or directory\n\
        tail: 'missing' has appeared;  following new file\n";
    let expected_stdout = "X\n";

    let mut delay = 1500;
    let mut args = vec!["--follow=name", "--retry", missing, "--use-polling"];
    for _ in 0..2 {
        let mut p = ts.ucmd().args(&args).run_no_wait();

        p.make_assertion_with_delay(delay).is_alive();

        at.touch(missing);
        p.delay(delay);

        at.truncate(missing, "X\n");
        p.delay(delay);

        p.kill()
            .make_assertion()
            .with_all_output()
            .stderr_is(expected_stderr)
            .stdout_is(expected_stdout);

        at.remove(missing);
        args.pop();
        delay /= 3;
    }
}

#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))] // FIXME: for currently not working platforms
fn test_retry4() {
    // inspired by: gnu/tests/tail-2/retry.sh
    // Ensure that `tail --retry --follow=descriptor` waits for the file to appear.
    // Ensure truncation is detected.

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let missing = "missing";

    let expected_stderr = "tail: warning: --retry only effective for the initial open\n\
        tail: cannot open 'missing' for reading: No such file or directory\n\
        tail: 'missing' has appeared;  following new file\n\
        tail: missing: file truncated\n";
    let expected_stdout = "X1\nX\n";
    let mut args = vec![
        "-s.1",
        "--max-unchanged-stats=1",
        "--follow=descriptor",
        "--retry",
        missing,
        "---disable-inotify",
    ];
    let mut delay = 1500;
    for _ in 0..2 {
        let mut p = ts.ucmd().args(&args).run_no_wait();

        p.make_assertion_with_delay(delay).is_alive();

        at.touch(missing);
        p.delay(delay);

        at.truncate(missing, "X1\n");
        p.delay(delay);

        at.truncate(missing, "X\n");
        p.delay(delay);

        p.make_assertion().is_alive();
        p.kill()
            .make_assertion()
            .with_all_output()
            .stderr_is(expected_stderr)
            .stdout_is(expected_stdout);

        at.remove(missing);
        args.pop();
        delay /= 3;
    }
}

#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))] // FIXME: for currently not working platforms
fn test_retry5() {
    // inspired by: gnu/tests/tail-2/retry.sh
    // Ensure that `tail --follow=descriptor --retry` exits when the file appears untailable.

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let missing = "missing";

    let expected_stderr = "tail: warning: --retry only effective for the initial open\n\
        tail: cannot open 'missing' for reading: No such file or directory\n\
        tail: 'missing' has been replaced with an untailable file; giving up on this name\n\
        tail: no files remaining\n";

    let mut delay = 1500;
    let mut args = vec!["--follow=descriptor", "--retry", missing, "--use-polling"];
    for _ in 0..2 {
        let mut p = ts.ucmd().args(&args).run_no_wait();

        p.make_assertion_with_delay(delay).is_alive();

        at.mkdir(missing);
        p.delay(delay);

        p.make_assertion()
            .is_not_alive()
            .with_all_output()
            .stderr_only(expected_stderr)
            .failure();

        at.rmdir(missing);
        args.pop();
        delay /= 3;
    }
}

// intermittent failures on android with diff
// Diff < left / right > :
// ==> existing <==
// >X
#[test]
#[cfg(all(not(target_os = "windows"), not(target_os = "android")))] // FIXME: for currently not working platforms
fn test_retry6() {
    // inspired by: gnu/tests/tail-2/retry.sh
    // Ensure that --follow=descriptor (without --retry) does *not* try
    // to open a file after an initial fail, even when there are other tailable files.

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let missing = "missing";
    let existing = "existing";
    at.touch(existing);

    let expected_stderr = "tail: cannot open 'missing' for reading: No such file or directory\n";
    let expected_stdout = "==> existing <==\nX\n";

    let mut p = ts
        .ucmd()
        .arg("--follow=descriptor")
        .arg("missing")
        .arg("existing")
        .run_no_wait();

    let delay = 1000;
    p.make_assertion_with_delay(delay).is_alive();

    at.truncate(missing, "Y\n");
    p.delay(delay);

    at.truncate(existing, "X\n");
    p.delay(delay);

    p.make_assertion().is_alive();
    p.kill()
        .make_assertion()
        .with_all_output()
        .stdout_is(expected_stdout)
        .stderr_is(expected_stderr);
}

#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))] // FIXME: for currently not working platforms
fn test_retry7() {
    // inspired by: gnu/tests/tail-2/retry.sh
    // Ensure that `tail -F` retries when the file is initially untailable.

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let untailable = "untailable";

    let expected_stderr = "tail: error reading 'untailable': Is a directory\n\
        tail: untailable: cannot follow end of this type of file\n\
        tail: 'untailable' has become accessible\n\
        tail: 'untailable' has become inaccessible: No such file or directory\n\
        tail: 'untailable' has been replaced with an untailable file\n\
        tail: 'untailable' has become accessible\n";
    let expected_stdout = "foo\nbar\n";

    let mut args = vec![
        "-s.1",
        "--max-unchanged-stats=1",
        "-F",
        untailable,
        "--use-polling",
    ];

    let mut delay = 1500;
    for _ in 0..2 {
        at.mkdir(untailable);

        let mut p = ts.ucmd().args(&args).run_no_wait();

        p.make_assertion_with_delay(delay).is_alive();

        // tail: 'untailable' has become accessible
        // or (The first is the common case, "has appeared" arises with slow rmdir):
        // tail: 'untailable' has appeared;  following new file
        at.rmdir(untailable);
        at.truncate(untailable, "foo\n");
        p.delay(delay);

        // NOTE: GNU's `tail` only shows "become inaccessible"
        // if there's a delay between rm and mkdir.
        // tail: 'untailable' has become inaccessible: No such file or directory
        at.remove(untailable);
        p.delay(delay);

        // tail: 'untailable' has been replaced with an untailable file\n";
        at.mkdir(untailable);
        p.delay(delay);

        // full circle, back to the beginning
        at.rmdir(untailable);
        at.truncate(untailable, "bar\n");
        p.delay(delay);

        p.make_assertion().is_alive();
        p.kill()
            .make_assertion()
            .with_all_output()
            .stderr_is(expected_stderr)
            .stdout_is(expected_stdout);

        args.pop();
        at.remove(untailable);
        delay /= 3;
    }
}

#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))] // FIXME: for currently not working platforms
fn test_retry8() {
    // Ensure that inotify will switch to polling mode if directory
    // of the watched file was initially missing and later created.
    // This is similar to test_retry9, but without:
    // tail: directory containing watched file was removed\n\
    // tail: inotify cannot be used, reverting to polling\n\

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let watched_file = Path::new("watched_file");
    let parent_dir = Path::new("parent_dir");
    let user_path = parent_dir.join(watched_file);
    let parent_dir = parent_dir.to_str().unwrap();
    let user_path = user_path.to_str().unwrap();

    let expected_stderr = "\
        tail: cannot open 'parent_dir/watched_file' for reading: No such file or directory\n\
        tail: 'parent_dir/watched_file' has appeared;  following new file\n\
        tail: 'parent_dir/watched_file' has become inaccessible: No such file or directory\n\
        tail: 'parent_dir/watched_file' has appeared;  following new file\n";
    let expected_stdout = "foo\nbar\n";

    let delay = 1000;

    let mut p = ts
        .ucmd()
        .arg("-F")
        .arg("-s.1")
        .arg("--max-unchanged-stats=1")
        .arg(user_path)
        .run_no_wait();

    p.make_assertion_with_delay(delay).is_alive();

    // 'parent_dir/watched_file' is orphan
    // tail: cannot open 'parent_dir/watched_file' for reading: No such file or directory\n\

    // tail: 'parent_dir/watched_file' has appeared;  following new file\n\
    at.mkdir(parent_dir); // not an orphan anymore
    at.append(user_path, "foo\n");
    p.delay(delay);

    // tail: 'parent_dir/watched_file' has become inaccessible: No such file or directory\n\
    at.remove(user_path);
    at.rmdir(parent_dir); // 'parent_dir/watched_file' is orphan *again*
    p.delay(delay);

    // Since 'parent_dir/watched_file' is orphan, this needs to be picked up by polling
    // tail: 'parent_dir/watched_file' has appeared;  following new file\n";
    at.mkdir(parent_dir); // not an orphan anymore
    at.append(user_path, "bar\n");
    p.delay(delay);

    p.make_assertion().is_alive();
    p.kill()
        .make_assertion()
        .with_all_output()
        .stderr_is(expected_stderr)
        .stdout_is(expected_stdout);
}

#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "android"),
    not(target_os = "windows"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))] // FIXME: for currently not working platforms
fn test_retry9() {
    // inspired by: gnu/tests/tail-2/inotify-dir-recreate.sh
    // Ensure that inotify will switch to polling mode if directory
    // of the watched file was removed and recreated.

    use text::BACKEND;

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let watched_file = Path::new("watched_file");
    let parent_dir = Path::new("parent_dir");
    let user_path = parent_dir.join(watched_file);
    let parent_dir = parent_dir.to_str().unwrap();
    let user_path = user_path.to_str().unwrap();

    let expected_stderr = format!(
        "\
            tail: 'parent_dir/watched_file' has become inaccessible: No such file or directory\n\
            tail: directory containing watched file was removed\n\
            tail: {BACKEND} cannot be used, reverting to polling\n\
            tail: 'parent_dir/watched_file' has appeared;  following new file\n\
            tail: 'parent_dir/watched_file' has become inaccessible: No such file or directory\n\
            tail: 'parent_dir/watched_file' has appeared;  following new file\n\
            tail: 'parent_dir/watched_file' has become inaccessible: No such file or directory\n\
            tail: 'parent_dir/watched_file' has appeared;  following new file\n"
    );
    let expected_stdout = "foo\nbar\nfoo\nbar\n";

    let delay = 1000;

    at.mkdir(parent_dir);
    at.truncate(user_path, "foo\n");
    let mut p = ts
        .ucmd()
        .arg("-F")
        .arg("-s.1")
        .arg("--max-unchanged-stats=1")
        .arg(user_path)
        .run_no_wait();

    p.make_assertion_with_delay(delay).is_alive();

    at.remove(user_path);
    at.rmdir(parent_dir);
    p.delay(delay);

    at.mkdir(parent_dir);
    at.truncate(user_path, "bar\n");
    p.delay(delay);

    at.remove(user_path);
    at.rmdir(parent_dir);
    p.delay(delay);

    at.mkdir(parent_dir);
    at.truncate(user_path, "foo\n");
    p.delay(delay);

    at.remove(user_path);
    at.rmdir(parent_dir);
    p.delay(delay);

    at.mkdir(parent_dir);
    at.truncate(user_path, "bar\n");
    p.delay(delay);

    p.make_assertion().is_alive();
    p.kill()
        .make_assertion()
        .with_all_output()
        .stderr_is(expected_stderr)
        .stdout_is(expected_stdout);
}

#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "android"),
    not(target_os = "windows"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))] // FIXME: for currently not working platforms
fn test_follow_descriptor_vs_rename1() {
    // inspired by: gnu/tests/tail-2/descriptor-vs-rename.sh
    // $ ((rm -f A && touch A && sleep 1 && echo -n "A\n" >> A && sleep 1 && \
    // mv A B && sleep 1 && echo -n "B\n" >> B &)>/dev/null 2>&1 &) ; \
    // sleep 1 && target/debug/tail --follow=descriptor A ---disable-inotify
    // $ A
    // $ B

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let file_a = "FILE_A";
    let file_b = "FILE_B";
    let file_c = "FILE_C";

    let mut args = vec![
        "--follow=descriptor",
        "-s.1",
        "--max-unchanged-stats=1",
        file_a,
        "---disable-inotify",
    ];

    let mut delay = 1500;
    for _ in 0..2 {
        at.touch(file_a);

        let mut p = ts.ucmd().args(&args).run_no_wait();

        p.make_assertion_with_delay(delay).is_alive();

        at.append(file_a, "A\n");
        p.delay(delay);

        at.rename(file_a, file_b);
        p.delay(delay);

        at.append(file_b, "B\n");
        p.delay(delay);

        at.rename(file_b, file_c);
        p.delay(delay);

        at.append(file_c, "C\n");
        p.delay(delay);

        p.make_assertion().is_alive();
        p.kill()
            .make_assertion()
            .with_all_output()
            .stdout_only("A\nB\nC\n");

        args.pop();
        delay /= 3;
    }
}

#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "android"),
    not(target_os = "windows"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))] // FIXME: for currently not working platforms
fn test_follow_descriptor_vs_rename2() {
    // Ensure the headers are correct for --verbose.
    // NOTE: GNU's tail does not update the header from FILE_A to FILE_C after `mv FILE_A FILE_C`

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let file_a = "FILE_A";
    let file_b = "FILE_B";
    let file_c = "FILE_C";

    let mut args = vec![
        "--follow=descriptor",
        "-s.1",
        "--max-unchanged-stats=1",
        file_a,
        file_b,
        "--verbose",
        "---disable-inotify",
    ];

    let mut delay = 1500;
    for _ in 0..2 {
        at.touch(file_a);
        at.touch(file_b);
        let mut p = ts.ucmd().args(&args).run_no_wait();

        p.make_assertion_with_delay(delay).is_alive();

        at.rename(file_a, file_c);
        p.delay(delay);

        at.append(file_c, "X\n");
        p.delay(delay);

        p.make_assertion().is_alive();
        p.kill()
            .make_assertion()
            .with_all_output()
            .stdout_only("==> FILE_A <==\n\n==> FILE_B <==\n\n==> FILE_A <==\nX\n");

        args.pop();
        delay /= 3;
    }
}

#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))] // FIXME: for currently not working platforms
fn test_follow_name_retry_headers() {
    // inspired by: "gnu/tests/tail-2/F-headers.sh"
    // Ensure tail -F distinguishes output with the
    // correct headers for created/renamed files

    /*
    $ tail --follow=descriptor -s.1 --max-unchanged-stats=1 -F a b
    tail: cannot open 'a' for reading: No such file or directory
    tail: cannot open 'b' for reading: No such file or directory
    tail: 'a' has appeared;  following new file
    ==> a <==
    x
    tail: 'b' has appeared;  following new file

    ==> b <==
    y
    */

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let file_a = "a";
    let file_b = "b";

    let mut args = vec![
        "-F",
        "-s.1",
        "--max-unchanged-stats=1",
        file_a,
        file_b,
        "---disable-inotify",
    ];

    let mut delay = 1500;
    for _ in 0..2 {
        let mut p = ts.ucmd().args(&args).run_no_wait();

        p.make_assertion_with_delay(delay).is_alive();

        at.truncate(file_a, "x\n");
        p.delay(delay);

        at.truncate(file_b, "y\n");
        p.delay(delay);

        let expected_stderr = "tail: cannot open 'a' for reading: No such file or directory\n\
                tail: cannot open 'b' for reading: No such file or directory\n\
                tail: 'a' has appeared;  following new file\n\
                tail: 'b' has appeared;  following new file\n";
        let expected_stdout = "\n==> a <==\nx\n\n==> b <==\ny\n";

        p.make_assertion().is_alive();
        p.kill()
            .make_assertion()
            .with_all_output()
            .stdout_is(expected_stdout)
            .stderr_is(expected_stderr);

        at.remove(file_a);
        at.remove(file_b);
        args.pop();
        delay /= 3;
    }
}

#[test]
#[cfg(all(not(target_os = "windows"), not(target_os = "android")))] // FIXME: for currently not working platforms
fn test_follow_name_remove() {
    // This test triggers a remove event while `tail --follow=name file` is running.
    // ((sleep 2 && rm file &)>/dev/null 2>&1 &) ; tail --follow=name file

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let source = FOLLOW_NAME_TXT;
    let source_copy = "source_copy";
    at.copy(source, source_copy);

    let expected_stdout = at.read(FOLLOW_NAME_SHORT_EXP);
    let expected_stderr = [
        format!(
            "{}: {}: No such file or directory\n{0}: no files remaining\n",
            ts.util_name, source_copy
        ),
        format!(
            "{}: {}: No such file or directory\n",
            ts.util_name, source_copy
        ),
    ];

    let mut args = vec!["--follow=name", source_copy, "--use-polling"];

    let mut delay = 1500;
    #[allow(clippy::needless_range_loop)]
    for i in 0..2 {
        at.copy(source, source_copy);

        let mut p = ts.ucmd().args(&args).run_no_wait();

        p.make_assertion_with_delay(delay).is_alive();

        at.remove(source_copy);
        p.delay(delay);

        if i == 0 {
            p.make_assertion()
                .is_not_alive()
                .with_all_output()
                .stdout_is(&expected_stdout)
                .stderr_is(&expected_stderr[i])
                .failure();
        } else {
            p.make_assertion().is_alive();
            p.kill()
                .make_assertion()
                .with_all_output()
                .stdout_is(&expected_stdout)
                .stderr_is(&expected_stderr[i]);
        }

        args.pop();
        delay /= 3;
    }
}

#[test]
#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "android"),
    not(target_os = "freebsd")
))] // FIXME: for currently not working platforms
fn test_follow_name_truncate1() {
    // This test triggers a truncate event while `tail --follow=name file` is running.
    // $ cp file backup && head file > file && sleep 1 && cp backup file

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let source = FOLLOW_NAME_TXT;
    let backup = "backup";

    let expected_stdout = at.read(FOLLOW_NAME_EXP);
    let expected_stderr = format!("{}: {}: file truncated\n", ts.util_name, source);

    let args = ["--follow=name", source];
    let mut p = ts.ucmd().args(&args).run_no_wait();
    let delay = 1000;
    p.make_assertion().is_alive();

    at.copy(source, backup);
    p.delay(delay);

    at.touch(source); // trigger truncate
    p.delay(delay);

    at.copy(backup, source);
    p.delay(delay);

    p.make_assertion().is_alive();
    p.kill()
        .make_assertion()
        .with_all_output()
        .stderr_is(expected_stderr)
        .stdout_is(expected_stdout);
}

#[test]
#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "android"),
    not(target_os = "freebsd")
))] // FIXME: for currently not working platforms
fn test_follow_name_truncate2() {
    // This test triggers a truncate event while `tail --follow=name file` is running.
    // $ ((sleep 1 && echo -n "x\nx\nx\n" >> file && sleep 1 && \
    // echo -n "x\n" > file &)>/dev/null 2>&1 &) ; tail --follow=name file

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let source = "file";
    at.touch(source);

    let expected_stdout = "x\nx\nx\nx\n";
    let expected_stderr = format!("{}: {}: file truncated\n", ts.util_name, source);

    let args = ["--follow=name", source];
    let mut p = ts.ucmd().args(&args).run_no_wait();

    let delay = 1000;
    p.make_assertion().is_alive();

    at.append(source, "x\n");
    p.delay(delay);

    at.append(source, "x\n");
    p.delay(delay);

    at.append(source, "x\n");
    p.delay(delay);

    at.truncate(source, "x\n");
    p.delay(delay);

    p.make_assertion().is_alive();

    p.kill()
        .make_assertion()
        .with_all_output()
        .stderr_is(expected_stderr)
        .stdout_is(expected_stdout);
}

#[test]
#[cfg(not(target_os = "windows"))] // FIXME: for currently not working platforms
fn test_follow_name_truncate3() {
    // Opening an empty file in truncate mode should not trigger a truncate event while
    // `tail --follow=name file` is running.
    // $ rm -f file && touch file
    // $ ((sleep 1 && echo -n "x\n" > file &)>/dev/null 2>&1 &) ; tail --follow=name file

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let source = "file";
    at.touch(source);

    let expected_stdout = "x\n";

    let args = ["--follow=name", source];
    let mut p = ts.ucmd().args(&args).run_no_wait();

    let delay = 1000;
    p.make_assertion_with_delay(delay).is_alive();

    at.truncate(source, "x\n");
    p.delay(delay);

    p.make_assertion().is_alive();
    p.kill()
        .make_assertion()
        .with_all_output()
        .stdout_only(expected_stdout);
}

#[test]
#[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))] // FIXME: for currently not working platforms
fn test_follow_name_truncate4() {
    // Truncating a file with the same content it already has should not trigger a truncate event

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let mut args = vec!["-s.1", "--max-unchanged-stats=1", "-F", "file"];

    let mut delay = 500;
    for i in 0..2 {
        at.append("file", "foobar\n");

        let mut p = ts.ucmd().args(&args).run_no_wait();

        p.make_assertion_with_delay(delay).is_alive();

        at.truncate("file", "foobar\n");
        p.delay(delay);

        p.make_assertion().is_alive();
        p.kill()
            .make_assertion()
            .with_all_output()
            .stdout_only("foobar\n");

        at.remove("file");
        if i == 0 {
            args.push("---disable-inotify");
        }
        delay *= 3;
    }
}

#[test]
#[cfg(not(target_os = "windows"))] // FIXME: for currently not working platforms
fn test_follow_truncate_fast() {
    // inspired by: "gnu/tests/tail-2/truncate.sh"
    // Ensure all logs are output upon file truncation

    // This is similar to `test_follow_name_truncate1-3` but uses very short delays
    // to better mimic the tight timings used in the "truncate.sh" test.
    // This is here to test for "speed" only, all the logic is already covered by other tests.

    if is_ci() {
        println!("TEST SKIPPED (too fast for CI)");
        return;
    }

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let mut args = vec!["-s.1", "--max-unchanged-stats=1", "f", "---disable-inotify"];
    let follow = vec!["-f", "-F"];

    let mut delay = 1000;
    for _ in 0..2 {
        for mode in &follow {
            args.push(mode);

            at.truncate("f", "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");

            let mut p = ts.ucmd().args(&args).run_no_wait();
            p.make_assertion_with_delay(delay).is_alive();

            at.truncate("f", "11\n12\n13\n14\n15\n");
            p.delay(delay);

            p.make_assertion().is_alive();
            p.kill()
                .make_assertion()
                .with_all_output()
                .stderr_is("tail: f: file truncated\n")
                .stdout_is("1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12\n13\n14\n15\n");

            args.pop();
        }
        args.pop();
        delay = 250;
    }
}

#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))] // FIXME: for currently not working platforms
fn test_follow_name_move_create1() {
    // This test triggers a move/create event while `tail --follow=name file` is running.
    // ((sleep 2 && mv file backup && sleep 2 && cp backup file &)>/dev/null 2>&1 &) ; tail --follow=name file

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let source = FOLLOW_NAME_TXT;
    let backup = "backup";

    #[cfg(target_os = "linux")]
    let expected_stdout = at.read(FOLLOW_NAME_EXP);

    #[cfg(target_os = "linux")]
    let expected_stderr = format!(
        "{}: {}: No such file or directory\n{0}: '{1}' has appeared;  following new file\n",
        ts.util_name, source
    );

    // NOTE: We are less strict if not on Linux (inotify backend).

    #[cfg(not(target_os = "linux"))]
    let expected_stdout = at.read(FOLLOW_NAME_SHORT_EXP);

    #[cfg(not(target_os = "linux"))]
    let expected_stderr = format!("{}: {}: No such file or directory\n", ts.util_name, source);

    let delay = 500;
    let args = ["--follow=name", source];

    let mut p = ts.ucmd().args(&args).run_no_wait();

    p.make_assertion_with_delay(delay).is_alive();

    at.rename(source, backup);
    p.delay(delay);

    at.copy(backup, source);
    p.delay(delay);

    p.make_assertion().is_alive();
    p.kill()
        .make_assertion()
        .with_all_output()
        .stderr_is(expected_stderr)
        .stdout_is(expected_stdout);
}

#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "android"),
    not(target_os = "windows"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))] // FIXME: for currently not working platforms
fn test_follow_name_move_create2() {
    // inspired by: "gnu/tests/tail-2/inotify-hash-abuse.sh"
    // Exercise an abort-inducing flaw in inotify-enabled tail -F

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    for n in ["1", "2", "3", "4", "5", "6", "7", "8", "9"] {
        at.touch(n);
    }

    let mut args = vec![
        "-s.1",
        "--max-unchanged-stats=1",
        "-q",
        "-F",
        "1",
        "2",
        "3",
        "4",
        "5",
        "6",
        "7",
        "8",
        "9",
    ];

    let mut delay = 500;
    for i in 0..2 {
        let mut p = ts.ucmd().args(&args).run_no_wait();

        p.make_assertion_with_delay(delay).is_alive();

        at.truncate("9", "x\n");
        p.delay(delay);

        at.rename("1", "f");
        p.delay(delay);

        at.truncate("1", "a\n");
        p.delay(delay);

        // NOTE: Because "gnu/tests/tail-2/inotify-hash-abuse.sh" 'forgets' to clear the files used
        // during the first loop iteration, we also don't clear them to get the same side-effects.
        // Side-effects are truncating a file with the same content, see: test_follow_name_truncate4
        // at.remove("1");
        // at.touch("1");
        // at.remove("9");
        // at.touch("9");
        let expected_stdout = if args.len() == 14 {
            "a\nx\na\n"
        } else {
            "x\na\n"
        };
        let expected_stderr = "tail: '1' has become inaccessible: No such file or directory\n\
                tail: '1' has appeared;  following new file\n";

        p.make_assertion().is_alive();
        p.kill()
            .make_assertion()
            .with_all_output()
            .stderr_is(expected_stderr)
            .stdout_is(expected_stdout);

        at.remove("f");
        if i == 0 {
            args.push("---disable-inotify");
        }
        delay *= 3;
    }
}

#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))] // FIXME: for currently not working platforms
fn test_follow_name_move1() {
    // This test triggers a move event while `tail --follow=name file` is running.
    // ((sleep 2 && mv file backup &)>/dev/null 2>&1 &) ; tail --follow=name file
    // NOTE: For `---disable-inotify` tail exits with "no file remaining", it stays open w/o it.

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let source = FOLLOW_NAME_TXT;
    let backup = "backup";

    let expected_stdout = at.read(FOLLOW_NAME_SHORT_EXP);
    let expected_stderr = [
        format!("{}: {}: No such file or directory\n", ts.util_name, source),
        format!(
            "{}: {}: No such file or directory\n{0}: no files remaining\n",
            ts.util_name, source
        ),
    ];

    let mut args = vec!["--follow=name", source];

    let mut delay = 500;
    #[allow(clippy::needless_range_loop)]
    for i in 0..2 {
        let mut p = ts.ucmd().args(&args).run_no_wait();

        p.make_assertion_with_delay(delay).is_alive();

        at.rename(source, backup);
        p.delay(delay);

        if i == 0 {
            p.make_assertion().is_alive();
            p.kill()
                .make_assertion()
                .with_all_output()
                .stderr_is(&expected_stderr[i])
                .stdout_is(&expected_stdout);
        } else {
            p.make_assertion()
                .is_not_alive()
                .with_all_output()
                .stderr_is(&expected_stderr[i])
                .stdout_is(&expected_stdout)
                .failure();
        }

        at.rename(backup, source);
        args.push("--use-polling");
        delay *= 3;
    }
}

#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))] // FIXME: for currently not working platforms
fn test_follow_name_move2() {
    // Like test_follow_name_move1, but move to a name that's already monitored.

    // $ echo file1_content > file1; echo file2_content > file2; \
    // ((sleep 2 ; mv file1 file2 ; sleep 1 ; echo "more_file2_content" >> file2 ; sleep 1 ; \
    // echo "more_file1_content" >> file1 &)>/dev/null 2>&1 &) ; \
    // tail --follow=name file1 file2
    // ==> file1 <==
    // file1_content
    //
    // ==> file2 <==
    // file2_content
    // tail: file1: No such file or directory
    // tail: 'file2' has been replaced;  following new file
    // file1_content
    // more_file2_content
    // tail: 'file1' has appeared;  following new file
    //
    // ==> file1 <==
    // more_file1_content

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let file1 = "file1";
    let file2 = "file2";

    let expected_stdout = format!(
        "==> {file1} <==\n{file1}_content\n\n==> {file2} <==\n{file2}_content\n{file1}_content\n\
            more_{file2}_content\n\n==> {file1} <==\nmore_{file1}_content\n"
    );
    let mut expected_stderr = format!(
        "{0}: {1}: No such file or directory\n\
            {0}: '{2}' has been replaced;  following new file\n\
            {0}: '{1}' has appeared;  following new file\n",
        ts.util_name, file1, file2
    );

    let mut args = vec!["--follow=name", file1, file2];

    let mut delay = 500;
    for i in 0..2 {
        at.truncate(file1, "file1_content\n");
        at.truncate(file2, "file2_content\n");

        let mut p = ts.ucmd().args(&args).run_no_wait();
        p.make_assertion_with_delay(delay).is_alive();

        at.rename(file1, file2);
        p.delay(delay);

        at.append(file2, "more_file2_content\n");
        p.delay(delay);

        at.append(file1, "more_file1_content\n");
        p.delay(delay);

        p.make_assertion().is_alive();
        p.kill()
            .make_assertion()
            .with_all_output()
            .stderr_is(&expected_stderr)
            .stdout_is(&expected_stdout);

        if i == 0 {
            args.push("--use-polling");
        }
        delay *= 3;
        // NOTE: Switch the first and second line because the events come in this order from
        //  `notify::PollWatcher`. However, for GNU's tail, the order between polling and not
        //  polling does not change.
        expected_stderr = format!(
            "{0}: '{2}' has been replaced;  following new file\n\
                {0}: {1}: No such file or directory\n\
                {0}: '{1}' has appeared;  following new file\n",
            ts.util_name, file1, file2
        );
    }
}

#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))] // FIXME: for currently not working platforms
fn test_follow_name_move_retry1() {
    // Similar to test_follow_name_move1 but with `--retry` (`-F`)
    // This test triggers two move/rename events while `tail --follow=name --retry file` is running.

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let source = FOLLOW_NAME_TXT;
    let backup = "backup";

    let expected_stderr = format!(
        "{0}: '{1}' has become inaccessible: No such file or directory\n\
            {0}: '{1}' has appeared;  following new file\n",
        ts.util_name, source
    );
    let expected_stdout = "tailed\nnew content\n";

    let mut args = vec!["--follow=name", "--retry", source, "--use-polling"];

    let mut delay = 1500;
    for _ in 0..2 {
        at.touch(source);
        let mut p = ts.ucmd().args(&args).run_no_wait();

        p.make_assertion_with_delay(delay).is_alive();

        at.append(source, "tailed\n");
        p.delay(delay);

        // with --follow=name, tail should stop monitoring the renamed file
        at.rename(source, backup);
        p.delay(delay);
        // overwrite backup while it's not monitored
        at.truncate(backup, "new content\n");
        p.delay(delay);
        // move back, tail should pick this up and print new content
        at.rename(backup, source);
        p.delay(delay);

        p.make_assertion().is_alive();
        p.kill()
            .make_assertion()
            .with_all_output()
            .stderr_is(&expected_stderr)
            .stdout_is(expected_stdout);

        at.remove(source);
        args.pop();
        delay /= 3;
    }
}

#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))] // FIXME: for currently not working platforms
fn test_follow_name_move_retry2() {
    // inspired by: "gnu/tests/tail-2/F-vs-rename.sh"
    // Similar to test_follow_name_move2 (move to a name that's already monitored)
    // but with `--retry` (`-F`)

    /*
    $ touch a b
    $ ((sleep 1; echo x > a; mv a b; echo x2 > a; echo y >> b; echo z >> a  &)>/dev/null 2>&1 &) ; tail -F a b
    ==> a <==

    ==> b <==

    ==> a <==
    x
    tail: 'a' has become inaccessible: No such file or directory
    tail: 'b' has been replaced;  following new file

    ==> b <==
    x
    tail: 'a' has appeared;  following new file

    ==> a <==
    x2

    ==> b <==
    y

    ==> a <==
    z
    */

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let file1 = "a";
    let file2 = "b";

    let expected_stdout = format!(
        "==> {file1} <==\n\n==> {file2} <==\n\n==> {file1} <==\nx\n\n==> {file2} <==\
            \nx\n\n==> {file1} <==\nx2\n\n==> {file2} <==\ny\n\n==> {file1} <==\nz\n"
    );
    let mut expected_stderr = format!(
        "{0}: '{1}' has become inaccessible: No such file or directory\n\
            {0}: '{2}' has been replaced;  following new file\n\
            {0}: '{1}' has appeared;  following new file\n",
        ts.util_name, file1, file2
    );

    let mut args = vec!["-s.1", "--max-unchanged-stats=1", "-F", file1, file2];

    let mut delay = 500;
    for i in 0..2 {
        at.touch(file1);
        at.touch(file2);

        let mut p = ts.ucmd().args(&args).run_no_wait();
        p.make_assertion_with_delay(delay).is_alive();

        at.truncate(file1, "x\n");
        p.delay(delay);

        at.rename(file1, file2);
        p.delay(delay);

        at.truncate(file1, "x2\n");
        p.delay(delay);

        at.append(file2, "y\n");
        p.delay(delay);

        at.append(file1, "z\n");
        p.delay(delay);

        p.make_assertion().is_alive();
        p.kill()
            .make_assertion()
            .with_all_output()
            .stderr_is(&expected_stderr)
            .stdout_is(&expected_stdout);

        at.remove(file1);
        at.remove(file2);
        if i == 0 {
            args.push("--use-polling");
        }
        delay *= 3;
        // NOTE: Switch the first and second line because the events come in this order from
        //  `notify::PollWatcher`. However, for GNU's tail, the order between polling and not
        //  polling does not change.
        expected_stderr = format!(
            "{0}: '{2}' has been replaced;  following new file\n\
                {0}: '{1}' has become inaccessible: No such file or directory\n\
                {0}: '{1}' has appeared;  following new file\n",
            ts.util_name, file1, file2
        );
    }
}

#[test]
#[cfg(not(target_os = "windows"))] // FIXME: for currently not working platforms
fn test_follow_inotify_only_regular() {
    // The GNU test inotify-only-regular.sh uses strace to ensure that `tail -f`
    // doesn't make inotify syscalls and only uses inotify for regular files or fifos.
    // We just check if tailing a character device has the same behavior as GNU's tail.

    let ts = TestScenario::new(util_name!());

    let mut p = ts.ucmd().arg("-f").arg("/dev/null").run_no_wait();

    p.make_assertion_with_delay(200).is_alive();
    p.kill()
        .make_assertion()
        .with_all_output()
        .no_stderr()
        .no_stdout();
}

#[test]
fn test_no_such_file() {
    new_ucmd!()
        .arg("missing")
        .fails()
        .stderr_is("tail: cannot open 'missing' for reading: No such file or directory\n")
        .no_stdout()
        .code_is(1);
}

#[test]
fn test_no_trailing_newline() {
    new_ucmd!().pipe_in("x").succeeds().stdout_only("x");
}

#[test]
fn test_lines_zero_terminated() {
    new_ucmd!()
        .args(&["-z", "-n", "2"])
        .pipe_in("a\0b\0c\0d\0e\0")
        .succeeds()
        .stdout_only("d\0e\0");
    new_ucmd!()
        .args(&["-z", "-n", "+2"])
        .pipe_in("a\0b\0c\0d\0e\0")
        .succeeds()
        .stdout_only("b\0c\0d\0e\0");
}

#[test]
fn test_presume_input_pipe_default() {
    new_ucmd!()
        .arg("---presume-input-pipe")
        .pipe_in_fixture(FOOBAR_TXT)
        .run()
        .stdout_is_fixture("foobar_stdin_default.expected")
        .no_stderr();
}

#[test]
#[cfg(not(windows))]
fn test_fifo() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkfifo("FIFO");

    let mut p = ts.ucmd().arg("FIFO").run_no_wait();
    p.make_assertion_with_delay(500).is_alive();
    p.kill()
        .make_assertion()
        .with_all_output()
        .no_stderr()
        .no_stdout();

    for arg in ["-f", "-F"] {
        let mut p = ts.ucmd().arg(arg).arg("FIFO").run_no_wait();
        p.make_assertion_with_delay(500).is_alive();
        p.kill()
            .make_assertion()
            .with_all_output()
            .no_stderr()
            .no_stdout();
    }
}

#[test]
#[cfg(unix)]
#[ignore = "disabled until fixed"]
fn test_illegal_seek() {
    // This is here for reference only.
    // We don't call seek on fifos, so we don't hit this error case.
    // (Also see: https://github.com/coreutils/coreutils/pull/36)

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.append("FILE", "foo\n");
    at.mkfifo("FIFO");

    let mut p = ts.ucmd().arg("FILE").run_no_wait();
    p.make_assertion_with_delay(500).is_alive();

    at.rename("FILE", "FIFO");
    p.delay(500);

    p.make_assertion().is_alive();
    let expected_stderr = "tail: 'FILE' has been replaced;  following new file\n\
                                 tail: FILE: cannot seek to offset 0: Illegal seek\n";
    p.kill()
        .make_assertion()
        .with_all_output()
        .stderr_is(expected_stderr)
        .stdout_is("foo\n")
        .code_is(1); // is this correct? after kill the code is not meaningful.
}

#[test]
fn test_pipe_when_lines_option_value_is_higher_than_contained_lines() {
    let test_string = "a\nb\n";
    new_ucmd!()
        .args(&["-n", "3"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(test_string);

    new_ucmd!()
        .args(&["-n", "4"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(test_string);

    new_ucmd!()
        .args(&["-n", "999"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(test_string);

    new_ucmd!()
        .args(&["-n", "+3"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();

    new_ucmd!()
        .args(&["-n", "+4"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();

    new_ucmd!()
        .args(&["-n", "+999"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();
}

#[test]
fn test_pipe_when_negative_lines_option_given_no_newline_at_eof() {
    let test_string = "a\nb";

    new_ucmd!()
        .args(&["-n", "0"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();

    new_ucmd!()
        .args(&["-n", "1"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("b");

    new_ucmd!()
        .args(&["-n", "2"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("a\nb");
}

#[test]
fn test_pipe_when_positive_lines_option_given_no_newline_at_eof() {
    let test_string = "a\nb";

    new_ucmd!()
        .args(&["-n", "+0"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("a\nb");

    new_ucmd!()
        .args(&["-n", "+1"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("a\nb");

    new_ucmd!()
        .args(&["-n", "+2"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("b");
}

#[test]
fn test_pipe_when_lines_option_given_multibyte_utf8_characters() {
    // the test string consists of from left to right a 4-byte,3-byte,2-byte,1-byte utf-8 character
    let test_string = "𝅘𝅥𝅮\n⏻\nƒ\na";

    new_ucmd!()
        .args(&["-n", "+0"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(test_string);

    new_ucmd!()
        .args(&["-n", "+2"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("⏻\nƒ\na");

    new_ucmd!()
        .args(&["-n", "+3"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("ƒ\na");

    new_ucmd!()
        .args(&["-n", "+4"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("a");

    new_ucmd!()
        .args(&["-n", "+5"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();

    new_ucmd!()
        .args(&["-n", "-4"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(test_string);

    new_ucmd!()
        .args(&["-n", "-3"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("⏻\nƒ\na");

    new_ucmd!()
        .args(&["-n", "-2"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("ƒ\na");

    new_ucmd!()
        .args(&["-n", "-1"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("a");

    new_ucmd!()
        .args(&["-n", "-0"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();
}

#[test]
fn test_pipe_when_lines_option_given_input_size_is_equal_to_buffer_size_no_newline_at_eof() {
    let total_lines = 1;
    let random_string = RandomizedString::generate_with_delimiter(
        Alphanumeric,
        b'\n',
        total_lines,
        false,
        CHUNK_BUFFER_SIZE,
    );
    let random_string = random_string.as_str();
    let lines = random_string.split_inclusive('\n');

    let expected = lines.clone().skip(1).collect::<String>();
    new_ucmd!()
        .args(&["-n", "+2"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(expected);

    let expected = lines.clone().skip(1).collect::<String>();
    new_ucmd!()
        .args(&["-n", "-1"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(expected);
}

#[test]
fn test_pipe_when_lines_option_given_input_size_is_equal_to_buffer_size() {
    let total_lines = 100;
    let random_string = RandomizedString::generate_with_delimiter(
        Alphanumeric,
        b'\n',
        total_lines,
        true,
        CHUNK_BUFFER_SIZE,
    );
    let random_string = random_string.as_str();
    let lines = random_string.split_inclusive('\n');

    new_ucmd!()
        .args(&["-n", "+0"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(random_string);

    let expected = lines.clone().skip(1).collect::<String>();
    new_ucmd!()
        .args(&["-n", "+2"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(expected);

    new_ucmd!()
        .args(&["-n", "-0"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();

    let expected = lines.clone().skip(total_lines - 1).collect::<String>();
    new_ucmd!()
        .args(&["-n", "-1"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(expected);

    let expected = lines.clone().skip(1).collect::<String>();
    new_ucmd!()
        .args(&["-n", "-99"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(expected);

    new_ucmd!()
        .args(&["-n", "-100"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(random_string);
}

#[test]
fn test_pipe_when_lines_option_given_input_size_is_one_byte_greater_than_buffer_size() {
    let total_lines = 100;
    let random_string = RandomizedString::generate_with_delimiter(
        Alphanumeric,
        b'\n',
        total_lines,
        true,
        CHUNK_BUFFER_SIZE + 1,
    );
    let random_string = random_string.as_str();
    let lines = random_string.split_inclusive('\n');

    new_ucmd!()
        .args(&["-n", "+0"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(random_string);

    let expected = lines.clone().skip(total_lines - 1).collect::<String>();
    new_ucmd!()
        .args(&["-n", "-1"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(expected);

    let expected = lines.clone().skip(1).collect::<String>();
    new_ucmd!()
        .args(&["-n", "+2"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(expected);

    let expected = lines.clone().skip(1).collect::<String>();
    new_ucmd!()
        .args(&["-n", "-99"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(expected);
}

// FIXME: windows: this test failed with timeout in the CI. Running this test in
// a Windows VirtualBox image produces no errors.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_pipe_when_lines_option_given_input_size_has_multiple_size_of_buffer_size() {
    let total_lines = 100;
    let random_string = RandomizedString::generate_with_delimiter(
        Alphanumeric,
        b'\n',
        total_lines,
        true,
        CHUNK_BUFFER_SIZE * 3 + 1,
    );
    let random_string = random_string.as_str();
    let lines = random_string.split_inclusive('\n');

    new_ucmd!()
        .args(&["-n", "+0"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(random_string);

    let expected = lines.clone().skip(1).collect::<String>();
    new_ucmd!()
        .args(&["-n", "+2"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(expected);

    new_ucmd!()
        .args(&["-n", "-0"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();

    let expected = lines.clone().skip(total_lines - 1).collect::<String>();
    new_ucmd!()
        .args(&["-n", "-1"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(expected);

    let expected = lines.clone().skip(1).collect::<String>();
    new_ucmd!()
        .args(&["-n", "-99"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(expected);

    new_ucmd!()
        .args(&["-n", "-100"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(random_string);
}

#[test]
fn test_pipe_when_bytes_option_value_is_higher_than_contained_bytes() {
    let test_string = "a\nb";
    new_ucmd!()
        .args(&["-c", "4"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(test_string);

    new_ucmd!()
        .args(&["-c", "5"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(test_string);

    new_ucmd!()
        .args(&["-c", "999"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(test_string);

    new_ucmd!()
        .args(&["-c", "+4"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();

    new_ucmd!()
        .args(&["-c", "+5"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();

    new_ucmd!()
        .args(&["-c", "+999"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();
}

#[test]
fn test_pipe_when_bytes_option_given_multibyte_utf8_characters() {
    // the test string consists of from left to right a 4-byte,3-byte,2-byte,1-byte utf-8 character
    let test_string = "𝅘𝅥𝅮⏻ƒa";

    new_ucmd!()
        .args(&["-c", "+0"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(test_string);

    new_ucmd!()
        .args(&["-c", "+2"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(&test_string.as_bytes()[1..]);

    new_ucmd!()
        .args(&["-c", "+5"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("⏻ƒa");

    new_ucmd!()
        .args(&["-c", "+8"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("ƒa");

    new_ucmd!()
        .args(&["-c", "+10"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("a");

    new_ucmd!()
        .args(&["-c", "+11"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();

    new_ucmd!()
        .args(&["-c", "-1"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("a");

    new_ucmd!()
        .args(&["-c", "-2"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(&"ƒa".as_bytes()[1..]);

    new_ucmd!()
        .args(&["-c", "-3"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("ƒa");

    new_ucmd!()
        .args(&["-c", "-6"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only("⏻ƒa");

    new_ucmd!()
        .args(&["-c", "-10"])
        .pipe_in(test_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(test_string);
}

#[test]
fn test_pipe_when_bytes_option_given_input_size_is_equal_to_buffer_size() {
    let random_string = RandomizedString::generate(AlphanumericNewline, CHUNK_BUFFER_SIZE);
    let random_string = random_string.as_str();

    new_ucmd!()
        .args(&["-c", "+0"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(random_string);

    let expected = &random_string.as_bytes()[1..];
    new_ucmd!()
        .args(&["-c", "+2"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(expected);

    new_ucmd!()
        .args(&["-c", "-0"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();

    let expected = &random_string.as_bytes()[1..];
    new_ucmd!()
        .args(&["-c", "-8191"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(expected);

    new_ucmd!()
        .args(&["-c", "-8192"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(random_string);

    new_ucmd!()
        .args(&["-c", "-8193"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(random_string);

    let expected = &random_string.as_bytes()[CHUNK_BUFFER_SIZE - 1..];
    new_ucmd!()
        .args(&["-c", "-1"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(expected);
}

#[test]
fn test_pipe_when_bytes_option_given_input_size_is_one_byte_greater_than_buffer_size() {
    let random_string = RandomizedString::generate(AlphanumericNewline, CHUNK_BUFFER_SIZE + 1);
    let random_string = random_string.as_str();

    new_ucmd!()
        .args(&["-c", "+0"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(random_string);

    let expected = &random_string.as_bytes()[1..];
    new_ucmd!()
        .args(&["-c", "+2"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(expected);

    new_ucmd!()
        .args(&["-c", "-0"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();

    let expected = &random_string.as_bytes()[CHUNK_BUFFER_SIZE..];
    new_ucmd!()
        .args(&["-c", "-1"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(expected);

    let expected = &random_string.as_bytes()[1..];
    new_ucmd!()
        .args(&["-c", "-8192"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(expected);

    new_ucmd!()
        .args(&["-c", "-8193"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(random_string);
}

// FIXME: windows: this test failed with timeout in the CI. Running this test in
// a Windows VirtualBox image produces no errors.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_pipe_when_bytes_option_given_input_size_has_multiple_size_of_buffer_size() {
    let random_string = RandomizedString::generate(AlphanumericNewline, CHUNK_BUFFER_SIZE * 3);
    let random_string = random_string.as_str();

    new_ucmd!()
        .args(&["-c", "+0"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(random_string);

    new_ucmd!()
        .args(&["-c", "-0"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();

    let expected = &random_string.as_bytes()[8192..];
    new_ucmd!()
        .args(&["-c", "+8193"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(expected);

    let expected = &random_string.as_bytes()[8193..];
    new_ucmd!()
        .args(&["-c", "+8194"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(expected);

    let expected = &random_string.as_bytes()[16384..];
    new_ucmd!()
        .args(&["-c", "+16385"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(expected);

    let expected = &random_string.as_bytes()[16385..];
    new_ucmd!()
        .args(&["-c", "+16386"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(expected);

    let expected = &random_string.as_bytes()[16384..];
    new_ucmd!()
        .args(&["-c", "-8192"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(expected);

    let expected = &random_string.as_bytes()[16383..];
    new_ucmd!()
        .args(&["-c", "-8193"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(expected);

    let expected = &random_string.as_bytes()[8192..];
    new_ucmd!()
        .args(&["-c", "-16384"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(expected);

    let expected = &random_string.as_bytes()[8191..];
    new_ucmd!()
        .args(&["-c", "-16385"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only_bytes(expected);

    new_ucmd!()
        .args(&["-c", "-24576"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(random_string);
}

#[test]
fn test_seek_bytes_backward_outside_file() {
    new_ucmd!()
        .arg("-c")
        .arg("100")
        .arg(FOOBAR_TXT)
        .run()
        .stdout_is_fixture(FOOBAR_TXT);
}

#[test]
fn test_seek_bytes_forward_outside_file() {
    new_ucmd!()
        .arg("-c")
        .arg("+100")
        .arg(FOOBAR_TXT)
        .run()
        .stdout_is("");
}

// Some basic tests for ---presume-input-pipe. These tests build upon the
// debug_assert in bounded tail to detect that we're using the bounded_tail in
// case the option is given on command line.
#[cfg(all(not(target_os = "android"), not(target_os = "windows")))] // FIXME:
#[test]
fn test_args_when_presume_input_pipe_given_input_is_pipe() {
    let random_string = RandomizedString::generate(AlphanumericNewline, 1000);
    let random_string = random_string.as_str();

    new_ucmd!()
        .args(&["---presume-input-pipe", "-c", "-0"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();

    new_ucmd!()
        .args(&["---presume-input-pipe", "-c", "+0"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(random_string);

    new_ucmd!()
        .args(&["---presume-input-pipe", "-n", "-0"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stdout()
        .no_stderr();

    new_ucmd!()
        .args(&["---presume-input-pipe", "-n", "+0"])
        .pipe_in(random_string)
        .ignore_stdin_write_error()
        .succeeds()
        .stdout_only(random_string);
}

#[test]
fn test_args_when_presume_input_pipe_given_input_is_file() {
    let random_string = RandomizedString::generate(AlphanumericNewline, 1000);
    let random_string = random_string.as_str();

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.write("data", random_string);

    ts.ucmd()
        .args(&["---presume-input-pipe", "-c", "-0", "data"])
        .succeeds()
        .no_stdout()
        .no_stderr();

    ts.ucmd()
        .args(&["---presume-input-pipe", "-c", "+0", "data"])
        .succeeds()
        .stdout_only(random_string);

    ts.ucmd()
        .args(&["---presume-input-pipe", "-n", "-0", "data"])
        .succeeds()
        .no_stdout()
        .no_stderr();

    ts.ucmd()
        .args(&["---presume-input-pipe", "-n", "+0", "data"])
        .succeeds()
        .stdout_only(random_string);
}

#[test]
#[ignore = "disabled until fixed"]
// FIXME: currently missing in the error message is the last line >>tail: no files remaining<<
fn test_when_follow_retry_given_redirected_stdin_from_directory_then_correct_error_message() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkdir("dir");

    let expected = "tail: warning: --retry only effective for the initial open\n\
                        tail: error reading 'standard input': Is a directory\n\
                        tail: 'standard input': cannot follow end of this type of file\n\
                        tail: no files remaining\n";
    ts.ucmd()
        .set_stdin(File::open(at.plus("dir")).unwrap())
        .args(&["-f", "--retry"])
        .fails()
        .stderr_only(expected)
        .code_is(1);
}

#[test]
fn test_when_argument_file_is_a_directory() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkdir("dir");

    let expected = "tail: error reading 'dir': Is a directory\n";
    ts.ucmd()
        .arg("dir")
        .fails()
        .stderr_only(expected)
        .code_is(1);
}

// TODO: make this work on windows
#[test]
#[cfg(unix)]
fn test_when_argument_file_is_a_symlink() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let mut file = at.make_file("target");

    at.symlink_file("target", "link");

    ts.ucmd()
        .args(&["-c", "+0", "link"])
        .succeeds()
        .no_stdout()
        .no_stderr();

    let random_string = RandomizedString::generate(AlphanumericNewline, 100);
    let result = file.write_all(random_string.as_bytes());
    assert!(result.is_ok());

    ts.ucmd()
        .args(&["-c", "+0", "link"])
        .succeeds()
        .stdout_only(random_string);

    at.mkdir("dir");

    at.symlink_file("dir", "dir_link");

    let expected = "tail: error reading 'dir_link': Is a directory\n";
    ts.ucmd()
        .arg("dir_link")
        .fails()
        .stderr_only(expected)
        .code_is(1);
}

// TODO: make this work on windows
#[test]
#[cfg(unix)]
fn test_when_argument_file_is_a_symlink_to_directory_then_error() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.mkdir("dir");
    at.symlink_file("dir", "dir_link");

    let expected = "tail: error reading 'dir_link': Is a directory\n";
    ts.ucmd()
        .arg("dir_link")
        .fails()
        .stderr_only(expected)
        .code_is(1);
}

// TODO: make this work on windows
#[test]
#[cfg(unix)]
#[ignore = "disabled until fixed"]
fn test_when_argument_file_is_a_faulty_symlink_then_error() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.symlink_file("self", "self");

    #[cfg(all(not(target_env = "musl"), not(target_os = "android")))]
    let expected = "tail: cannot open 'self' for reading: Too many levels of symbolic links";
    #[cfg(all(not(target_env = "musl"), target_os = "android"))]
    let expected = "tail: cannot open 'self' for reading: Too many symbolic links encountered";
    #[cfg(all(target_env = "musl", not(target_os = "android")))]
    let expected = "tail: cannot open 'self' for reading: Symbolic link loop";

    ts.ucmd()
        .arg("self")
        .fails()
        .stderr_only(expected)
        .code_is(1);

    at.symlink_file("missing", "broken");

    let expected = "tail: cannot open 'broken' for reading: No such file or directory";
    ts.ucmd()
        .arg("broken")
        .fails()
        .stderr_only(expected)
        .code_is(1);
}

#[test]
#[cfg(unix)]
#[ignore = "disabled until fixed"]
fn test_when_argument_file_is_non_existent_unix_socket_address_then_error() {
    use std::os::unix::net;

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let socket = "socket";

    // We only bind to create the socket file but do not listen
    let result = net::UnixListener::bind(at.plus(socket));
    assert!(result.is_ok());

    #[cfg(all(not(target_os = "freebsd"), not(target_os = "macos")))]
    let expected_stderr =
        format!("tail: cannot open '{socket}' for reading: No such device or address\n");
    #[cfg(target_os = "freebsd")]
    let expected_stderr = format!(
        "tail: cannot open '{}' for reading: Operation not supported\n",
        socket
    );
    #[cfg(target_os = "macos")]
    let expected_stderr = format!(
        "tail: cannot open '{}' for reading: Operation not supported on socket\n",
        socket
    );

    ts.ucmd()
        .arg(socket)
        .fails()
        .stderr_only(&expected_stderr)
        .code_is(1);

    let path = "file";
    let mut file = at.make_file(path);

    let random_string = RandomizedString::generate(AlphanumericNewline, 100);
    let result = file.write_all(random_string.as_bytes());
    assert!(result.is_ok());

    let expected_stdout = [format!("==> {path} <=="), random_string].join("\n");
    ts.ucmd()
        .args(&["-c", "+0", path, socket])
        .fails()
        .stdout_is(&expected_stdout)
        .stderr_is(&expected_stderr);

    // tail does not stop processing files when having encountered a "No such
    // device or address" error.
    ts.ucmd()
        .args(&["-c", "+0", socket, path])
        .fails()
        .stdout_is(&expected_stdout)
        .stderr_is(&expected_stderr);
}

#[test]
#[ignore = "disabled until fixed"]
fn test_when_argument_files_are_simple_combinations_of_stdin_and_regular_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("empty");
    at.write("data", "file data");
    at.write("fifo", "fifo data");

    let expected = "==> standard input <==\n\
                fifo data\n\
                ==> empty <==\n";
    scene
        .ucmd()
        .args(&["-c", "+0", "-", "empty"])
        .set_stdin(File::open(at.plus("fifo")).unwrap())
        .run()
        .success()
        .stdout_only(expected);

    let expected = "==> standard input <==\n\
                \n\
                ==> empty <==\n";
    scene
        .ucmd()
        .args(&["-c", "+0", "-", "empty"])
        .pipe_in("")
        .run()
        .success()
        .stdout_only(expected);

    let expected = "==> empty <==\n\
                \n\
                ==> standard input <==\n";
    scene
        .ucmd()
        .args(&["-c", "+0", "empty", "-"])
        .pipe_in("")
        .run()
        .success()
        .stdout_only(expected);

    let expected = "==> empty <==\n\
                \n\
                ==> standard input <==\n\
                fifo data";
    scene
        .ucmd()
        .args(&["-c", "+0", "empty", "-"])
        .set_stdin(File::open(at.plus("fifo")).unwrap())
        .run()
        .success()
        .stdout_only(expected);

    let expected = "==> standard input <==\n\
                pipe data\n\
                ==> data <==\n\
                file data";
    scene
        .ucmd()
        .args(&["-c", "+0", "-", "data"])
        .pipe_in("pipe data")
        .run()
        .success()
        .stdout_only(expected);

    let expected = "==> data <==\n\
                file data\n\
                ==> standard input <==\n\
                pipe data";
    scene
        .ucmd()
        .args(&["-c", "+0", "data", "-"])
        .pipe_in("pipe data")
        .run()
        .success()
        .stdout_only(expected);

    let expected = "==> standard input <==\n\
                pipe data\n\
                ==> standard input <==\n";
    scene
        .ucmd()
        .args(&["-c", "+0", "-", "-"])
        .pipe_in("pipe data")
        .run()
        .success()
        .stdout_only(expected);

    let expected = "==> standard input <==\n\
                fifo data\n\
                ==> standard input <==\n";
    scene
        .ucmd()
        .args(&["-c", "+0", "-", "-"])
        .set_stdin(File::open(at.plus("fifo")).unwrap())
        .run()
        .success()
        .stdout_only(expected);
}

#[test]
#[ignore = "disabled until fixed"]
fn test_when_argument_files_are_triple_combinations_of_fifo_pipe_and_regular_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("empty");
    at.write("data", "file data");
    at.write("fifo", "fifo data");

    let expected = "==> standard input <==\n\
                \n\
                ==> empty <==\n\
                \n\
                ==> standard input <==\n";

    scene
        .ucmd()
        .args(&["-c", "+0", "-", "empty", "-"])
        .set_stdin(File::open(at.plus("empty")).unwrap())
        .run()
        .stdout_only(expected)
        .success();

    let expected = "==> standard input <==\n\
                \n\
                ==> empty <==\n\
                \n\
                ==> standard input <==\n";
    scene
        .ucmd()
        .args(&["-c", "+0", "-", "empty", "-"])
        .pipe_in("")
        .stderr_to_stdout()
        .run()
        .stdout_only(expected)
        .success();

    let expected = "==> standard input <==\n\
                pipe data\n\
                ==> data <==\n\
                file data\n\
                ==> standard input <==\n";
    scene
        .ucmd()
        .args(&["-c", "+0", "-", "data", "-"])
        .pipe_in("pipe data")
        .run()
        .stdout_only(expected)
        .success();

    // Correct behavior in a sh shell is to remember the file pointer for the fifo, so we don't
    // print the fifo twice. This matches the behavior, if only the pipe is present without fifo
    // (See test above). Note that for example a zsh shell prints the pipe data and has therefore
    // different output from the sh shell (or cmd shell on windows).

    // windows: tail returns with success although there is an error message present (on some
    // windows systems). This error message comes from `echo` (the line ending `\r\n` indicates that
    // too) which cannot write to the pipe because tail finished before echo was able to write to
    // the pipe. Seems that windows `cmd` (like posix shells) ignores pipes when a fifo is present.
    // This is actually the wished behavior and the test therefore succeeds.
    #[cfg(windows)]
    let expected = "==> standard input <==\n\
        fifo data\n\
        ==> data <==\n\
        file data\n\
        ==> standard input <==\n\
        (The process tried to write to a nonexistent pipe.\r\n)?";
    #[cfg(unix)]
    let expected = "==> standard input <==\n\
        fifo data\n\
        ==> data <==\n\
        file data\n\
        ==> standard input <==\n";

    #[cfg(windows)]
    let cmd = ["cmd", "/C"];
    #[cfg(unix)]
    let cmd = ["sh", "-c"];

    scene
        .cmd(cmd[0])
        .arg(cmd[1])
        .arg(format!(
            "echo pipe data | {} tail -c +0  - data - < fifo",
            scene.bin_path.display(),
        ))
        .run()
        .stdout_only(expected)
        .success();

    let expected = "==> standard input <==\n\
                fifo data\n\
                ==> data <==\n\
                file data\n\
                ==> standard input <==\n";
    scene
        .ucmd()
        .args(&["-c", "+0", "-", "data", "-"])
        .set_stdin(File::open(at.plus("fifo")).unwrap())
        .run()
        .stdout_only(expected)
        .success();
}

// Bug description: The content of a file is not printed to stdout if the output data does not
// contain newlines and --follow was given as arguments.
//
// This test is only formal on linux, since we currently do not detect this kind of error within the
// test system. However, this behavior shows up on the command line and, at the time of writing this
// description, with this test on macos and windows.
#[test]
#[ignore = "disabled until fixed"]
fn test_when_follow_retry_then_initial_print_of_file_is_written_to_stdout() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let expected_stdout = "file data";
    at.write("data", expected_stdout);

    let mut child = scene
        .ucmd()
        .args(&["--follow=name", "--retry", "data"])
        .run_no_wait();

    child
        .delay(1500)
        .kill()
        .make_assertion()
        .with_current_output()
        .stdout_only(expected_stdout);
}

// TODO: Add test for the warning `--pid=PID is not supported on this system`
#[test]
fn test_args_when_settings_check_warnings_then_shows_warnings() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_data = "file data\n";
    at.write("data", file_data);

    let expected_stdout = format!(
        "tail: warning: --retry ignored; --retry is useful only when following\n\
        {file_data}"
    );
    scene
        .ucmd()
        .args(&["--retry", "data"])
        .stderr_to_stdout()
        .run()
        .stdout_only(expected_stdout)
        .success();

    let expected_stdout = format!(
        "tail: warning: --retry only effective for the initial open\n\
        {file_data}"
    );
    let mut child = scene
        .ucmd()
        .args(&["--follow=descriptor", "--retry", "data"])
        .stderr_to_stdout()
        .run_no_wait();

    child
        .delay(500)
        .kill()
        .make_assertion()
        .with_current_output()
        .stdout_only(expected_stdout);

    let expected_stdout = format!(
        "tail: warning: PID ignored; --pid=PID is useful only when following\n\
        {file_data}"
    );
    scene
        .ucmd()
        .args(&["--pid=1000", "data"])
        .stderr_to_stdout()
        .run()
        .stdout_only(expected_stdout)
        .success();

    let expected_stdout = format!(
        "tail: warning: --retry ignored; --retry is useful only when following\n\
        tail: warning: PID ignored; --pid=PID is useful only when following\n\
        {file_data}"
    );
    scene
        .ucmd()
        .args(&["--pid=1000", "--retry", "data"])
        .stderr_to_stdout()
        .run()
        .stdout_only(&expected_stdout)
        .success();
    scene
        .ucmd()
        .args(&["--pid=1000", "--pid=1000", "--retry", "data"])
        .stderr_to_stdout()
        .run()
        .stdout_only(expected_stdout)
        .success();
}

/// TODO: Write similar tests for windows
#[test]
#[cfg(target_os = "linux")]
fn test_args_when_settings_check_warnings_follow_indefinitely_then_warning() {
    let scene = TestScenario::new(util_name!());

    let file_data = "file data\n";
    scene.fixtures.write("data", file_data);

    let expected_stdout = "==> standard input <==\n";
    let expected_stderr = "tail: warning: following standard input indefinitely is ineffective\n";

    // `tail -f - data` (without any redirect) would also print this warning in a terminal but we're
    // not attached to a `tty` in the ci, so it's not possible to setup a test case for this
    // particular usage. However, setting stdin to a `tty` behaves equivalently and we're faking an
    // attached `tty` that way.

    // testing here that the warning is printed to stderr
    // tail -f - data < /dev/ptmx
    let mut child = scene
        .ucmd()
        .args(&["--follow=descriptor", "-", "data"])
        .set_stdin(File::open(text::DEV_PTMX).unwrap())
        .run_no_wait();

    child.make_assertion_with_delay(500).is_alive();
    child
        .kill()
        .make_assertion()
        .with_current_output()
        .stderr_is(expected_stderr)
        .stdout_is(expected_stdout);

    let expected_stdout = "tail: warning: following standard input indefinitely is ineffective\n\
                                 ==> standard input <==\n";
    // same like above but this time the order of the output matters and we're redirecting stderr to
    // stdout
    // tail -f - data < /dev/ptmx
    let mut child = scene
        .ucmd()
        .args(&["--follow=descriptor", "-", "data"])
        .set_stdin(File::open(text::DEV_PTMX).unwrap())
        .stderr_to_stdout()
        .run_no_wait();

    child.make_assertion_with_delay(500).is_alive();
    child
        .kill()
        .make_assertion()
        .with_current_output()
        .stdout_only(expected_stdout);

    let expected_stdout = format!(
        "tail: warning: following standard input indefinitely is ineffective\n\
        ==> data <==\n\
        {file_data}\n\
        ==> standard input <==\n"
    );
    // tail -f data - < /dev/ptmx
    let mut child = scene
        .ucmd()
        .args(&["--follow=descriptor", "data", "-"])
        .set_stdin(File::open(text::DEV_PTMX).unwrap())
        .stderr_to_stdout()
        .run_no_wait();

    child.make_assertion_with_delay(500).is_alive();
    child
        .kill()
        .make_assertion()
        .with_current_output()
        .stdout_only(expected_stdout);

    let expected_stdout = "tail: warning: following standard input indefinitely is ineffective\n\
                                 ==> standard input <==\n";
    // tail -f - - < /dev/ptmx
    let mut child = scene
        .ucmd()
        .args(&["--follow=descriptor", "-", "-"])
        .set_stdin(File::open(text::DEV_PTMX).unwrap())
        .stderr_to_stdout()
        .run_no_wait();

    child.make_assertion_with_delay(500).is_alive();
    child
        .kill()
        .make_assertion()
        .with_current_output()
        .stdout_only(expected_stdout);

    let expected_stdout = "tail: warning: following standard input indefinitely is ineffective\n\
                                 ==> standard input <==\n";
    // tail -f - - data < /dev/ptmx
    let mut child = scene
        .ucmd()
        .args(&["--follow=descriptor", "-", "-", "data"])
        .set_stdin(File::open(text::DEV_PTMX).unwrap())
        .stderr_to_stdout()
        .run_no_wait();

    child.make_assertion_with_delay(500).is_alive();
    child
        .kill()
        .make_assertion()
        .with_current_output()
        .stdout_only(expected_stdout);

    let expected_stdout = "tail: warning: following standard input indefinitely is ineffective\n\
                                 ==> standard input <==\n";
    // tail --pid=100000 -f - data < /dev/ptmx
    let mut child = scene
        .ucmd()
        .args(&["--follow=descriptor", "--pid=100000", "-", "data"])
        .set_stdin(File::open(text::DEV_PTMX).unwrap())
        .stderr_to_stdout()
        .run_no_wait();

    child.make_assertion_with_delay(500).is_alive();
    child
        .kill()
        .make_assertion()
        .with_current_output()
        .stdout_only(expected_stdout);
}

#[test]
#[cfg(unix)]
fn test_args_when_settings_check_warnings_follow_indefinitely_then_no_warning() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    #[cfg(target_vendor = "apple")]
    let delay = 1000;
    #[cfg(not(target_vendor = "apple"))]
    let delay = 500;

    let file_data = "file data\n";
    let fifo_data = "fifo data\n";
    let fifo_name = "fifo";
    let file_name = "data";
    at.write(file_name, file_data);
    at.write(fifo_name, fifo_data);

    let pipe_data = "pipe data";
    let expected_stdout = format!(
        "==> standard input <==\n\
        {pipe_data}\n\
        ==> {file_name} <==\n\
        {file_data}"
    );
    let mut child = scene
        .ucmd()
        .args(&["--follow=descriptor", "-", file_name])
        .pipe_in(pipe_data)
        .stderr_to_stdout()
        .run_no_wait();

    child.make_assertion_with_delay(delay).is_alive();
    child
        .kill()
        .make_assertion_with_delay(delay)
        .with_current_output()
        .stdout_only(expected_stdout);

    // Test with regular file instead of /dev/tty
    // Fails currently on macos with
    // Diff < left / right > :
    // <tail: cannot open 'standard input' for reading: No such file or directory
    // >==> standard input <==
    // >fifo data
    // >
    //  ==> data <==
    //  file data
    #[cfg(not(target_vendor = "apple"))]
    {
        let expected_stdout = format!(
            "==> standard input <==\n\
        {fifo_data}\n\
        ==> {file_name} <==\n\
        {file_data}"
        );
        let mut child = scene
            .ucmd()
            .args(&["--follow=descriptor", "-", file_name])
            .set_stdin(File::open(at.plus(fifo_name)).unwrap())
            .stderr_to_stdout()
            .run_no_wait();

        child.make_assertion_with_delay(delay).is_alive();
        child
            .kill()
            .make_assertion_with_delay(delay)
            .with_current_output()
            .stdout_only(expected_stdout);

        let expected_stdout = format!(
            "==> standard input <==\n\
        {fifo_data}\n\
        ==> {file_name} <==\n\
        {file_data}"
        );
        let mut child = scene
            .ucmd()
            .args(&["--follow=descriptor", "--pid=0", "-", file_name])
            .set_stdin(File::open(at.plus(fifo_name)).unwrap())
            .stderr_to_stdout()
            .run_no_wait();

        child.make_assertion_with_delay(delay).is_alive();
        child
            .kill()
            .make_assertion_with_delay(delay)
            .with_current_output()
            .stdout_only(expected_stdout);
    }
}

/// The expected test outputs come from gnu's tail.
#[test]
#[ignore = "disabled until fixed"]
fn test_follow_when_files_are_pointing_to_same_relative_file_and_data_is_appended() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_data = "file data";
    let relative_path_name = "data";

    at.write(relative_path_name, file_data);
    let absolute_path = at.plus("data").canonicalize().unwrap();

    // run with relative path first and then the absolute path
    let mut child = scene
        .ucmd()
        .args(&[
            "--follow=name",
            relative_path_name,
            absolute_path.to_str().unwrap(),
        ])
        .run_no_wait();

    let more_data = "more data";
    child.delay(500);

    at.append(relative_path_name, more_data);

    let expected_stdout = format!(
        "==> {0} <==\n\
        {1}\n\
        ==> {2} <==\n\
        {1}\n\
        ==> {0} <==\n\
        {3}",
        relative_path_name,
        file_data,
        absolute_path.to_str().unwrap(),
        more_data
    );

    child
        .make_assertion_with_delay(DEFAULT_SLEEP_INTERVAL_MILLIS)
        .is_alive();
    child
        .kill()
        .make_assertion()
        .with_current_output()
        .stderr_only(expected_stdout);

    // run with absolute path first and then the relative path
    at.write(relative_path_name, file_data);
    let mut child = scene
        .ucmd()
        .args(&[
            "--follow=name",
            absolute_path.to_str().unwrap(),
            relative_path_name,
        ])
        .run_no_wait();

    child.delay(500);
    let more_data = "more data";
    at.append(relative_path_name, more_data);

    let expected_stdout = format!(
        "==> {0} <==\n\
        {1}\n\
        ==> {2} <==\n\
        {1}\n\
        ==> {0} <==\n\
        {3}",
        absolute_path.to_str().unwrap(),
        file_data,
        relative_path_name,
        more_data
    );

    child
        .make_assertion_with_delay(DEFAULT_SLEEP_INTERVAL_MILLIS)
        .is_alive();
    child
        .kill()
        .make_assertion()
        .with_current_output()
        .stdout_only(expected_stdout);
}

/// The expected test outputs come from gnu's tail.
#[test]
#[ignore = "disabled until fixed"]
fn test_follow_when_files_are_pointing_to_same_relative_file_and_file_is_truncated() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_data = "file data";
    let relative_path_name = "data";

    at.write(relative_path_name, file_data);
    let absolute_path = at.plus("data").canonicalize().unwrap();

    let mut child = scene
        .ucmd()
        .args(&[
            "--follow=descriptor",
            "--max-unchanged-stats=1",
            "--sleep-interval=0.1",
            relative_path_name,
            absolute_path.to_str().unwrap(),
        ])
        .stderr_to_stdout()
        .run_no_wait();

    child.delay(500);
    let less_data = "less";
    at.write(relative_path_name, "less");

    let expected_stdout = format!(
        "==> {0} <==\n\
        {1}\n\
        ==> {2} <==\n\
        {1}{4}: {0}: file truncated\n\
        \n\
        ==> {0} <==\n\
        {3}",
        relative_path_name,              // 0
        file_data,                       // 1
        absolute_path.to_str().unwrap(), // 2
        less_data,                       // 3
        scene.util_name                  // 4
    );

    child.make_assertion_with_delay(500).is_alive();
    child
        .kill()
        .make_assertion()
        .with_current_output()
        .stdout_only(expected_stdout);
}

/// The expected test outputs come from gnu's tail.
#[test]
#[cfg(unix)]
#[ignore = "disabled until fixed"]
fn test_follow_when_file_and_symlink_are_pointing_to_same_file_and_append_data() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_data = "file data";
    let path_name = "data";
    let link_name = "link";

    at.write(path_name, file_data);
    at.symlink_file(path_name, link_name);

    let mut child = scene
        .ucmd()
        .args(&[
            "--follow=descriptor",
            "--max-unchanged-stats=1",
            "--sleep-interval=0.1",
            path_name,
            link_name,
        ])
        .run_no_wait();

    child.delay(500);
    let more_data = "more data";
    at.append(path_name, more_data);

    let expected_stdout = format!(
        "==> {path_name} <==\n\
        {file_data}\n\
        ==> {link_name} <==\n\
        {file_data}\n\
        ==> {path_name} <==\n\
        {more_data}\n\
        ==> {link_name} <==\n\
        {more_data}"
    );

    child.make_assertion_with_delay(500).is_alive();
    child
        .kill()
        .make_assertion()
        .with_current_output()
        .stdout_only(expected_stdout);

    at.write(path_name, file_data);
    let mut child = scene
        .ucmd()
        .args(&[
            "--follow=descriptor",
            "--max-unchanged-stats=1",
            "--sleep-interval=0.1",
            link_name,
            path_name,
        ])
        .run_no_wait();

    child.delay(500);
    let more_data = "more data";
    at.append(path_name, more_data);

    let expected_stdout = format!(
        "==> {link_name} <==\n\
        {file_data}\n\
        ==> {path_name} <==\n\
        {file_data}\n\
        ==> {link_name} <==\n\
        {more_data}\n\
        ==> {path_name} <==\n\
        {more_data}"
    );

    child.make_assertion_with_delay(500).is_alive();
    child
        .kill()
        .make_assertion()
        .with_current_output()
        .stdout_only(expected_stdout);
}

#[test]
fn test_args_when_directory_given_shorthand_big_f_together_with_retry() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let dirname = "dir";
    at.mkdir(dirname);
    let expected_stderr = format!(
        "tail: error reading '{dirname}': Is a directory\n\
         tail: {dirname}: cannot follow end of this type of file\n"
    );
    let mut child = scene.ucmd().args(&["-F", "--retry", "dir"]).run_no_wait();

    child.make_assertion_with_delay(500).is_alive();
    child
        .kill()
        .make_assertion()
        .with_current_output()
        .stderr_only(&expected_stderr);

    let mut child = scene.ucmd().args(&["--retry", "-F", "dir"]).run_no_wait();

    child.make_assertion_with_delay(500).is_alive();
    child
        .kill()
        .make_assertion()
        .with_current_output()
        .stderr_only(expected_stderr);
}

/// Fails on macos sometimes with
/// Diff < left / right > :
/// ==> data <==
/// file data
/// ==> /absolute/path/to/data <==
/// <file datasame data
/// >file data
///
/// Fails on windows with
/// Diff < left / right > :
//  ==> data <==
//  file data
//  ==> \\?\C:\Users\runneradmin\AppData\Local\Temp\.tmpi6lNnX\data <==
// >file data
// <
//
// Fails on freebsd with
// Diff < left / right > :
//  ==> data <==
//  file data
//  ==> /tmp/.tmpZPXPlS/data <==
// >file data
// <
#[test]
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))]
fn test_follow_when_files_are_pointing_to_same_relative_file_and_file_stays_same_size() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_data = "file data";
    let relative_path_name = "data";

    at.write(relative_path_name, file_data);
    let absolute_path = scene.fixtures.plus("data").canonicalize().unwrap();

    let mut child = scene
        .ucmd()
        .args(&[
            "--follow=descriptor",
            "--max-unchanged-stats=1",
            "--sleep-interval=0.1",
            relative_path_name,
            absolute_path.to_str().unwrap(),
        ])
        .run_no_wait();

    child.delay(500);
    let same_data = "same data"; // equal size to file_data
    at.write(relative_path_name, same_data);

    let expected_stdout = format!(
        "==> {0} <==\n\
        {1}\n\
        ==> {2} <==\n\
        {1}",
        relative_path_name,              // 0
        file_data,                       // 1
        absolute_path.to_str().unwrap(), // 2
    );

    child.make_assertion_with_delay(500).is_alive();
    child
        .kill()
        .make_assertion()
        .with_current_output()
        .stdout_only(expected_stdout);
}

#[rstest]
#[case::exponent_exceed_float_max("1.0e100000")]
#[case::underscore_delimiter("1_000")]
#[case::only_point(".")]
#[case::space_in_primes("' '")]
#[case::space(" ")]
#[case::empty("")]
#[case::comma_separator("0,0")]
#[case::words_nominator_fract("one.zero")]
#[case::words_fract(".zero")]
#[case::words_nominator("one.")]
#[case::two_points("0..0")]
#[case::seconds_unit("1.0s")]
#[case::circumflex_exponent("1.0e^1000")]
fn test_args_sleep_interval_when_illegal_argument_then_usage_error(#[case] sleep_interval: &str) {
    new_ucmd!()
        .args(&["--sleep-interval", sleep_interval])
        .run()
        .usage_error(format!("invalid number of seconds: '{sleep_interval}'"))
        .code_is(1);
}

#[test]
fn test_gnu_args_plus_c() {
    let scene = TestScenario::new(util_name!());

    // obs-plus-c1
    scene
        .ucmd()
        .arg("+2c")
        .pipe_in("abcd")
        .succeeds()
        .stdout_only("bcd");
    // obs-plus-c2
    scene
        .ucmd()
        .arg("+8c")
        .pipe_in("abcd")
        .succeeds()
        .stdout_only("");
    // obs-plus-x1: same as +10c
    scene
        .ucmd()
        .arg("+c")
        .pipe_in(format!("x{}z", "y".repeat(10)))
        .succeeds()
        .stdout_only("yyz");
}

#[test]
fn test_gnu_args_c() {
    let scene = TestScenario::new(util_name!());

    // obs-c3
    scene
        .ucmd()
        .arg("-1c")
        .pipe_in("abcd")
        .succeeds()
        .stdout_only("d");
    // obs-c4
    scene
        .ucmd()
        .arg("-9c")
        .pipe_in("abcd")
        .succeeds()
        .stdout_only("abcd");
    // obs-c5
    scene
        .ucmd()
        .arg("-12c")
        .pipe_in(format!("x{}z", "y".repeat(12)))
        .succeeds()
        .stdout_only(format!("{}z", "y".repeat(11)));
}

#[test]
fn test_gnu_args_l() {
    let scene = TestScenario::new(util_name!());

    // obs-l1
    scene
        .ucmd()
        .arg("-1l")
        .pipe_in("x")
        .succeeds()
        .stdout_only("x");
    // obs-l2
    scene
        .ucmd()
        .arg("-1l")
        .pipe_in("x\ny\n")
        .succeeds()
        .stdout_only("y\n");
    // obs-l3
    scene
        .ucmd()
        .arg("-1l")
        .pipe_in("x\ny")
        .succeeds()
        .stdout_only("y");
    // obs-l: same as -10l
    scene
        .ucmd()
        .arg("-l")
        .pipe_in(format!("x{}z", "y\n".repeat(10)))
        .succeeds()
        .stdout_only(format!("{}z", "y\n".repeat(9)));
}

#[test]
fn test_gnu_args_plus_l() {
    let scene = TestScenario::new(util_name!());

    // obs-plus-l4
    scene
        .ucmd()
        .arg("+1l")
        .pipe_in("x\ny\n")
        .succeeds()
        .stdout_only("x\ny\n");
    // ops-plus-l5
    scene
        .ucmd()
        .arg("+2l")
        .pipe_in("x\ny\n")
        .succeeds()
        .stdout_only("y\n");
    // obs-plus-x2: same as +10l
    scene
        .ucmd()
        .arg("+l")
        .pipe_in(format!("x\n{}z", "y\n".repeat(10)))
        .succeeds()
        .stdout_only("y\ny\nz");
}

#[test]
fn test_gnu_args_number() {
    let scene = TestScenario::new(util_name!());

    // obs-1
    scene
        .ucmd()
        .arg("-1")
        .pipe_in("x")
        .succeeds()
        .stdout_only("x");
    // obs-2
    scene
        .ucmd()
        .arg("-1")
        .pipe_in("x\ny\n")
        .succeeds()
        .stdout_only("y\n");
    // obs-3
    scene
        .ucmd()
        .arg("-1")
        .pipe_in("x\ny")
        .succeeds()
        .stdout_only("y");
}

#[test]
fn test_gnu_args_plus_number() {
    let scene = TestScenario::new(util_name!());

    // obs-plus-4
    scene
        .ucmd()
        .arg("+1")
        .pipe_in("x\ny\n")
        .succeeds()
        .stdout_only("x\ny\n");
    // ops-plus-5
    scene
        .ucmd()
        .arg("+2")
        .pipe_in("x\ny\n")
        .succeeds()
        .stdout_only("y\n");
}

#[test]
fn test_gnu_args_b() {
    let scene = TestScenario::new(util_name!());

    // obs-b
    scene
        .ucmd()
        .arg("-b")
        .pipe_in("x\n".repeat(512 * 10 / 2 + 1))
        .succeeds()
        .stdout_only("x\n".repeat(512 * 10 / 2));
}

#[test]
fn test_gnu_args_err() {
    let scene = TestScenario::new(util_name!());

    // err-1
    scene
        .ucmd()
        .arg("+cl")
        .fails()
        .no_stdout()
        .stderr_is("tail: cannot open '+cl' for reading: No such file or directory\n")
        .code_is(1);
    // err-2
    scene
        .ucmd()
        .arg("-cl")
        .fails()
        .no_stdout()
        .stderr_is("tail: invalid number of bytes: 'l'\n")
        .code_is(1);
    // err-3
    scene
        .ucmd()
        .arg("+2cz")
        .fails()
        .no_stdout()
        .stderr_is("tail: cannot open '+2cz' for reading: No such file or directory\n")
        .code_is(1);
    // err-4
    scene
        .ucmd()
        .arg("-2cX")
        .fails()
        .no_stdout()
        .stderr_is("tail: option used in invalid context -- 2\n")
        .code_is(1);
    // err-5
    scene
        .ucmd()
        .arg("-c99999999999999999999")
        .fails()
        .no_stdout()
        .stderr_is("tail: invalid number of bytes: '99999999999999999999'\n")
        .code_is(1);
    // err-6
    scene
        .ucmd()
        .arg("-c --")
        .fails()
        .no_stdout()
        .stderr_is("tail: invalid number of bytes: '-'\n")
        .code_is(1);
    scene
        .ucmd()
        .arg("-5cz")
        .fails()
        .no_stdout()
        .stderr_is("tail: option used in invalid context -- 5\n")
        .code_is(1);
    scene
        .ucmd()
        .arg("-9999999999999999999b")
        .fails()
        .no_stdout()
        .stderr_is("tail: invalid number: '-9999999999999999999b'\n")
        .code_is(1);
    scene
        .ucmd()
        .arg("-999999999999999999999b")
        .fails()
        .no_stdout()
        .stderr_is(
            "tail: invalid number: '-999999999999999999999b': Numerical result out of range\n",
        )
        .code_is(1);
}

#[test]
fn test_gnu_args_f() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let source = "file";
    at.touch(source);
    let mut p = scene.ucmd().args(&["+f", source]).run_no_wait();
    p.make_assertion_with_delay(500).is_alive();
    p.kill()
        .make_assertion()
        .with_all_output()
        .no_stderr()
        .no_stdout();

    let mut p = scene
        .ucmd()
        .set_stdin(Stdio::piped())
        .arg("+f")
        .run_no_wait();
    p.make_assertion_with_delay(500).is_alive();
    p.kill()
        .make_assertion()
        .with_all_output()
        .no_stderr()
        .no_stdout();
}

#[test]
#[cfg(unix)]
fn test_obsolete_encoding_unix() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let scene = TestScenario::new(util_name!());
    let invalid_utf8_arg = OsStr::from_bytes(&[b'-', INVALID_UTF8, b'b']);

    scene
        .ucmd()
        .arg(invalid_utf8_arg)
        .fails()
        .no_stdout()
        .stderr_is("tail: bad argument encoding: '-�b'\n")
        .code_is(1);
}

#[test]
#[cfg(windows)]
fn test_obsolete_encoding_windows() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let scene = TestScenario::new(util_name!());
    let invalid_utf16_arg = OsString::from_wide(&['-' as u16, INVALID_UTF16, 'b' as u16]);

    scene
        .ucmd()
        .arg(&invalid_utf16_arg)
        .fails()
        .no_stdout()
        .stderr_is("tail: bad argument encoding: '-�b'\n")
        .code_is(1);
}

#[test]
#[cfg(not(target_vendor = "apple"))] // FIXME: for currently not working platforms
fn test_following_with_pid() {
    use std::process::Command;

    let ts = TestScenario::new(util_name!());

    #[cfg(not(windows))]
    let mut sleep_command = Command::new("sleep")
        .arg("999d")
        .spawn()
        .expect("failed to start sleep command");
    #[cfg(windows)]
    let mut sleep_command = Command::new("powershell")
        .arg("-Command")
        .arg("Start-Sleep -Seconds 999")
        .spawn()
        .expect("failed to start sleep command");

    let sleep_pid = sleep_command.id();

    let at = &ts.fixtures;
    at.touch("f");
    // when -f is specified, tail should die after
    // the pid from --pid also dies
    let mut child = ts
        .ucmd()
        .args(&[
            "--pid",
            &sleep_pid.to_string(),
            "-f",
            at.plus("f").to_str().unwrap(),
        ])
        .stderr_to_stdout()
        .run_no_wait();
    child.make_assertion_with_delay(2000).is_alive();

    #[cfg(not(windows))]
    Command::new("kill")
        .arg("-9")
        .arg(sleep_pid.to_string())
        .output()
        .expect("failed to kill sleep command");
    #[cfg(windows)]
    Command::new("taskkill")
        .arg("/PID")
        .arg(sleep_pid.to_string())
        .arg("/F")
        .output()
        .expect("failed to kill sleep command");

    let _ = sleep_command.wait();

    child.make_assertion_with_delay(2000).is_not_alive();

    child.kill();
}
