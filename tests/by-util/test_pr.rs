// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) Sdivide ading

use jiff::{Timestamp, ToSpan};
use regex::Regex;
use std::fs::metadata;
use uutests::util::UCommand;
use uutests::{at_and_ucmd, new_ucmd};

const DATE_TIME_FORMAT_DEFAULT: &str = "%Y-%m-%d %H:%M";

fn file_last_modified_time_format(ucmd: &UCommand, path: &str, format: &str) -> String {
    let tmp_dir_path = ucmd.get_full_fixture_path(path);
    metadata(tmp_dir_path)
        .and_then(|meta| meta.modified())
        .map(|mtime| {
            let dt: Timestamp = mtime.try_into().unwrap();
            dt.strftime(format).to_string()
        })
        .unwrap_or_default()
}

fn file_last_modified_time(ucmd: &UCommand, path: &str) -> String {
    file_last_modified_time_format(ucmd, path, DATE_TIME_FORMAT_DEFAULT)
}

fn all_minutes(from: Timestamp, to: Timestamp) -> Vec<String> {
    let to = to + 1.minute();
    let mut vec = vec![];
    let mut current = from;
    while current < to {
        vec.push(current.strftime(DATE_TIME_FORMAT_DEFAULT).to_string());
        current += 1.minute();
    }
    vec
}

fn valid_last_modified_template_vars(from: Timestamp) -> Vec<Vec<(String, String)>> {
    all_minutes(from, Timestamp::now())
        .into_iter()
        .map(|time| vec![("{last_modified_time}".to_string(), time)])
        .collect()
}

#[test]
fn test_invalid_flag() {
    new_ucmd!()
        .arg("--invalid-argument")
        .fails_with_code(1)
        .no_stdout();
}

#[test]
fn test_expand_tabs_multibyte_char_is_rejected() {
    for arg in ["-e€", "-e€3"] {
        new_ucmd!()
            .args(&[arg, "test_one_page.log"])
            .fails_with_code(1)
            .stderr_contains("pr: '-e' extra characters or invalid number in the argument");
    }
}

#[test]
fn test_number_lines_multibyte_separator_is_rejected() {
    for arg in ["-n€", "-n€5"] {
        new_ucmd!()
            .args(&[arg, "test_one_page.log"])
            .fails_with_code(1)
            .stderr_contains("pr: '-n' extra characters or invalid number in the argument");
    }
}

#[test]
fn test_number_lines_empty_value_is_rejected() {
    new_ucmd!()
        .args(&["--number-lines=", "test_one_page.log"])
        .fails_with_code(1)
        .stderr_contains("pr: '-n' extra characters or invalid number in the argument");
}

#[test]
fn test_number_lines_without_value_numbers_lines() {
    // A bare -n/--number-lines numbers lines with the default 5-wide, tab format.
    for arg in ["-n", "--number-lines"] {
        new_ucmd!()
            .args(&["-t", arg])
            .pipe_in("a\nb\n")
            .succeeds()
            .stdout_is("    1\ta\n    2\tb\n");
    }
}

#[test]
fn test_without_any_options() {
    let test_file_path = "test_one_page.log";
    let expected_test_file_path = "test_one_page.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&[test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
}

#[test]
fn test_with_numbering_option_with_number_width() {
    let test_file_path = "test_num_page.log";
    let expected_test_file_path = "test_num_page_2.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["-n", "2", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
}

#[test]
fn test_with_long_header_option() {
    let whitespace = " ".repeat(21);
    let blank_lines = "\n".repeat(61);
    let datetime_pattern = r"\d\d\d\d-\d\d-\d\d \d\d:\d\d";
    let pattern =
        format!("\n\n{datetime_pattern}{whitespace}new file{whitespace}Page 1\n\n\na{blank_lines}");
    let regex = Regex::new(&pattern).unwrap();
    new_ucmd!()
        .args(&["-h", "new file"])
        .pipe_in("a")
        .succeeds()
        .stdout_matches(&regex);
    new_ucmd!()
        .args(&["--header=new file"])
        .pipe_in("a")
        .succeeds()
        .stdout_matches(&regex);
}

#[test]
fn test_with_double_space_option() {
    let test_file_path = "test_one_page.log";
    let expected_test_file_path = "test_one_page_double_line.log.expected";
    for arg in ["-d", "--double-space"] {
        let mut scenario = new_ucmd!();
        let value = file_last_modified_time(&scenario, test_file_path);
        scenario
            .args(&[arg, test_file_path])
            .succeeds()
            .stdout_is_templated_fixture(
                expected_test_file_path,
                &[("{last_modified_time}", &value)],
            );
    }
}

#[test]
fn test_with_first_line_number_option() {
    let test_file_path = "test_one_page.log";
    let expected_test_file_path = "test_one_page_first_line.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["-N", "5", "-n", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
}

#[test]
fn test_with_first_line_number_long_option() {
    let test_file_path = "test_one_page.log";
    let expected_test_file_path = "test_one_page_first_line.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["--first-line-number=5", "-n", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
}

#[test]
fn test_with_number_option_with_custom_separator_char() {
    let test_file_path = "test_num_page.log";
    let expected_test_file_path = "test_num_page_char.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["-nc", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
}

#[test]
fn test_with_number_option_with_custom_separator_char_and_width() {
    let test_file_path = "test_num_page.log";
    let expected_test_file_path = "test_num_page_char_one.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["-nc1", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
}

