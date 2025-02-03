// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (paths) gnutest ronna quetta

use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_should_not_round_floats() {
    new_ucmd!()
        .args(&["0.99", "1.01", "1.1", "1.22", ".1", "-0.1"])
        .succeeds()
        .stdout_is("0.99\n1.01\n1.1\n1.22\n0.1\n-0.1\n");
}

#[test]
fn test_should_preserve_trailing_zeros() {
    new_ucmd!()
        .args(&["0.1000", "10.00"])
        .succeeds()
        .stdout_is("0.1000\n10.00\n");
}

#[test]
fn test_from_si() {
    new_ucmd!()
        .args(&["--from=si"])
        .pipe_in("1000\n1.1M\n0.1G")
        .succeeds()
        .stdout_is("1000\n1100000\n100000000\n");
}

#[test]
fn test_from_iec() {
    new_ucmd!()
        .args(&["--from=iec"])
        .pipe_in("1024\n1.1M\n0.1G")
        .succeeds()
        .stdout_is("1024\n1153434\n107374183\n");
}

#[test]
fn test_from_iec_i() {
    new_ucmd!()
        .args(&["--from=iec-i"])
        .pipe_in("1.1Mi\n0.1Gi")
        .succeeds()
        .stdout_is("1153434\n107374183\n");
}

#[test]
fn test_from_iec_i_requires_suffix() {
    let numbers = vec!["1024", "10M"];

    for number in numbers {
        new_ucmd!()
            .args(&["--from=iec-i", number])
            .fails()
            .code_is(2)
            .stderr_is(format!(
                "numfmt: missing 'i' suffix in input: '{number}' (e.g Ki/Mi/Gi)\n"
            ));
    }
}

#[test]
fn test_from_auto() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("1K\n1Ki")
        .succeeds()
        .stdout_is("1000\n1024\n");
}

#[test]
fn test_to_si() {
    new_ucmd!()
        .args(&["--to=si"])
        .pipe_in("1000\n1100000\n100000000")
        .succeeds()
        .stdout_is("1.0K\n1.1M\n100M\n");
}

#[test]
fn test_to_iec() {
    new_ucmd!()
        .args(&["--to=iec"])
        .pipe_in("1024\n1153434\n107374182")
        .succeeds()
        .stdout_is("1.0K\n1.2M\n103M\n");
}

#[test]
fn test_to_iec_i() {
    new_ucmd!()
        .args(&["--to=iec-i"])
        .pipe_in("1024\n1153434\n107374182")
        .succeeds()
        .stdout_is("1.0Ki\n1.2Mi\n103Mi\n");
}

#[test]
fn test_input_from_free_arguments() {
    new_ucmd!()
        .args(&["--from=si", "1K", "1.1M", "0.1G"])
        .succeeds()
        .stdout_is("1000\n1100000\n100000000\n");
}

#[test]
fn test_padding() {
    new_ucmd!()
        .args(&["--from=si", "--padding=8"])
        .pipe_in("1K\n1.1M\n0.1G")
        .succeeds()
        .stdout_is("    1000\n 1100000\n100000000\n");
}

#[test]
fn test_negative_padding() {
    new_ucmd!()
        .args(&["--from=si", "--padding=-8"])
        .pipe_in("1K\n1.1M\n0.1G")
        .succeeds()
        .stdout_is("1000    \n1100000 \n100000000\n");
}

#[test]
fn test_header() {
    new_ucmd!()
        .args(&["--from=si", "--header=2"])
        .pipe_in("header\nheader2\n1K\n1.1M\n0.1G")
        .succeeds()
        .stdout_is("header\nheader2\n1000\n1100000\n100000000\n");
}

#[test]
fn test_header_default() {
    new_ucmd!()
        .args(&["--from=si", "--header"])
        .pipe_in("header\n1K\n1.1M\n0.1G")
        .succeeds()
        .stdout_is("header\n1000\n1100000\n100000000\n");
}

#[test]
fn test_header_error_if_non_numeric() {
    new_ucmd!()
        .args(&["--header=two"])
        .fails()
        .stderr_is("numfmt: invalid header value 'two'\n");
}

#[test]
fn test_header_error_if_0() {
    new_ucmd!()
        .args(&["--header=0"])
        .fails()
        .stderr_is("numfmt: invalid header value '0'\n");
}

