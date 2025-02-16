// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn basic_literal() {
    new_ucmd!()
        .args(&["hello world"])
        .succeeds()
        .stdout_only("hello world");
}

#[test]
fn escaped_tab() {
    new_ucmd!()
        .args(&["hello\\t world"])
        .succeeds()
        .stdout_only("hello\t world");
}

#[test]
fn escaped_newline() {
    new_ucmd!()
        .args(&["hello\\n world"])
        .succeeds()
        .stdout_only("hello\n world");
}

#[test]
fn escaped_slash() {
    new_ucmd!()
        .args(&["hello\\\\ world"])
        .succeeds()
        .stdout_only("hello\\ world");
}

#[test]
fn unescaped_double_quote() {
    new_ucmd!().args(&["\\\""]).succeeds().stdout_only("\"");
}

#[test]
fn escaped_hex() {
    new_ucmd!().args(&["\\x41"]).succeeds().stdout_only("A");
}

#[test]
fn test_missing_escaped_hex_value() {
    new_ucmd!()
        .arg(r"\x")
        .fails()
        .code_is(1)
        .stderr_only("printf: missing hexadecimal number in escape\n");
}

#[test]
fn escaped_octal() {
    new_ucmd!().args(&["\\101"]).succeeds().stdout_only("A");
}

#[test]
fn escaped_unicode_four_digit() {
    new_ucmd!().args(&["\\u0125"]).succeeds().stdout_only("ĥ");
}

#[test]
fn escaped_unicode_eight_digit() {
    new_ucmd!()
        .args(&["\\U00000125"])
        .succeeds()
        .stdout_only("ĥ");
}

#[test]
fn escaped_percent_sign() {
    new_ucmd!()
        .args(&["hello%% world"])
        .succeeds()
        .stdout_only("hello% world");
}

#[test]
fn escaped_unrecognized() {
    new_ucmd!().args(&["c\\d"]).succeeds().stdout_only("c\\d");
}

#[test]
fn sub_string() {
    new_ucmd!()
        .args(&["hello %s", "world"])
        .succeeds()
        .stdout_only("hello world");
}

#[test]
fn sub_multi_field() {
    new_ucmd!()
        .args(&["%s %s", "hello", "world"])
        .succeeds()
        .stdout_only("hello world");
}

#[test]
fn sub_repeat_format_str() {
    new_ucmd!()
        .args(&["%s.", "hello", "world"])
        .succeeds()
        .stdout_only("hello.world.");
}

#[test]
fn sub_string_ignore_escapes() {
    new_ucmd!()
        .args(&["hello %s", "\\tworld"])
        .succeeds()
        .stdout_only("hello \\tworld");
}

#[test]
fn sub_b_string_handle_escapes() {
    new_ucmd!()
        .args(&["hello %b", "\\tworld"])
        .succeeds()
        .stdout_only("hello \tworld");
}

#[test]
fn sub_b_string_validate_field_params() {
    new_ucmd!()
        .args(&["hello %7b", "world"])
        .run()
        .stdout_is("hello ")
        .stderr_is("printf: %7b: invalid conversion specification\n");
}

#[test]
fn sub_b_string_ignore_subs() {
    new_ucmd!()
        .args(&["hello %b", "world %% %i"])
        .succeeds()
        .stdout_only("hello world %% %i");
}

#[test]
fn sub_q_string_non_printable() {
    new_ucmd!()
        .args(&["non-printable: %q", "\"$test\""])
        .succeeds()
        .stdout_only("non-printable: '\"$test\"'");
}

#[test]
fn sub_q_string_validate_field_params() {
    new_ucmd!()
        .args(&["hello %7q", "world"])
        .run()
        .stdout_is("hello ")
        .stderr_is("printf: %7q: invalid conversion specification\n");
}

#[test]
fn sub_q_string_special_non_printable() {
    new_ucmd!()
        .args(&["non-printable: %q", "test~"])
        .succeeds()
        .stdout_only("non-printable: test~");
}

#[test]
fn sub_char() {
    new_ucmd!()
        .args(&["the letter %c", "A"])
        .succeeds()
        .stdout_only("the letter A");
}

