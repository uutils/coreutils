// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore axxbxx bxxaxx axxx axxxx xxaxx xxax xxxxa axyz zyax zyxa bbaaa aaabc bcdddd cddddaaabc xyzabc abcxyzabc nbbaaa EISDIR
#[cfg(target_os = "linux")]
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
#[cfg(target_os = "linux")]
fn test_tac_non_utf8_paths() {
    use std::os::unix::ffi::OsStringExt;
    let (at, mut ucmd) = at_and_ucmd!();

    let filename = std::ffi::OsString::from_vec(vec![0xFF, 0xFE]);
    std::fs::write(at.plus(&filename), b"line1\nline2\nline3\n").unwrap();

    ucmd.arg(&filename)
        .succeeds()
        .stdout_is("line3\nline2\nline1\n");
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_stdin_default() {
    new_ucmd!()
        .pipe_in("100\n200\n300\n400\n500")
        .succeeds()
        .stdout_is("500400\n300\n200\n100\n");
}

#[test]
fn test_stdin_non_newline_separator() {
    new_ucmd!()
        .args(&["-s", ":"])
        .pipe_in("100:200:300:400:500")
        .succeeds()
        .stdout_is("500400:300:200:100:");
}

#[test]
fn test_stdin_non_newline_separator_before() {
    new_ucmd!()
        .args(&["-b", "-s", ":"])
        .pipe_in("100:200:300:400:500")
        .succeeds()
        .stdout_is(":500:400:300:200100");
}

#[test]
fn test_single_default() {
    new_ucmd!()
        .arg("prime_per_line.txt")
        .succeeds()
        .stdout_is_fixture("prime_per_line.expected");
}

#[test]
fn test_single_non_newline_separator() {
    new_ucmd!()
        .args(&["-s", ":", "delimited_primes.txt"])
        .succeeds()
        .stdout_is_fixture("delimited_primes.expected");
}

#[test]
fn test_single_non_newline_separator_before() {
    new_ucmd!()
        .args(&["-b", "-s", ":", "delimited_primes.txt"])
        .succeeds()
        .stdout_is_fixture("delimited_primes_before.expected");
}

#[test]
fn test_invalid_input() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    #[cfg(not(windows))]
    let not_found_err = "failed to open 'b' for reading: No such file or directory";
    #[cfg(windows)]
    let not_found_err =
        "failed to open 'b' for reading: The system cannot find the file specified.";

    scene.ucmd().arg("b").fails().stderr_contains(not_found_err);

    at.mkdir("a");
    // On Unix, File::open succeeds on directories but read_to_end fails with EISDIR.
    // On Windows, File::open on a directory fails with "Access is denied".
    #[cfg(not(windows))]
    let dir_err = "a: read error: Is a directory";
    #[cfg(windows)]
    let dir_err = "failed to open 'a' for reading: Access is denied";

    scene.ucmd().arg("a").fails().stderr_contains(dir_err);
}

#[test]
fn test_no_line_separators() {
    new_ucmd!().pipe_in("a").succeeds().stdout_is("a");
}

#[test]
fn test_before_trailing_separator_no_leading_separator() {
    new_ucmd!()
        .arg("-b")
        .pipe_in("a\nb\n")
        .succeeds()
        .stdout_is("\n\nba");
}

#[test]
fn test_before_trailing_separator_and_leading_separator() {
    new_ucmd!()
        .arg("-b")
        .pipe_in("\na\nb\n")
        .succeeds()
        .stdout_is("\n\nb\na");
}

#[test]
fn test_before_leading_separator_no_trailing_separator() {
    new_ucmd!()
        .arg("-b")
        .pipe_in("\na\nb")
        .succeeds()
        .stdout_is("\nb\na");
}

#[test]
fn test_before_no_separator() {
    new_ucmd!()
        .arg("-b")
        .pipe_in("ab")
        .succeeds()
        .stdout_is("ab");
}

#[test]
fn test_before_empty_file() {
    new_ucmd!().arg("-b").pipe_in("").succeeds().stdout_is("");
}

#[test]
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
fn test_null_separator() {
    new_ucmd!()
        .args(&["-s", ""])
        .pipe_in("a\0b\0")
        .succeeds()
        .stdout_is("b\0a\0");
}

#[test]
#[cfg(unix)]
fn test_non_utf8_separator() {
    use std::os::unix::ffi::OsStringExt;
    new_ucmd!()
        .arg("-s")
        .arg(std::ffi::OsString::from_vec(b"\xe9".to_vec()))
        .pipe_in(b"1\xe92".to_vec())
        .succeeds()
        .no_stderr()
        .stdout_is_bytes(b"21\xe9");
}

