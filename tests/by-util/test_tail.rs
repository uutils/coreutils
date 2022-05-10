//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) abcdefghijklmnopqrstuvwxyz efghijklmnopqrstuvwxyz vwxyz emptyfile bogusfile siette ocho nueve diez

extern crate tail;

use crate::common::util::*;
use std::char::from_digit;
use std::io::Write;

static FOOBAR_TXT: &str = "foobar.txt";
static FOOBAR_2_TXT: &str = "foobar2.txt";
static FOOBAR_WITH_NULL_TXT: &str = "foobar_with_null.txt";

#[test]
fn test_stdin_default() {
    new_ucmd!()
        .pipe_in_fixture(FOOBAR_TXT)
        .run()
        .stdout_is_fixture("foobar_stdin_default.expected");
}

#[test]
fn test_stdin_explicit() {
    new_ucmd!()
        .pipe_in_fixture(FOOBAR_TXT)
        .arg("-")
        .run()
        .stdout_is_fixture("foobar_stdin_default.expected");
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
fn test_follow() {
    let (at, mut ucmd) = at_and_ucmd!();

    let mut child = ucmd.arg("-f").arg(FOOBAR_TXT).run_no_wait();

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
fn test_follow_non_utf8_bytes() {
    // Tail the test file and start following it.
    let (at, mut ucmd) = at_and_ucmd!();
    let mut child = ucmd.arg("-f").arg(FOOBAR_TXT).run_no_wait();
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
fn test_follow_multiple() {
    let (at, mut ucmd) = at_and_ucmd!();
    let mut child = ucmd
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
fn test_follow_stdin() {
    new_ucmd!()
        .arg("-f")
        .pipe_in_fixture(FOOBAR_TXT)
        .run()
        .stdout_is_fixture("follow_stdin.expected");
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
fn test_bytes_stdin() {
    new_ucmd!()
        .arg("-c")
        .arg("13")
        .pipe_in_fixture(FOOBAR_TXT)
        .run()
        .stdout_is_fixture("foobar_bytes_stdin.expected");
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
        .arg(FOOBAR_TXT)
        .arg(FOOBAR_2_TXT)
        .run()
        .stdout_is_fixture("foobar_follow_multiple.expected");
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
    new_ucmd!()
        .args(&["-c", "+0"])
        .pipe_in("abcde")
        .succeeds()
        .stdout_is("abcde");
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
    new_ucmd!()
        .args(&["-n", "+0"])
        .pipe_in("a\nb\nc\nd\ne\n")
        .succeeds()
        .stdout_is("a\nb\nc\nd\ne\n");
}

#[test]
fn test_tail_invalid_num() {
    new_ucmd!()
        .args(&["-c", "1024R", "emptyfile.txt"])
        .fails()
        .stderr_is("tail: invalid number of bytes: '1024R'");
    new_ucmd!()
        .args(&["-n", "1024R", "emptyfile.txt"])
        .fails()
        .stderr_is("tail: invalid number of lines: '1024R'");
    #[cfg(not(target_pointer_width = "128"))]
    new_ucmd!()
        .args(&["-c", "1Y", "emptyfile.txt"])
        .fails()
        .stderr_is("tail: invalid number of bytes: '1Y': Value too large for defined data type");
    #[cfg(not(target_pointer_width = "128"))]
    new_ucmd!()
        .args(&["-n", "1Y", "emptyfile.txt"])
        .fails()
        .stderr_is("tail: invalid number of lines: '1Y': Value too large for defined data type");
    #[cfg(target_pointer_width = "32")]
    {
        let sizes = ["1000G", "10T"];
        for size in &sizes {
            new_ucmd!()
                .args(&["-c", size])
                .fails()
                .code_is(1)
                .stderr_only("tail: Insufficient addressable memory");
        }
    }
    new_ucmd!()
        .args(&["-c", "-³"])
        .fails()
        .stderr_is("tail: invalid number of bytes: '³'");
}

#[test]
fn test_tail_num_with_undocumented_sign_bytes() {
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
fn test_tail_bytes_for_funny_files() {
    // gnu/tests/tail-2/tail-c.sh
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
fn test_no_such_file() {
    new_ucmd!()
        .arg("bogusfile")
        .fails()
        .no_stdout()
        .stderr_contains("cannot open 'bogusfile' for reading: No such file or directory");
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
        .stdout_is_fixture("foobar_stdin_default.expected");
}
