// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) autoformat nocheck FILENUM

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "netbsd"))]
use std::fs::OpenOptions;
#[cfg(unix)]
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
#[cfg(windows)]
use std::{ffi::OsString, os::windows::ffi::OsStringExt};
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn empty_files() {
    new_ucmd!()
        .arg("empty.txt")
        .arg("empty.txt")
        .succeeds()
        .stdout_only("");

    new_ucmd!()
        .arg("empty.txt")
        .arg("fields_1.txt")
        .succeeds()
        .stdout_only("");

    new_ucmd!()
        .arg("fields_1.txt")
        .arg("empty.txt")
        .succeeds()
        .stdout_only("");
}

#[test]
fn empty_intersection() {
    new_ucmd!()
        .arg("fields_1.txt")
        .arg("fields_2.txt")
        .arg("-2")
        .arg("2")
        .succeeds()
        .stdout_only("");
}

#[test]
fn default_arguments() {
    new_ucmd!()
        .arg("fields_1.txt")
        .arg("fields_2.txt")
        .succeeds()
        .stdout_only_fixture("default.expected");
}

#[test]
fn only_whitespace_separators_merge() {
    new_ucmd!()
        .arg("contiguous_separators.txt")
        .arg("-")
        .pipe_in(" a  ,c ")
        .succeeds()
        .stdout_only("a ,,,b ,c \n");

    new_ucmd!()
        .arg("contiguous_separators.txt")
        .arg("-t")
        .arg(",")
        .arg("-")
        .pipe_in(" a  ,c ")
        .succeeds()
        .stdout_only(" a  ,,,b,c \n");
}

#[test]
fn different_fields() {
    new_ucmd!()
        .arg("fields_2.txt")
        .arg("fields_4.txt")
        .arg("-j")
        .arg("2")
        .succeeds()
        .stdout_only_fixture("different_fields.expected");

    new_ucmd!()
        .arg("fields_2.txt")
        .arg("fields_4.txt")
        .arg("-1")
        .arg("2")
        .arg("-2")
        .arg("2")
        .succeeds()
        .stdout_only_fixture("different_fields.expected");
}

#[test]
fn different_field() {
    new_ucmd!()
        .arg("fields_2.txt")
        .arg("fields_3.txt")
        .arg("-2")
        .arg("2")
        .succeeds()
        .stdout_only_fixture("different_field.expected");
}

#[test]
fn out_of_bounds_fields() {
    new_ucmd!()
        .arg("fields_1.txt")
        .arg("fields_4.txt")
        .arg("-1")
        .arg("3")
        .arg("-2")
        .arg("5")
        .succeeds()
        .stdout_only_fixture("out_of_bounds_fields.expected");

    new_ucmd!()
        .arg("fields_1.txt")
        .arg("fields_4.txt")
        .arg("-j")
        .arg("100000000000000000000") // > usize::MAX for 64 bits
        .succeeds()
        .stdout_only_fixture("out_of_bounds_fields.expected");
}

#[test]
fn unpaired_lines() {
    new_ucmd!()
        .arg("fields_2.txt")
        .arg("fields_3.txt")
        .arg("-a")
        .arg("1")
        .succeeds()
        .stdout_only_fixture("fields_2.txt");

    new_ucmd!()
        .arg("fields_3.txt")
        .arg("fields_2.txt")
        .arg("-1")
        .arg("2")
        .arg("-a")
        .arg("2")
        .succeeds()
        .stdout_only_fixture("unpaired_lines.expected");

    new_ucmd!()
        .arg("fields_3.txt")
        .arg("fields_2.txt")
        .arg("-1")
        .arg("2")
        .arg("-a")
        .arg("1")
        .arg("-a")
        .arg("2")
        .succeeds()
        .stdout_only_fixture("unpaired_lines_outer.expected");
}

#[test]
fn suppress_joined() {
    new_ucmd!()
        .arg("fields_3.txt")
        .arg("fields_2.txt")
        .arg("-1")
        .arg("2")
        .arg("-v")
        .arg("2")
        .succeeds()
        .stdout_only_fixture("suppress_joined.expected");

    new_ucmd!()
        .arg("fields_3.txt")
        .arg("fields_2.txt")
        .arg("-1")
        .arg("2")
        .arg("-a")
        .arg("1")
        .arg("-v")
        .arg("2")
        .succeeds()
        .stdout_only_fixture("suppress_joined_outer.expected");
}

#[test]
fn case_insensitive() {
    new_ucmd!()
        .arg("capitalized.txt")
        .arg("fields_3.txt")
        .arg("-i")
        .succeeds()
        .stdout_only_fixture("case_insensitive.expected");
}