#[test]
#[cfg(unix)]
fn test_non_utf8_regex_separator() {
    use std::os::unix::ffi::OsStringExt;

    new_ucmd!()
        .args(&["-r", "-s"])
        .arg(std::ffi::OsString::from_vec(b"\xe9".to_vec()))
        .pipe_in(b"a.b.\xe9c.d?".to_vec())
        .succeeds()
        .no_stderr()
        .stdout_is_bytes(b"c.d?a.b.\xe9");

    new_ucmd!()
        .args(&["-r", "-s"])
        .arg(std::ffi::OsString::from_vec(b"[.\xe9?]".to_vec()))
        .pipe_in(b"a.b.\xe9c.d?".to_vec())
        .succeeds()
        .no_stderr()
        .stdout_is_bytes(b"d?\xe9c.b.a.");

    new_ucmd!()
        .args(&["-r", "-s"])
        .arg(std::ffi::OsString::from_vec(b"[.?]\xe9".to_vec()))
        .pipe_in(b"a.b\xe9c.d?")
        .succeeds()
        .no_stderr()
        .stdout_is_bytes(b"a.b\xe9c.d?");

    new_ucmd!()
        .args(&["-r", "-s"])
        .arg(std::ffi::OsString::from_vec(b"[.?]\xe9".to_vec()))
        .pipe_in(b"a.b[.?]\xe9c.d?")
        .succeeds()
        .no_stderr()
        .stdout_is_bytes(b"a.b[.?]\xe9c.d?");

    new_ucmd!()
        .args(&["-r", "-s"])
        .arg(std::ffi::OsString::from_vec(b"[.?]\xe9".to_vec()))
        .pipe_in(b"a.\xe9b")
        .succeeds()
        .no_stderr()
        .stdout_is_bytes(b"ba.\xe9");
}

#[test]
fn test_regex_bare_anchors() {
    new_ucmd!()
        .args(&["-r", "-s", "^"])
        .pipe_in("a\nb\nc\n")
        .succeeds()
        .no_stderr()
        .stdout_is_bytes(b"c\nb\na\n");

    new_ucmd!()
        .args(&["-r", "-s", "$"])
        .pipe_in("a\nb\nc\n")
        .succeeds()
        .stdout_is_bytes(b"\n\nc\nba");

    new_ucmd!()
        .args(&["-r", "-s", "^$"])
        .pipe_in("a\nb\nc\n")
        .succeeds()
        .stdout_is_bytes(b"a\nb\nc\n");
}

#[test]
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

#[cfg(target_os = "linux")]
#[test]
fn test_failed_write_is_reported() {
    new_ucmd!()
        .pipe_in("hello")
        .set_stdout(std::fs::File::create("/dev/full").unwrap())
        .fails()
        .stderr_is("tac: failed to write to stdout: No space left on device\n");
}

#[cfg(target_os = "linux")]
#[test]
fn test_stdin_bad_tmpdir_fallback() {
    // When TMPDIR is invalid, tac falls back to reading stdin directly into memory
    new_ucmd!()
        .env("TMPDIR", "/nonexistent/dir")
        .arg("-")
        .pipe_in("a\nb\nc\n")
        .succeeds()
        .stdout_is("c\nb\na\n");
}

#[test]
fn test_regex_or_operator() {
    new_ucmd!()
        .args(&["-r", "-s", r"[^x]\|x"])
        .pipe_in("abc")
        .succeeds()
        .stdout_is("cba");
}

#[test]
fn test_unescaped_middle_anchor() {
    new_ucmd!()
        .args(&["-r", "-s", r"1^2"])
        .pipe_in("111^222")
        .succeeds()
        .stdout_is("22111^2");

    new_ucmd!()
        .args(&["-r", "-s", r"a$b"])
        .pipe_in("aaa$bbb")
        .succeeds()
        .stdout_is("bbaaa$b");
}

#[test]
fn test_escaped_middle_anchor() {
    new_ucmd!()
        .args(&["-r", "-s", r"c\^b"])
        .pipe_in("aaabc^bcdddd")
        .succeeds()
        .stdout_is("cddddaaabc^b");

    new_ucmd!()
        .args(&["-r", "-s", r"c\$b"])
        .pipe_in("aaabc$bcdddd")
        .succeeds()
        .stdout_is("cddddaaabc$b");
}

#[test]
fn test_regular_start_anchor() {
    new_ucmd!()
        .args(&["-r", "-s", r"^abc"])
        .pipe_in("xyzabc123abc")
        .succeeds()
        .stdout_is("xyzabc123abc");

    new_ucmd!()
        .args(&["-r", "-s", r"^b"])
        .pipe_in("aaa\nbbb\nccc\n")
        .succeeds()
        .stdout_is("bb\nccc\naaa\nb");
}

#[test]
fn test_regular_end_anchor() {
    new_ucmd!()
        .args(&["-r", "-s", r"abc$"])
        .pipe_in("123abcxyzabc")
        .succeeds()
        .stdout_is("123abcxyzabc");

    new_ucmd!()
        .args(&["-r", "-s", r"b$"])
        .pipe_in("aaa\nbbb\nccc\n")
        .succeeds()
        .stdout_is("\nccc\nbbaaa\nb");
}