#[test]
fn test_header_error_if_negative() {
    new_ucmd!()
        .args(&["--header=-3"])
        .fails()
        .stderr_is("numfmt: invalid header value '-3'\n");
}

#[test]
fn test_negative() {
    new_ucmd!()
        .args(&["--from=si"])
        .pipe_in("-1000\n-1.1M\n-0.1G")
        .succeeds()
        .stdout_is("-1000\n-1100000\n-100000000\n");
    new_ucmd!()
        .args(&["--to=iec-i"])
        .pipe_in("-1024\n-1153434\n-107374182")
        .succeeds()
        .stdout_is("-1.0Ki\n-1.2Mi\n-103Mi\n");
}

#[test]
fn test_negative_zero() {
    new_ucmd!()
        .pipe_in("-0\n-0.0")
        .succeeds()
        .stdout_is("0\n0.0\n");
}

#[test]
fn test_no_op() {
    new_ucmd!()
        .pipe_in("1024\n1234567")
        .succeeds()
        .stdout_is("1024\n1234567\n");
}

#[test]
fn test_normalize() {
    new_ucmd!()
        .args(&["--from=si", "--to=si"])
        .pipe_in("10000000K\n0.001K")
        .succeeds()
        .stdout_is("10G\n1\n");
}

#[test]
fn test_si_to_iec() {
    new_ucmd!()
        .args(&["--from=si", "--to=iec", "15334263563K"])
        .succeeds()
        .stdout_is("14T\n");
}

#[test]
fn test_should_report_invalid_empty_number_on_empty_stdin() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("\n")
        .fails()
        .stderr_is("numfmt: invalid number: ''\n");
}

#[test]
fn test_should_report_invalid_empty_number_on_blank_stdin() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("  \t  \n")
        .fails()
        .stderr_is("numfmt: invalid number: ''\n");
}

#[test]
fn test_suffixes() {
    // TODO add support for ronna (R) and quetta (Q)
    let valid_suffixes = ['K', 'M', 'G', 'T', 'P', 'E', 'Z', 'Y' /*'R' , 'Q'*/];

    // TODO implement special handling of 'K'
    for c in ('A'..='Z').chain('a'..='z') {
        let args = ["--from=si", "--to=si", &format!("1{c}")];

        if valid_suffixes.contains(&c) {
            new_ucmd!()
                .args(&args)
                .succeeds()
                .stdout_only(format!("1.0{c}\n"));
        } else {
            new_ucmd!()
                .args(&args)
                .fails()
                .code_is(2)
                .stderr_only(format!("numfmt: invalid suffix in input: '1{c}'\n"));
        }
    }
}

#[test]
fn test_should_report_invalid_suffix_on_nan() {
    // GNU numfmt reports this one as “invalid number”
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("NaN")
        .fails()
        .stderr_is("numfmt: invalid suffix in input: 'NaN'\n");
}

#[test]
fn test_should_report_invalid_number_with_interior_junk() {
    // GNU numfmt reports this as “invalid suffix”
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("1x0K")
        .fails()
        .stderr_is("numfmt: invalid number: '1x0K'\n");
}

#[test]
fn test_should_skip_leading_space_from_stdin() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in(" 2Ki")
        .succeeds()
        .stdout_is("2048\n");

    // multi-line
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("\t1Ki\n  2K")
        .succeeds()
        .stdout_is("1024\n2000\n");
}

#[test]
fn test_should_convert_only_first_number_in_line() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("1Ki 2M 3G")
        .succeeds()
        .stdout_is("1024 2M 3G\n");
}

#[test]
fn test_leading_whitespace_should_imply_padding() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("   1K")
        .succeeds()
        .stdout_is(" 1000\n");

    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("    202Ki")
        .succeeds()
        .stdout_is("   206848\n");
}

#[test]
fn test_should_calculate_implicit_padding_per_line() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("   1Ki\n        2K")
        .succeeds()
        .stdout_is("  1024\n      2000\n");
}

#[test]
fn test_leading_whitespace_in_free_argument_should_imply_padding() {
    new_ucmd!()
        .args(&["--from=auto", "   1Ki"])
        .succeeds()
        .stdout_is("  1024\n");
}

#[test]
fn test_should_calculate_implicit_padding_per_free_argument() {
    new_ucmd!()
        .args(&["--from=auto", "   1Ki", "        2K"])
        .succeeds()
        .stdout_is("  1024\n      2000\n");
}