#[test]
fn sub_char_from_string() {
    new_ucmd!()
        .args(&["%c%c%c", "five", "%", "oval"])
        .succeeds()
        .stdout_only("f%o");
}

#[test]
fn sub_num_int() {
    new_ucmd!()
        .args(&["twenty is %i", "20"])
        .succeeds()
        .stdout_only("twenty is 20");
}

#[test]
fn sub_num_int_min_width() {
    new_ucmd!()
        .args(&["twenty is %1i", "20"])
        .succeeds()
        .stdout_only("twenty is 20");
}

#[test]
fn sub_num_int_neg() {
    new_ucmd!()
        .args(&["neg. twenty is %i", "-20"])
        .succeeds()
        .stdout_only("neg. twenty is -20");
}

#[test]
fn sub_num_int_oct_in() {
    new_ucmd!()
        .args(&["twenty is %i", "024"])
        .succeeds()
        .stdout_only("twenty is 20");
}

#[test]
fn sub_num_int_oct_in_neg() {
    new_ucmd!()
        .args(&["neg. twenty is %i", "-024"])
        .succeeds()
        .stdout_only("neg. twenty is -20");
}

#[test]
fn sub_num_int_hex_in() {
    new_ucmd!()
        .args(&["twenty is %i", "0x14"])
        .succeeds()
        .stdout_only("twenty is 20");
}

#[test]
fn sub_num_int_hex_in_neg() {
    new_ucmd!()
        .args(&["neg. twenty is %i", "-0x14"])
        .succeeds()
        .stdout_only("neg. twenty is -20");
}

#[test]
fn sub_num_int_char_const_in() {
    new_ucmd!()
        .args(&["ninety seven is %i", "'a"])
        .succeeds()
        .stdout_only("ninety seven is 97");

    new_ucmd!()
        .args(&["emoji is %i", "'🙃"])
        .succeeds()
        .stdout_only("emoji is 128579");
}

#[test]
fn sub_num_uint() {
    new_ucmd!()
        .args(&["twenty is %u", "20"])
        .succeeds()
        .stdout_only("twenty is 20");
}

#[test]
fn sub_num_octal() {
    new_ucmd!()
        .args(&["twenty in octal is %o", "20"])
        .succeeds()
        .stdout_only("twenty in octal is 24");
}

#[test]
fn sub_num_hex_lower() {
    new_ucmd!()
        .args(&["thirty in hex is %x", "30"])
        .succeeds()
        .stdout_only("thirty in hex is 1e");
}

#[test]
fn sub_num_hex_upper() {
    new_ucmd!()
        .args(&["thirty in hex is %X", "30"])
        .succeeds()
        .stdout_only("thirty in hex is 1E");
}

#[test]
fn sub_num_hex_non_numerical() {
    new_ucmd!()
        .args(&["parameters need to be numbers %X", "%194"])
        .fails()
        .code_is(1);
}

#[test]
fn sub_num_float() {
    new_ucmd!()
        .args(&["twenty is %f", "20"])
        .succeeds()
        .stdout_only("twenty is 20.000000");
}

#[test]
fn sub_num_float_e_round() {
    new_ucmd!()
        .args(&["%e", "99999999"])
        .succeeds()
        .stdout_only("1.000000e+08");
}

#[test]
fn sub_num_float_e_no_round() {
    new_ucmd!()
        .args(&["%e", "99999994"])
        .succeeds()
        .stdout_only("9.999999e+07");
}

#[test]
fn sub_num_float_round_to_one() {
    new_ucmd!()
        .args(&["one is %f", "0.9999995"])
        .succeeds()
        .stdout_only("one is 1.000000");
}

#[test]
#[ignore = "Requires 'long double' precision floats to be used internally"]
fn sub_num_float_round_to_two() {
    new_ucmd!()
        .args(&["two is %f", "1.9999995"])
        .succeeds()
        .stdout_only("two is 2.000000");
}

#[test]
fn sub_num_float_round_nines_dec() {
    new_ucmd!()
        .args(&["%f", "0.99999999"])
        .succeeds()
        .stdout_only("1.000000");
}

#[test]
fn sub_num_sci_lower() {
    new_ucmd!()
        .args(&["twenty is %e", "20"])
        .succeeds()
        .stdout_only("twenty is 2.000000e+01");
}

