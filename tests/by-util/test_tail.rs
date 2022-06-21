//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) abcdefghijklmnopqrstuvwxyz efghijklmnopqrstuvwxyz vwxyz emptyfile file siette ocho nueve diez
// spell-checker:ignore (libs) kqueue
// spell-checker:ignore (jargon) tailable untailable

extern crate tail;

use crate::common::util::*;
use std::char::from_digit;
#[cfg(unix)]
use std::io::Read;
use std::io::Write;
use std::process::Stdio;
#[cfg(unix)]
use std::thread::sleep;
#[cfg(unix)]
use std::time::Duration;

#[cfg(target_os = "linux")]
pub static BACKEND: &str = "inotify";
// #[cfg(all(unix, not(target_os = "linux")))]
// pub static BACKEND: &str = "kqueue";

static FOOBAR_TXT: &str = "foobar.txt";
static FOOBAR_2_TXT: &str = "foobar2.txt";
static FOOBAR_WITH_NULL_TXT: &str = "foobar_with_null.txt";
#[cfg(unix)]
static FOLLOW_NAME_TXT: &str = "follow_name.txt";
#[cfg(unix)]
static FOLLOW_NAME_SHORT_EXP: &str = "follow_name_short.expected";
#[cfg(target_os = "linux")]
static FOLLOW_NAME_EXP: &str = "follow_name.expected";

#[test]
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
fn test_stdin_default() {
    new_ucmd!()
        .pipe_in_fixture(FOOBAR_TXT)
        .run()
        .stdout_is_fixture("foobar_stdin_default.expected")
        .no_stderr();
}

#[test]
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
fn test_stdin_explicit() {
    new_ucmd!()
        .pipe_in_fixture(FOOBAR_TXT)
        .arg("-")
        .run()
        .stdout_is_fixture("foobar_stdin_default.expected")
        .no_stderr();
}

#[test]
#[cfg(all(unix, not(any(target_os = "android", target_vendor = "apple"))))] // FIXME: make this work not just on Linux
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
        .set_stdin(std::fs::File::open(at.plus("f")).unwrap())
        .run()
        .stdout_is("foo")
        .succeeded();
    ts.ucmd()
        .set_stdin(std::fs::File::open(at.plus("f")).unwrap())
        .arg("-v")
        .run()
        .stdout_is("==> standard input <==\nfoo")
        .succeeded();

    let mut p = ts
        .ucmd()
        .arg("-f")
        .set_stdin(std::fs::File::open(at.plus("f")).unwrap())
        .run_no_wait();

    sleep(Duration::from_millis(500));
    p.kill().unwrap();

    let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
    assert!(buf_stdout.eq("foo"));
    assert!(buf_stderr.is_empty());
}

#[test]
fn test_nc_0_wo_follow() {
    // verify that -[nc]0 without -f, exit without reading

    let ts = TestScenario::new(util_name!());
    ts.ucmd()
        .set_stdin(Stdio::null())
        .args(&["-n0", "missing"])
        .run()
        .no_stderr()
        .no_stdout()
        .succeeded();
    ts.ucmd()
        .set_stdin(Stdio::null())
        .args(&["-c0", "missing"])
        .run()
        .no_stderr()
        .no_stdout()
        .succeeded();
}

#[test]
#[cfg(all(unix, not(target_os = "freebsd")))]
fn test_nc_0_wo_follow2() {
    // verify that -[nc]0 without -f, exit without reading

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    use std::os::unix::fs::PermissionsExt;
    at.make_file("unreadable")
        .set_permissions(PermissionsExt::from_mode(0o000))
        .unwrap();

    ts.ucmd()
        .set_stdin(Stdio::null())
        .args(&["-n0", "unreadable"])
        .run()
        .no_stderr()
        .no_stdout()
        .succeeded();
    ts.ucmd()
        .set_stdin(Stdio::null())
        .args(&["-c0", "unreadable"])
        .run()
        .no_stderr()
        .no_stdout()
        .succeeded();
}

#[test]
#[cfg(all(unix, not(target_os = "freebsd")))]
fn test_permission_denied() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    use std::os::unix::fs::PermissionsExt;
    at.make_file("unreadable")
        .set_permissions(PermissionsExt::from_mode(0o000))
        .unwrap();

    ts.ucmd()
        .set_stdin(Stdio::null())
        .arg("unreadable")
        .fails()
        .stderr_is("tail: cannot open 'unreadable' for reading: Permission denied\n")
        .no_stdout()
        .code_is(1);
}

#[test]
#[cfg(all(unix, not(target_os = "freebsd")))]
fn test_permission_denied_multiple() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.touch("file1");
    at.touch("file2");

    use std::os::unix::fs::PermissionsExt;
    at.make_file("unreadable")
        .set_permissions(PermissionsExt::from_mode(0o000))
        .unwrap();

    ts.ucmd()
        .set_stdin(Stdio::null())
        .args(&["file1", "unreadable", "file2"])
        .fails()
        .stderr_is("tail: cannot open 'unreadable' for reading: Permission denied\n")
        .stdout_is("==> file1 <==\n\n==> file2 <==\n")
        .code_is(1);
}

#[test]
#[cfg(target_os = "linux")]
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
            .set_stdin(std::fs::File::open(at.plus("f")).unwrap())
            .args(&args)
            .fails()
            .no_stdout()
            .stderr_is("tail: cannot follow '-' by name")
            .code_is(1);
        args.pop();
    }
}