#[test]
fn test_to_si_should_truncate_output() {
    new_ucmd!()
        .args(&["--to=si"])
        .pipe_in_fixture("gnutest_si_input.txt")
        .succeeds()
        .stdout_is_fixture("gnutest_si_result.txt");
}

#[test]
fn test_to_iec_should_truncate_output() {
    new_ucmd!()
        .args(&["--to=iec"])
        .pipe_in_fixture("gnutest_iec_input.txt")
        .succeeds()
        .stdout_is_fixture("gnutest_iec_result.txt");
}

#[test]
fn test_to_iec_i_should_truncate_output() {
    new_ucmd!()
        .args(&["--to=iec-i"])
        .pipe_in_fixture("gnutest_iec_input.txt")
        .succeeds()
        .stdout_is_fixture("gnutest_iec-i_result.txt");
}

#[test]
fn test_format_selected_field() {
    new_ucmd!()
        .args(&["--from=auto", "--field", "3", "1K 2K 3K"])
        .succeeds()
        .stdout_only("1K 2K 3000\n");
    new_ucmd!()
        .args(&["--from=auto", "--field", "2", "1K 2K 3K"])
        .succeeds()
        .stdout_only("1K 2000 3K\n");
}

#[test]
fn test_format_selected_fields() {
    new_ucmd!()
        .args(&["--from=auto", "--field", "1,4,3", "1K 2K 3K 4K 5K 6K"])
        .succeeds()
        .stdout_only("1000 2K 3000 4000 5K 6K\n");

    new_ucmd!()
        .args(&["--from=auto", "--field", "1,4 3", "1K 2K 3K 4K 5K 6K"])
        .succeeds()
        .stdout_only("1000 2K 3000 4000 5K 6K\n");
}

#[test]
fn test_format_implied_range_and_field() {
    new_ucmd!()
        .args(&["--from=auto", "--field", "-2,4", "1K 2K 3K 4K 5K 6K"])
        .succeeds()
        .stdout_only("1000 2000 3K 4000 5K 6K\n");
}

#[test]
fn test_should_succeed_if_selected_field_out_of_range() {
    new_ucmd!()
        .args(&["--from=auto", "--field", "9", "1K 2K 3K"])
        .succeeds()
        .stdout_only("1K 2K 3K\n");
}

#[test]
fn test_format_selected_field_range() {
    new_ucmd!()
        .args(&["--from=auto", "--field", "2-5", "1K 2K 3K 4K 5K 6K"])
        .succeeds()
        .stdout_only("1K 2000 3000 4000 5000 6K\n");
}

#[test]
fn test_format_all_fields() {
    let all_fields_patterns = vec!["-", "-,3", "3,-", "1,-,3", "- 3"];

    for pattern in all_fields_patterns {
        new_ucmd!()
            .args(&["--from=auto", "--field", pattern, "1K 2K 3K 4K 5K 6K"])
            .succeeds()
            .stdout_only("1000 2000 3000 4000 5000 6000\n");
    }
}

#[test]
fn test_should_succeed_if_range_out_of_bounds() {
    new_ucmd!()
        .args(&["--from=auto", "--field", "5-10", "1K 2K 3K 4K 5K 6K"])
        .succeeds()
        .stdout_only("1K 2K 3K 4K 5000 6000\n");
}

#[test]
fn test_implied_initial_field_value() {
    new_ucmd!()
        .args(&["--from=auto", "--field", "-2", "1K 2K 3K"])
        .succeeds()
        .stdout_only("1000 2000 3K\n");

    // same as above but with the equal sign
    new_ucmd!()
        .args(&["--from=auto", "--field=-2", "1K 2K 3K"])
        .succeeds()
        .stdout_only("1000 2000 3K\n");
}

#[test]
fn test_field_df_example() {
    // df -B1 | numfmt --header --field 2-4 --to=si
    new_ucmd!()
        .args(&["--header", "--field", "2-4", "--to=si"])
        .pipe_in_fixture("df_input.txt")
        .succeeds()
        .stdout_is_fixture("df_expected.txt");
}

#[test]
fn test_delimiter_must_not_be_empty() {
    new_ucmd!().args(&["-d"]).fails();
}

#[test]
fn test_delimiter_must_not_be_more_than_one_character() {
    new_ucmd!()
        .args(&["--delimiter", "sad"])
        .fails()
        .stderr_is("numfmt: the delimiter must be a single character\n");
}

