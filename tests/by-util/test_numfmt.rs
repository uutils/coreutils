// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (paths) gnutest ronna quetta unitless

use uutests::new_ucmd;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

// This test failed when fixing #11653.
// Add a `--` separator to ensure floats are not rounded(it match the gnu pattern).
#[test]
fn test_should_not_round_floats() {
    new_ucmd!()
        .args(&["--", "0.99", "1.01", "1.1", "1.22", ".1", "-0.1"])
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
        .stdout_is("1000\n1100000\n100000000");
}

#[test]
fn test_from_iec() {
    new_ucmd!()
        .args(&["--from=iec"])
        .pipe_in("1024\n1.1M\n0.1G")
        .succeeds()
        .stdout_is("1024\n1153434\n107374183");
}

#[test]
fn test_from_iec_i() {
    new_ucmd!()
        .args(&["--from=iec-i"])
        .pipe_in("1.1Mi\n0.1Gi")
        .succeeds()
        .stdout_is("1153434\n107374183");
}

#[test]
fn test_from_iec_i_requires_suffix() {
    new_ucmd!()
        .args(&["--from=iec-i", "10M"])
        .fails_with_code(2)
        .stderr_is("numfmt: missing 'i' suffix in input: '10M' (e.g Ki/Mi/Gi)\n");
}

#[test]
fn test_from_iec_fails_if_i_suffix() {
    new_ucmd!()
        .args(&["--from=iec", "10Mi"])
        .fails_with_code(2)
        .stderr_is("numfmt: invalid suffix in input '10Mi': 'i'\n");
}

#[test]
fn test_from_iec_i_without_suffix_are_bytes() {
    new_ucmd!()
        .args(&["--from=iec-i", "1024"])
        .succeeds()
        .stdout_is("1024\n");
}

#[test]
fn test_from_auto() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("1K\n1Ki")
        .succeeds()
        .stdout_is("1000\n1024");
}

#[test]
fn test_to_si() {
    new_ucmd!()
        .args(&["--to=si"])
        .pipe_in("1000\n1100000\n100000000")
        .succeeds()
        .stdout_is("1.0k\n1.1M\n100M");
}

#[test]
fn test_to_iec() {
    new_ucmd!()
        .args(&["--to=iec"])
        .pipe_in("1024\n1153434\n107374182")
        .succeeds()
        .stdout_is("1.0K\n1.2M\n103M");
}

#[test]
fn test_to_iec_i() {
    new_ucmd!()
        .args(&["--to=iec-i"])
        .pipe_in("1024\n1153434\n107374182")
        .succeeds()
        .stdout_is("1.0Ki\n1.2Mi\n103Mi");
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
        .stdout_is("    1000\n 1100000\n100000000");
}

#[test]
fn test_negative_padding() {
    new_ucmd!()
        .args(&["--from=si", "--padding=-8"])
        .pipe_in("1K\n1.1M\n0.1G")
        .succeeds()
        .stdout_is("1000    \n1100000 \n100000000");
}

#[test]
fn test_header() {
    new_ucmd!()
        .args(&["--from=si", "--header=2"])
        .pipe_in("header\nheader2\n1K\n1.1M\n0.1G")
        .succeeds()
        .stdout_is("header\nheader2\n1000\n1100000\n100000000");
}

#[test]
fn test_header_default() {
    new_ucmd!()
        .args(&["--from=si", "--header"])
        .pipe_in("header\n1K\n1.1M\n0.1G")
        .succeeds()
        .stdout_is("header\n1000\n1100000\n100000000");
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
        .stdout_is("-1000\n-1100000\n-100000000");
    new_ucmd!()
        .args(&["--to=iec-i"])
        .pipe_in("-1024\n-1153434\n-107374182")
        .succeeds()
        .stdout_is("-1.0Ki\n-1.2Mi\n-103Mi");
}

#[test]
fn test_negative_zero() {
    new_ucmd!()
        .pipe_in("-0\n-0.0")
        .succeeds()
        .stdout_is("0\n0.0");
}

#[test]
fn test_no_op() {
    new_ucmd!()
        .pipe_in("1024\n1234567")
        .succeeds()
        .stdout_is("1024\n1234567");
}