#[test]
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
fn test_stdin_redirect_dir() {
    // $ mkdir dir
    // $ tail < dir, $ tail - < dir
    // tail: error reading 'standard input': Is a directory

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkdir("dir");

    ts.ucmd()
        .set_stdin(std::fs::File::open(at.plus("dir")).unwrap())
        .fails()
        .no_stdout()
        .stderr_is("tail: error reading 'standard input': Is a directory")
        .code_is(1);
    ts.ucmd()
        .set_stdin(std::fs::File::open(at.plus("dir")).unwrap())
        .arg("-")
        .fails()
        .no_stdout()
        .stderr_is("tail: error reading 'standard input': Is a directory")
        .code_is(1);
}

#[test]
#[cfg(target_os = "linux")]
fn test_follow_stdin_descriptor() {
    let ts = TestScenario::new(util_name!());

    let mut args = vec!["-f", "-"];
    for _ in 0..2 {
        let mut p = ts.ucmd().args(&args).run_no_wait();
        sleep(Duration::from_millis(500));

        p.kill().unwrap();

        let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
        assert!(buf_stdout.is_empty());
        assert!(buf_stderr.is_empty());

        args.pop();
    }
}

#[test]
#[cfg(target_os = "linux")]
fn test_follow_stdin_name_retry() {
    // $ tail -F -
    // tail: cannot follow '-' by name
    let mut args = vec!["-F", "-"];
    for _ in 0..2 {
        new_ucmd!()
            .args(&args)
            .run()
            .no_stdout()
            .stderr_is("tail: cannot follow '-' by name")
            .code_is(1);
        args.pop();
    }
}

#[test]
#[cfg(target_os = "linux")]
#[cfg(disable_until_fixed)]
fn test_follow_stdin_explicit_indefinitely() {
    // inspired by: "gnu/tests/tail-2/follow-stdin.sh"
    // tail -f - /dev/null </dev/tty
    // tail: warning: following standard input indefinitely is ineffective
    // ==> standard input <==

    let ts = TestScenario::new(util_name!());

    let mut p = ts
        .ucmd()
        .set_stdin(Stdio::null())
        .args(&["-f", "-", "/dev/null"])
        .run_no_wait();

    sleep(Duration::from_millis(500));
    p.kill().unwrap();

    let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
    assert!(buf_stdout.eq("==> standard input <=="));
    assert!(buf_stderr.eq("tail: warning: following standard input indefinitely is ineffective"));

    // Also:
    // $ echo bar > foo
    //
    // $ tail -f - -
    // tail: warning: following standard input indefinitely is ineffective
    // ==> standard input <==
    //
    // $ tail -f - foo
    // tail: warning: following standard input indefinitely is ineffective
    // ==> standard input <==
    //
    //
    // $ tail -f - foo
    // tail: warning: following standard input indefinitely is ineffective
    // ==> standard input <==
    //
    // $ tail -f foo -
    // tail: warning: following standard input indefinitely is ineffective
    // ==> foo <==
    // bar
    //
    // ==> standard input <==
    //

    // $ echo f00 | tail -f foo -
    // bar
    //

    // TODO: Implement the above behavior of GNU's tail for following stdin indefinitely
}

#[test]
#[cfg(target_os = "linux")]
#[cfg(disable_until_fixed)]
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
#[cfg(unix)]
fn test_follow_single() {
    let (at, mut ucmd) = at_and_ucmd!();

    let mut child = ucmd
        .set_stdin(Stdio::null())
        .arg("-f")
        .arg(FOOBAR_TXT)
        .run_no_wait();

    let expected = at.read("foobar_single_default.expected");
    assert_eq!(read_size(&mut child, expected.len()), expected);

    // We write in a temporary copy of foobar.txt
    let expected = "line1\nline2\n";
    at.append(FOOBAR_TXT, expected);

    assert_eq!(read_size(&mut child, expected.len()), expected);

    child.kill().unwrap();
}

/// Test for following when bytes are written that are not valid UTF-8.
#[test]
#[cfg(unix)]
fn test_follow_non_utf8_bytes() {
    // Tail the test file and start following it.
    let (at, mut ucmd) = at_and_ucmd!();
    let mut child = ucmd
        .arg("-f")
        .set_stdin(Stdio::null())
        .arg(FOOBAR_TXT)
        .run_no_wait();
    let expected = at.read("foobar_single_default.expected");
    assert_eq!(read_size(&mut child, expected.len()), expected);

    // Now append some bytes that are not valid UTF-8.
    //
    // The binary integer "10000000" is *not* a valid UTF-8 encoding
    // of a character: https://en.wikipedia.org/wiki/UTF-8#Encoding
    //
    // We also write the newline character because our implementation
    // of `tail` is attempting to read a line of input, so the
    // presence of a newline character will force the `follow()`
    // function to conclude reading input bytes and start writing them
    // to output. The newline character is not fundamental to this
    // test, it is just a requirement of the current implementation.
    let expected = [0b10000000, b'\n'];
    at.append_bytes(FOOBAR_TXT, &expected);
    let actual = read_size_bytes(&mut child, expected.len());
    assert_eq!(actual, expected.to_vec());

    child.kill().unwrap();
}

#[test]
#[cfg(unix)]
fn test_follow_multiple() {
    let (at, mut ucmd) = at_and_ucmd!();
    let mut child = ucmd
        .set_stdin(Stdio::null())
        .arg("-f")
        .arg(FOOBAR_TXT)
        .arg(FOOBAR_2_TXT)
        .run_no_wait();

    let expected = at.read("foobar_follow_multiple.expected");
    assert_eq!(read_size(&mut child, expected.len()), expected);

    let first_append = "trois\n";
    at.append(FOOBAR_2_TXT, first_append);
    assert_eq!(read_size(&mut child, first_append.len()), first_append);

    let second_append = "twenty\nthirty\n";
    let expected = at.read("foobar_follow_multiple_appended.expected");
    at.append(FOOBAR_TXT, second_append);
    assert_eq!(read_size(&mut child, expected.len()), expected);

    child.kill().unwrap();
}

