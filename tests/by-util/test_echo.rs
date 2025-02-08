// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) araba merci mright

use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util::UCommand;
use uutests::util_name;

#[test]
fn test_default() {
    new_ucmd!().arg("hi").succeeds().stdout_only("hi\n");
}

#[test]
fn test_no_trailing_newline() {
    new_ucmd!().arg("-n").arg("hi").succeeds().stdout_only("hi");
}

#[test]
fn test_escape_alert() {
    new_ucmd!()
        .args(&["-e", "\\a"])
        .succeeds()
        .stdout_only("\x07\n");
}

#[test]
fn test_escape_backslash() {
    new_ucmd!()
        .args(&["-e", "\\\\"])
        .succeeds()
        .stdout_only("\\\n");
}

#[test]
fn test_escape_backspace() {
    new_ucmd!()
        .args(&["-e", "\\b"])
        .succeeds()
        .stdout_only("\x08\n");
}

#[test]
fn test_escape_carriage_return() {
    new_ucmd!()
        .args(&["-e", "\\r"])
        .succeeds()
        .stdout_only("\r\n");
}

#[test]
fn test_escape_escape() {
    new_ucmd!()
        .args(&["-e", "\\e"])
        .succeeds()
        .stdout_only("\x1B\n");
}

#[test]
fn test_escape_form_feed() {
    new_ucmd!()
        .args(&["-e", "\\f"])
        .succeeds()
        .stdout_only("\x0C\n");
}

#[test]
fn test_escape_hex() {
    new_ucmd!()
        .args(&["-e", "\\x41"])
        .succeeds()
        .stdout_only("A\n");
}

#[test]
fn test_escape_short_hex() {
    new_ucmd!()
        .args(&["-e", "foo\\xa bar"])
        .succeeds()
        .stdout_only("foo\n bar\n");
}

#[test]
fn test_escape_no_hex() {
    new_ucmd!()
        .args(&["-e", "foo\\x bar"])
        .succeeds()
        .stdout_only("foo\\x bar\n");
}

#[test]
fn test_escape_one_slash() {
    new_ucmd!()
        .args(&["-e", "foo\\ bar"])
        .succeeds()
        .stdout_only("foo\\ bar\n");
}

#[test]
fn test_escape_one_slash_multi() {
    new_ucmd!()
        .args(&["-e", "foo\\", "bar"])
        .succeeds()
        .stdout_only("foo\\ bar\n");
}

#[test]
fn test_escape_newline() {
    new_ucmd!()
        .args(&["-e", "\\na"])
        .succeeds()
        .stdout_only("\na\n");
}

#[test]
fn test_escape_override() {
    new_ucmd!()
        .args(&["-e", "-E", "\\na"])
        .succeeds()
        .stdout_only("\\na\n");

    new_ucmd!()
        .args(&["-E", "-e", "\\na"])
        .succeeds()
        .stdout_only("\na\n");
}

#[test]
fn test_escape_no_further_output() {
    new_ucmd!()
        .args(&["-e", "a\\cb", "c"])
        .succeeds()
        .stdout_only("a");
}

#[test]
fn test_escape_octal() {
    new_ucmd!()
        .args(&["-e", "\\0100"])
        .succeeds()
        .stdout_only("@\n");
}

#[test]
fn test_escape_short_octal() {
    new_ucmd!()
        .args(&["-e", "foo\\040bar"])
        .succeeds()
        .stdout_only("foo bar\n");
}

#[test]
fn test_escape_nul() {
    new_ucmd!()
        .args(&["-e", "foo\\0 bar"])
        .succeeds()
        .stdout_only("foo\0 bar\n");
}

#[test]
fn test_escape_octal_invalid_digit() {
    new_ucmd!()
        .args(&["-e", "foo\\08 bar"])
        .succeeds()
        .stdout_only("foo\u{0}8 bar\n");
}

#[test]
fn test_escape_tab() {
    new_ucmd!()
        .args(&["-e", "\\t"])
        .succeeds()
        .stdout_only("\t\n");
}

#[test]
fn test_escape_vertical_tab() {
    new_ucmd!()
        .args(&["-e", "\\v"])
        .succeeds()
        .stdout_only("\x0B\n");
}

#[test]
fn test_disable_escapes() {
    let input_str = "\\a \\\\ \\b \\r \\e \\f \\x41 \\n a\\cb \\u0100 \\t \\v";
    new_ucmd!()
        .arg("-E")
        .arg(input_str)
        .succeeds()
        .stdout_only(format!("{input_str}\n"));
}

#[test]
fn test_hyphen_value() {
    new_ucmd!().arg("-abc").succeeds().stdout_is("-abc\n");
}

#[test]
fn test_multiple_hyphen_values() {
    new_ucmd!()
        .args(&["-abc", "-def", "-edf"])
        .succeeds()
        .stdout_is("-abc -def -edf\n");
}