#[test]
fn test_with_valid_page_ranges() {
    let test_file_path = "test_num_page.log";
    let mut scenario = new_ucmd!();
    scenario
        .args(&["--pages=20:5", test_file_path])
        .fails()
        .stderr_only("pr: invalid --pages argument '20:5'\n");
    new_ucmd!()
        .args(&["--pages=1:5", test_file_path])
        .succeeds();
    new_ucmd!().args(&["--pages=1", test_file_path]).succeeds();
    new_ucmd!()
        .args(&["--pages=-1:5", test_file_path])
        .fails()
        .stderr_only("pr: invalid --pages argument '-1:5'\n");
    new_ucmd!()
        .args(&["--pages=1:-5", test_file_path])
        .fails()
        .stderr_only("pr: invalid --pages argument '1:-5'\n");
    new_ucmd!()
        .args(&["--pages=5:1", test_file_path])
        .fails()
        .stderr_only("pr: invalid --pages argument '5:1'\n");
}

#[test]
fn test_start_page_exceeds_page_count() {
    new_ucmd!()
        .args(&["--pages=2", "hosts.log"])
        .succeeds()
        .stderr_only("pr: starting page number 2 exceeds page count 1\n");
}

#[test]
fn test_with_page_range() {
    let test_file_path = "test.log";
    let expected_test_file_path = "test_page_range_1.log.expected";
    let expected_test_file_path1 = "test_page_range_2.log.expected";
    for arg in ["--pages=15", "+15"] {
        let mut scenario = new_ucmd!();
        let value = file_last_modified_time(&scenario, test_file_path);
        scenario
            .args(&[arg, test_file_path])
            .succeeds()
            .stdout_is_templated_fixture(
                expected_test_file_path,
                &[("{last_modified_time}", &value)],
            );
    }
    for arg in ["--pages=15:17", "+15:17"] {
        let mut scenario = new_ucmd!();
        let value = file_last_modified_time(&scenario, test_file_path);
        scenario
            .args(&[arg, test_file_path])
            .succeeds()
            .stdout_is_templated_fixture(
                expected_test_file_path1,
                &[("{last_modified_time}", &value)],
            );
    }
}

#[test]
fn test_with_no_header_trailer_option() {
    let test_file_path = "test_one_page.log";
    let expected_test_file_path = "test_one_page_no_ht.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["-t", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
}

#[test]
fn test_with_page_length_option() {
    let test_file_path = "test.log";
    for (arg, expected) in [
        ("100", "test_page_length.log.expected"),
        ("5", "test_page_length1.log.expected"),
    ] {
        let mut scenario = new_ucmd!();
        let value = file_last_modified_time(&scenario, test_file_path);
        scenario
            .args(&["--pages=2:3", "-l", arg, "-n", test_file_path])
            .succeeds()
            .stdout_is_templated_fixture(expected, &[("{last_modified_time}", &value)]);
    }
}

#[test]
fn test_with_suppress_error_option() {
    let test_file_path = "test_num_page.log";
    let mut scenario = new_ucmd!();
    scenario
        .args(&["--pages=20:5", "-r", test_file_path])
        .fails()
        .no_output();
}

#[test]
fn test_with_stdin() {
    let expected_file_path = "stdin.log.expected";
    let mut scenario = new_ucmd!();
    let start = Timestamp::now();
    scenario
        .pipe_in_fixture("stdin.log")
        .args(&["--pages=1:2", "-n", "-"])
        .succeeds()
        .stdout_is_templated_fixture_any(
            expected_file_path,
            &valid_last_modified_template_vars(start),
        );
}

#[test]
fn test_with_columns() {
    let test_file_path = "column.log";
    let expected_test_file_path = "column.log.expected";
    for arg in ["-3", "--columns=3"] {
        let mut scenario = new_ucmd!();
        let value = file_last_modified_time(&scenario, test_file_path);
        scenario
            .args(&["--pages=3:5", arg, "-n", test_file_path])
            .succeeds()
            .stdout_is_templated_fixture(
                expected_test_file_path,
                &[("{last_modified_time}", &value)],
            );
    }
}

#[test]
fn test_with_columns_and_across_option() {
    let test_file_path = "column.log";
    let expected_test_file_path = "column_across.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["--pages=3:5", "--columns=3", "-a", "-n", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
}

#[test]
fn test_with_columns_across_option_and_column_separator() {
    let test_file_path = "column.log";
    for (arg, expected) in [
        ("-s|", "column_across_sep.log.expected"),
        ("-Sdivide", "column_across_sep1.log.expected"),
    ] {
        let mut scenario = new_ucmd!();
        let value = file_last_modified_time(&scenario, test_file_path);
        scenario
            .args(&[
                "--pages=3:5",
                "--columns=3",
                arg,
                "-a",
                "-n",
                test_file_path,
            ])
            .succeeds()
            .stdout_is_templated_fixture(expected, &[("{last_modified_time}", &value)]);
    }
}

#[test]
fn test_with_mpr() {
    let test_file_path = "column.log";
    let test_file_path1 = "hosts.log";
    let expected_test_file_path = "mpr.log.expected";
    let expected_test_file_path1 = "mpr1.log.expected";
    let expected_test_file_path2 = "mpr2.log.expected";
    let start = Timestamp::now();
    new_ucmd!()
        .args(&["--pages=1:2", "-m", "-n", test_file_path, test_file_path1])
        .succeeds()
        .stdout_is_templated_fixture_any(
            expected_test_file_path,
            &valid_last_modified_template_vars(start),
        );

    let start = Timestamp::now();
    new_ucmd!()
        .args(&["--pages=2:4", "-m", "-n", test_file_path, test_file_path1])
        .succeeds()
        .stdout_is_templated_fixture_any(
            expected_test_file_path1,
            &valid_last_modified_template_vars(start),
        );

    let start = Timestamp::now();
    new_ucmd!()
        .args(&[
            "--pages=1:2",
            "-l",
            "100",
            "-n",
            "-m",
            test_file_path,
            test_file_path1,
            test_file_path,
        ])
        .succeeds()
        .stdout_is_templated_fixture_any(
            expected_test_file_path2,
            &valid_last_modified_template_vars(start),
        );
}