#[test]
fn sub_num_sci_upper() {
    new_ucmd!()
        .args(&["twenty is %E", "20"])
        .succeeds()
        .stdout_only("twenty is 2.000000E+01");
}

#[test]
fn sub_num_sci_trunc() {
    new_ucmd!()
        .args(&["pi is ~ %e", "3.1415926535"])
        .succeeds()
        .stdout_only("pi is ~ 3.141593e+00");
}

#[test]
fn sub_num_dec_trunc() {
    new_ucmd!()
        .args(&["pi is ~ %g", "3.1415926535"])
        .succeeds()
        .stdout_only("pi is ~ 3.14159");
}

#[cfg_attr(not(feature = "test_unimplemented"), ignore)]
#[test]
fn sub_num_hex_float_lower() {
    new_ucmd!()
        .args(&["%a", ".875"])
        .succeeds()
        .stdout_only("0xep-4");
}

#[cfg_attr(not(feature = "test_unimplemented"), ignore)]
#[test]
fn sub_num_hex_float_upper() {
    new_ucmd!()
        .args(&["%A", ".875"])
        .succeeds()
        .stdout_only("0XEP-4");
}

#[test]
fn sub_min_width() {
    new_ucmd!()
        .args(&["hello %7s", "world"])
        .succeeds()
        .stdout_only("hello   world");
}

#[test]
fn sub_min_width_negative() {
    new_ucmd!()
        .args(&["hello %-7s", "world"])
        .succeeds()
        .stdout_only("hello world  ");
}

#[test]
fn sub_str_max_chars_input() {
    new_ucmd!()
        .args(&["hello %7.2s", "world"])
        .succeeds()
        .stdout_only("hello      wo");
}

#[test]
fn sub_int_decimal() {
    new_ucmd!()
        .args(&["%0.i", "11"])
        .succeeds()
        .stdout_only("11");
}

#[test]
fn sub_int_leading_zeroes() {
    new_ucmd!()
        .args(&["%.4i", "11"])
        .succeeds()
        .stdout_only("0011");
}

#[test]
fn sub_int_leading_zeroes_padded() {
    new_ucmd!()
        .args(&["%5.4i", "11"])
        .succeeds()
        .stdout_only(" 0011");
}

#[test]
fn sub_float_dec_places() {
    new_ucmd!()
        .args(&["pi is ~ %.11f", "3.1415926535"])
        .succeeds()
        .stdout_only("pi is ~ 3.14159265350");
}

#[test]
fn sub_float_hex_in() {
    new_ucmd!()
        .args(&["%f", "0xF1.1F"])
        .succeeds()
        .stdout_only("241.121094");
}

#[test]
fn sub_float_no_octal_in() {
    new_ucmd!()
        .args(&["%f", "077"])
        .succeeds()
        .stdout_only("77.000000");
}

#[test]
fn sub_any_asterisk_first_param() {
    new_ucmd!()
        .args(&["%*i", "3", "11", "4", "12"])
        .succeeds()
        .stdout_only(" 11  12");
}

#[test]
fn sub_any_asterisk_second_param() {
    new_ucmd!()
        .args(&["%.*i", "3", "11", "4", "12"])
        .succeeds()
        .stdout_only("0110012");
}

#[test]
fn sub_any_asterisk_both_params() {
    new_ucmd!()
        .args(&["%*.*i", "4", "3", "11", "5", "4", "12"])
        .succeeds()
        .stdout_only(" 011 0012");
}

#[test]
fn sub_any_asterisk_octal_arg() {
    new_ucmd!()
        .args(&["%.*i", "011", "12345678"])
        .succeeds()
        .stdout_only("012345678");
}

#[test]
fn sub_any_asterisk_hex_arg() {
    new_ucmd!()
        .args(&["%.*i", "0xA", "123456789"])
        .succeeds()
        .stdout_only("0123456789");
}