#[test]
fn test_delimiter_only() {
    new_ucmd!()
        .args(&["-d", ","])
        .pipe_in("1234,56")
        .succeeds()
        .stdout_only("1234,56\n");
}

#[test]
fn test_line_is_field_with_no_delimiter() {
    new_ucmd!()
        .args(&["-d,", "--to=iec"])
        .pipe_in("123456")
        .succeeds()
        .stdout_only("121K\n");
}

#[test]
fn test_delimiter_to_si() {
    new_ucmd!()
        .args(&["-d=,", "--to=si"])
        .pipe_in("1234,56")
        .succeeds()
        .stdout_only("1.3K,56\n");
}

#[test]
fn test_delimiter_skips_leading_whitespace() {
    new_ucmd!()
        .args(&["-d=,", "--to=si"])
        .pipe_in("     \t               1234,56")
        .succeeds()
        .stdout_only("1.3K,56\n");
}

#[test]
fn test_delimiter_preserves_leading_whitespace_in_unselected_fields() {
    new_ucmd!()
        .args(&["-d=|", "--to=si"])
        .pipe_in("             1000|   2000")
        .succeeds()
        .stdout_only("1.0K|   2000\n");
}

#[test]
fn test_delimiter_from_si() {
    new_ucmd!()
        .args(&["-d=,", "--from=si"])
        .pipe_in("1.2K,56")
        .succeeds()
        .stdout_only("1200,56\n");
}

#[test]
fn test_delimiter_overrides_whitespace_separator() {
    // GNU numfmt reports this as “invalid suffix”
    new_ucmd!()
        .args(&["-d,"])
        .pipe_in("1 234,56")
        .fails()
        .stderr_is("numfmt: invalid number: '1 234'\n");
}

#[test]
fn test_delimiter_with_padding() {
    new_ucmd!()
        .args(&["-d=|", "--to=si", "--padding=5"])
        .pipe_in("1000|2000")
        .succeeds()
        .stdout_only(" 1.0K|2000\n");
}

#[test]
fn test_delimiter_with_padding_and_fields() {
    new_ucmd!()
        .args(&["-d=|", "--to=si", "--padding=5", "--field=-"])
        .pipe_in("1000|2000")
        .succeeds()
        .stdout_only(" 1.0K| 2.0K\n");
}

#[test]
fn test_round() {
    for (method, exp) in [
        ("from-zero", ["9.1K", "-9.1K", "9.1K", "-9.1K"]),
        ("from-zer", ["9.1K", "-9.1K", "9.1K", "-9.1K"]),
        ("f", ["9.1K", "-9.1K", "9.1K", "-9.1K"]),
        ("towards-zero", ["9.0K", "-9.0K", "9.0K", "-9.0K"]),
        ("up", ["9.1K", "-9.0K", "9.1K", "-9.0K"]),
        ("down", ["9.0K", "-9.1K", "9.0K", "-9.1K"]),
        ("nearest", ["9.0K", "-9.0K", "9.1K", "-9.1K"]),
        ("near", ["9.0K", "-9.0K", "9.1K", "-9.1K"]),
        ("n", ["9.0K", "-9.0K", "9.1K", "-9.1K"]),
    ] {
        new_ucmd!()
            .args(&[
                "--to=si",
                &format!("--round={method}"),
                "--",
                "9001",
                "-9001",
                "9099",
                "-9099",
            ])
            .succeeds()
            .stdout_only(exp.join("\n") + "\n");
    }
}

#[test]
fn test_round_with_to_unit() {
    for (method, exp) in [
        ("from-zero", ["6", "-6", "5.9", "-5.9", "5.86", "-5.86"]),
        ("towards-zero", ["5", "-5", "5.8", "-5.8", "5.85", "-5.85"]),
        ("up", ["6", "-5", "5.9", "-5.8", "5.86", "-5.85"]),
        ("down", ["5", "-6", "5.8", "-5.9", "5.85", "-5.86"]),
        ("nearest", ["6", "-6", "5.9", "-5.9", "5.86", "-5.86"]),
    ] {
        new_ucmd!()
            .args(&[
                "--to-unit=1024",
                &format!("--round={method}"),
                "--",
                "6000",
                "-6000",
                "6000.0",
                "-6000.0",
                "6000.00",
                "-6000.00",
            ])
            .succeeds()
            .stdout_only(exp.join("\n") + "\n");
    }
}