#[test]
fn test_hyphen_values_inside_string() {
    new_ucmd!()
        .arg("'\"\n'CXXFLAGS=-g -O2'\n\"'") // spell-checker:disable-line
        .succeeds()
        .stdout_contains("CXXFLAGS"); // spell-checker:disable-line
}

#[test]
fn test_hyphen_values_at_start() {
    new_ucmd!()
        .arg("-E")
        .arg("-test")
        .arg("araba")
        .arg("-merci")
        .succeeds()
        .stdout_does_not_contain("-E")
        .stdout_is("-test araba -merci\n");
}

#[test]
fn test_hyphen_values_between() {
    new_ucmd!()
        .arg("test")
        .arg("-E")
        .arg("araba")
        .succeeds()
        .stdout_is("test -E araba\n");

    new_ucmd!()
        .arg("dumdum ")
        .arg("dum dum dum")
        .arg("-e")
        .arg("dum")
        .succeeds()
        .stdout_is("dumdum  dum dum dum -e dum\n");
}

#[test]
fn test_double_hyphens() {
    new_ucmd!().arg("--").succeeds().stdout_only("--\n");
    new_ucmd!()
        .arg("--")
        .arg("--")
        .succeeds()
        .stdout_only("-- --\n");
}

#[test]
fn wrapping_octal() {
    // Some odd behavior of GNU. Values of \0400 and greater do not fit in the
    // u8 that we write to stdout. So we test that it wraps:
    //
    // We give it this input:
    //     \o501 = 1_0100_0001 (yes, **9** bits)
    // This should be wrapped into:
    //     \o101 = 'A' = 0100_0001,
    // because we only write a single character
    new_ucmd!()
        .arg("-e")
        .arg("\\0501")
        .succeeds()
        .stdout_is("A\n");
}

#[test]
fn old_octal_syntax() {
    new_ucmd!()
        .arg("-e")
        .arg("\\1foo")
        .succeeds()
        .stdout_is("\x01foo\n");

    new_ucmd!()
        .arg("-e")
        .arg("\\43foo")
        .succeeds()
        .stdout_is("#foo\n");

    new_ucmd!()
        .arg("-e")
        .arg("\\101 foo")
        .succeeds()
        .stdout_is("A foo\n");

    new_ucmd!()
        .arg("-e")
        .arg("\\1011")
        .succeeds()
        .stdout_is("A1\n");
}

#[test]
fn partial_version_argument() {
    new_ucmd!().arg("--ver").succeeds().stdout_is("--ver\n");
}

#[test]
fn partial_help_argument() {
    new_ucmd!().arg("--he").succeeds().stdout_is("--he\n");
}

#[test]
fn multibyte_escape_unicode() {
    // spell-checker:disable-next-line
    // Tests suggested by kkew3
    // https://github.com/uutils/coreutils/issues/6741

    // \u{1F602} is:
    //
    // "Face with Tears of Joy"
    // U+1F602
    // "😂"

    new_ucmd!()
        .args(&["-e", r"\xf0\x9f\x98\x82"])
        .succeeds()
        .stdout_only("\u{1F602}\n");

    new_ucmd!()
        .args(&["-e", r"\x41\xf0\x9f\x98\x82\x42"])
        .succeeds()
        .stdout_only("A\u{1F602}B\n");

    new_ucmd!()
        .args(&["-e", r"\xf0\x41\x9f\x98\x82"])
        .succeeds()
        .stdout_only_bytes(b"\xF0A\x9F\x98\x82\n");

    new_ucmd!()
        .args(&["-e", r"\x41\xf0\c\x9f\x98\x82"])
        .succeeds()
        .stdout_only_bytes(b"A\xF0");
}

#[test]
fn non_utf_8_hex_round_trip() {
    new_ucmd!()
        .args(&["-e", r"\xFF"])
        .succeeds()
        .stdout_only_bytes(b"\xFF\n");
}

#[test]
fn nine_bit_octal() {
    const RESULT: &[u8] = b"\xFF\n";

    new_ucmd!()
        .args(&["-e", r"\0777"])
        .succeeds()
        .stdout_only_bytes(RESULT);

    new_ucmd!()
        .args(&["-e", r"\777"])
        .succeeds()
        .stdout_only_bytes(RESULT);
}

#[test]
#[cfg(target_family = "unix")]
fn non_utf_8() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    // ISO-8859-1 encoded text
    // spell-checker:disable
    const INPUT_AND_OUTPUT: &[u8] =
        b"Swer an rehte g\xFCete wendet s\xEEn gem\xFCete, dem volget s\xE6lde und \xEAre.";
    // spell-checker:enable

    let os_str = OsStr::from_bytes(INPUT_AND_OUTPUT);

    new_ucmd!()
        .arg("-n")
        .arg(os_str)
        .succeeds()
        .stdout_only_bytes(INPUT_AND_OUTPUT);
}

