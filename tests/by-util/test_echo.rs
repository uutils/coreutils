// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) araba merci

use crate::common::util::TestScenario;

#[test]
fn test_default() {
    new_ucmd!().arg("hi").succeeds().stdout_only("hi\n");
}

#[test]
fn test_no_trailing_newline() {
    new_ucmd!()
        .arg("-n")
        .arg("hi")
        .succeeds()
        .no_stderr()
        .stdout_only("hi");
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
        .run()
        .success()
        .stdout_does_not_contain("-E")
        .stdout_is("-test araba -merci\n");
}

#[test]
fn test_hyphen_values_between() {
    new_ucmd!()
        .arg("test")
        .arg("-E")
        .arg("araba")
        .run()
        .success()
        .stdout_is("test -E araba\n");

    new_ucmd!()
        .arg("dumdum ")
        .arg("dum dum dum")
        .arg("-e")
        .arg("dum")
        .run()
        .success()
        .stdout_is("dumdum  dum dum dum -e dum\n");
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