#[test]
fn test_with_mpr_and_columns_options() {
    let test_file_path = "column.log";
    new_ucmd!()
        .args(&["--columns=2", "-m", "-n", test_file_path])
        .fails()
        .stderr_only("pr: cannot specify number of columns when printing in parallel\n");

    new_ucmd!()
        .args(&["-a", "-m", "-n", test_file_path])
        .fails()
        .stderr_only("pr: cannot specify both printing across and printing in parallel\n");
}

#[test]
fn test_with_offset_space_option() {
    let test_file_path = "column.log";
    let expected_test_file_path = "column_spaces_across.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&[
            "-o",
            "5",
            "--pages=3:5",
            "--columns=3",
            "-a",
            "-n",
            test_file_path,
        ])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
}

#[test]
fn test_offset_too_large() {
    let arg = "2147483648";
    new_ucmd!()
        .args(&["-o", arg])
        .fails_with_code(1)
        .stderr_is(format!(
            "pr: '-o MARGIN' invalid line offset: '{arg}': Value too large for defined data type\n"
        ));
}

#[test]
fn test_start_line_number_too_large() {
    let arg = "18446744073709551615";
    new_ucmd!()
        .args(&["-n", "-N", arg])
        .fails_with_code(1)
        .stderr_is(format!(
            "pr: '-N NUMBER' invalid starting line number: '{arg}': Value too large for defined data type\n"
        ));
}

#[test]
fn test_page_length_too_large() {
    let arg = "9999999999999999999";
    new_ucmd!()
        .args(&["-l", arg, "-3"])
        .fails_with_code(1)
        .stderr_is(format!(
            "pr: '-l PAGE_LENGTH' invalid number of lines: '{arg}': Value too large for defined data type\n"
        ));
}

#[test]
fn test_number_width_too_large() {
    let arg = "18446744073709551615";
    new_ucmd!()
        .args(&["-n", arg])
        .fails_with_code(1)
        .stderr_is(format!(
            "pr: '-n' extra characters or invalid number in the argument: '{arg}': Value too large for defined data type\nTry 'pr --help' for more information.\n"
        ));
}

#[test]
fn test_page_width_too_large() {
    let arg = "18446744073709551615";
    new_ucmd!()
        .args(&["-W", arg])
        .fails_with_code(1)
        .stderr_is(format!(
            "pr: '-W PAGE_WIDTH' invalid number of characters: '{arg}': Value too large for defined data type\n"
        ));
}

#[test]
fn test_column_width_too_large() {
    let arg = "18446744073709551615";
    new_ucmd!()
        .args(&["-w", arg, "-2"])
        .fails_with_code(1)
        .stderr_is(format!(
            "pr: '-w PAGE_WIDTH' invalid number of characters: '{arg}': Value too large for defined data type\n"
        ));
}

#[test]
fn test_column_count_too_large() {
    let arg = "9999999999999999999";
    new_ucmd!()
        .args(&["--columns", arg])
        .fails_with_code(1)
        .stderr_is(format!(
            "pr: invalid number of columns: '{arg}': Value too large for defined data type\n"
        ));

    // The legacy -COLUMN operand form behaves the same.
    new_ucmd!()
        .args(&[format!("-{arg}")])
        .fails_with_code(1)
        .stderr_is(format!(
            "pr: invalid number of columns: '{arg}': Value too large for defined data type\n"
        ));
}

#[test]
fn test_large_number_width_does_not_panic() {
    // Widths above u16::MAX used to panic with "Formatting argument out
    // of range"; GNU pads the number out to the full width instead.
    // With -t (no page headers/footers) GNU emits no page padding, so the
    // output is exactly the numbered line.
    new_ucmd!()
        .args(&["-t", "-n", "70000"])
        .pipe_in("x\n")
        .succeeds()
        .stdout_is(format!("{}1\tx\n", " ".repeat(69999)));
}

#[test]
fn test_large_page_width_does_not_panic() {
    // A page width above u16::MAX used to panic in the header layout.
    new_ucmd!()
        .args(&["-W", "200000"])
        .pipe_in("x\n")
        .succeeds();
}

#[cfg(target_os = "linux")]
#[test]
fn test_offset_large_value_does_not_abort_under_memory_limit() {
    use rlimit::Resource;
    use std::process::Stdio;

    const AS_LIMIT: u64 = 200 * 1024 * 1024;

    new_ucmd!()
        .limit(Resource::AS, AS_LIMIT, AS_LIMIT)
        .set_stdout(Stdio::null())
        .args(&["-t", "-o", "999999999"])
        .pipe_in("hi\n")
        .succeeds();
}

#[test]
fn test_offset_invalid() {
    new_ucmd!()
        .args(&["--indent=-5"])
        .fails_with_code(1)
        .stderr_is("pr: '-o MARGIN' invalid line offset: '-5'\n");

    new_ucmd!()
        .args(&["-o", "abc"])
        .fails_with_code(1)
        .stderr_is("pr: '-o MARGIN' invalid line offset: 'abc'\n");
}