#[test]
fn sub_any_asterisk_negative_first_param() {
    new_ucmd!()
        .args(&["a(%*s)b", "-5", "xyz"])
        .succeeds()
        .stdout_only("a(xyz  )b"); // Would be 'a(  xyz)b' if -5 was 5

    // Negative octal
    new_ucmd!()
        .args(&["a(%*s)b", "-010", "xyz"])
        .succeeds()
        .stdout_only("a(xyz     )b");

    // Negative hexadecimal
    new_ucmd!()
        .args(&["a(%*s)b", "-0x10", "xyz"])
        .succeeds()
        .stdout_only("a(xyz             )b");

    // Should also work on %c
    new_ucmd!()
        .args(&["a(%*c)b", "-5", "x"])
        .succeeds()
        .stdout_only("a(x    )b"); // Would be 'a(    x)b' if -5 was 5
}

#[test]
fn sub_any_specifiers_no_params() {
    new_ucmd!()
        .args(&["%ztlhLji", "3"]) //spell-checker:disable-line
        .succeeds()
        .stdout_only("3");
}

#[test]
fn sub_any_specifiers_after_first_param() {
    new_ucmd!()
        .args(&["%0ztlhLji", "3"]) //spell-checker:disable-line
        .succeeds()
        .stdout_only("3");
}

#[test]
fn sub_any_specifiers_after_period() {
    new_ucmd!()
        .args(&["%0.ztlhLji", "3"]) //spell-checker:disable-line
        .succeeds()
        .stdout_only("3");
}

#[test]
fn unspecified_left_justify_is_1_width() {
    new_ucmd!().args(&["%-o"]).succeeds().stdout_only("0");
}

#[test]
fn sub_any_specifiers_after_second_param() {
    new_ucmd!()
        .args(&["%0.0ztlhLji", "3"]) //spell-checker:disable-line
        .succeeds()
        .stdout_only("3");
}

#[test]
fn stop_after_additional_escape() {
    new_ucmd!()
        .args(&["A%sC\\cD%sF", "B", "E"])
        .succeeds()
        .stdout_only("ABC");
}

#[test]
fn sub_float_leading_zeroes() {
    new_ucmd!()
        .args(&["%010f", "1"])
        .succeeds()
        .stdout_only("001.000000");
}

#[test]
fn sub_general_float() {
    new_ucmd!()
        .args(&["%g", "1.1"])
        .succeeds()
        .stdout_only("1.1");
}

#[test]
fn sub_general_truncate_to_integer() {
    new_ucmd!().args(&["%g", "1.0"]).succeeds().stdout_only("1");
}

#[test]
fn sub_general_scientific_notation() {
    new_ucmd!()
        .args(&["%g", "1000010"])
        .succeeds()
        .stdout_only("1.00001e+06");
}

#[test]
fn sub_general_round_scientific_notation() {
    new_ucmd!()
        .args(&["%g", "123456789"])
        .succeeds()
        .stdout_only("1.23457e+08");
}

#[test]
fn sub_general_round_float() {
    new_ucmd!()
        .args(&["%g", "12345.6789"])
        .succeeds()
        .stdout_only("12345.7");
}

#[test]
fn sub_general_round_float_to_integer() {
    new_ucmd!()
        .args(&["%g", "123456.7"])
        .succeeds()
        .stdout_only("123457");
}

#[test]
fn sub_general_round_float_leading_zeroes() {
    new_ucmd!()
        .args(&["%g", "1.000009"])
        .succeeds()
        .stdout_only("1.00001");
}

#[test]
fn partial_float() {
    new_ucmd!()
        .args(&["%.2f is %s", "42.03x", "a lot"])
        .fails()
        .code_is(1)
        .stdout_is("42.03 is a lot")
        .stderr_is("printf: '42.03x': value not completely converted\n");
}

#[test]
fn partial_integer() {
    new_ucmd!()
        .args(&["%d is %s", "42x23", "a lot"])
        .fails()
        .code_is(1)
        .stdout_is("42 is a lot")
        .stderr_is("printf: '42x23': value not completely converted\n");
}

#[test]
fn test_overflow() {
    new_ucmd!()
        .args(&["%d", "36893488147419103232"])
        .fails()
        .code_is(1)
        .stderr_is("printf: '36893488147419103232': Numerical result out of range\n");
}