#[test]
#[cfg(unix)]
fn test_follow_name_multiple() {
    let (at, mut ucmd) = at_and_ucmd!();
    let mut child = ucmd
        .set_stdin(Stdio::null())
        .arg("--follow=name")
        .arg(FOOBAR_TXT)
        .arg(FOOBAR_2_TXT)
        .run_no_wait();

    let expected = at.read("foobar_follow_multiple.expected");
    assert_eq!(read_size(&mut child, expected.len()), expected);

    let first_append = "trois\n";
    at.append(FOOBAR_2_TXT, first_append);
    assert_eq!(read_size(&mut child, first_append.len()), first_append);

    let second_append = "twenty\nthirty\n";
    let expected = at.read("foobar_follow_multiple_appended.expected");
    at.append(FOOBAR_TXT, second_append);
    assert_eq!(read_size(&mut child, expected.len()), expected);

    child.kill().unwrap();
}

#[test]
#[cfg(unix)]
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
    ucmd.set_stdin(Stdio::null())
        .arg("-f")
        .arg("DIR1")
        .arg("DIR2")
        .fails()
        .stderr_is(expected_stderr)
        .stdout_is(expected_stdout)
        .code_is(1);
}

#[test]
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
fn test_follow_stdin_pipe() {
    new_ucmd!()
        .arg("-f")
        .pipe_in_fixture(FOOBAR_TXT)
        .run()
        .stdout_is_fixture("follow_stdin.expected")
        .no_stderr();
}

#[test]
#[cfg(unix)]
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
            "tail: invalid PID: '{}': number too large to fit in target type\n",
            max_pid
        ));
}

// FixME: test PASSES for usual windows builds, but fails for coverage testing builds (likely related to the specific RUSTFLAGS '-Zpanic_abort_tests -Cpanic=abort')  This test also breaks tty settings under bash requiring a 'stty sane' or reset. // spell-checker:disable-line
#[cfg(disable_until_fixed)]
#[test]
fn test_follow_with_pid() {
    use std::process::{Command, Stdio};
    use std::thread::sleep;
    use std::time::Duration;

    let (at, mut ucmd) = at_and_ucmd!();

    #[cfg(unix)]
    let dummy_cmd = "sh";
    #[cfg(windows)]
    let dummy_cmd = "cmd";

    let mut dummy = Command::new(dummy_cmd)
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let pid = dummy.id();

    let mut child = ucmd
        .arg("-f")
        .arg(format!("--pid={}", pid))
        .arg(FOOBAR_TXT)
        .arg(FOOBAR_2_TXT)
        .run_no_wait();

    let expected = at.read("foobar_follow_multiple.expected");
    assert_eq!(read_size(&mut child, expected.len()), expected);

    let first_append = "trois\n";
    at.append(FOOBAR_2_TXT, first_append);
    assert_eq!(read_size(&mut child, first_append.len()), first_append);

    let second_append = "twenty\nthirty\n";
    let expected = at.read("foobar_follow_multiple_appended.expected");
    at.append(FOOBAR_TXT, second_append);
    assert_eq!(read_size(&mut child, expected.len()), expected);

    // kill the dummy process and give tail time to notice this
    dummy.kill().unwrap();
    let _ = dummy.wait();
    sleep(Duration::from_secs(1));

    let third_append = "should\nbe\nignored\n";
    at.append(FOOBAR_TXT, third_append);
    assert_eq!(read_size(&mut child, 1), "\u{0}");

    // On Unix, trying to kill a process that's already dead is fine; on Windows it's an error.
    #[cfg(unix)]
    child.kill().unwrap();
    #[cfg(windows)]
    assert_eq!(child.kill().is_err(), true);
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
        writeln!(big_input, "Line {}", i).expect("Could not write to FILE");
    }
    big_input.flush().expect("Could not flush FILE");

    let mut big_expected = at.make_file(EXPECTED_FILE);
    for i in (LINES - N_ARG)..LINES {
        writeln!(big_expected, "Line {}", i).expect("Could not write to EXPECTED_FILE");
    }
    big_expected.flush().expect("Could not flush EXPECTED_FILE");

    ucmd.arg(FILE)
        .arg("-n")
        .arg(format!("{}", N_ARG))
        .run()
        .stdout_is(at.read(EXPECTED_FILE));
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
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
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
        write!(big_input, "{}", digit).expect("Could not write to FILE");
    }
    big_input.flush().expect("Could not flush FILE");

    let mut big_expected = at.make_file(EXPECTED_FILE);
    for i in (BYTES - N_ARG)..BYTES {
        let digit = from_digit((i % 10) as u32, 10).unwrap();
        write!(big_expected, "{}", digit).expect("Could not write to EXPECTED_FILE");
    }
    big_expected.flush().expect("Could not flush EXPECTED_FILE");

    let result = ucmd
        .arg(FILE)
        .arg("-c")
        .arg(format!("{}", N_ARG))
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
        writeln!(big_input, "Line {}", i).expect("Could not write to FILE");
    }
    big_input.flush().expect("Could not flush FILE");

    let mut big_expected = at.make_file(EXPECTED_FILE);
    for i in (LINES - N_ARG)..LINES {
        writeln!(big_expected, "Line {}", i).expect("Could not write to EXPECTED_FILE");
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
        .set_stdin(Stdio::null())
        .arg(FOOBAR_TXT)
        .arg(FOOBAR_2_TXT)
        .run()
        .no_stderr()
        .stdout_is_fixture("foobar_follow_multiple.expected");
}

