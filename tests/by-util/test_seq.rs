// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore lmnop xlmnop
use crate::common::util::TestScenario;
use std::process::Stdio;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_no_args() {
    new_ucmd!()
        .fails()
        .code_is(1)
        .stderr_contains("missing operand");
}

#[test]
fn test_hex_rejects_sign_after_identifier() {
    new_ucmd!()
        .args(&["0x-123ABC"])
        .fails()
        .no_stdout()
        .usage_error("invalid floating point argument: '0x-123ABC'");
    new_ucmd!()
        .args(&["0x+123ABC"])
        .fails()
        .no_stdout()
        .usage_error("invalid floating point argument: '0x+123ABC'");

    new_ucmd!()
        .args(&["--", "-0x-123ABC"])
        .fails()
        .no_stdout()
        .usage_error("invalid floating point argument: '-0x-123ABC'");
    new_ucmd!()
        .args(&["--", "-0x+123ABC"])
        .fails()
        .no_stdout()
        .usage_error("invalid floating point argument: '-0x+123ABC'");

    // test without "--" => argument parsed as (invalid) flag
    new_ucmd!()
        .args(&["-0x-123ABC"])
        .fails()
        .no_stdout()
        .stderr_contains("unexpected argument '-0' found");
    new_ucmd!()
        .args(&["-0x+123ABC"])
        .fails()
        .no_stdout()
        .stderr_contains("unexpected argument '-0' found");
}

#[test]
fn test_hex_lowercase_uppercase() {
    new_ucmd!()
        .args(&["0xa", "0xA"])
        .succeeds()
        .stdout_is("10\n");
    new_ucmd!()
        .args(&["0Xa", "0XA"])
        .succeeds()
        .stdout_is("10\n");
}

#[test]
fn test_hex_big_number() {
    new_ucmd!()
        .args(&[
            "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
            "0x100000000000000000000000000000000",
        ])
        .succeeds()
        .stdout_is(
            "340282366920938463463374607431768211455\n340282366920938463463374607431768211456\n",
        );
}

#[test]
fn test_hex_identifier_in_wrong_place() {
    new_ucmd!()
        .args(&["1234ABCD0x"])
        .fails()
        .no_stdout()
        .usage_error("invalid floating point argument: '1234ABCD0x'");
}

#[test]
fn test_rejects_nan() {
    new_ucmd!()
        .arg("NaN")
        .fails()
        .usage_error("invalid 'not-a-number' argument: 'NaN'");
}

#[test]
fn test_rejects_non_floats() {
    new_ucmd!()
        .arg("foo")
        .fails()
        .usage_error("invalid floating point argument: 'foo'");
}

#[test]
fn test_accepts_option_argument_directly() {
    new_ucmd!()
        .arg("-s,")
        .arg("2")
        .succeeds()
        .stdout_is("1,2\n");
}

#[test]
fn test_option_with_detected_negative_argument() {
    new_ucmd!()
        .arg("-s,")
        .args(&["-1", "2"])
        .succeeds()
        .stdout_is("-1,0,1,2\n");
}

#[test]
fn test_negative_number_as_separator() {
    new_ucmd!()
        .arg("-s")
        .args(&["-1", "2"])
        .succeeds()
        .stdout_is("1-12\n");
}

#[test]
fn test_invalid_float() {
    new_ucmd!()
        .args(&["1e2.3"])
        .fails()
        .no_stdout()
        .usage_error("invalid floating point argument: '1e2.3'");
    new_ucmd!()
        .args(&["1e2.3", "2"])
        .fails()
        .no_stdout()
        .usage_error("invalid floating point argument: '1e2.3'");
    new_ucmd!()
        .args(&["1", "1e2.3"])
        .fails()
        .no_stdout()
        .usage_error("invalid floating point argument: '1e2.3'");
    new_ucmd!()
        .args(&["1e2.3", "2", "3"])
        .fails()
        .no_stdout()
        .usage_error("invalid floating point argument: '1e2.3'");
    new_ucmd!()
        .args(&["1", "1e2.3", "3"])
        .fails()
        .no_stdout()
        .usage_error("invalid floating point argument: '1e2.3'");
    new_ucmd!()
        .args(&["1", "2", "1e2.3"])
        .fails()
        .no_stdout()
        .usage_error("invalid floating point argument: '1e2.3'");
}