#[test]
fn partial_char() {
    new_ucmd!()
        .args(&["%d", "'abc"])
        .fails()
        .code_is(1)
        .stdout_is("97")
        .stderr_is(
            "printf: warning: bc: character(s) following character constant have been ignored\n",
        );
}

#[test]
fn sub_alternative_lower_hex_0() {
    new_ucmd!().args(&["%#x", "0"]).succeeds().stdout_only("0");
}

#[test]
fn sub_alternative_lower_hex() {
    new_ucmd!()
        .args(&["%#x", "42"])
        .succeeds()
        .stdout_only("0x2a");
}

#[test]
fn sub_alternative_upper_hex_0() {
    new_ucmd!().args(&["%#X", "0"]).succeeds().stdout_only("0");
}

#[test]
fn sub_alternative_upper_hex() {
    new_ucmd!()
        .args(&["%#X", "42"])
        .succeeds()
        .stdout_only("0X2A");
}

#[test]
fn char_as_byte() {
    new_ucmd!()
        .args(&["%c", "🙃"])
        .succeeds()
        .no_stderr()
        .stdout_is_bytes(b"\xf0");
}

#[test]
fn no_infinite_loop() {
    new_ucmd!()
        .args(&["a", "b"])
        .succeeds()
        .stdout_is("a")
        .stderr_contains("warning: ignoring excess arguments, starting with 'b'");
}

#[test]
fn pad_octal_with_prefix() {
    new_ucmd!()
        .args(&[">%#15.6o<", "0"])
        .succeeds()
        .stdout_only(">         000000<");

    new_ucmd!()
        .args(&[">%#15.6o<", "01"])
        .succeeds()
        .stdout_only(">         000001<");

    new_ucmd!()
        .args(&[">%#15.6o<", "01234"])
        .succeeds()
        .stdout_only(">         001234<");

    new_ucmd!()
        .args(&[">%#15.6o<", "012345"])
        .succeeds()
        .stdout_only(">         012345<");

    new_ucmd!()
        .args(&[">%#15.6o<", "0123456"])
        .succeeds()
        .stdout_only(">        0123456<");
}

#[test]
fn pad_unsigned_zeroes() {
    for format in ["%.3u", "%.3x", "%.3X", "%.3o"] {
        new_ucmd!()
            .args(&[format, "0"])
            .succeeds()
            .stdout_only("000");
    }
}

#[test]
fn pad_unsigned_three() {
    for (format, expected) in [
        ("%.3u", "003"),
        ("%.3x", "003"),
        ("%.3X", "003"),
        ("%.3o", "003"),
        ("%#.3x", "0x003"),
        ("%#.3X", "0X003"),
        ("%#.3o", "003"),
    ] {
        new_ucmd!()
            .args(&[format, "3"])
            .succeeds()
            .stdout_only(expected);
    }
}

#[test]
fn pad_char() {
    for (format, expected) in [("%3c", "  X"), ("%1c", "X"), ("%-1c", "X"), ("%-3c", "X  ")] {
        new_ucmd!()
            .args(&[format, "X"])
            .succeeds()
            .stdout_only(expected);
    }
}

#[test]
fn pad_string() {
    for (format, expected) in [
        ("%8s", "  bottle"),
        ("%-8s", "bottle  "),
        ("%6s", "bottle"),
        ("%-6s", "bottle"),
    ] {
        new_ucmd!()
            .args(&[format, "bottle"])
            .succeeds()
            .stdout_only(expected);
    }
}

#[test]
fn format_spec_zero_char_fails() {
    // It is invalid to have the format spec '%0c'
    new_ucmd!().args(&["%0c", "3"]).fails().code_is(1);
}

#[test]
fn format_spec_zero_string_fails() {
    // It is invalid to have the format spec '%0s'
    new_ucmd!().args(&["%0s", "3"]).fails().code_is(1);
}

#[test]
fn invalid_precision_fails() {
    // It is invalid to have length of output string greater than i32::MAX
    new_ucmd!()
        .args(&["%.*d", "2147483648", "0"])
        .fails()
        .stderr_is("printf: invalid precision: '2147483648'\n");
}

#[test]
fn float_invalid_precision_fails() {
    // It is invalid to have length of output string greater than i32::MAX
    new_ucmd!()
        .args(&["%.*f", "2147483648", "0"])
        .fails()
        .stderr_is("printf: invalid precision: '2147483648'\n");
}