#[test]
fn semicolon_separated() {
    new_ucmd!()
        .arg("semicolon_fields_1.txt")
        .arg("semicolon_fields_2.txt")
        .arg("-t")
        .arg(";")
        .succeeds()
        .stdout_only_fixture("semicolon_separated.expected");
}

#[test]
fn new_line_separated() {
    new_ucmd!()
        .arg("-")
        .arg("fields_2.txt")
        .arg("-t")
        .arg("")
        .pipe_in("1 a\n1 b\n8 h\n")
        .succeeds()
        .stdout_only("1 a\n8 h\n");
}

#[test]
fn tab_multi_character() {
    new_ucmd!()
        .arg("semicolon_fields_1.txt")
        .arg("semicolon_fields_2.txt")
        .arg("-t")
        .arg("ab")
        .fails()
        .stderr_is("join: multi-character tab 'ab'\n");
}

#[test]
fn tab_hyphen_leading_as_separate_arg() {
    // A hyphen-leading separator value passed as its own argument (not
    // attached with `-t-x`/`=`) must not be mistaken for a new,
    // unrecognized flag.
    new_ucmd!()
        .arg("semicolon_fields_1.txt")
        .arg("semicolon_fields_2.txt")
        .arg("-t")
        .arg("-x")
        .fails()
        .stderr_is("join: multi-character tab '-x'\n");
}

#[test]
fn default_format() {
    new_ucmd!()
        .arg("fields_1.txt")
        .arg("fields_2.txt")
        .arg("-o")
        .arg("1.1 2.2")
        .succeeds()
        .stdout_only_fixture("default.expected");

    new_ucmd!()
        .arg("fields_1.txt")
        .arg("fields_2.txt")
        .arg("-o")
        .arg("0 2.2")
        .succeeds()
        .stdout_only_fixture("default.expected");
}

#[test]
fn repeated_o_accumulates_fields() {
    // GNU lets -o repeat; the fields accumulate in the order they appear,
    // here giving the reverse of the default format.
    new_ucmd!()
        .arg("fields_1.txt")
        .arg("fields_2.txt")
        .arg("-o")
        .arg("2.2")
        .arg("-o")
        .arg("1.1")
        .succeeds()
        .stdout_only("a 1\nb 2\nc 3\ne 5\nh 8\n");
}

#[test]
fn repeated_o_ignores_auto_when_mixed() {
    // When -o is repeated and not every value is `auto`, each `auto` is
    // ignored: only the explicit fields are printed, repeats included.
    new_ucmd!()
        .arg("fields_1.txt")
        .arg("fields_2.txt")
        .arg("-o")
        .arg("auto")
        .arg("-o")
        .arg("2.2 2.2 1.1")
        .succeeds()
        .stdout_only("a a 1\nb b 2\nc c 3\ne e 5\nh h 8\n");
}

#[test]
fn repeated_o_auto_stays_auto() {
    // `auto` still applies when every -o value is `auto`.
    new_ucmd!()
        .arg("fields_1.txt")
        .arg("fields_2.txt")
        .arg("-o")
        .arg("auto")
        .arg("-o")
        .arg("auto")
        .succeeds()
        .stdout_only_fixture("default.expected");
}

#[test]
fn unpaired_lines_format() {
    new_ucmd!()
        .arg("fields_2.txt")
        .arg("fields_3.txt")
        .arg("-a")
        .arg("2")
        .arg("-o")
        .arg("1.2 1.1 2.4 2.3 2.2 0")
        .succeeds()
        .stdout_only_fixture("unpaired_lines_format.expected");
}

#[test]
fn autoformat() {
    new_ucmd!()
        .arg("fields_2.txt")
        .arg("different_lengths.txt")
        .arg("-o")
        .arg("auto")
        .succeeds()
        .stdout_only_fixture("autoformat.expected");

    new_ucmd!()
        .arg("-")
        .arg("fields_2.txt")
        .arg("-o")
        .arg("auto")
        .pipe_in("1 x y z\n2 p")
        .succeeds()
        .stdout_only("1 x y z a\n2 p   b\n");

    new_ucmd!()
        .arg("-")
        .arg("fields_2.txt")
        .arg("-a")
        .arg("1")
        .arg("-o")
        .arg("auto")
        .arg("-e")
        .arg(".")
        .pipe_in("1 x y z\n2 p\n99 a b\n")
        .succeeds()
        .stdout_only("1 x y z a\n2 p . . b\n99 a b . .\n");
}