#[test]
fn test_multiple_input_files_missing() {
    new_ucmd!()
        .set_stdin(Stdio::null())
        .arg(FOOBAR_TXT)
        .arg("missing1")
        .arg(FOOBAR_2_TXT)
        .arg("missing2")
        .run()
        .stdout_is_fixture("foobar_follow_multiple.expected")
        .stderr_is(
            "tail: cannot open 'missing1' for reading: No such file or directory\n\
                tail: cannot open 'missing2' for reading: No such file or directory",
        )
        .code_is(1);
}

#[test]
fn test_follow_missing() {
    // Ensure that --follow=name does not imply --retry.
    // Ensure that --follow={descriptor,name} (without --retry) does *not wait* for the
    // file to appear.
    for follow_mode in &["--follow=descriptor", "--follow=name"] {
        new_ucmd!()
            .set_stdin(Stdio::null())
            .arg(follow_mode)
            .arg("missing")
            .run()
            .no_stdout()
            .stderr_is(
                "tail: cannot open 'missing' for reading: No such file or directory\n\
                    tail: no files remaining",
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
        .stderr_is("tail: cannot follow '-' by name")
        .code_is(1);
    ts.ucmd()
        .arg("--follow=name")
        .arg("FILE1")
        .arg("-")
        .arg("FILE2")
        .run()
        .stderr_is("tail: cannot follow '-' by name")
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
            .set_stdin(Stdio::null())
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
        .set_stdin(Stdio::null())
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
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
fn test_positive_bytes() {
    new_ucmd!()
        .args(&["-c", "+3"])
        .pipe_in("abcde")
        .succeeds()
        .stdout_is("cde");
}

/// Test for reading all bytes, specified by `tail -c +0`.
#[test]
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
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
        .succeeds()
        .no_stdout()
        .no_stderr();
}

/// Test for reading all but the first NUM lines: `tail -n +3`.
#[test]
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
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
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
fn test_obsolete_syntax_positive_lines() {
    new_ucmd!()
        .args(&["-3"])
        .pipe_in("a\nb\nc\nd\ne\n")
        .succeeds()
        .stdout_is("c\nd\ne\n");
}

/// Test for reading all but the first NUM lines: `tail -n -10`.
#[test]
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
fn test_small_file() {
    new_ucmd!()
        .args(&["-n -10"])
        .pipe_in("a\nb\nc\nd\ne\n")
        .succeeds()
        .stdout_is("a\nb\nc\nd\ne\n");
}

/// Test for reading all but the first NUM lines: `tail -10`.
#[test]
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
fn test_obsolete_syntax_small_file() {
    new_ucmd!()
        .args(&["-10"])
        .pipe_in("a\nb\nc\nd\ne\n")
        .succeeds()
        .stdout_is("a\nb\nc\nd\ne\n");
}

/// Test for reading all lines, specified by `tail -n +0`.
#[test]
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
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
        .succeeds()
        .no_stderr()
        .no_stdout();
}

#[test]
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
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
    #[cfg(not(target_pointer_width = "128"))]
    new_ucmd!()
        .args(&["-c", "1Y", "emptyfile.txt"])
        .fails()
        .stderr_str()
        .starts_with("tail: invalid number of bytes: '1Y': Value too large for defined data type");
    #[cfg(not(target_pointer_width = "128"))]
    new_ucmd!()
        .args(&["-n", "1Y", "emptyfile.txt"])
        .fails()
        .stderr_str()
        .starts_with("tail: invalid number of lines: '1Y': Value too large for defined data type");
    #[cfg(target_pointer_width = "32")]
    {
        let sizes = ["1000G", "10T"];
        for size in &sizes {
            new_ucmd!()
                .args(&["-c", size])
                .fails()
                .code_is(1)
                .stderr_str()
                .starts_with("tail: Insufficient addressable memory");
        }
    }
    new_ucmd!()
        .args(&["-c", "-³"])
        .fails()
        .stderr_str()
        .starts_with("tail: invalid number of bytes: '³'");
}

#[test]
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
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
fn test_bytes_for_funny_files() {
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
#[cfg(unix)]
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
#[cfg(unix)]
fn test_retry2() {
    // inspired by: gnu/tests/tail-2/retry.sh
    // The same as test_retry2 with a missing file: expect error message and exit 1.

    let ts = TestScenario::new(util_name!());
    let missing = "missing";

    let result = ts
        .ucmd()
        .set_stdin(Stdio::null())
        .arg(missing)
        .arg("--retry")
        .run();
    result
        .stderr_is(
            "tail: warning: --retry ignored; --retry is useful only when following\n\
                tail: cannot open 'missing' for reading: No such file or directory\n",
        )
        .code_is(1);
}

#[test]
#[cfg(target_os = "linux")] // FIXME: fix this test for BSD/macOS
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
        let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();
        sleep(Duration::from_millis(delay));

        at.touch(missing);
        sleep(Duration::from_millis(delay));

        at.truncate(missing, "X\n");
        sleep(Duration::from_millis(delay));

        p.kill().unwrap();

        let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
        assert_eq!(buf_stdout, expected_stdout);
        assert_eq!(buf_stderr, expected_stderr);

        at.remove(missing);
        args.pop();
        delay /= 3;
    }
}

#[test]
#[cfg(target_os = "linux")] // FIXME: fix this test for BSD/macOS
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
        let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();
        sleep(Duration::from_millis(delay));

        at.touch(missing);
        sleep(Duration::from_millis(delay));

        at.truncate(missing, "X1\n");
        sleep(Duration::from_millis(delay));

        at.truncate(missing, "X\n");
        sleep(Duration::from_millis(delay));

        p.kill().unwrap();

        let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
        assert_eq!(buf_stdout, expected_stdout);
        assert_eq!(buf_stderr, expected_stderr);

        at.remove(missing);
        args.pop();
        delay /= 3;
    }
}