#[test]
fn test_suffix_is_added_if_not_supplied() {
    new_ucmd!()
        .args(&["--suffix=TEST"])
        .pipe_in("1000")
        .succeeds()
        .stdout_only("1000TEST\n");
}

#[test]
fn test_suffix_is_preserved() {
    new_ucmd!()
        .args(&["--suffix=TEST"])
        .pipe_in("1000TEST")
        .succeeds()
        .stdout_only("1000TEST\n");
}

#[test]
fn test_suffix_is_only_applied_to_selected_field() {
    new_ucmd!()
        .args(&["--suffix=TEST", "--field=2"])
        .pipe_in("1000 2000 3000")
        .succeeds()
        .stdout_only("1000 2000TEST 3000\n");
}

#[test]
fn test_transform_with_suffix_on_input() {
    new_ucmd!()
        .args(&["--suffix=b", "--to=si"])
        .pipe_in("2000b")
        .succeeds()
        .stdout_only("2.0Kb\n");
}

#[test]
fn test_transform_without_suffix_on_input() {
    new_ucmd!()
        .args(&["--suffix=b", "--to=si"])
        .pipe_in("2000")
        .succeeds()
        .stdout_only("2.0Kb\n");
}

#[test]
fn test_transform_with_suffix_and_delimiter() {
    new_ucmd!()
        .args(&["--suffix=b", "--to=si", "-d=|"])
        .pipe_in("1000b|2000|3000")
        .succeeds()
        .stdout_only("1.0Kb|2000|3000\n");
}

#[test]
fn test_suffix_with_padding() {
    new_ucmd!()
        .args(&["--suffix=pad", "--padding=12"])
        .pipe_in("1000 2000 3000")
        .succeeds()
        .stdout_only("     1000pad 2000 3000\n");
}

#[test]
fn test_invalid_stdin_number_returns_status_2() {
    new_ucmd!().pipe_in("hello").fails().code_is(2);
}

#[test]
fn test_invalid_stdin_number_in_middle_of_input() {
    new_ucmd!()
        .pipe_in("100\nhello\n200")
        .ignore_stdin_write_error()
        .fails()
        .stdout_is("100\n")
        .code_is(2);
}

#[test]
fn test_invalid_stdin_number_with_warn_returns_status_0() {
    new_ucmd!()
        .args(&["--invalid=warn"])
        .pipe_in("4Q")
        .succeeds()
        .stdout_is("4Q\n")
        .stderr_is("numfmt: invalid suffix in input: '4Q'\n");
}

#[test]
fn test_invalid_stdin_number_with_ignore_returns_status_0() {
    new_ucmd!()
        .args(&["--invalid=ignore"])
        .pipe_in("4Q")
        .succeeds()
        .stdout_only("4Q\n");
}

#[test]
fn test_invalid_stdin_number_with_abort_returns_status_2() {
    new_ucmd!()
        .args(&["--invalid=abort"])
        .pipe_in("4Q")
        .fails()
        .code_is(2)
        .stderr_only("numfmt: invalid suffix in input: '4Q'\n");
}

#[test]
fn test_invalid_stdin_number_with_fail_returns_status_2() {
    new_ucmd!()
        .args(&["--invalid=fail"])
        .pipe_in("4Q")
        .fails()
        .code_is(2)
        .stdout_is("4Q\n")
        .stderr_is("numfmt: invalid suffix in input: '4Q'\n");
}

#[test]
fn test_invalid_arg_number_with_warn_returns_status_0() {
    new_ucmd!()
        .args(&["--invalid=warn", "4Q"])
        .succeeds()
        .stdout_is("4Q\n")
        .stderr_is("numfmt: invalid suffix in input: '4Q'\n");
}

#[test]
fn test_invalid_arg_number_with_ignore_returns_status_0() {
    new_ucmd!()
        .args(&["--invalid=ignore", "4Q"])
        .succeeds()
        .stdout_only("4Q\n");
}

#[test]
fn test_invalid_arg_number_with_abort_returns_status_2() {
    new_ucmd!()
        .args(&["--invalid=abort", "4Q"])
        .fails()
        .code_is(2)
        .stderr_only("numfmt: invalid suffix in input: '4Q'\n");
}