#[test]
fn test_width_invalid_float() {
    new_ucmd!()
        .args(&["-w", "1e2.3"])
        .fails()
        .no_stdout()
        .usage_error("invalid floating point argument: '1e2.3'");
}

// ---- Tests for the big integer based path ----

#[test]
fn test_count_up() {
    new_ucmd!()
        .args(&["10"])
        .run()
        .stdout_is("1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");
}

#[test]
fn test_count_down() {
    new_ucmd!()
        .args(&["--", "5", "-1", "1"])
        .run()
        .stdout_is("5\n4\n3\n2\n1\n");
    new_ucmd!()
        .args(&["5", "-1", "1"])
        .run()
        .stdout_is("5\n4\n3\n2\n1\n");
}

#[test]
fn test_separator_and_terminator() {
    new_ucmd!()
        .args(&["-s", ",", "-t", "!", "2", "6"])
        .run()
        .stdout_is("2,3,4,5,6!");
    new_ucmd!()
        .args(&["-s", ",", "2", "6"])
        .run()
        .stdout_is("2,3,4,5,6\n");
    new_ucmd!()
        .args(&["-s", "\n", "2", "6"])
        .run()
        .stdout_is("2\n3\n4\n5\n6\n");
    new_ucmd!()
        .args(&["-s", "\\n", "2", "6"])
        .run()
        .stdout_is("2\\n3\\n4\\n5\\n6\n");
}

#[test]
fn test_equalize_widths() {
    let args = ["-w", "--equal-width"];
    for arg in args {
        new_ucmd!()
            .args(&[arg, "5", "10"])
            .run()
            .stdout_is("05\n06\n07\n08\n09\n10\n");
    }
}

#[test]
fn test_seq_wrong_arg() {
    new_ucmd!().args(&["-w", "5", "10", "33", "32"]).fails();
}

#[test]
fn test_zero_step() {
    new_ucmd!().args(&["10", "0", "32"]).fails();
}

#[test]
fn test_big_numbers() {
    new_ucmd!()
        .args(&[
            "1000000000000000000000000000",
            "1000000000000000000000000001",
        ])
        .succeeds()
        .stdout_only("1000000000000000000000000000\n1000000000000000000000000001\n");
}

// ---- Tests for the floating point based path ----

#[test]
fn test_count_up_floats() {
    new_ucmd!()
        .args(&["10.0"])
        .run()
        .stdout_is("1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");
}

#[test]
fn test_count_down_floats() {
    new_ucmd!()
        .args(&["--", "5", "-1.0", "1"])
        .run()
        .stdout_is("5.0\n4.0\n3.0\n2.0\n1.0\n");
    new_ucmd!()
        .args(&["5", "-1", "1.0"])
        .run()
        .stdout_is("5\n4\n3\n2\n1\n");
}

#[test]
fn test_separator_and_terminator_floats() {
    new_ucmd!()
        .args(&["-s", ",", "-t", "!", "2.0", "6"])
        .run()
        .stdout_is("2.0,3.0,4.0,5.0,6.0!");
}

#[test]
fn test_equalize_widths_floats() {
    new_ucmd!()
        .args(&["-w", "5", "10.0"])
        .run()
        .stdout_is("05\n06\n07\n08\n09\n10\n");
}

#[test]
fn test_seq_wrong_arg_floats() {
    new_ucmd!().args(&["-w", "5", "10.0", "33", "32"]).fails();
}

#[test]
fn test_zero_step_floats() {
    new_ucmd!().args(&["10.0", "0", "32"]).fails();
}

#[test]
fn test_preserve_negative_zero_start() {
    new_ucmd!()
        .args(&["-0", "1"])
        .succeeds()
        .stdout_is("-0\n1\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-0", "1", "2"])
        .succeeds()
        .stdout_is("-0\n1\n2\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-0", "1", "2.0"])
        .succeeds()
        .stdout_is("-0\n1\n2\n")
        .no_stderr();
}

#[test]
fn test_drop_negative_zero_end() {
    new_ucmd!()
        .args(&["1", "-1", "-0"])
        .succeeds()
        .stdout_is("1\n0\n")
        .no_stderr();
}