#[test]
#[cfg(target_os = "linux")] // FIXME: fix this test for BSD/macOS
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
        let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();
        sleep(Duration::from_millis(delay));

        at.mkdir(missing);
        sleep(Duration::from_millis(delay));

        p.kill().unwrap();

        let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
        assert!(buf_stdout.is_empty());
        assert_eq!(buf_stderr, expected_stderr);

        at.rmdir(missing);
        args.pop();
        delay /= 3;
    }
}

#[test]
#[cfg(target_os = "linux")] // FIXME: fix this test for BSD/macOS
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
        .set_stdin(Stdio::null())
        .arg("--follow=descriptor")
        .arg("missing")
        .arg("existing")
        .run_no_wait();

    let delay = 1000;
    sleep(Duration::from_millis(delay));
    at.truncate(missing, "Y\n");
    sleep(Duration::from_millis(delay));
    at.truncate(existing, "X\n");
    sleep(Duration::from_millis(delay));

    p.kill().unwrap();

    let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
    assert_eq!(buf_stdout, expected_stdout);
    assert_eq!(buf_stderr, expected_stderr);
}

#[test]
#[cfg(target_os = "linux")] // FIXME: fix this test for BSD/macOS
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

        let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();
        sleep(Duration::from_millis(delay));

        // tail: 'untailable' has become accessible
        // or (The first is the common case, "has appeared" arises with slow rmdir):
        // tail: 'untailable' has appeared;  following new file
        at.rmdir(untailable);
        at.truncate(untailable, "foo\n");
        sleep(Duration::from_millis(delay));

        // NOTE: GNU's `tail` only shows "become inaccessible"
        // if there's a delay between rm and mkdir.
        // tail: 'untailable' has become inaccessible: No such file or directory
        at.remove(untailable);
        sleep(Duration::from_millis(delay));

        // tail: 'untailable' has been replaced with an untailable file\n";
        at.mkdir(untailable);
        sleep(Duration::from_millis(delay));

        // full circle, back to the beginning
        at.rmdir(untailable);
        at.truncate(untailable, "bar\n");
        sleep(Duration::from_millis(delay));

        p.kill().unwrap();

        let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
        assert_eq!(buf_stdout, expected_stdout);
        assert_eq!(buf_stderr, expected_stderr);

        args.pop();
        at.remove(untailable);
        delay /= 3;
    }
}

#[test]
#[cfg(all(unix, not(any(target_os = "android", target_vendor = "apple"))))] // FIXME: make this work not just on Linux
fn test_retry8() {
    // Ensure that inotify will switch to polling mode if directory
    // of the watched file was initially missing and later created.
    // This is similar to test_retry9, but without:
    // tail: directory containing watched file was removed\n\
    // tail: inotify cannot be used, reverting to polling\n\

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let watched_file = std::path::Path::new("watched_file");
    let parent_dir = std::path::Path::new("parent_dir");
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
        .set_stdin(Stdio::null())
        .arg("-F")
        .arg("-s.1")
        .arg("--max-unchanged-stats=1")
        .arg(user_path)
        .run_no_wait();
    sleep(Duration::from_millis(delay));

    // 'parent_dir/watched_file' is orphan
    // tail: cannot open 'parent_dir/watched_file' for reading: No such file or directory\n\

    // tail: 'parent_dir/watched_file' has appeared;  following new file\n\
    at.mkdir(parent_dir); // not an orphan anymore
    at.append(user_path, "foo\n");
    sleep(Duration::from_millis(delay));

    // tail: 'parent_dir/watched_file' has become inaccessible: No such file or directory\n\
    at.remove(user_path);
    at.rmdir(parent_dir); // 'parent_dir/watched_file' is orphan *again*
    sleep(Duration::from_millis(delay));

    // Since 'parent_dir/watched_file' is orphan, this needs to be picked up by polling
    // tail: 'parent_dir/watched_file' has appeared;  following new file\n";
    at.mkdir(parent_dir); // not an orphan anymore
    at.append(user_path, "bar\n");
    sleep(Duration::from_millis(delay));

    p.kill().unwrap();

    let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
    assert_eq!(buf_stdout, expected_stdout);
    assert_eq!(buf_stderr, expected_stderr);
}