#[test]
fn test_invalid_arg_number_with_fail_returns_status_2() {
    new_ucmd!()
        .args(&["--invalid=fail", "4Q"])
        .fails()
        .code_is(2)
        .stdout_is("4Q\n")
        .stderr_is("numfmt: invalid suffix in input: '4Q'\n");
}

#[test]
fn test_invalid_argument_returns_status_1() {
    new_ucmd!()
        .args(&["--header=hello"])
        .pipe_in("53478")
        .ignore_stdin_write_error()
        .fails()
        .code_is(1);
}

#[test]
fn test_invalid_padding_value() {
    let padding_values = vec!["A", "0"];

    for padding_value in padding_values {
        new_ucmd!()
            .arg(format!("--padding={padding_value}"))
            .arg("5")
            .fails()
            .code_is(1)
            .stderr_contains(format!("invalid padding value '{padding_value}'"));
    }
}

#[test]
fn test_from_unit() {
    new_ucmd!()
        .args(&["--from-unit=512", "4"])
        .succeeds()
        .stdout_is("2048\n");
}

#[test]
fn test_to_unit() {
    new_ucmd!()
        .args(&["--to-unit=512", "2048"])
        .succeeds()
        .stdout_is("4\n");
}

#[test]
fn test_invalid_unit_size() {
    let commands = vec!["from", "to"];
    let invalid_sizes = vec!["A", "0", "18446744073709551616"];

    for command in commands {
        for invalid_size in &invalid_sizes {
            new_ucmd!()
                .arg(format!("--{command}-unit={invalid_size}"))
                .fails()
                .code_is(1)
                .stderr_contains(format!("invalid unit size: '{invalid_size}'"));
        }
    }
}

#[test]
fn test_valid_but_forbidden_suffix() {
    let numbers = vec!["12K", "12Ki"];

    for number in numbers {
        new_ucmd!()
            .arg(number)
            .fails()
            .code_is(2)
            .stderr_contains(format!(
                "rejecting suffix in input: '{number}' (consider using --from)"
            ));
    }
}

#[test]
fn test_format() {
    new_ucmd!()
        .args(&["--format=--%f--", "50"])
        .succeeds()
        .stdout_is("--50--\n");
}

#[test]
fn test_format_with_separate_value() {
    new_ucmd!()
        .args(&["--format", "--%f--", "50"])
        .succeeds()
        .stdout_is("--50--\n");
}

#[test]
fn test_format_padding_with_prefix_and_suffix() {
    new_ucmd!()
        .args(&["--format=--%6f--", "50"])
        .succeeds()
        .stdout_is("--    50--\n");
}

#[test]
fn test_format_negative_padding_with_prefix_and_suffix() {
    new_ucmd!()
        .args(&["--format=--%-6f--", "50"])
        .succeeds()
        .stdout_is("--50    --\n");
}

#[test]
fn test_format_with_format_padding_overriding_padding_option() {
    new_ucmd!()
        .args(&["--format=%6f", "--padding=10", "1234"])
        .succeeds()
        .stdout_is("  1234\n");
}

#[test]
fn test_format_with_format_padding_overriding_implicit_padding() {
    new_ucmd!()
        .args(&["--format=%6f", "      1234"])
        .succeeds()
        .stdout_is("  1234\n");
}

#[test]
fn test_format_with_negative_format_padding_and_suffix() {
    new_ucmd!()
        .args(&["--format=%-6f", "1234 ?"])
        .succeeds()
        .stdout_is("1234   ?\n");
}

#[test]
fn test_format_with_zero_padding() {
    let formats = vec!["%06f", "%0 6f"];

    for format in formats {
        new_ucmd!()
            .args(&[format!("--format={format}"), String::from("1234")])
            .succeeds()
            .stdout_is("001234\n");
    }
}

#[test]
fn test_format_with_zero_padding_and_padding_option() {
    new_ucmd!()
        .args(&["--format=%06f", "--padding=8", "1234"])
        .succeeds()
        .stdout_is("  001234\n");
}

#[test]
fn test_format_with_zero_padding_and_negative_padding_option() {
    new_ucmd!()
        .args(&["--format=%06f", "--padding=-8", "1234"])
        .succeeds()
        .stdout_is("001234  \n");
}

#[test]
fn test_format_with_zero_padding_and_implicit_padding() {
    new_ucmd!()
        .args(&["--format=%06f", "    1234"])
        .succeeds()
        .stdout_is("  001234\n");
}