#[test]
fn test_width_scientific_notation() {
    new_ucmd!()
        .args(&["-w", "999", "1e3"])
        .succeeds()
        .stdout_is("0999\n1000\n")
        .no_stderr();
}

#[test]
fn test_width_negative_zero() {
    new_ucmd!()
        .args(&["-w", "-0", "1"])
        .succeeds()
        .stdout_is("-0\n01\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0", "1", "2"])
        .succeeds()
        .stdout_is("-0\n01\n02\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0", "1", "2.0"])
        .succeeds()
        .stdout_is("-0\n01\n02\n")
        .no_stderr();
}

#[test]
fn test_width_negative_zero_decimal_notation() {
    new_ucmd!()
        .args(&["-w", "-0.0", "1"])
        .succeeds()
        .stdout_is("-0.0\n01.0\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0.0", "1.0"])
        .succeeds()
        .stdout_is("-0.0\n01.0\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0.0", "1", "2"])
        .succeeds()
        .stdout_is("-0.0\n01.0\n02.0\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0.0", "1", "2.0"])
        .succeeds()
        .stdout_is("-0.0\n01.0\n02.0\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0.0", "1.0", "2"])
        .succeeds()
        .stdout_is("-0.0\n01.0\n02.0\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0.0", "1.0", "2.0"])
        .succeeds()
        .stdout_is("-0.0\n01.0\n02.0\n")
        .no_stderr();
}

#[test]
fn test_width_negative_zero_scientific_notation() {
    new_ucmd!()
        .args(&["-w", "-0e0", "1"])
        .succeeds()
        .stdout_is("-0\n01\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0e0", "1", "2"])
        .succeeds()
        .stdout_is("-0\n01\n02\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0e0", "1", "2.0"])
        .succeeds()
        .stdout_is("-0\n01\n02\n")
        .no_stderr();

    new_ucmd!()
        .args(&["-w", "-0e+1", "1"])
        .succeeds()
        .stdout_is("-00\n001\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0e+1", "1", "2"])
        .succeeds()
        .stdout_is("-00\n001\n002\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0e+1", "1", "2.0"])
        .succeeds()
        .stdout_is("-00\n001\n002\n")
        .no_stderr();

    new_ucmd!()
        .args(&["-w", "-0.000e0", "1"])
        .succeeds()
        .stdout_is("-0.000\n01.000\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0.000e0", "1", "2"])
        .succeeds()
        .stdout_is("-0.000\n01.000\n02.000\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0.000e0", "1", "2.0"])
        .succeeds()
        .stdout_is("-0.000\n01.000\n02.000\n")
        .no_stderr();

    new_ucmd!()
        .args(&["-w", "-0.000e-2", "1"])
        .succeeds()
        .stdout_is("-0.00000\n01.00000\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0.000e-2", "1", "2"])
        .succeeds()
        .stdout_is("-0.00000\n01.00000\n02.00000\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0.000e-2", "1", "2.0"])
        .succeeds()
        .stdout_is("-0.00000\n01.00000\n02.00000\n")
        .no_stderr();

    new_ucmd!()
        .args(&["-w", "-0.000e5", "1"])
        .succeeds()
        .stdout_is("-000000\n0000001\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0.000e5", "1", "2"])
        .succeeds()
        .stdout_is("-000000\n0000001\n0000002\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0.000e5", "1", "2.0"])
        .succeeds()
        .stdout_is("-000000\n0000001\n0000002\n")
        .no_stderr();

    new_ucmd!()
        .args(&["-w", "-0.000e5", "1"])
        .succeeds()
        .stdout_is("-000000\n0000001\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0.000e5", "1", "2"])
        .succeeds()
        .stdout_is("-000000\n0000001\n0000002\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0.000e5", "1", "2.0"])
        .succeeds()
        .stdout_is("-000000\n0000001\n0000002\n")
        .no_stderr();
}

