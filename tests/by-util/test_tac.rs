// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore axxbxx bxxaxx axxx axxxx xxaxx xxax xxxxa axyz zyax zyxa
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
// FIXME: See https://github.com/uutils/coreutils/issues/4204
#[cfg(not(windows))]
fn test_stdin_default() {
    new_ucmd!()
        .pipe_in("100\n200\n300\n400\n500")
        .run()
        .stdout_is("500400\n300\n200\n100\n");
}

#[test]
// FIXME: See https://github.com/uutils/coreutils/issues/4204
#[cfg(not(windows))]
fn test_stdin_non_newline_separator() {
    new_ucmd!()
        .args(&["-s", ":"])
        .pipe_in("100:200:300:400:500")
        .run()
        .stdout_is("500400:300:200:100:");
}

#[test]
// FIXME: See https://github.com/uutils/coreutils/issues/4204
#[cfg(not(windows))]
fn test_stdin_non_newline_separator_before() {
    new_ucmd!()
        .args(&["-b", "-s", ":"])
        .pipe_in("100:200:300:400:500")
        .run()
        .stdout_is(":500:400:300:200100");
}

#[test]
fn test_single_default() {
    new_ucmd!()
        .arg("prime_per_line.txt")
        .run()
        .stdout_is_fixture("prime_per_line.expected");
}

#[test]
fn test_single_non_newline_separator() {
    new_ucmd!()
        .args(&["-s", ":", "delimited_primes.txt"])
        .run()
        .stdout_is_fixture("delimited_primes.expected");
}

#[test]
fn test_single_non_newline_separator_before() {
    new_ucmd!()
        .args(&["-b", "-s", ":", "delimited_primes.txt"])
        .run()
        .stdout_is_fixture("delimited_primes_before.expected");
}

#[test]
fn test_invalid_input() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    scene
        .ucmd()
        .arg("b")
        .fails()
        .stderr_contains("failed to open 'b' for reading: No such file or directory");

    at.mkdir("a");
    scene
        .ucmd()
        .arg("a")
        .fails()
        .stderr_contains("a: read error: Invalid argument");
}

#[test]
#[cfg(not(windows))] // FIXME: https://github.com/uutils/coreutils/issues/4204
fn test_no_line_separators() {
    new_ucmd!().pipe_in("a").succeeds().stdout_is("a");
}

#[test]
// FIXME: See https://github.com/uutils/coreutils/issues/4204
#[cfg(not(windows))]
fn test_before_trailing_separator_no_leading_separator() {
    new_ucmd!()
        .arg("-b")
        .pipe_in("a\nb\n")
        .succeeds()
        .stdout_is("\n\nba");
}

#[test]
// FIXME: See https://github.com/uutils/coreutils/issues/4204
#[cfg(not(windows))]
fn test_before_trailing_separator_and_leading_separator() {
    new_ucmd!()
        .arg("-b")
        .pipe_in("\na\nb\n")
        .succeeds()
        .stdout_is("\n\nb\na");
}

#[test]
// FIXME: See https://github.com/uutils/coreutils/issues/4204
#[cfg(not(windows))]
fn test_before_leading_separator_no_trailing_separator() {
    new_ucmd!()
        .arg("-b")
        .pipe_in("\na\nb")
        .succeeds()
        .stdout_is("\nb\na");
}

#[test]
// FIXME: See https://github.com/uutils/coreutils/issues/4204
#[cfg(not(windows))]
fn test_before_no_separator() {
    new_ucmd!()
        .arg("-b")
        .pipe_in("ab")
        .succeeds()
        .stdout_is("ab");
}

#[test]
// FIXME: See https://github.com/uutils/coreutils/issues/4204
#[cfg(not(windows))]
fn test_before_empty_file() {
    new_ucmd!().arg("-b").pipe_in("").succeeds().stdout_is("");
}

#[test]
// FIXME: See https://github.com/uutils/coreutils/issues/4204
#[cfg(not(windows))]
fn test_multi_char_separator() {
    new_ucmd!()
        .args(&["-s", "xx"])
        .pipe_in("axxbxx")
        .succeeds()
        .stdout_is("bxxaxx");
}