#[test]
fn test_format_with_zero_padding_and_suffix() {
    new_ucmd!()
        .args(&["--format=%06f", "1234 ?"])
        .succeeds()
        .stdout_is("001234 ?\n");
}

#[test]
fn test_format_with_precision() {
    let values = vec![("0.99", "1.0"), ("1", "1.0"), ("1.01", "1.1")];

    for (input, expected) in values {
        new_ucmd!()
            .args(&["--format=%.1f", input])
            .succeeds()
            .stdout_is(format!("{expected}\n"));
    }

    let values = vec![("0.99", "0.99"), ("1", "1.00"), ("1.01", "1.01")];

    for (input, expected) in values {
        new_ucmd!()
            .args(&["--format=%.2f", input])
            .succeeds()
            .stdout_is(format!("{expected}\n"));
    }
}

#[test]
fn test_format_with_precision_and_down_rounding() {
    let values = vec![("0.99", "0.9"), ("1", "1.0"), ("1.01", "1.0")];

    for (input, expected) in values {
        new_ucmd!()
            .args(&["--format=%.1f", input, "--round=down"])
            .succeeds()
            .stdout_is(format!("{expected}\n"));
    }
}

#[test]
fn test_format_with_precision_and_to_arg() {
    let values = vec![("%.1f", "10.0G"), ("%.4f", "9.9913G")];

    for (format, expected) in values {
        new_ucmd!()
            .args(&[
                format!("--format={format}"),
                "9991239123".to_string(),
                "--to=si".to_string(),
            ])
            .succeeds()
            .stdout_is(format!("{expected}\n"));
    }
}

#[test]
fn test_format_preserve_trailing_zeros_if_no_precision_is_specified() {
    let values = vec!["10.0", "0.0100"];

    for value in values {
        new_ucmd!()
            .args(&["--format=%f", value])
            .succeeds()
            .stdout_is(format!("{value}\n"));
    }
}

#[test]
fn test_format_without_percentage_directive() {
    let invalid_formats = vec!["", "hello"];

    for invalid_format in invalid_formats {
        new_ucmd!()
            .arg(format!("--format={invalid_format}"))
            .fails()
            .code_is(1)
            .stderr_contains(format!("format '{invalid_format}' has no % directive"));
    }
}

#[test]
fn test_format_with_percentage_directive_at_end() {
    let invalid_format = "hello%";

    new_ucmd!()
        .arg(format!("--format={invalid_format}"))
        .fails()
        .code_is(1)
        .stderr_contains(format!("format '{invalid_format}' ends in %"));
}

#[test]
fn test_format_with_too_many_percentage_directives() {
    let invalid_format = "%f %f";

    new_ucmd!()
        .arg(format!("--format={invalid_format}"))
        .fails()
        .code_is(1)
        .stderr_contains(format!(
            "format '{invalid_format}' has too many % directives"
        ));
}

#[test]
fn test_format_with_invalid_format() {
    let invalid_formats = vec!["%d", "% -43 f"];

    for invalid_format in invalid_formats {
        new_ucmd!()
            .arg(format!("--format={invalid_format}"))
            .fails()
            .code_is(1)
            .stderr_contains(format!(
                "invalid format '{invalid_format}', directive must be %[0]['][-][N][.][N]f"
            ));
    }
}

#[test]
fn test_format_with_width_overflow() {
    let invalid_format = "%18446744073709551616f";
    new_ucmd!()
        .arg(format!("--format={invalid_format}"))
        .fails()
        .code_is(1)
        .stderr_contains(format!(
            "invalid format '{invalid_format}' (width overflow)"
        ));
}

#[test]
fn test_format_with_invalid_precision() {
    let invalid_formats = vec!["%.-1f", "%.+1f", "%. 1f", "%.18446744073709551616f"];

    for invalid_format in invalid_formats {
        new_ucmd!()
            .arg(format!("--format={invalid_format}"))
            .fails()
            .code_is(1)
            .stderr_contains(format!("invalid precision in format '{invalid_format}'"));
    }
}

#[test]
fn test_format_grouping_conflicts_with_to_option() {
    new_ucmd!()
        .args(&["--format=%'f", "--to=si"])
        .fails()
        .code_is(1)
        .stderr_contains("grouping cannot be combined with --to");
}