#[test]
#[cfg(all(unix, not(any(target_os = "android", target_vendor = "apple"))))] // FIXME: make this work not just on Linux
fn test_retry9() {
    // inspired by: gnu/tests/tail-2/inotify-dir-recreate.sh
    // Ensure that inotify will switch to polling mode if directory
    // of the watched file was removed and recreated.

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let watched_file = std::path::Path::new("watched_file");
    let parent_dir = std::path::Path::new("parent_dir");
    let user_path = parent_dir.join(watched_file);
    let parent_dir = parent_dir.to_str().unwrap();
    let user_path = user_path.to_str().unwrap();

    let expected_stderr = format!(
        "\
            tail: 'parent_dir/watched_file' has become inaccessible: No such file or directory\n\
            tail: directory containing watched file was removed\n\
            tail: {} cannot be used, reverting to polling\n\
            tail: 'parent_dir/watched_file' has appeared;  following new file\n\
            tail: 'parent_dir/watched_file' has become inaccessible: No such file or directory\n\
            tail: 'parent_dir/watched_file' has appeared;  following new file\n\
            tail: 'parent_dir/watched_file' has become inaccessible: No such file or directory\n\
            tail: 'parent_dir/watched_file' has appeared;  following new file\n",
        BACKEND
    );
    let expected_stdout = "foo\nbar\nfoo\nbar\n";

    let delay = 1000;

    at.mkdir(parent_dir);
    at.truncate(user_path, "foo\n");
    let mut p = ts
        .ucmd()
        .set_stdin(Stdio::null())
        .arg("-F")
        .arg("-s.1")
        .arg("--max-unchanged-stats=1")
        .arg(user_path)
        .run_no_wait();

    sleep(Duration::from_millis(delay));

    at.remove(user_path);
    at.rmdir(parent_dir);
    sleep(Duration::from_millis(delay));

    at.mkdir(parent_dir);
    at.truncate(user_path, "bar\n");
    sleep(Duration::from_millis(delay));

    at.remove(user_path);
    at.rmdir(parent_dir);
    sleep(Duration::from_millis(delay));

    at.mkdir(parent_dir);
    at.truncate(user_path, "foo\n");
    sleep(Duration::from_millis(delay));

    at.remove(user_path);
    at.rmdir(parent_dir);
    sleep(Duration::from_millis(delay));

    at.mkdir(parent_dir);
    at.truncate(user_path, "bar\n");
    sleep(Duration::from_millis(delay));

    p.kill().unwrap();

    let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
    assert_eq!(buf_stdout, expected_stdout);
    assert_eq!(buf_stderr, expected_stderr);
}

#[test]
#[cfg(target_os = "linux")] // FIXME: fix this test for BSD/macOS
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

        let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();
        sleep(Duration::from_millis(delay));

        at.append(file_a, "A\n");
        sleep(Duration::from_millis(delay));

        at.rename(file_a, file_b);
        sleep(Duration::from_millis(delay));

        at.append(file_b, "B\n");
        sleep(Duration::from_millis(delay));

        at.rename(file_b, file_c);
        sleep(Duration::from_millis(delay));

        at.append(file_c, "C\n");
        sleep(Duration::from_millis(delay));

        p.kill().unwrap();

        let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
        assert_eq!(buf_stdout, "A\nB\nC\n");
        assert!(buf_stderr.is_empty());

        args.pop();
        delay /= 3;
    }
}

#[test]
#[cfg(target_os = "linux")] // FIXME: fix this test for BSD/macOS
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
        let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();
        sleep(Duration::from_millis(delay));

        at.rename(file_a, file_c);
        sleep(Duration::from_millis(delay));

        at.append(file_c, "X\n");
        sleep(Duration::from_millis(delay));

        p.kill().unwrap();

        let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
        assert_eq!(
            buf_stdout,
            "==> FILE_A <==\n\n==> FILE_B <==\n\n==> FILE_A <==\nX\n"
        );
        assert!(buf_stderr.is_empty());

        args.pop();
        delay /= 3;
    }
}

#[test]
#[cfg(target_os = "linux")]
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
        let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();
        sleep(Duration::from_millis(delay));
        at.truncate(file_a, "x\n");
        sleep(Duration::from_millis(delay));
        at.truncate(file_b, "y\n");
        sleep(Duration::from_millis(delay));
        p.kill().unwrap();

        let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
        assert_eq!(buf_stdout, "\n==> a <==\nx\n\n==> b <==\ny\n");
        assert_eq!(
            buf_stderr,
            "tail: cannot open 'a' for reading: No such file or directory\n\
                tail: cannot open 'b' for reading: No such file or directory\n\
                tail: 'a' has appeared;  following new file\n\
                tail: 'b' has appeared;  following new file\n"
        );

        at.remove(file_a);
        at.remove(file_b);
        args.pop();
        delay /= 3;
    }
}

#[test]
#[cfg(all(unix, not(target_os = "android")))] // FIXME: make this work not just on Linux
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

        let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();
        sleep(Duration::from_millis(delay));

        at.remove(source_copy);
        sleep(Duration::from_millis(delay));

        p.kill().unwrap();

        let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
        assert_eq!(buf_stdout, expected_stdout);
        assert_eq!(buf_stderr, expected_stderr[i]);

        args.pop();
        delay /= 3;
    }
}

#[test]
#[cfg(target_os = "linux")] // FIXME: fix this test for BSD/macOS
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
    let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();

    let delay = 1000;

    at.copy(source, backup);
    sleep(Duration::from_millis(delay));
    at.touch(source); // trigger truncate
    sleep(Duration::from_millis(delay));
    at.copy(backup, source);
    sleep(Duration::from_millis(delay));

    p.kill().unwrap();

    let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
    assert_eq!(buf_stdout, expected_stdout);
    assert_eq!(buf_stderr, expected_stderr);
}

#[test]
#[cfg(target_os = "linux")] // FIXME: fix this test for BSD/macOS
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
    let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();

    let delay = 1000;

    at.append(source, "x\n");
    sleep(Duration::from_millis(delay));
    at.append(source, "x\n");
    sleep(Duration::from_millis(delay));
    at.append(source, "x\n");
    sleep(Duration::from_millis(delay));
    at.truncate(source, "x\n");
    sleep(Duration::from_millis(delay));

    p.kill().unwrap();

    let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
    assert_eq!(buf_stdout, expected_stdout);
    assert_eq!(buf_stderr, expected_stderr);
}

#[test]
#[cfg(target_os = "linux")] // FIXME: fix this test for BSD/macOS
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
    let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();

    let delay = 1000;
    sleep(Duration::from_millis(delay));
    at.truncate(source, "x\n");
    sleep(Duration::from_millis(delay));

    p.kill().unwrap();

    let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
    assert_eq!(buf_stdout, expected_stdout);
    assert!(buf_stderr.is_empty());
}