#[test]
fn test_normalize() {
    new_ucmd!()
        .args(&["--from=si", "--to=si"])
        .pipe_in("10000000K\n0.001K")
        .succeeds()
        .stdout_is("10G\n1");
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
    let valid_suffixes = ['K', 'M', 'G', 'T', 'P', 'E', 'Z', 'Y', 'R', 'Q', 'k'];

    for c in ('A'..='Z').chain('a'..='z') {
        let args = ["--from=si", "--to=si", &format!("1{c}")];

        if valid_suffixes.contains(&c) {
            let s = if c == 'K' { 'k' } else { c };
            new_ucmd!()
                .args(&args)
                .succeeds()
                .stdout_only(format!("1.0{s}\n"));
        } else {
            new_ucmd!()
                .args(&args)
                .fails_with_code(2)
                .stderr_only(format!("numfmt: invalid suffix in input: '1{c}'\n"));
        }
    }
}

#[test]
fn test_invalid_following_valid_suffix() {
    let valid_suffixes = ['K', 'M', 'G', 'T', 'P', 'E', 'Z', 'Y', 'R', 'Q', 'k'];

    for valid_suffix in valid_suffixes {
        for c in ('A'..='Z').chain('a'..='z') {
            let args = ["--from=si", "--to=si", &format!("1{valid_suffix}{c}")];

            new_ucmd!()
                .args(&args)
                .fails_with_code(2)
                .stderr_only(format!(
                    "numfmt: invalid suffix in input '1{valid_suffix}{c}': '{c}'\n"
                ));
        }
    }
}

#[test]
fn test_long_invalid_suffix() {
    let args = ["--from=si", "--to=si", "1500VVVVVVVV"];

    new_ucmd!()
        .args(&args)
        .fails_with_code(2)
        .stderr_only("numfmt: invalid suffix in input: '1500VVVVVVVV'\n");
}

#[test]
fn test_should_report_invalid_suffix_on_nan() {
    // GNU numfmt reports this one as "invalid number"
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("NaN")
        .fails()
        .stderr_is("numfmt: invalid number: 'NaN'\n");
}

#[test]
fn test_should_report_invalid_number_with_interior_junk() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("1x0K")
        .fails()
        .stderr_is("numfmt: invalid suffix in input: '1x0K'\n");
}

#[test]
fn test_should_report_invalid_number_with_sign_after_decimal() {
    new_ucmd!()
        .args(&["--", "-0.-1"])
        .fails_with_code(2)
        .stderr_is("numfmt: invalid number: '-0.-1'\n");
}

#[test]
fn test_should_skip_leading_space_from_stdin() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in(" 2Ki")
        .succeeds()
        .stdout_is("2048");

    // multi-line
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("\t1Ki\n  2K")
        .succeeds()
        .stdout_is("1024\n2000");
}

#[test]
fn test_should_convert_only_first_number_in_line() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("1Ki 2M 3G")
        .succeeds()
        .stdout_is("1024 2M 3G");
}

#[test]
fn test_leading_whitespace_should_imply_padding() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("   1K")
        .succeeds()
        .stdout_is(" 1000");

    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("    202Ki")
        .succeeds()
        .stdout_is("   206848");
}

#[test]
fn test_should_calculate_implicit_padding_per_line() {
    new_ucmd!()
        .args(&["--from=auto"])
        .pipe_in("   1Ki\n        2K")
        .succeeds()
        .stdout_is("  1024\n      2000");
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
        .stdout_only("1234,56");
}

#[test]
fn test_line_is_field_with_no_delimiter() {
    new_ucmd!()
        .args(&["-d,", "--to=iec"])
        .pipe_in("123456")
        .succeeds()
        .stdout_only("121K");
}

#[test]
fn test_delimiter_to_si() {
    new_ucmd!()
        .args(&["-d=,", "--to=si"])
        .pipe_in("1234,56")
        .succeeds()
        .stdout_only("1.3k,56");
}

#[test]
fn test_delimiter_skips_leading_whitespace() {
    new_ucmd!()
        .args(&["-d=,", "--to=si"])
        .pipe_in("     \t               1234,56")
        .succeeds()
        .stdout_only("1.3k,56");
}