#[test]
fn empty_format() {
    new_ucmd!()
        .arg("fields_1.txt")
        .arg("fields_2.txt")
        .arg("-o")
        .arg("")
        .fails()
        .stderr_is("join: invalid file number in field spec: ''\n");
}

#[test]
fn empty_key() {
    new_ucmd!()
        .arg("fields_1.txt")
        .arg("empty.txt")
        .arg("-j")
        .arg("2")
        .arg("-a")
        .arg("1")
        .arg("-e")
        .arg("x")
        .succeeds()
        .stdout_only_fixture("empty_key.expected");
}

#[test]
fn missing_format_fields() {
    new_ucmd!()
        .arg("fields_2.txt")
        .arg("different_lengths.txt")
        .arg("-o")
        .arg("0 1.2 2.4")
        .arg("-e")
        .arg("x")
        .succeeds()
        .stdout_only_fixture("missing_format_fields.expected");
}

#[test]
fn empty_fields_use_empty_filler() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    // A blank line splits into a single empty field, so the join field is
    // present but zero length rather than missing.
    at.write("blank", "hello\n\n\n");

    ts.ucmd()
        .args(&["-e", "EMPTY", "blank", "blank"])
        .succeeds()
        .stdout_only("hello\nEMPTY\nEMPTY\nEMPTY\nEMPTY\n");

    // The same holds for a line containing only whitespace.
    at.write("spaces", "   \n");

    ts.ucmd()
        .args(&["-e", "EMPTY", "-o", "0,1.1", "spaces", "spaces"])
        .succeeds()
        .stdout_only("EMPTY EMPTY\n");

    // A field between two adjacent separators is also present but empty.
    at.write("gap", "a,,b\n");

    ts.ucmd()
        .args(&[
            "-t",
            ",",
            "-e",
            "EMPTY",
            "-o",
            "0,1.1,1.2,1.3",
            "gap",
            "gap",
        ])
        .succeeds()
        .stdout_only("a,a,EMPTY,b\n");

    ts.ucmd()
        .args(&["-t", ",", "-e", "EMPTY", "gap", "gap"])
        .succeeds()
        .stdout_only("a,EMPTY,b,EMPTY,b\n");
}

#[test]
fn empty_fields_kept_without_empty_filler() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.write("gap", "a,,b\n");

    // Without -e an empty field stays empty.
    ts.ucmd()
        .args(&["-t", ",", "-o", "0,1.1,1.2,1.3", "gap", "gap"])
        .succeeds()
        .stdout_only("a,a,,b\n");

    // Passing an empty string to -e is equivalent to not passing it at all.
    ts.ucmd()
        .args(&["-t", ",", "-e", "", "-o", "0,1.1,1.2,1.3", "gap", "gap"])
        .succeeds()
        .stdout_only("a,a,,b\n");
}

#[test]
fn nocheck_order() {
    new_ucmd!()
        .arg("fields_1.txt")
        .arg("fields_2.txt")
        .arg("--nocheck-order")
        .succeeds()
        .stdout_only_fixture("default.expected");
}

#[test]
fn wrong_line_order() {
    let ts = TestScenario::new(util_name!());
    new_ucmd!()
        .arg("fields_2.txt")
        .arg("fields_4.txt")
        .fails()
        .stdout_contains("7 g f 4 fg")
        .stderr_is(format!(
            "{0}: fields_4.txt:5: is not sorted: 11 g 5 gh\n{0}: input is not in sorted order\n",
            ts.util_name
        ));

    new_ucmd!()
        .arg("--check-order")
        .arg("fields_2.txt")
        .arg("fields_4.txt")
        .fails()
        .stdout_does_not_contain("7 g f 4 fg")
        .stderr_is(format!(
            "{0}: fields_4.txt:5: is not sorted: 11 g 5 gh\n",
            ts.util_name
        ));
}

#[test]
fn both_files_wrong_line_order() {
    let ts = TestScenario::new(util_name!());
    new_ucmd!()
        .arg("fields_4.txt")
        .arg("fields_5.txt")
        .fails()
        .stdout_contains("5 e 3 ef")
        .stderr_is(format!(
            "{0}: fields_5.txt:4: is not sorted: 3\n{0}: fields_4.txt:5: is not sorted: 11 g 5 gh\n{0}: input is not in sorted order\n",
            ts.util_name
        ));

    new_ucmd!()
        .arg("--check-order")
        .arg("fields_4.txt")
        .arg("fields_5.txt")
        .fails()
        .stdout_does_not_contain("5 e 3 ef")
        .stderr_is(format!(
            "{0}: fields_5.txt:4: is not sorted: 3\n",
            ts.util_name
        ));
}

