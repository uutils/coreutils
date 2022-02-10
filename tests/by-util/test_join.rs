// spell-checker:ignore (words) autoformat

use crate::common::util::*;
#[cfg(unix)]
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
#[cfg(windows)]
use std::{ffi::OsString, os::windows::ffi::OsStringExt};

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
        .arg("э")
        .fails()
        .stderr_is("join: multi-character tab э");
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
        .stderr_is("join: invalid file number in field spec: ''");
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
fn wrong_line_order() {
    let ts = TestScenario::new(util_name!());
    new_ucmd!()
        .arg("fields_2.txt")
        .arg("fields_4.txt")
        .fails()
        .stderr_is(&format!(
        "{0} {1}: fields_4.txt:5: is not sorted: 11 g 5 gh\n{0} {1}: input is not in sorted order",
        ts.bin_path.to_string_lossy(),
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
        .stderr_is(&format!(
            "{0} {1}: fields_5.txt:4: is not sorted: 3\n{0} {1}: fields_4.txt:5: is not sorted: 11 g 5 gh\n{0} {1}: input is not in sorted order",
            ts.bin_path.to_string_lossy(),
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
        let invalid_utf8: u8 = 167;
        new_ucmd!()
            .arg("-t")
            .arg(OsStr::from_bytes(&[invalid_utf8]))
            .arg("non-unicode_1.bin")
            .arg("non-unicode_2.bin")
            .succeeds()
            .stdout_only_fixture("non-unicode_sep.expected");
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
                "join: unprintable field separators are only supported on unix-like platforms",
            );
    }
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