#[test]
#[cfg(unix)]
fn test_follow_name_truncate4() {
    // Truncating a file with the same content it already has should not trigger a truncate event

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let mut args = vec!["-s.1", "--max-unchanged-stats=1", "-F", "file"];

    let mut delay = 500;
    for _ in 0..2 {
        at.append("file", "foobar\n");

        let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();
        sleep(Duration::from_millis(delay));

        at.truncate("file", "foobar\n");
        sleep(Duration::from_millis(delay));

        p.kill().unwrap();

        let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
        assert!(buf_stderr.is_empty());
        assert_eq!(buf_stdout, "foobar\n");

        at.remove("file");
        args.push("---disable-inotify");
        delay *= 3;
    }
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
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

            let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();
            sleep(Duration::from_millis(delay));

            at.truncate("f", "11\n12\n13\n14\n15\n");
            sleep(Duration::from_millis(delay));

            p.kill().unwrap();

            let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
            assert_eq!(
                buf_stdout,
                "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12\n13\n14\n15\n"
            );
            assert_eq!(buf_stderr, "tail: f: file truncated\n");

            args.pop();
        }
        args.pop();
        delay = 250;
    }
}

#[test]
#[cfg(all(unix, not(any(target_os = "android", target_vendor = "apple"))))] // FIXME: make this work not just on Linux
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

    let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();
    sleep(Duration::from_millis(delay));

    at.rename(source, backup);
    sleep(Duration::from_millis(delay));

    at.copy(backup, source);
    sleep(Duration::from_millis(delay));

    p.kill().unwrap();

    let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
    assert_eq!(buf_stdout, expected_stdout);
    assert_eq!(buf_stderr, expected_stderr);
}

#[test]
#[cfg(all(unix, not(any(target_os = "android", target_vendor = "apple"))))] // FIXME: make this work not just on Linux
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
    for _ in 0..2 {
        let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();
        sleep(Duration::from_millis(delay));

        at.truncate("9", "x\n");
        sleep(Duration::from_millis(delay));

        at.rename("1", "f");
        sleep(Duration::from_millis(delay));

        at.truncate("1", "a\n");
        sleep(Duration::from_millis(delay));

        p.kill().unwrap();

        let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
        assert_eq!(
            buf_stderr,
            "tail: '1' has become inaccessible: No such file or directory\n\
                tail: '1' has appeared;  following new file\n"
        );

        // NOTE: Because "gnu/tests/tail-2/inotify-hash-abuse.sh" 'forgets' to clear the files used
        // during the first loop iteration, we also don't clear them to get the same side-effects.
        // Side-effects are truncating a file with the same content, see: test_follow_name_truncate4
        // at.remove("1");
        // at.touch("1");
        // at.remove("9");
        // at.touch("9");
        if args.len() == 14 {
            assert_eq!(buf_stdout, "a\nx\na\n");
        } else {
            assert_eq!(buf_stdout, "x\na\n");
        }

        at.remove("f");
        args.push("---disable-inotify");
        delay *= 3;
    }
}

#[test]
#[cfg(all(unix, not(any(target_os = "android", target_vendor = "apple"))))] // FIXME: make this work not just on Linux
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
        let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();
        sleep(Duration::from_millis(delay));

        at.rename(source, backup);
        sleep(Duration::from_millis(delay));

        p.kill().unwrap();

        let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
        assert_eq!(buf_stdout, expected_stdout);
        assert_eq!(buf_stderr, expected_stderr[i]);

        at.rename(backup, source);
        args.push("--use-polling");
        delay *= 3;
    }
}