#[test]
fn slash_eight_off_by_one() {
    new_ucmd!()
        .args(&["-e", "-n", r"\8"])
        .succeeds()
        .stdout_only(r"\8");
}

#[test]
fn test_normalized_newlines_stdout_is() {
    let res = new_ucmd!().args(&["-ne", "A\r\nB\nC"]).run();

    res.normalized_newlines_stdout_is("A\r\nB\nC");
    res.normalized_newlines_stdout_is("A\nB\nC");
    res.normalized_newlines_stdout_is("A\nB\r\nC");
}

#[test]
fn test_normalized_newlines_stdout_is_fail() {
    new_ucmd!()
        .args(&["-ne", "A\r\nB\nC"])
        .run()
        .stdout_is("A\r\nB\nC");
}

#[test]
fn test_cmd_result_stdout_check_and_stdout_str_check() {
    let result = new_ucmd!().arg("Hello world").run();

    result.stdout_str_check(|stdout| stdout.ends_with("world\n"));
    result.stdout_check(|stdout| stdout.get(0..2).unwrap().eq(b"He"));
    result.no_stderr();
}

#[test]
fn test_cmd_result_stderr_check_and_stderr_str_check() {
    let ts = TestScenario::new("echo");

    let result = UCommand::new()
        .arg(format!(
            "{} {} Hello world >&2",
            ts.bin_path.display(),
            ts.util_name
        ))
        .run();

    result.stderr_str_check(|stderr| stderr.ends_with("world\n"));
    result.stderr_check(|stdout| stdout.get(0..2).unwrap().eq(b"He"));
    result.no_stdout();
}

#[test]
fn test_cmd_result_stdout_str_check_when_false_then_panics() {
    new_ucmd!()
        .args(&["-e", "\\f"])
        .succeeds()
        .stdout_only("\x0C\n");
}

#[cfg(unix)]
#[test]
fn test_cmd_result_signal_when_normal_exit_then_no_signal() {
    let result = TestScenario::new("echo").ucmd().run();
    assert!(result.signal().is_none());
}

mod posixly_correct {
    use super::*;

    #[test]
    fn ignore_options() {
        for arg in ["--help", "--version", "-E -n 'foo'", "-nE 'foo'"] {
            new_ucmd!()
                .env("POSIXLY_CORRECT", "1")
                .arg(arg)
                .succeeds()
                .stdout_only(format!("{arg}\n"));
        }
    }

    #[test]
    fn process_n_option() {
        new_ucmd!()
            .env("POSIXLY_CORRECT", "1")
            .args(&["-n", "foo"])
            .succeeds()
            .stdout_only("foo");

        // ignore -E & process escapes
        new_ucmd!()
            .env("POSIXLY_CORRECT", "1")
            .args(&["-n", "-E", "foo\\cbar"])
            .succeeds()
            .stdout_only("foo");
    }

    #[test]
    fn process_escapes() {
        new_ucmd!()
            .env("POSIXLY_CORRECT", "1")
            .arg("foo\\n")
            .succeeds()
            .stdout_only("foo\n\n");

        new_ucmd!()
            .env("POSIXLY_CORRECT", "1")
            .arg("foo\\tbar")
            .succeeds()
            .stdout_only("foo\tbar\n");

        new_ucmd!()
            .env("POSIXLY_CORRECT", "1")
            .arg("foo\\ctbar")
            .succeeds()
            .stdout_only("foo");
    }
}

#[test]
fn test_child_when_run_with_a_non_blocking_util() {
    new_ucmd!()
        .arg("hello world")
        .run()
        .success()
        .stdout_only("hello world\n");
}

// Test basically that most of the methods of UChild are working
#[test]
fn test_uchild_when_run_no_wait_with_a_non_blocking_util() {
    let mut child = new_ucmd!().arg("hello world").run_no_wait();

    // check `child.is_alive()` and `child.delay()` is working
    let mut trials = 10;
    while child.is_alive() {
        assert!(
            trials > 0,
            "Assertion failed: child process is still alive."
        );

        child.delay(500);
        trials -= 1;
    }

    assert!(!child.is_alive());

    // check `child.is_not_alive()` is working
    assert!(child.is_not_alive());

    // check the current output is correct
    std::assert_eq!(child.stdout(), "hello world\n");
    assert!(child.stderr().is_empty());

    // check the current output of echo is empty. We already called `child.stdout()` and `echo`
    // exited so there's no additional output after the first call of `child.stdout()`
    assert!(child.stdout().is_empty());
    assert!(child.stderr().is_empty());

    // check that we're still able to access all output of the child process, even after exit
    // and call to `child.stdout()`
    std::assert_eq!(child.stdout_all(), "hello world\n");
    assert!(child.stderr_all().is_empty());

    // we should be able to call kill without panics, even if the process already exited
    child.make_assertion().is_not_alive();
    child.kill();

    // we should be able to call wait without panics and apply some assertions
    child.wait().unwrap().code_is(0).no_stdout().no_stderr();
}