// The following padding-tests test for the cases in which flags in ['0', ' '] are given.
// For integer, only try to pad when no precision is given, while
// for float, always try to pad
#[test]
fn space_padding_with_space_test() {
    //  Check if printf gives an extra space in the beginning
    new_ucmd!()
        .args(&["% 3d", "1"])
        .succeeds()
        .stdout_only("  1");
}

#[test]
fn zero_padding_with_space_test() {
    new_ucmd!()
        .args(&["% 03d", "1"])
        .succeeds()
        .stdout_only(" 01");
}

#[test]
fn zero_padding_with_plus_test() {
    new_ucmd!()
        .args(&["%+04d", "1"])
        .succeeds()
        .stdout_only("+001");
}

#[test]
fn negative_zero_padding_test() {
    new_ucmd!()
        .args(&["%03d", "-1"])
        .succeeds()
        .stdout_only("-01");
}

#[test]
fn negative_zero_padding_with_space_test() {
    new_ucmd!()
        .args(&["% 03d", "-1"])
        .succeeds()
        .stdout_only("-01");
}

#[test]
fn float_with_zero_precision_should_pad() {
    new_ucmd!()
        .args(&["%03.0f", "-1"])
        .succeeds()
        .stdout_only("-01");
}

#[test]
fn precision_check() {
    new_ucmd!()
        .args(&["%.3d", "1"])
        .succeeds()
        .stdout_only("001");
}

#[test]
fn space_padding_with_precision() {
    new_ucmd!()
        .args(&["%4.3d", "1"])
        .succeeds()
        .stdout_only(" 001");
}

#[test]
fn float_zero_padding_with_precision() {
    new_ucmd!()
        .args(&["%04.1f", "1"])
        .succeeds()
        .stdout_only("01.0");
}

#[test]
fn float_space_padding_with_precision() {
    new_ucmd!()
        .args(&["%4.1f", "1"])
        .succeeds()
        .stdout_only(" 1.0");
}

#[test]
fn negative_float_zero_padding_with_precision() {
    new_ucmd!()
        .args(&["%05.1f", "-1"])
        .succeeds()
        .stdout_only("-01.0");
}

#[test]
fn float_default_precision_space_padding() {
    new_ucmd!()
        .args(&["%10f", "1"])
        .succeeds()
        .stdout_only("  1.000000");
}

#[test]
fn float_default_precision_zero_padding() {
    new_ucmd!()
        .args(&["%010f", "1"])
        .succeeds()
        .stdout_only("001.000000");
}

#[test]
fn flag_position_space_padding() {
    new_ucmd!()
        .args(&["% +3.1d", "1"])
        .succeeds()
        .stdout_only(" +1");
}

#[test]
fn float_flag_position_space_padding() {
    new_ucmd!()
        .args(&["% +5.1f", "1"])
        .succeeds()
        .stdout_only(" +1.0");
}

#[test]
fn float_abs_value_less_than_one() {
    new_ucmd!()
        .args(&["%g", "0.1171875"])
        .succeeds()
        .stdout_only("0.117188");

    // The original value from #7031 issue
    new_ucmd!()
        .args(&["%g", "-0.1171875"])
        .succeeds()
        .stdout_only("-0.117188");

    new_ucmd!()
        .args(&["%g", "0.01171875"])
        .succeeds()
        .stdout_only("0.0117188");

    new_ucmd!()
        .args(&["%g", "-0.01171875"])
        .succeeds()
        .stdout_only("-0.0117188");

    new_ucmd!()
        .args(&["%g", "0.001171875001"])
        .succeeds()
        .stdout_only("0.00117188");

    new_ucmd!()
        .args(&["%g", "-0.001171875001"])
        .succeeds()
        .stdout_only("-0.00117188");
}

#[test]
fn float_switch_switch_decimal_scientific() {
    new_ucmd!()
        .args(&["%g", "0.0001"])
        .succeeds()
        .stdout_only("0.0001");

    new_ucmd!()
        .args(&["%g", "0.00001"])
        .succeeds()
        .stdout_only("1e-05");
}