#[test]
fn headers() {
    new_ucmd!()
        .arg("header_1.txt")
        .arg("header_2.txt")
        .arg("--header")
        .succeeds()
        .stdout_only_fixture("header.expected");
}

#[test]
fn headers_autoformat() {
    new_ucmd!()
        .arg("header_1.txt")
        .arg("header_2.txt")
        .arg("--header")
        .arg("-o")
        .arg("auto")
        .succeeds()
        .stdout_only_fixture("header_autoformat.expected");
}

#[test]
fn single_file_with_header() {
    new_ucmd!()
        .arg("capitalized.txt")
        .arg("empty.txt")
        .arg("--header")
        .succeeds()
        .stdout_is("A 1\n");

    new_ucmd!()
        .arg("empty.txt")
        .arg("capitalized.txt")
        .arg("--header")
        .succeeds()
        .stdout_is("A 1\n");
}

#[test]
fn non_line_feeds() {
    new_ucmd!()
        .arg("non-line_feeds_1.txt")
        .arg("non-line_feeds_2.txt")
        .succeeds()
        .stdout_only_fixture("non-line_feeds.expected");
}

#[test]
fn non_unicode() {
    new_ucmd!()
        .arg("non-unicode_1.bin")
        .arg("non-unicode_2.bin")
        .succeeds()
        .stdout_only_fixture("non-unicode.expected");

    #[cfg(unix)]
    {
        let non_utf8_byte: u8 = 167;
        new_ucmd!()
            .arg("-t")
            .arg(OsStr::from_bytes(&[non_utf8_byte]))
            .arg("non-unicode_1.bin")
            .arg("non-unicode_2.bin")
            .succeeds()
            .stdout_only_fixture("non-unicode_sep.expected");

        new_ucmd!()
            .arg("-t")
            .arg(OsStr::from_bytes(&[non_utf8_byte, non_utf8_byte]))
            .arg("non-unicode_1.bin")
            .arg("non-unicode_2.bin")
            .fails()
            .stderr_is("join: non-UTF-8 multi-byte tab\n");
    }

    #[cfg(windows)]
    {
        let invalid_utf16: OsString = OsStringExt::from_wide(&[0xD800]);
        new_ucmd!()
            .arg("-t")
            .arg(&invalid_utf16)
            .arg("non-unicode_1.bin")
            .arg("non-unicode_2.bin")
            .fails()
            .stderr_is(
                "join: unprintable field separators are only supported on unix-like platforms\n",
            );
    }
}

#[test]
fn multibyte_sep() {
    new_ucmd!()
        .arg("-t§")
        .arg("multibyte_sep_1.txt")
        .arg("multibyte_sep_2.txt")
        .succeeds()
        .stdout_only_fixture("multibyte_sep.expected");
}

#[test]
fn null_field_separators() {
    new_ucmd!()
        .arg("-t")
        .arg("\\0")
        .arg("non-unicode_1.bin")
        .arg("non-unicode_2.bin")
        .succeeds()
        .stdout_only_fixture("null-sep.expected");
}

#[test]
fn null_line_endings() {
    new_ucmd!()
        .arg("-z")
        .arg("non-unicode_1.bin")
        .arg("non-unicode_2.bin")
        .succeeds()
        .stdout_only_fixture("z.expected");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "netbsd"))]
fn test_full() {
    let dev_full = OpenOptions::new().write(true).open("/dev/full").unwrap();
    new_ucmd!()
        .arg("fields_1.txt")
        .arg("fields_2.txt")
        .set_stdout(dev_full)
        .fails()
        .stderr_contains("No space left on device");
}

#[test]
#[cfg(target_os = "linux")]
fn test_join_non_utf8_paths() {
    use std::fs::File;
    use std::io::Write;

    let ts = TestScenario::new(util_name!());
    let test_dir = ts.fixtures.subdir.as_path();

    // Create files directly with non-UTF-8 names
    let file1_bytes = b"test_\xFF\xFE_1.txt";
    let file2_bytes = b"test_\xFF\xFE_2.txt";

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let file1_name = OsStr::from_bytes(file1_bytes);
        let file2_name = OsStr::from_bytes(file2_bytes);

        let mut file1 = File::create(test_dir.join(file1_name)).unwrap();
        file1.write_all(b"a 1\n").unwrap();

        let mut file2 = File::create(test_dir.join(file2_name)).unwrap();
        file2.write_all(b"a 2\n").unwrap();

        ts.ucmd()
            .arg(file1_name)
            .arg(file2_name)
            .succeeds()
            .stdout_only("a 1 2\n");
    }
}