#[test]
fn test_width_decimal_scientific_notation_increment() {
    new_ucmd!()
        .args(&["-w", ".1", "1e-2", ".11"])
        .succeeds()
        .stdout_is("0.10\n0.11\n")
        .no_stderr();

    new_ucmd!()
        .args(&["-w", ".0", "1.500e-1", ".2"])
        .succeeds()
        .stdout_is("0.0000\n0.1500\n")
        .no_stderr();
}

/// Test that trailing zeros in the start argument contribute to precision.
#[test]
fn test_width_decimal_scientific_notation_trailing_zeros_start() {
    new_ucmd!()
        .args(&["-w", ".1000", "1e-2", ".11"])
        .succeeds()
        .stdout_is("0.1000\n0.1100\n")
        .no_stderr();
}

/// Test that trailing zeros in the increment argument contribute to precision.
#[test]
fn test_width_decimal_scientific_notation_trailing_zeros_increment() {
    new_ucmd!()
        .args(&["-w", "1e-1", "0.0100", ".11"])
        .succeeds()
        .stdout_is("0.1000\n0.1100\n")
        .no_stderr();
}

#[test]
fn test_width_negative_decimal_notation() {
    new_ucmd!()
        .args(&["-w", "-.1", ".1", ".11"])
        .succeeds()
        .stdout_is("-0.1\n00.0\n00.1\n")
        .no_stderr();
}

#[test]
fn test_width_negative_scientific_notation() {
    new_ucmd!()
        .args(&["-w", "-1e-3", "1"])
        .succeeds()
        .stdout_is("-0.001\n00.999\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-1.e-3", "1"])
        .succeeds()
        .stdout_is("-0.001\n00.999\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-1.0e-4", "1"])
        .succeeds()
        .stdout_is("-0.00010\n00.99990\n")
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-.1e2", "10", "100"])
        .succeeds()
        .stdout_is(
            "-010
0000
0010
0020
0030
0040
0050
0060
0070
0080
0090
0100
",
        )
        .no_stderr();
    new_ucmd!()
        .args(&["-w", "-0.1e2", "10", "100"])
        .succeeds()
        .stdout_is(
            "-010
0000
0010
0020
0030
0040
0050
0060
0070
0080
0090
0100
",
        )
        .no_stderr();
}

/// Test that trailing zeros in the end argument do not contribute to width.
#[test]
fn test_width_decimal_scientific_notation_trailing_zeros_end() {
    new_ucmd!()
        .args(&["-w", "1e-1", "1e-2", ".1100"])
        .succeeds()
        .stdout_is("0.10\n0.11\n")
        .no_stderr();
}

#[test]
fn test_width_floats() {
    new_ucmd!()
        .args(&["-w", "9.0", "10.0"])
        .succeeds()
        .stdout_is("09.0\n10.0\n")
        .no_stderr();
}

// TODO This is duplicated from `test_yes.rs`; refactor them.
/// Run `seq`, capture some of the output, close the pipe, and verify it.
fn run(args: &[&str], expected: &[u8]) {
    let mut cmd = new_ucmd!();
    let mut child = cmd.args(args).set_stdout(Stdio::piped()).run_no_wait();
    let buf = child.stdout_exact_bytes(expected.len());
    child.close_stdout();
    child.wait().unwrap().success();
    assert_eq!(buf.as_slice(), expected);
}

#[test]
fn test_neg_inf() {
    run(&["--", "-inf", "0"], b"-inf\n-inf\n-inf\n");
}

#[test]
fn test_neg_infinity() {
    run(&["--", "-infinity", "0"], b"-inf\n-inf\n-inf\n");
}

#[test]
fn test_inf() {
    run(&["inf"], b"1\n2\n3\n");
}

#[test]
fn test_infinity() {
    run(&["infinity"], b"1\n2\n3\n");
}

#[test]
fn test_inf_width() {
    run(
        &["-w", "1.000", "inf", "inf"],
        b"1.000\n  inf\n  inf\n  inf\n",
    );
}

#[test]
fn test_neg_inf_width() {
    run(
        &["-w", "1.000", "-inf", "-inf"],
        b"1.000\n -inf\n -inf\n -inf\n",
    );
}

#[test]
fn test_ignore_leading_whitespace() {
    new_ucmd!()
        .arg("   1")
        .succeeds()
        .stdout_is("1\n")
        .no_stderr();
}