#[test]
fn test_with_date_format() {
    let whitespace = " ".repeat(50);
    let blank_lines = "\n".repeat(61);
    let datetime_pattern = r"\d{4}__\d{10}";
    let pattern = format!("\n\n{datetime_pattern}{whitespace}Page 1\n\n\na{blank_lines}");
    let regex = Regex::new(&pattern).unwrap();
    new_ucmd!()
        .args(&["-D", "%Y__%s"])
        .pipe_in("a")
        .succeeds()
        .stdout_matches(&regex);

    // "Format" doesn't need to contain any replaceable token.
    let whitespace = " ".repeat(60);
    let blank_lines = "\n".repeat(61);
    new_ucmd!()
        .args(&["-D", "Hello!"])
        .pipe_in("a")
        .succeeds()
        .stdout_only(format!("\n\nHello!{whitespace}Page 1\n\n\na{blank_lines}"));

    // Long option also works
    new_ucmd!()
        .args(&["--date-format=Hello!"])
        .pipe_in("a")
        .succeeds()
        .stdout_only(format!("\n\nHello!{whitespace}Page 1\n\n\na{blank_lines}"));

    // Option takes precedence over environment variables
    new_ucmd!()
        .env("POSIXLY_CORRECT", "1")
        .env("LC_TIME", "POSIX")
        .args(&["--date-format=Hello!"])
        .pipe_in("a")
        .succeeds()
        .stdout_only(format!("\n\nHello!{whitespace}Page 1\n\n\na{blank_lines}"));
}

#[test]
fn test_with_date_format_env() {
    // POSIXLY_CORRECT + LC_ALL/TIME=POSIX uses "%b %e %H:%M %Y" date format
    let whitespace = " ".repeat(49);
    let blank_lines = "\n".repeat(61);
    let datetime_pattern = r"[A-Z][a-z][a-z] [ \d]\d \d\d:\d\d \d{4}";
    let pattern = format!("\n\n{datetime_pattern}{whitespace}Page 1\n\n\na{blank_lines}");
    let regex = Regex::new(&pattern).unwrap();
    new_ucmd!()
        .env("POSIXLY_CORRECT", "1")
        .env("LC_ALL", "POSIX")
        .pipe_in("a")
        .succeeds()
        .stdout_matches(&regex);
    new_ucmd!()
        .env("POSIXLY_CORRECT", "1")
        .env("LC_TIME", "POSIX")
        .pipe_in("a")
        .succeeds()
        .stdout_matches(&regex);

    // But not if POSIXLY_CORRECT/LC_ALL is something else.
    let whitespace = " ".repeat(50);
    let datetime_pattern = r"\d\d\d\d-\d\d-\d\d \d\d:\d\d";
    let pattern = format!("\n\n{datetime_pattern}{whitespace}Page 1\n\n\na{blank_lines}");
    let regex = Regex::new(&pattern).unwrap();
    new_ucmd!()
        .env("LC_TIME", "POSIX")
        .pipe_in("a")
        .succeeds()
        .stdout_matches(&regex);
    new_ucmd!()
        .env("POSIXLY_CORRECT", "1")
        .env("LC_TIME", "C")
        .pipe_in("a")
        .succeeds()
        .stdout_matches(&regex);
}

#[test]
fn test_with_join_lines_option() {
    let test_file_1 = "hosts.log";
    let test_file_2 = "test.log";
    let expected_file_path = "joined.log.expected";
    let start = Timestamp::now();

    for join_lines_arg in ["-J", "--join-lines"] {
        new_ucmd!()
            .args(&["+1:2", join_lines_arg, "-m", test_file_1, test_file_2])
            .succeeds()
            .stdout_is_templated_fixture_any(
                expected_file_path,
                &valid_last_modified_template_vars(start),
            );
    }
}

#[test]
fn test_value_for_number_lines() {
    // *5 is of the form [SEP[NUMBER]] so is accepted and succeeds
    new_ucmd!().args(&["-n", "*5", "test.log"]).succeeds();

    // a is of the form [SEP[NUMBER]] so is accepted and succeeds
    new_ucmd!().args(&["-n", "a", "test.log"]).succeeds();

    // foo5.txt is of not the form [SEP[NUMBER]] so is not used as value.
    // Therefore, pr tries to access the file, which does not exist.
    new_ucmd!().args(&["-n", "foo5.txt", "test.log"]).fails();
}

#[test]
fn test_header_formatting_with_custom_date_format() {
    // This test verifies that the header is properly formatted with:
    // - Date/time on the left
    // - Filename centered
    // - "Page X" on the right
    // This matches GNU pr behavior for the time-style test

    let test_file_path = "test_one_page.log";

    // Set a specific date format for consistent output
    let output = new_ucmd!()
        .args(&["-D", "+%Y-%m-%d %H:%M:%S %z (%Z)", test_file_path])
        .succeeds()
        .stdout_move_str();

    // Extract the header line (3rd line of output)
    let lines: Vec<&str> = output.lines().collect();
    assert!(
        lines.len() >= 5,
        "Output should have at least 5 lines for header"
    );

    let header_line = lines[2];

    // The header should be 72 characters wide (default page width)
    assert_eq!(header_line.chars().count(), 72);

    // Check that it contains the expected parts
    assert!(header_line.contains(test_file_path));
    assert!(header_line.contains("Page 1"));

    // Verify the filename is roughly centered
    let filename_pos = header_line.find(test_file_path).unwrap();
    let page_pos = header_line.find("Page 1").unwrap();

    // Filename should be somewhere in the middle third of the line
    assert!(filename_pos > 24 && filename_pos < 48);

    // Page should be right-aligned (near the end)
    assert!(page_pos >= 60);
}