#[test]
fn join_emoji_delim_inner_key() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.write("file1", "a🗿b\n");
    at.write("file2", "u🗿b\n");

    ts.ucmd()
        .args(&["-t🗿", "-1", "2", "-2", "2", "file1", "file2"])
        .succeeds()
        .stdout_only("b🗿a🗿u\n");
}

#[cfg(unix)]
#[test]
fn test_locale_collation() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.write("f1.sorted", "abc:d 2\nab:d  1\n");
    at.write("f2.sorted", "abc:d y\nab:d  x\n");

    ts.ucmd()
        .env("LC_ALL", "en_US.UTF-8")
        .arg("--check-order")
        .arg("f1.sorted")
        .arg("f2.sorted")
        .succeeds()
        .stdout_contains("abc:d 2 y")
        .stdout_contains("ab:d 1 x");
}

#[test]
fn test_incompatible_fields_reports_exact_field_number() {
    // An out-of-range field clamps to usize::MAX, which used to overflow the
    // one-based increment. Field numbers past 2^53 also used to be rounded on
    // their way through the localization layer.
    //
    // `parse_field_number` uses `usize`, so a value at or above `usize::MAX`
    // saturates to it. The saturation ceiling is therefore pointer-width
    // dependent, and the expected text is built from `usize::MAX` rather than a
    // hard-coded 64-bit literal.
    let max_field = usize::MAX.to_string();

    // A small field number takes the i64 number path through the localization
    // layer and is platform-independent.
    new_ucmd!()
        .args(&["-j", "3", "-1", "5", "/dev/null", "/dev/null"])
        .fails()
        .stderr_contains("incompatible join fields 3, 5");

    // Values at or above `usize::MAX` saturate to `usize::MAX` on every
    // platform; the localization layer must carry that ceiling as an exact
    // decimal string rather than rounding it through Fluent's f64-backed number
    // type.
    for field in ["18446744073709551615", "99999999999999999999999"] {
        new_ucmd!()
            .args(&["-j", field, "-1", "5", "/dev/null", "/dev/null"])
            .fails()
            .stderr_contains(format!("incompatible join fields {max_field}, 5"));
    }

    // A value above f64 precision (2^53) but below `usize::MAX` is reported
    // exactly on 64-bit (where `usize` holds it); on 32-bit it saturates to
    // `usize::MAX` like the cases above.
    #[cfg(target_pointer_width = "64")]
    let expected_above: String = "9007199254740993".to_string();
    #[cfg(not(target_pointer_width = "64"))]
    let expected_above: String = max_field.clone();
    new_ucmd!()
        .args(&[
            "-j",
            "9007199254740993",
            "-1",
            "5",
            "/dev/null",
            "/dev/null",
        ])
        .fails()
        .stderr_contains(format!("incompatible join fields {expected_above}, 5"));
}

#[cfg(all(feature = "feat_diagnostics", not(wasi_runner)))]
mod diagnostics {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn test_snippet_points_at_the_failing_field_of_a_list() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["-o", "1.2,2.x", "/dev/null", "/dev/null"])
            .fails_with_code(1);

        // The first field is fine; only the second one is at fault.
        assert_eq!(
            result.stderr_as_displayed(),
            "\
join: invalid field number: 'x'
   ╭─[ join:1:13 ]
   │
 1 │ join -o 1.2,2.x /dev/null /dev/null
   │             ───
   │
   │ Help: an output field is FILENUM.FIELD, as in -o 1.2,2.1; 0 stands for the join field
───╯"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_snippet_points_inside_a_glued_short_option() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["-o1.2,0.4", "/dev/null", "/dev/null"])
            .fails_with_code(1);
        let stderr = result.stderr_as_displayed();

        assert!(stderr.contains("join:1:12"), "{stderr}");
        assert!(stderr.contains("invalid field specifier"), "{stderr}");
    }

    #[test]
    fn test_plain_message_when_stderr_is_a_pipe() {
        new_ucmd!()
            .args(&["-o", "1.2,2.x", "/dev/null", "/dev/null"])
            .fails_with_code(1)
            .stderr_is("join: invalid field number: 'x'\n");
    }
}

#[test]
fn test_hyphen_leading_field_number_is_reported_as_invalid() {
    // GNU hands the argument after the option to that option even when it
    // starts with a hyphen, so it is reported as an invalid field number
    // rather than as an unknown option.
    for opt in ["-1", "-2", "-j"] {
        new_ucmd!()
            .args(&[opt, "-1", "empty.txt", "empty.txt"])
            .fails()
            .stderr_contains("invalid field number: '-1'");
    }
}