#[test]
fn test_delimiter_preserves_leading_whitespace_in_unselected_fields() {
    new_ucmd!()
        .args(&["-d=|", "--to=si"])
        .pipe_in("             1000|   2000")
        .succeeds()
        .stdout_only("1.0k|   2000");
}

#[test]
fn test_delimiter_from_si() {
    new_ucmd!()
        .args(&["-d=,", "--from=si"])
        .pipe_in("1.2K,56")
        .succeeds()
        .stdout_only("1200,56");
}

#[test]
fn test_delimiter_overrides_whitespace_separator() {
    new_ucmd!()
        .args(&["-d,"])
        .pipe_in("1 234,56")
        .fails()
        .stderr_is("numfmt: invalid suffix in input: '1 234'\n");
}

#[test]
fn test_delimiter_with_padding() {
    new_ucmd!()
        .args(&["-d=|", "--to=si", "--padding=5"])
        .pipe_in("1000|2000")
        .succeeds()
        .stdout_only(" 1.0k|2000");
}

#[test]
fn test_delimiter_with_padding_and_fields() {
    new_ucmd!()
        .args(&["-d=|", "--to=si", "--padding=5", "--field=-"])
        .pipe_in("1000|2000")
        .succeeds()
        .stdout_only(" 1.0k| 2.0k");
}