#[test]
fn test_help() {
    new_ucmd!().arg("--help").succeeds();
}

#[test]
fn test_version() {
    new_ucmd!().arg("--version").succeeds();
}

#[cfg(unix)]
#[test]
fn test_pr_char_device_dev_null() {
    new_ucmd!().arg("/dev/null").succeeds();
}

#[test]
fn test_b_flag_backwards_compat() {
    // -b is a no-op for backwards compatibility (column-down is now the default)
    new_ucmd!().args(&["-b", "-t"]).pipe_in("a\nb\n").succeeds();
}

#[test]
fn test_page_header_width() {
    let whitespace = " ".repeat(50);
    let blank_lines = "\n".repeat(61);
    let datetime_pattern = r"\d\d\d\d-\d\d-\d\d \d\d:\d\d";
    let pattern = format!("\n\n{datetime_pattern}{whitespace}Page 1\n\n\na{blank_lines}");
    let regex = Regex::new(&pattern).unwrap();
    new_ucmd!().pipe_in("a").succeeds().stdout_matches(&regex);
}

#[test]
fn test_separator_options_default_values() {
    // -s and -S without arguments should use default values (TAB and space)
    // TODO: verify output matches GNU pr behavior
    new_ucmd!()
        .args(&["-t", "-2", "-s"])
        .pipe_in("a\nb\n")
        .succeeds();
    new_ucmd!()
        .args(&["-t", "-2", "-S"])
        .pipe_in("a\nb\n")
        .succeeds();
}

#[test]
fn test_omit_pagination_option() {
    // -T/--omit-pagination omits headers/trailers and eliminates form feeds
    // TODO: verify output matches GNU pr behavior (form feed elimination)
    new_ucmd!().args(&["-T"]).pipe_in("a\nb\n").succeeds();
    new_ucmd!()
        .args(&["--omit-pagination"])
        .pipe_in("a\nb\n")
        .succeeds();
}

#[test]
fn test_form_feed_newlines() {
    // Here we define the expected output.
    //
    // Each page should have the same number of blank lines before the
    // form-feed character.
    let whitespace = " ".repeat(50);
    let datetime_pattern = r"\d\d\d\d-\d\d-\d\d \d\d:\d\d";
    let page1 = format!("\n\n{datetime_pattern}{whitespace}Page 1\n\n\n\n\x0c");
    let page2 = format!("\n\n{datetime_pattern}{whitespace}Page 2\n\n\n\n\x0c");
    let pattern = format!("{page1}{page2}");
    let regex = Regex::new(&pattern).unwrap();

    // Command line: `printf "\f\f" | pr -f`.
    //
    // Escape code `\x0c` in a Rust string literal is the ASCII escape
    // code `\f` for the "form feed" character (which appears like
    // `^L` in the terminal).
    new_ucmd!()
        .arg("-f")
        .pipe_in("\x0c\x0c")
        .succeeds()
        .stdout_matches(&regex);
}

#[test]
fn test_new_line_followed_by_form_feed() {
    // Here we define the expected output.
    let whitespace = " ".repeat(50);
    let datetime_pattern = r"\d\d\d\d-\d\d-\d\d \d\d:\d\d";
    let pattern = format!("\n\n{datetime_pattern}{whitespace}Page 1\n\n\nabc\n\x0c");
    let regex = Regex::new(&pattern).unwrap();

    // Command line: `printf "abc\n\f" | pr -f`.
    new_ucmd!()
        .arg("-f")
        .pipe_in("abc\n\x0c")
        .succeeds()
        .stdout_matches(&regex);
}

#[test]
fn test_form_feed_followed_by_new_line() {
    // Here we define the expected output.
    let whitespace = " ".repeat(50);
    let datetime_pattern = r"\d\d\d\d-\d\d-\d\d \d\d:\d\d";
    let blank_lines_61 = "\n".repeat(61);
    let blank_lines_60 = "\n".repeat(60);
    let page1 = format!("\n\n{datetime_pattern}{whitespace}Page 1\n\n\n{blank_lines_61}");
    let page2 = format!("\n\n{datetime_pattern}{whitespace}Page 2\n\n\nabc\n{blank_lines_60}");
    let pattern = format!("{page1}{page2}");
    let regex = Regex::new(&pattern).unwrap();

    // Command line: `printf "\f\nabc" | pr`.
    new_ucmd!()
        .pipe_in("\x0c\nabc")
        .succeeds()
        .stdout_matches(&regex);
}

#[test]
fn test_columns() {
    let whitespace = " ".repeat(50);
    let datetime_pattern = r"\d\d\d\d-\d\d-\d\d \d\d:\d\d";
    let header = format!("\n\n{datetime_pattern}{whitespace}Page 1\n\n\n");
    // TODO Our output still does not match the behavior of GNU
    // pr. The correct output should be:
    //
    //     "a\t\t\t\t    b\n";
    //
    let data = "a                                  \tb                                  \n";
    let blank_lines_60 = "\n".repeat(60);
    let pattern = format!("{header}{data}{blank_lines_60}");
    let regex = Regex::new(&pattern).unwrap();

    // Command line: `printf "a\nb\n" | pr -2`.
    new_ucmd!()
        .arg("-2")
        .pipe_in("a\nb\n")
        .succeeds()
        .stdout_matches(&regex);
}