#[test]
fn test_trailing_whitespace_error() {
    // In some locales, the GNU error message has curly quotes (‘)
    // instead of straight quotes ('). We just test the straight single
    // quotes.
    new_ucmd!()
        .arg("1 ")
        .fails()
        .usage_error("invalid floating point argument: '1 '");
}

#[test]
fn test_negative_zero_int_start_float_increment() {
    new_ucmd!()
        .args(&["-0", "0.1", "0.1"])
        .succeeds()
        .stdout_is("-0.0\n0.1\n")
        .no_stderr();
}

#[test]
fn test_float_precision_increment() {
    new_ucmd!()
        .args(&["999", "0.1", "1000.1"])
        .succeeds()
        .stdout_is(
            "999.0
999.1
999.2
999.3
999.4
999.5
999.6
999.7
999.8
999.9
1000.0
1000.1
",
        )
        .no_stderr();
}

/// Test for floating point precision issues.
#[test]
fn test_negative_increment_decimal() {
    new_ucmd!()
        .args(&["0.1", "-0.1", "-0.2"])
        .succeeds()
        .stdout_is("0.1\n0.0\n-0.1\n-0.2\n")
        .no_stderr();
}

#[test]
fn test_zero_not_first() {
    new_ucmd!()
        .args(&["-w", "-0.1", "0.1", "0.1"])
        .succeeds()
        .stdout_is("-0.1\n00.0\n00.1\n")
        .no_stderr();
}

#[test]
fn test_rounding_end() {
    new_ucmd!()
        .args(&["1", "-1", "0.1"])
        .succeeds()
        .stdout_is("1\n")
        .no_stderr();
}

#[test]
fn test_parse_error_float() {
    new_ucmd!()
        .arg("lmnop")
        .fails()
        .usage_error("invalid floating point argument: 'lmnop'");
}

#[test]
fn test_parse_error_hex() {
    new_ucmd!()
        .arg("0xlmnop")
        .fails()
        .usage_error("invalid hexadecimal argument: '0xlmnop'");
}

#[test]
fn test_format_option() {
    new_ucmd!()
        .args(&["-f", "%.2f", "0.0", "0.1", "0.5"])
        .succeeds()
        .stdout_only("0.00\n0.10\n0.20\n0.30\n0.40\n0.50\n");
}

#[test]
#[ignore = "Need issue #6233 to be fixed"]
fn test_auto_precision() {
    new_ucmd!()
        .args(&["1", "0x1p-1", "2"])
        .succeeds()
        .stdout_only("1\n1.5\n2\n");
}

#[test]
#[ignore = "Need issue #6234 to be fixed"]
fn test_undefined() {
    new_ucmd!()
        .args(&["1e-9223372036854775808"])
        .succeeds()
        .no_output();
}

#[test]
#[ignore = "Need issue #6235 to be fixed"]
fn test_invalid_float_point_fail_properly() {
    new_ucmd!()
        .args(&["66000e000000000000000000000000000000000000000000000000000009223372036854775807"])
        .fails()
        .stdout_only(""); // might need to be updated
}

#[test]
fn test_invalid_zero_increment_value() {
    new_ucmd!()
        .args(&["0", "0", "1"])
        .fails()
        .no_stdout()
        .usage_error("invalid Zero increment value: '0'");
}

#[test]
fn test_power_of_ten_display() {
    new_ucmd!()
        .args(&["-f", "%.2g", "10", "10"])
        .succeeds()
        .stdout_only("10\n");
}

#[test]
fn test_default_g_precision() {
    new_ucmd!()
        .args(&["-f", "%010g", "1e5", "1e5"])
        .succeeds()
        .stdout_only("0000100000\n");
    new_ucmd!()
        .args(&["-f", "%010g", "1e6", "1e6"])
        .succeeds()
        .stdout_only("000001e+06\n");
}

#[test]
fn test_invalid_format() {
    new_ucmd!()
        .args(&["-f", "%%g", "1"])
        .fails()
        .no_stdout()
        .stderr_contains("format '%%g' has no % directive");
    new_ucmd!()
        .args(&["-f", "%g%g", "1"])
        .fails()
        .no_stdout()
        .stderr_contains("format '%g%g' has too many % directives");
}