#[test]
fn test_round() {
    for (method, exp) in [
        ("from-zero", ["9.1k", "-9.1k", "9.1k", "-9.1k"]),
        ("from-zer", ["9.1k", "-9.1k", "9.1k", "-9.1k"]),
        ("f", ["9.1k", "-9.1k", "9.1k", "-9.1k"]),
        ("towards-zero", ["9.0k", "-9.0k", "9.0k", "-9.0k"]),
        ("up", ["9.1k", "-9.0k", "9.1k", "-9.0k"]),
        ("down", ["9.0k", "-9.1k", "9.0k", "-9.1k"]),
        ("nearest", ["9.0k", "-9.0k", "9.1k", "-9.1k"]),
        ("near", ["9.0k", "-9.0k", "9.1k", "-9.1k"]),
        ("n", ["9.0k", "-9.0k", "9.1k", "-9.1k"]),
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
fn test_to_unitless_small_values_use_display_rounding() {
    new_ucmd!()
        .args(&[
            "--to=si", "--", "0.4", "0.5", "0.6", "1.4", "3.14", "-0.4", "-0.5", "-0.6", "-1.4",
        ])
        .succeeds()
        .stdout_only("0\n0\n1\n1\n3\n-0\n-0\n-1\n-1\n");
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
fn test_to_unit_with_unitless_small_value_uses_display_rounding() {
    new_ucmd!()
        .args(&["--to=iec", "--to-unit=689", "701"])
        .succeeds()
        .stdout_only("1\n");

    new_ucmd!()
        .args(&["--to=si", "--to-unit=689", "701"])
        .succeeds()
        .stdout_only("1\n");

    new_ucmd!()
        .args(&["--to=none", "--to-unit=689", "701"])
        .succeeds()
        .stdout_only("2\n");
}

#[test]
fn test_suffix_is_added_if_not_supplied() {
    new_ucmd!()
        .args(&["--suffix=TEST"])
        .pipe_in("1000")
        .succeeds()
        .stdout_only("1000TEST");
}

#[test]
fn test_suffix_is_preserved() {
    new_ucmd!()
        .args(&["--suffix=TEST"])
        .pipe_in("1000TEST")
        .succeeds()
        .stdout_only("1000TEST");
}

#[test]
fn test_suffix_is_only_applied_to_selected_field() {
    new_ucmd!()
        .args(&["--suffix=TEST", "--field=2"])
        .pipe_in("1000 2000 3000")
        .succeeds()
        .stdout_only("1000 2000TEST 3000");
}

#[test]
fn test_transform_with_suffix_on_input() {
    new_ucmd!()
        .args(&["--suffix=b", "--to=si"])
        .pipe_in("2000b")
        .succeeds()
        .stdout_only("2.0kb");
}

#[test]
fn test_transform_without_suffix_on_input() {
    new_ucmd!()
        .args(&["--suffix=b", "--to=si"])
        .pipe_in("2000")
        .succeeds()
        .stdout_only("2.0kb");
}

#[test]
fn test_transform_with_suffix_and_delimiter() {
    new_ucmd!()
        .args(&["--suffix=b", "--to=si", "-d=|"])
        .pipe_in("1000b|2000|3000")
        .succeeds()
        .stdout_only("1.0kb|2000|3000");
}

#[test]
fn test_suffix_with_padding() {
    new_ucmd!()
        .args(&["--suffix=pad", "--padding=12"])
        .pipe_in("1000 2000 3000")
        .succeeds()
        .stdout_only("     1000pad 2000 3000");
}

#[test]
fn test_invalid_stdin_number_returns_status_2() {
    new_ucmd!().pipe_in("hello").fails_with_code(2);
}

#[test]
fn test_invalid_stdin_number_in_middle_of_input() {
    new_ucmd!()
        .pipe_in("100\nhello\n200")
        .ignore_stdin_write_error()
        .fails_with_code(2)
        .stdout_is("100\n");
}

#[test]
fn test_invalid_stdin_number_with_warn_returns_status_0() {
    new_ucmd!()
        .args(&["--invalid=warn"])
        .pipe_in("4Q")
        .succeeds()
        .stdout_is("4Q")
        .stderr_is("numfmt: rejecting suffix in input: '4Q' (consider using --from)\n");
}

#[test]
fn test_invalid_stdin_number_with_ignore_returns_status_0() {
    new_ucmd!()
        .args(&["--invalid=ignore"])
        .pipe_in("4Q")
        .succeeds()
        .stdout_only("4Q");
}

#[test]
fn test_invalid_stdin_number_with_abort_returns_status_2() {
    new_ucmd!()
        .args(&["--invalid=abort"])
        .pipe_in("4Q")
        .fails_with_code(2)
        .stderr_only("numfmt: rejecting suffix in input: '4Q' (consider using --from)\n");
}

#[test]
fn test_invalid_stdin_number_with_fail_returns_status_2() {
    new_ucmd!()
        .args(&["--invalid=fail"])
        .pipe_in("4Q")
        .fails_with_code(2)
        .stdout_is("4Q")
        .stderr_is("numfmt: rejecting suffix in input: '4Q' (consider using --from)\n");
}

#[test]
fn test_invalid_arg_number_with_warn_returns_status_0() {
    new_ucmd!()
        .args(&["--invalid=warn", "4Q"])
        .succeeds()
        .stdout_is("4Q\n")
        .stderr_is("numfmt: rejecting suffix in input: '4Q' (consider using --from)\n");
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
        .fails_with_code(2)
        .stderr_only("numfmt: rejecting suffix in input: '4Q' (consider using --from)\n");
}

#[test]
fn test_invalid_arg_number_with_fail_returns_status_2() {
    new_ucmd!()
        .args(&["--invalid=fail", "4Q"])
        .fails_with_code(2)
        .stdout_is("4Q\n")
        .stderr_is("numfmt: rejecting suffix in input: '4Q' (consider using --from)\n");
}

#[test]
fn test_invalid_argument_returns_status_1() {
    new_ucmd!()
        .args(&["--header=hello"])
        .pipe_in("53478")
        .ignore_stdin_write_error()
        .fails_with_code(1);
}

#[test]
fn test_invalid_padding_value() {
    let padding_values = vec!["A", "0"];

    for padding_value in padding_values {
        new_ucmd!()
            .arg(format!("--padding={padding_value}"))
            .arg("5")
            .fails_with_code(1)
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
                .fails_with_code(1)
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
            .fails_with_code(2)
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
fn test_format_with_precision_and_unitless_to_arg() {
    new_ucmd!()
        .args(&["--to=si", "--format=%.1f", "3.14"])
        .succeeds()
        .stdout_is("4.0\n");

    new_ucmd!()
        .args(&["--to=si", "--format=%.1f", "--round=down", "3.14"])
        .succeeds()
        .stdout_is("3.0\n");
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
            .fails_with_code(1)
            .stderr_contains(format!("format '{invalid_format}' has no % directive"));
    }
}

#[test]
fn test_format_with_percentage_directive_at_end() {
    let invalid_format = "hello%";

    new_ucmd!()
        .arg(format!("--format={invalid_format}"))
        .fails_with_code(1)
        .stderr_contains(format!("format '{invalid_format}' ends in %"));
}

#[test]
fn test_format_with_too_many_percentage_directives() {
    let invalid_format = "%f %f";

    new_ucmd!()
        .arg(format!("--format={invalid_format}"))
        .fails_with_code(1)
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
            .fails_with_code(1)
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
        .fails_with_code(1)
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
            .fails_with_code(1)
            .stderr_contains(format!("invalid precision in format '{invalid_format}'"));
    }
}

#[test]
fn test_format_grouping_conflicts_with_to_option() {
    new_ucmd!()
        .args(&["--format=%'f", "--to=si"])
        .fails_with_code(1)
        .stderr_contains("grouping cannot be combined with --to");
}

#[test]
fn test_grouping_conflicts_with_format_option() {
    new_ucmd!()
        .args(&["--format=%f", "--grouping"])
        .fails_with_code(1)
        .stderr_contains("--grouping cannot be combined with --format");
}

#[test]
fn test_zero_terminated_command_line_args() {
    new_ucmd!()
        .args(&["--zero-terminated", "--to=si", "1000"])
        .succeeds()
        .stdout_is("1.0k\x00");

    new_ucmd!()
        .args(&["-z", "--to=si", "1000"])
        .succeeds()
        .stdout_is("1.0k\x00");

    new_ucmd!()
        .args(&["-z", "--to=si", "1000", "2000"])
        .succeeds()
        .stdout_is("1.0k\x002.0k\x00");
}

#[test]
fn test_zero_terminated_input() {
    let values = vec![
        ("1000", "1.0k"),
        ("1000\x00", "1.0k\x00"),
        ("1000\x002000\x00", "1.0k\x002.0k\x00"),
    ];

    for (input, expected) in values {
        new_ucmd!()
            .args(&["-z", "--to=si"])
            .pipe_in(input)
            .succeeds()
            .stdout_is(expected);
    }
}

#[test]
fn test_zero_terminated_embedded_newline() {
    new_ucmd!()
        .args(&["-z", "--from=si", "--field=-"])
        .pipe_in("1K\n2K\x003K\n4K\x00")
        .succeeds()
        // Newlines get replaced by a single space
        .stdout_is("1000 2000\x003000 4000\x00");
}

#[cfg(unix)]
#[test]
#[cfg_attr(wasi_runner, ignore = "WASI: argv/filenames must be valid UTF-8")]
fn test_non_utf8_delimiter() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    // Single-byte non-UTF8 (0xFF) and multi-byte (0xA2E3, e.g. GB18030)
    for delim in [&[0xFFu8][..], &[0xA2, 0xE3]] {
        let input: Vec<u8> = [b"1", delim, b"2K"].concat();
        let expected: Vec<u8> = [b"1", delim, b"2000\n"].concat();
        new_ucmd!()
            .args(&["--from=si", "--field=2", "-d"])
            .arg(OsStr::from_bytes(delim))
            .arg(OsStr::from_bytes(&input))
            .succeeds()
            .stdout_is_bytes(expected);
    }
}

#[test]
fn test_unit_separator() {
    for (args, expected) in [
        (&["--to=si", "--unit-separator= ", "1000"][..], "1.0 k\n"),
        (&["--to=iec", "--unit-separator= ", "1024"], "1.0 K\n"),
        (&["--to=iec-i", "--unit-separator= ", "2048"], "2.0 Ki\n"),
        (&["--to=si", "--unit-separator=__", "1000"], "1.0__k\n"),
        (&["--to=si", "--unit-separator= ", "500"], "500\n"), // no unit = no separator
    ] {
        new_ucmd!().args(args).succeeds().stdout_only(expected);
    }
}

#[test]
fn test_debug_warnings() {
    new_ucmd!()
        .args(&["--debug", "4096"])
        .succeeds()
        .stdout_is("4096\n")
        .stderr_is("numfmt: no conversion option specified\n");

    new_ucmd!()
        .args(&["--debug", "--padding=10", "4096"])
        .succeeds()
        .stdout_only("      4096\n");

    new_ucmd!()
        .args(&["--debug", "--header", "--to=iec", "4096"])
        .succeeds()
        .stdout_is("4.0K\n")
        .stderr_is("numfmt: --header ignored with command-line input\n");

    new_ucmd!()
        .env("LC_ALL", "C")
        .args(&["--debug", "--grouping", "--from=si", "4.0K"])
        .succeeds()
        .stdout_is("4000\n")
        .stderr_is("numfmt: grouping has no effect in this locale\n");
}

#[test]
fn test_debug_reports_failed_conversions_summary() {
    new_ucmd!()
        .args(&[
            "--invalid=fail",
            "--debug",
            "--to=si",
            "1000",
            "Foo",
            "3000",
        ])
        .fails_with_code(2)
        .stdout_is("1.0k\nFoo\n3.0k\n")
        .stderr_is(
            "numfmt: invalid number: 'Foo'\nnumfmt: failed to convert some of the input numbers\n",
        );
}

#[test]
fn test_invalid_fail_with_fields_does_not_duplicate_output() {
    new_ucmd!()
        .args(&["--invalid=fail", "--field=2", "--from=si", "--to=iec"])
        .pipe_in("A 1K x\nB Foo y\nC 3G z\n")
        .fails_with_code(2)
        .stdout_is("A 1000 x\nB Foo y\nC 2.8G z\n")
        .stderr_is("numfmt: invalid number: 'Foo'\n");
}

#[test]
fn test_abort_with_fields_preserves_partial_output() {
    new_ucmd!()
        .args(&["--field=3", "--from=auto", "Hello 40M World 90G"])
        .fails_with_code(2)
        .stdout_is("Hello 40M ")
        .stderr_is("numfmt: invalid number: 'World'\n");
}

#[test]
fn test_rejects_malformed_number_forms() {
    new_ucmd!()
        .args(&["--from=si", "12.K"])
        .fails_with_code(2)
        .stderr_contains("invalid number: '12.K'");

    new_ucmd!()
        .args(&["--from=si", "--delimiter=,", "12.  2"])
        .fails_with_code(2)
        .stderr_contains("invalid number: '12.  2'");

    new_ucmd!()
        .arg("..1")
        .fails_with_code(2)
        .stderr_contains("invalid suffix in input: '..1'");
}

#[test]
fn test_empty_delimiter_success() {
    for (args, expected) in [
        // Single space between number and suffix is allowed by default
        (&["-d", "", "--from=si", "4.0 K"][..], "4000\n"),
        // Trailing spaces without suffix are allowed
        (&["-d", "", "--from=si", "4  "], "4\n"),
        (&["-d", "", "--from=auto", "2 "], "2\n"),
        (&["-d", "", "--from=auto", "2  "], "2\n"),
        // Trailing space after suffix is allowed
        (&["-d", "", "--from=auto", "2K "], "2000\n"),
        // Explicit --unit-separator=" " allows single space
        (
            &["-d", "", "--from=si", "--unit-separator= ", "1 K"],
            "1000\n",
        ),
        (
            &["-d", "", "--from=iec", "--unit-separator= ", "2 M"],
            "2097152\n",
        ),
    ] {
        new_ucmd!().args(args).succeeds().stdout_only(expected);
    }
}

#[test]
fn test_empty_delimiter_multi_char_unit_separator() {
    // Two-space unit separator allows two spaces between number and suffix
    new_ucmd!()
        .args(&["-d", "", "--from=si", "--unit-separator=  "])
        .pipe_in("1  K\n2  M\n3  G\n")
        .succeeds()
        .stdout_only("1000\n2000000\n3000000000\n");
}

#[test]
fn test_whitespace_mode_parses_custom_unit_separator_inputs() {
    new_ucmd!()
        .args(&["--from=iec", "--unit-separator=::"])
        .pipe_in("4::K\n")
        .succeeds()
        .stdout_only("4096\n");

    new_ucmd!()
        .args(&["--from=iec", "--unit-separator=\u{a0}"])
        .pipe_in("4\u{a0}K\n")
        .succeeds()
        .stdout_only("4096\n");
}

#[test]
fn test_empty_delimiter_whitespace_rejection() {
    new_ucmd!()
        .args(&["-d", "", "--from=auto", "2  K"])
        .fails_with_code(2)
        .stderr_contains("invalid suffix in input");

    new_ucmd!()
        .args(&["-d", "", "--from=si", "--unit-separator=", "1 K"])
        .fails_with_code(2)
        .stderr_contains("invalid suffix in input");
}

#[test]
fn test_null_byte_input() {
    new_ucmd!()
        .pipe_in("1000\x00\n")
        .succeeds()
        .stdout_is("1000\n");

    new_ucmd!().pipe_in("1000\x00").succeeds().stdout_is("1000");
}

#[test]
fn test_null_byte_input_multiline() {
    new_ucmd!()
        .pipe_in("1000\x00\n2000\x00")
        .succeeds()
        .stdout_is("1000\n2000");

    new_ucmd!()
        .pipe_in("1000\x002000\n3000")
        .succeeds()
        .stdout_is("1000\n3000");
}

// https://github.com/uutils/coreutils/issues/11653
// GNU rejects `-9923868` as an invalid short option (leading `-9`) and
// requires `--` separator; uutils accepts it as a negative positional number.
#[test]
fn test_negative_number_without_double_dash_gnu_compat_issue_11653() {
    new_ucmd!()
        .args(&["--to=iec", "-9923868"])
        .fails_with_code(1)
        .stderr_contains("unexpected argument");
}

// https://github.com/uutils/coreutils/issues/11653
// GNU rejects `-9923868` as an invalid short option (leading `-9`) and
// requires `--` separator; uutils accepts it as a negative positional number.
#[test]
fn test_negative_number_with_double_dash_gnu_compat_issue_11653() {
    new_ucmd!()
        .args(&["--to=iec", "--", "-9923868"])
        .succeeds()
        .stdout_is("-9.5M\n");
}

// https://github.com/uutils/coreutils/issues/11654
// uutils parses large integers through f64, losing precision past 2^53.
#[test]
fn test_large_integer_precision_loss_issue_11654() {
    new_ucmd!()
        .args(&["--from=iec", "9153396227555392131"])
        .succeeds()
        .stdout_is("9153396227555392131\n");
}

// https://github.com/uutils/coreutils/issues/11655
// uutils accepts scientific notation (`1e9`, `5e-3`, ...); GNU rejects it
// as "invalid suffix in input".
#[test]
fn test_scientific_notation_rejected_by_gnu_issue_11655() {
    new_ucmd!()
        .arg("1e9")
        .fails_with_code(2)
        .stderr_contains("invalid suffix in input");
}

#[test]
fn test_to_auto_rejected_at_parse_time() {
    new_ucmd!()
        .args(&["--to=auto", "100"])
        .fails_with_code(1)
        .stderr_contains("invalid argument 'auto' for '--to'");
}

// https://github.com/uutils/coreutils/issues/11663
// `--from-unit` multiplication with fractional input rounds to an integer;
// GNU preserves the fractional digits.
#[test]
fn test_from_unit_fractional_precision_issue_11663() {
    new_ucmd!()
        .args(&["--from=iec", "--from-unit=959", "--", "-615484.454"])
        .succeeds()
        .stdout_is("-590249591.386\n");
}

// https://github.com/uutils/coreutils/issues/11664
// Zero-padded `--format` places padding zeros before the sign for negative
// numbers; GNU (and C printf) puts the sign first.
#[test]
fn test_zero_pad_sign_order_issue_11664() {
    new_ucmd!()
        .args(&["--from=none", "--format=%018.2f", "--", "-9869647"])
        .succeeds()
        .stdout_is("-00000009869647.00\n");
}

#[test]
fn test_to_unit_prefix_selection() {
    new_ucmd!()
        .args(&["--to=iec-i", "--to-unit=885", "100000"])
        .succeeds()
        .stdout_is("113\n");
}

// https://github.com/uutils/coreutils/issues/11667
// `--format='%.0f'` with `--to=<scale>` still prints one fractional digit;
// the precision specifier `.0` is ignored.
#[test]
fn test_format_precision_zero_with_to_scale_issue_11667() {
    new_ucmd!()
        .args(&["--to=iec", "--format=%.0f", "5183776"])
        .succeeds()
        .stdout_is("5M\n");
}

#[test]
fn test_invalid_utf8_input() {
    // 0xFF is invalid UTF-8
    new_ucmd!()
        .pipe_in([b'1', b'0', b'\n', b'\xFF'])
        .fails_with_code(2)
        .stdout_is("10\n")
        .stderr_is("numfmt: invalid number: '\\377'\n");
}

#[test]
fn test_format_value_too_large_issue_11936() {
    // value * 10^precision needing 20+ digits should be rejected
    let cases = [
        (vec!["--format=%5.1f", "1000000000000000000"], "1e+18/1"),
        (vec!["--format=%.2f", "100000000000000000"], "1e+17/2"),
        (vec!["--format=%.3f", "10000000000000000"], "1e+16/3"),
    ];
    for (args, hint) in cases {
        new_ucmd!()
            .args(&args)
            .fails_with_code(2)
            .stderr_contains("value/precision too large")
            .stderr_contains(hint);
    }
}

#[test]
fn test_format_value_below_large_threshold_ok() {
    // one below the cutoff still formats
    new_ucmd!()
        .args(&["--format=%5.1f", "999999999999999999"])
        .succeeds()
        .stdout_is("999999999999999999.0\n");
}

#[test]
#[cfg_attr(wasi_runner, ignore = "WASI: locale env vars not propagated")]
fn test_locale_fr_output() {
    // Output uses the locale separator
    new_ucmd!()
        .env("LC_ALL", "fr_FR.UTF-8")
        .args(&["--to=iec", "1500"])
        .succeeds()
        .stdout_is("1,5K\n");
}

#[test]
#[cfg_attr(wasi_runner, ignore = "WASI: locale env vars not propagated")]
fn test_locale_fr_input_comma() {
    // fr_FR should take '1,5' as a number
    new_ucmd!()
        .env("LC_ALL", "fr_FR.UTF-8")
        .args(&["--format=%.3f", "1,5"])
        .succeeds()
        .stdout_is("1,500\n");
}

#[test]
#[cfg_attr(wasi_runner, ignore = "WASI: locale env vars not propagated")]
fn test_locale_fr_rejects_period() {
    // '.' isn't valid in fr_FR, should bail
    new_ucmd!()
        .env("LC_ALL", "fr_FR.UTF-8")
        .args(&["--format=%.3f", "1.5"])
        .fails()
        .stderr_contains("invalid");
}

#[test]
fn test_locale_c_uses_period() {
    // C locale should still use '.' as usual
    new_ucmd!()
        .env("LC_ALL", "C")
        .args(&["--to=iec", "1500"])
        .succeeds()
        .stdout_is("1.5K\n");
}

// https://github.com/uutils/coreutils/issues/11935
// the rejection path bypasses --invalid=warn/ignore/fail handling
#[test]
fn test_ignores_invalid_mode_issue11935() {
    new_ucmd!()
        .args(&["--invalid=warn", "100", "1e5", "200"])
        .succeeds()
        .stderr_is("numfmt: invalid suffix in input: '1e5'\n")
        .stdout_is("100\n1e5\n200\n");
}

#[test]
fn test_iec_format_precision_cap() {
    // gnu zero pads after 3 decimals on iec
    let cases = [
        ("1500", "1.46500K"),
        ("999999", "976.56200K"),
        ("310174", "302.90500K"),
    ];
    for (input, expected) in cases {
        new_ucmd!()
            .args(&["--to=iec", "--format=%.5f", input])
            .succeeds()
            .stdout_is(format!("{expected}\n"));
    }
}

#[test]
fn test_si_format_precision_no_cap() {
    // si shouldn't get the cap, full precision
    new_ucmd!()
        .args(&["--to=si", "--format=%.5f", "1234567"])
        .succeeds()
        .stdout_is("1.23457M\n");
}

// https://github.com/uutils/coreutils/issues/11937
// numfmt: --format width accounting diverges from GNU for multi-byte --suffix
#[test]
fn test_multibyte_suffix_issue11937() {
    new_ucmd!()
        .args(&["--suffix=€", "--format=%10.2f", "692"])
        .succeeds()
        .stdout_is("   692.00€\n");
}