#[test]
fn test_columns_partly_filled_page() {
    // Three lines do not divide evenly into two columns. GNU pr puts the
    // ceiling of 3 / 2 in the first column and what is left in the second, so
    // "c" has to show up next to "a".
    //
    // Command line: `printf "a\nb\nc\n" | pr -t -2 -w 20`.
    new_ucmd!()
        .args(&["-t", "-2", "-w", "20"])
        .pipe_in("a\nb\nc\n")
        .succeeds()
        .stdout_is("a        \tc        \nb        \n");
}

#[test]
fn test_columns_partly_filled_page_across() {
    // Same input read across instead of down.
    //
    // Command line: `printf "a\nb\nc\n" | pr -t -a -2 -w 20`.
    new_ucmd!()
        .args(&["-t", "-a", "-2", "-w", "20"])
        .pipe_in("a\nb\nc\n")
        .succeeds()
        .stdout_is("a        \tb        \nc        \n");
}

#[test]
fn test_columns_fewer_lines_than_columns() {
    // Fewer lines than columns still has to print those lines.
    //
    // Command line: `printf "a\n" | pr -t -3 -w 20`.
    new_ucmd!()
        .args(&["-t", "-3", "-w", "20"])
        .pipe_in("a\n")
        .succeeds()
        .stdout_is("a     \n");

    // Command line: `printf "a\nb\n" | pr -t -3 -w 20`.
    new_ucmd!()
        .args(&["-t", "-3", "-w", "20"])
        .pipe_in("a\nb\n")
        .succeeds()
        .stdout_is("a     \tb     \n");
}

#[test]
fn test_columns_last_page() {
    // Nine lines over pages that hold four each leave the ninth alone on page
    // three, which must not come out as an empty page.
    //
    // Command line: `seq 9 | pr -2 -l 12 -w 20 -D DATE`.
    let header = "\n\nDATE          Page ";
    let expected = format!(
        "{header}1\n\n\n1        \t3        \n2        \t4        \n{blank}\
         {header}2\n\n\n5        \t7        \n6        \t8        \n{blank}\
         {header}3\n\n\n9        \n\n\n\n\n\n\n",
        blank = "\n".repeat(5),
    );
    new_ucmd!()
        .args(&["-2", "-l", "12", "-w", "20", "-D", "DATE"])
        .pipe_in("1\n2\n3\n4\n5\n6\n7\n8\n9\n")
        .succeeds()
        .stdout_is(expected);
}

#[test]
fn test_merge() {
    // Create the two files to merge.
    let (at, mut ucmd) = at_and_ucmd!();
    at.write("f", "a\n");
    at.write("g", "b\n");

    let whitespace = " ".repeat(50);
    let datetime_pattern = r"\d\d\d\d-\d\d-\d\d \d\d:\d\d";
    let header = format!("\n\n{datetime_pattern}{whitespace}Page 1\n\n\n");
    // TODO Our output still does not match the behavior of GNU
    // pr. The correct output should be:
    //
    //     "a\t\t\t\t    b\n";
    //
    // and the blank lines should actually be empty lines.
    let data = "a                                  \tb                                  \n";
    let blank_lines_55 =
        "                                   \t                                   \n".repeat(55);
    let footer = "\n".repeat(5);
    let pattern = format!("{header}{data}{blank_lines_55}{footer}");
    let regex = Regex::new(&pattern).unwrap();

    // Command line: `(echo "a" > f; echo "b" > g; pr -m f g)`.
    ucmd.args(&["-m", "f", "g"])
        .succeeds()
        .stdout_matches(&regex);
}

#[test]
fn test_merge_one_long_one_short() {
    // Create the two files to merge.
    let (at, mut ucmd) = at_and_ucmd!();
    at.write("f", "a\na\n");
    at.write("g", "b\n");

    // Page 1 should have the first line of `f` and the first line of
    // `b` side-by-side.
    let whitespace = " ".repeat(50);
    let datetime_pattern = r"\d\d\d\d-\d\d-\d\d \d\d:\d\d";
    let header = format!("\n\n{datetime_pattern}{whitespace}Page 1\n\n\n");
    let data = "a                                  \tb                                  \n";
    let footer = "\n".repeat(5);
    let page1 = format!("{header}{data}{footer}");

    // Page 2 should have just the second line of `f`.
    let header = format!("\n\n{datetime_pattern}{whitespace}Page 2\n\n\n");
    let data = "a                                  \t                                   \n";
    let page2 = format!("{header}{data}{footer}");

    let pattern = format!("{page1}{page2}");
    let regex = Regex::new(&pattern).unwrap();

    // Command line:
    //
    //     printf "a\na\n" > f
    //     printf "b\n" > g
    //     pr -l11 -m f g
    //
    // The line length of 11 leaves room for a 5-line header, a 5-line
    // footer, and one line of data from the input files. The extra
    // line from the file `f` will be on the second page.
    ucmd.args(&["-l", "11", "-m", "f", "g"])
        .succeeds()
        .stdout_matches(&regex);
}

#[test]
fn test_simple_expand_tab() {
    let whitespace = " ".repeat(50);
    let datetime_pattern = r"\d\d\d\d-\d\d-\d\d \d\d:\d\d";
    let page_1_beginning = format!("\n\n{datetime_pattern}{whitespace}Page 1\n\n\n");

    let output_regex = Regex::new(&format!("{page_1_beginning}hello   world\nabc     def\n        leading\ntrail   \n8chars00        \n")).unwrap();

    new_ucmd!()
        .arg("-e")
        .pipe_in("hello\tworld\nabc\tdef\n\tleading\ntrail\t\n8chars00\t\n")
        .succeeds()
        .stdout_matches(&output_regex);
}