#[test]
#[cfg(all(unix, not(any(target_os = "android", target_vendor = "apple"))))] // FIXME: make this work not just on Linux
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
        "==> {0} <==\n{0}_content\n\n==> {1} <==\n{1}_content\n{0}_content\n\
            more_{1}_content\n\n==> {0} <==\nmore_{0}_content\n",
        file1, file2
    );
    let mut expected_stderr = format!(
        "{0}: {1}: No such file or directory\n\
            {0}: '{2}' has been replaced;  following new file\n\
            {0}: '{1}' has appeared;  following new file\n",
        ts.util_name, file1, file2
    );

    let mut args = vec!["--follow=name", file1, file2];

    let mut delay = 500;
    for _ in 0..2 {
        at.truncate(file1, "file1_content\n");
        at.truncate(file2, "file2_content\n");

        let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();
        sleep(Duration::from_millis(delay));

        at.rename(file1, file2);
        sleep(Duration::from_millis(delay));

        at.append(file2, "more_file2_content\n");
        sleep(Duration::from_millis(delay));

        at.append(file1, "more_file1_content\n");
        sleep(Duration::from_millis(delay));

        p.kill().unwrap();

        let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
        println!("out:\n{}\nerr:\n{}", buf_stdout, buf_stderr);
        assert_eq!(buf_stdout, expected_stdout);
        assert_eq!(buf_stderr, expected_stderr);

        args.push("--use-polling");
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
#[cfg(all(unix, not(any(target_os = "android", target_vendor = "apple"))))] // FIXME: make this work not just on Linux
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
        let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();
        sleep(Duration::from_millis(delay));

        at.append(source, "tailed\n");
        sleep(Duration::from_millis(delay));

        // with --follow=name, tail should stop monitoring the renamed file
        at.rename(source, backup);
        sleep(Duration::from_millis(delay));
        // overwrite backup while it's not monitored
        at.truncate(backup, "new content\n");
        sleep(Duration::from_millis(delay));
        // move back, tail should pick this up and print new content
        at.rename(backup, source);
        sleep(Duration::from_millis(delay));

        p.kill().unwrap();

        let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
        assert_eq!(buf_stdout, expected_stdout);
        assert_eq!(buf_stderr, expected_stderr);

        at.remove(source);
        args.pop();
        delay /= 3;
    }
}
#[test]
#[cfg(all(unix, not(any(target_os = "android", target_vendor = "apple"))))] // FIXME: make this work not just on Linux
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
        "==> {0} <==\n\n==> {1} <==\n\n==> {0} <==\nx\n\n==> {1} <==\
            \nx\n\n==> {0} <==\nx2\n\n==> {1} <==\ny\n\n==> {0} <==\nz\n",
        file1, file2
    );
    let mut expected_stderr = format!(
        "{0}: '{1}' has become inaccessible: No such file or directory\n\
            {0}: '{2}' has been replaced;  following new file\n\
            {0}: '{1}' has appeared;  following new file\n",
        ts.util_name, file1, file2
    );

    let mut args = vec!["-s.1", "--max-unchanged-stats=1", "-F", file1, file2];

    let mut delay = 500;
    for _ in 0..2 {
        at.touch(file1);
        at.touch(file2);

        let mut p = ts.ucmd().set_stdin(Stdio::null()).args(&args).run_no_wait();
        sleep(Duration::from_millis(delay));

        at.truncate(file1, "x\n");
        sleep(Duration::from_millis(delay));

        at.rename(file1, file2);
        sleep(Duration::from_millis(delay));

        at.truncate(file1, "x2\n");
        sleep(Duration::from_millis(delay));

        at.append(file2, "y\n");
        sleep(Duration::from_millis(delay));

        at.append(file1, "z\n");
        sleep(Duration::from_millis(delay));

        p.kill().unwrap();

        let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
        assert_eq!(buf_stdout, expected_stdout);
        assert_eq!(buf_stderr, expected_stderr);

        at.remove(file1);
        at.remove(file2);
        args.push("--use-polling");
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
#[cfg(unix)]
fn test_follow_inotify_only_regular() {
    // The GNU test inotify-only-regular.sh uses strace to ensure that `tail -f`
    // doesn't make inotify syscalls and only uses inotify for regular files or fifos.
    // We just check if tailing a character device has the same behavior as GNU's tail.

    let ts = TestScenario::new(util_name!());

    let mut p = ts
        .ucmd()
        .set_stdin(Stdio::null())
        .arg("-f")
        .arg("/dev/null")
        .run_no_wait();
    sleep(Duration::from_millis(200));

    p.kill().unwrap();

    let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
    assert_eq!(buf_stdout, "".to_string());
    assert_eq!(buf_stderr, "".to_string());
}

#[cfg(unix)]
fn take_stdout_stderr(p: &mut std::process::Child) -> (String, String) {
    let mut buf_stdout = String::new();
    let mut p_stdout = p.stdout.take().unwrap();
    p_stdout.read_to_string(&mut buf_stdout).unwrap();
    let mut buf_stderr = String::new();
    let mut p_stderr = p.stderr.take().unwrap();
    p_stderr.read_to_string(&mut buf_stderr).unwrap();
    (buf_stdout, buf_stderr)
}

#[test]
fn test_no_such_file() {
    new_ucmd!()
        .set_stdin(Stdio::null())
        .arg("missing")
        .fails()
        .stderr_is("tail: cannot open 'missing' for reading: No such file or directory")
        .no_stdout()
        .code_is(1);
}

#[test]
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
fn test_no_trailing_newline() {
    new_ucmd!().pipe_in("x").succeeds().stdout_only("x");
}

#[test]
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
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
#[cfg(all(unix, not(target_os = "android")))] // FIXME: fix this test for Android
fn test_presume_input_pipe_default() {
    new_ucmd!()
        .arg("---presume-input-pipe")
        .pipe_in_fixture(FOOBAR_TXT)
        .run()
        .stdout_is_fixture("foobar_stdin_default.expected")
        .no_stderr();
}

#[test]
#[cfg(unix)]
fn test_fifo() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkfifo("FIFO");

    let mut p = ts.ucmd().arg("FIFO").run_no_wait();
    sleep(Duration::from_millis(500));
    p.kill().unwrap();
    let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
    assert!(buf_stdout.is_empty());
    assert!(buf_stderr.is_empty());

    for arg in ["-f", "-F"] {
        let mut p = ts.ucmd().arg(arg).arg("FIFO").run_no_wait();
        sleep(Duration::from_millis(500));
        p.kill().unwrap();

        let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
        assert!(buf_stdout.is_empty());
        assert!(buf_stderr.is_empty());
    }
}

#[test]
#[cfg(unix)]
#[cfg(disable_until_fixed)]
fn test_illegal_seek() {
    // This is here for reference only.
    // We don't call seek on fifos, so we don't hit this error case.
    // (Also see: https://github.com/coreutils/coreutils/pull/36)

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.append("FILE", "foo\n");
    at.mkfifo("FIFO");

    let mut p = ts.ucmd().arg("FILE").run_no_wait();
    sleep(Duration::from_millis(500));
    at.rename("FILE", "FIFO");
    sleep(Duration::from_millis(500));

    p.kill().unwrap();
    let (buf_stdout, buf_stderr) = take_stdout_stderr(&mut p);
    dbg!(&buf_stdout, &buf_stderr);
    assert_eq!(buf_stdout, "foo\n");
    assert_eq!(
        buf_stderr,
        "tail: 'FILE' has been replaced;  following new file\n\
            tail: FILE: cannot seek to offset 0: Illegal seek\n"
    );
    assert_eq!(p.wait().unwrap().code().unwrap(), 1);
}