#[test]
// FIXME: See https://github.com/uutils/coreutils/issues/4204
#[cfg(not(windows))]
fn test_multi_char_separator_overlap() {
    // The right-most pair of "x" characters in the input is treated as
    // the only line separator. That is, "axxx" is interpreted as having
    // one line comprising the string "ax" followed by the line
    // separator "xx".
    new_ucmd!()
        .args(&["-s", "xx"])
        .pipe_in("axxx")
        .succeeds()
        .stdout_is("axxx");

    // Each non-overlapping pair of "x" characters in the input is
    // treated as a line separator. That is, "axxxx" is interpreted as
    // having two lines:
    //
    // * the second line is the empty string "" followed by the line
    //   separator "xx",
    // * the first line is the string "a" followed by the line separator
    //   "xx".
    //
    // The lines are printed in reverse, resulting in "xx" followed by
    // "axx".
    new_ucmd!()
        .args(&["-s", "xx"])
        .pipe_in("axxxx")
        .succeeds()
        .stdout_is("xxaxx");
}

#[test]
// FIXME: See https://github.com/uutils/coreutils/issues/4204
#[cfg(not(windows))]
fn test_multi_char_separator_overlap_before() {
    // With the "-b" option, the line separator is assumed to be at the
    // beginning of the line. In this case, That is, "axxx" is
    // interpreted as having two lines:
    //
    // * the second line is the empty string "" preceded by the line
    //   separator "xx",
    // * the first line is the string "ax" preceded by no line
    //   separator, since there are no more characters preceding it.
    //
    // The lines are printed in reverse, resulting in "xx" followed by
    // "ax".
    new_ucmd!()
        .args(&["-b", "-s", "xx"])
        .pipe_in("axxx")
        .succeeds()
        .stdout_is("xxax");

    // With the "-b" option, the line separator is assumed to be at the
    // beginning of the line. Each non-overlapping pair of "x"
    // characters in the input is treated as a line separator. That is,
    // "axxxx" is interpreted as having three lines:
    //
    // * the third line is the empty string "" preceded by the line
    //   separator "xx" (the last two "x" characters in the input
    //   string),
    // * the second line is the empty string "" preceded by the line
    //   separator "xx" (the first two "x" characters in the input
    //   string),
    // * the first line is the string "a" preceded by no line separator,
    //   since there are no more characters preceding it.
    //
    // The lines are printed in reverse, resulting in "xx" followed by
    // "xx" followed by "a".
    new_ucmd!()
        .args(&["-b", "-s", "xx"])
        .pipe_in("axxxx")
        .succeeds()
        .stdout_is("xxxxa");
}

#[test]
// FIXME: See https://github.com/uutils/coreutils/issues/4204
#[cfg(not(windows))]
fn test_null_separator() {
    new_ucmd!()
        .args(&["-s", ""])
        .pipe_in("a\0b\0")
        .succeeds()
        .stdout_is("b\0a\0");
}

#[test]
// FIXME: See https://github.com/uutils/coreutils/issues/4204
#[cfg(not(windows))]
fn test_regex() {
    new_ucmd!()
        .args(&["-r", "-s", "[xyz]+"])
        .pipe_in("axyz")
        .succeeds()
        .no_stderr()
        .stdout_is("zyax");

    new_ucmd!()
        .args(&["-r", "-s", ":+"])
        .pipe_in("a:b::c:::d::::")
        .succeeds()
        .no_stderr()
        .stdout_is(":::d:::c::b:a:");

    new_ucmd!()
        .args(&["-r", "-s", r"[\+]+[-]+[\+]+"])
        //   line  0     1        2
        //        |--||-----||--------|
        .pipe_in("a+-+b++--++c+d-e+---+")
        .succeeds()
        .no_stderr()
        //   line       2        1    0
        //          |--------||-----||--|
        .stdout_is("c+d-e+---+b++--++a+-+");
}

#[test]
// FIXME: See https://github.com/uutils/coreutils/issues/4204
#[cfg(not(windows))]
fn test_regex_before() {
    new_ucmd!()
        .args(&["-b", "-r", "-s", "[xyz]+"])
        .pipe_in("axyz")
        .succeeds()
        .no_stderr()
        .stdout_is("zyxa");

    new_ucmd!()
        .args(&["-b", "-r", "-s", ":+"])
        .pipe_in(":a::b:::c::::d")
        .succeeds()
        .stdout_is(":d::::c:::b::a");

    // Because `tac` searches for matches of the regular expression from
    // right to left, the second to last line is
    //
    //     +--++b
    //
    // not
    //
    //     ++--++b
    //
    new_ucmd!()
        .args(&["-b", "-r", "-s", r"[\+]+[-]+[\+]+"])
        //   line   0     1       2
        //        |---||----||--------|
        .pipe_in("+-+a++--++b+---+c+d-e")
        .succeeds()
        .no_stderr()
        //   line       2        1    0
        //          |--------||----||---|
        .stdout_is("+---+c+d-e+--++b+-+a+");
}