#[test]
fn test_expand_tab_at_end_of_short_flag_cluster() {
    // `-e` closing a cluster of value-less short flags carries no attached argument, so it has
    // to fall back to the defaults rather than report a missing value.
    for (arg, expected) in [
        ("-tre", "oi\n"),
        ("-tre8", "oi\n"),
        ("-tfre", "oi\n\x0c"),
        ("-tfre8", "oi\n\x0c"),
    ] {
        new_ucmd!()
            .arg(arg)
            .pipe_in("oi\n")
            .succeeds()
            .stdout_only(expected);
    }
}

#[test]
fn test_expand_tab_does_not_consume_following_operand() {
    // A bare `-e` must leave the file operand alone.
    new_ucmd!()
        .args(&["-t", "-e"])
        .pipe_in("a\tb\n")
        .succeeds()
        .stdout_only("a       b\n");
}

#[test]
fn test_simple_expand_tab_with_digit_argument() {
    let whitespace = " ".repeat(50);
    let datetime_pattern = r"\d\d\d\d-\d\d-\d\d \d\d:\d\d";
    let page_1_beginning = format!("\n\n{datetime_pattern}{whitespace}Page 1\n\n\n");
    let input = "hello\tworld\nabc\tdef\n\tleading\ntrail\t\n8chars00\t\n";

    let test_cases = vec![
        ("-e2", Regex::new(&format!("{page_1_beginning}hello world\nabc def\n  leading\ntrail \n8chars00  \n")).unwrap()),
        ("-e3", Regex::new(&format!("{page_1_beginning}hello world\nabc   def\n   leading\ntrail \n8chars00 \n")).unwrap()),
        ("-e8", Regex::new(&format!("{page_1_beginning}hello   world\nabc     def\n        leading\ntrail   \n8chars00        \n")).unwrap()),
        ("-e10", Regex::new(&format!("{page_1_beginning}hello     world\nabc       def\n          leading\ntrail     \n8chars00  \n")).unwrap()),
    ];
    for (arg, output_regex) in test_cases {
        new_ucmd!()
            .arg(arg)
            .pipe_in(input)
            .succeeds()
            .stdout_matches(&output_regex);
    }
}

#[test]
fn test_simple_expand_tab_with_char_argument() {
    let whitespace = " ".repeat(50);
    let datetime_pattern = r"\d\d\d\d-\d\d-\d\d \d\d:\d\d";
    let page_1_beginning = format!("\n\n{datetime_pattern}{whitespace}Page 1\n\n\n");
    let input = "hello\tworld\nabc\tdef\n\tleading\ntrail\t\n8chars00\t\n";

    let test_cases = vec![
        ("-ea", Regex::new(&format!("{page_1_beginning}hello   world\n        bc      def\n        le      ding\ntr      il      \n8ch     rs00    \n")).unwrap()),
        ("-ee", Regex::new(&format!("{page_1_beginning}h       llo     world\nabc     d       f\n        l       ading\ntrail   \n8chars00        \n")).unwrap()),
    ];
    for (arg, output_regex) in test_cases {
        new_ucmd!()
            .arg(arg)
            .pipe_in(input)
            .succeeds()
            .stdout_matches(&output_regex);
    }
}

#[test]
fn test_simple_expand_tab_with_both_arguments() {
    // test different variations of what char to expand
    // a2, e3, t10
    let whitespace = " ".repeat(50);
    let datetime_pattern = r"\d\d\d\d-\d\d-\d\d \d\d:\d\d";
    let page_1_beginning = format!("\n\n{datetime_pattern}{whitespace}Page 1\n\n\n");
    let input = "hello\tworld\nabc\tdef\n\tleading\ntrail\t\n8chars00\t\n";

    let test_cases = vec![
        ("-ea2", Regex::new(&format!("{page_1_beginning}hello   world\n  bc    def\n        le  ding\ntr  il  \n8ch rs00        \n")).unwrap()),
        ("-ee3", Regex::new(&format!("{page_1_beginning}h  llo  world\nabc     d   f\n        l   ading\ntrail   \n8chars00        \n")).unwrap()),
        ("-et10", Regex::new(&format!("{page_1_beginning}hello   world\nabc     def\n        leading\n          rail  \n8chars00        \n")).unwrap()),
    ];
    for (arg, output_regex) in test_cases {
        new_ucmd!()
            .arg(arg)
            .pipe_in(input)
            .succeeds()
            .stdout_matches(&output_regex);
    }
}

/* cSpell:disable */
#[test]
fn test_invalid_expand_tab_arguments() {
    let test_file_path = "empty_test_file";

    let test_cases = vec![
        // incorrect argument
        ("-esdgjiojiosdgjiogd", "dgjiojiosdgjiogd"),
        // 2 non digit parameter
        ("-eab", "b"),
        // non digit after first digit
        ("-e1a", "1a"),
        // non digit after first digit after allowed input char
        ("-ea1a", "1a"),
        // > i32 max
        ("-e2147483648", "2147483648"),
        // > i32 max after allowed input char
        ("-ea2147483648", "2147483648"),
    ];

    for (arg, error_msg_field) in test_cases {
        new_ucmd!()
            .args(&[arg, test_file_path])
            .fails()
            .stderr_contains(format!("pr: '-e' extra characters or invalid number in the argument: ‘{error_msg_field}’\nTry 'pr --help' for more information."));
    }
}
/* cSpell:enable */

#[test]
fn test_expand_tab_does_not_consume_next_argument() {
    let test_file_path = "empty_test_file";
    new_ucmd!().args(&["-e", test_file_path]).succeeds();
    new_ucmd!().args(&["-ea", test_file_path]).succeeds();
    new_ucmd!().args(&["-ea1", test_file_path]).succeeds();
}

#[test]
fn test_zero_columns() {
    new_ucmd!()
        .arg("--columns=0")
        .fails_with_code(1)
        .stderr_contains("pr: invalid --columns argument '0'");
}

#[test]
fn test_zero_columns_shortcut() {
    new_ucmd!()
        .arg("-0")
        .fails_with_code(1)
        .stderr_contains("pr: invalid --columns argument '0'");
}

#[test]
fn test_filename_ending_with_dash_number_is_not_an_option() {
    for name in ["a-0", "a-b-0", "a-3"] {
        let (at, mut ucmd) = at_and_ucmd!();
        at.write(name, "RUST-pr\n");
        ucmd.args(&["-t", name])
            .succeeds()
            .stdout_contains("RUST-pr");
    }
}

#[test]
fn test_double_dash_terminates_option_parsing() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write("-0", "RUST-pr\n");
    ucmd.args(&["-t", "--", "-0"])
        .succeeds()
        .stdout_contains("RUST-pr");
}

#[test]
fn test_double_dash_shields_expand_tabs_filename() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write("-e", "RUST-pr\n");
    ucmd.args(&["-t", "--", "-e"])
        .succeeds()
        .stdout_contains("RUST-pr");
}

#[test]
fn test_double_dash_shields_number_lines_filename() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write("-n", "first\n");
    at.write("data", "second\n");
    ucmd.args(&["-t", "--", "-n", "data"])
        .succeeds()
        .stdout_contains("first")
        .stdout_contains("second");
}

#[test]
fn test_double_dash_shields_filename_ending_with_dash_zero() {
    for name in ["a-0", "a-b-0"] {
        let (at, mut ucmd) = at_and_ucmd!();
        at.write(name, "RUST-pr\n");

        ucmd.args(&["-t", "--", name])
            .succeeds()
            .stdout_contains("RUST-pr\n");
    }
}

#[test]
fn test_zero_expand_tab_width() {
    let expected = "pr: '-e' extra characters or invalid number in the argument: ‘0’\nTry 'pr --help' for more information.\n";
    new_ucmd!()
        .arg("-e0")
        .fails_with_code(1)
        .stderr_only(expected);
    new_ucmd!()
        .arg("-eX0")
        .fails_with_code(1)
        .stderr_only(expected);
}

#[test]
fn test_zero_column_width() {
    new_ucmd!()
        .args(&["-w", "0"])
        .fails_with_code(1)
        .stderr_is("pr: invalid --width argument '0'\n");
}

#[test]
fn test_zero_page_width() {
    new_ucmd!()
        .args(&["-W", "0"])
        .fails_with_code(1)
        .stderr_is("pr: invalid --page-width argument '0'\n");
}

#[test]
fn test_page_length_ten_implies_omit_header() {
    // `pr --help` states it twice: a page length of 10 or less implies `-t`.
    // At exactly 10 the header and trailer were kept and then subtracted from
    // the page, which left no room for content: `-h` printed empty pages and
    // `-t` printed nothing at all.
    new_ucmd!()
        .args(&["-l", "10", "-h", "hdr"])
        .pipe_in("a\nb\nc\n")
        .succeeds()
        .stdout_only("a\nb\nc\n");

    new_ucmd!()
        .args(&["-l", "10", "-t"])
        .pipe_in("a\nb\nc\n")
        .succeeds()
        .stdout_only("a\nb\nc\n");
}

#[test]
fn test_page_length_eleven_keeps_header() {
    new_ucmd!()
        .args(&["-l", "11", "-h", "hdr"])
        .pipe_in("a\nb\nc\n")
        .succeeds()
        .stdout_contains("hdr");
}

#[test]
fn test_zero_length() {
    new_ucmd!()
        .args(&["-l", "0"])
        .fails_with_code(1)
        .stderr_is("pr: invalid --length argument '0'\n");
}

#[test]
fn test_zero_pages() {
    new_ucmd!()
        .args(&["--pages", "0"])
        .fails_with_code(1)
        .stderr_is("pr: invalid --pages argument '0'\n");
}

#[test]
fn test_negative_expand_tabs() {
    new_ucmd!()
        .arg("-e=-1")
        .fails_with_code(1)
        .stderr_is("pr: '-e' extra characters or invalid number in the argument: ‘-1’\nTry 'pr --help' for more information.\n");
}

#[cfg(unix)]
#[test]
fn test_merge_empty_input() {
    new_ucmd!()
        .args(&["-m", "/dev/null", "/dev/null"])
        .succeeds()
        .no_output();
}

#[test]
fn test_missing_file_error_message() {
    // A nonexistent operand must be reported as "pr: <file>: <message>", like
    // GNU pr, with the raw "(os error N)" suffix stripped. The exact message
    // text is platform dependent, so only assert the portable parts.
    new_ucmd!()
        .arg("nonexistent_file")
        .fails_with_code(1)
        .stderr_contains("pr: nonexistent_file: ")
        .stderr_does_not_contain("(os error");
}
