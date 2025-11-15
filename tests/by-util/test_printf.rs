// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore fffffffffffffffc
use uutests::new_ucmd;

#[test]
fn basic_literal() {
    new_ucmd!()
        .args(&["hello world"])
        .succeeds()
        .stdout_only("hello world");
}

#[test]
fn test_missing_escaped_hex_value() {
    new_ucmd!()
        .arg(r"\x")
        .fails_with_code(1)
        .stderr_only("printf: missing hexadecimal number in escape\n");
}

#[test]
fn escaped_octal_and_newline() {
    new_ucmd!()
        .args(&["\\101\\0377\\n"])
        .succeeds()
        .stdout_only("A\x1F7\n");
}

#[test]
fn variable_sized_octal() {
    for x in ["|\\5|", "|\\05|", "|\\005|"] {
        new_ucmd!()
            .arg(x)
            .succeeds()
            .stdout_only_bytes([b'|', 5u8, b'|']);
    }

    new_ucmd!()
        .arg("|\\0005|")
        .succeeds()
        .stdout_only_bytes([b'|', 0, b'5', b'|']);
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
fn escaped_unicode_null_byte() {
    new_ucmd!()
        .args(&["\\0001_"])
        .succeeds()
        .stdout_is_bytes([0u8, b'1', b'_']);

    new_ucmd!()
        .args(&["%b", "\\0001_"])
        .succeeds()
        .stdout_is_bytes([1u8, b'_']);
}

#[test]
fn escaped_unicode_incomplete() {
    for arg in ["\\u", "\\U", "\\uabc", "\\Uabcd"] {
        new_ucmd!()
            .arg(arg)
            .fails_with_code(1)
            .stderr_only("printf: missing hexadecimal number in escape\n");
    }
}

#[test]
fn escaped_unicode_invalid() {
    for arg in ["\\ud9d0", "\\U0000D8F9"] {
        new_ucmd!()
            .arg(arg)
            .fails_with_code(1)
            .stderr_only(format!("printf: invalid universal character name {arg}\n"));
    }
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
fn sub_b_string_handle_escapes() {
    new_ucmd!()
        .args(&["hello %b", "\\tworld"])
        .succeeds()
        .stdout_only("hello \tworld");
}

#[test]
fn sub_b_string_variable_size_unicode() {
    for x in ["\\5|", "\\05|", "\\005|", "\\0005|"] {
        new_ucmd!()
            .args(&["|%b", x])
            .succeeds()
            .stdout_only_bytes([b'|', 5u8, b'|']);
    }

    new_ucmd!()
        .args(&["|%b", "\\00005|"])
        .succeeds()
        .stdout_only_bytes([b'|', 0, b'5', b'|']);
}

#[test]
fn sub_b_string_validate_field_params() {
    new_ucmd!()
        .args(&["hello %7b", "world"])
        .fails()
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
        .fails()
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
fn sub_q_string_empty() {
    new_ucmd!().args(&["%q", ""]).succeeds().stdout_only("''");
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

    new_ucmd!()
        .args(&["ninety seven is %i", "\"a"])
        .succeeds()
        .stdout_only("ninety seven is 97");

    new_ucmd!()
        .args(&["emoji is %i", "\"🙃"])
        .succeeds()
        .stdout_only("emoji is 128579");
}

#[test]
fn sub_num_thousands() {
    // For "C" locale, the thousands separator is ignored but should
    // not result in an error
    new_ucmd!()
        .args(&["%'i", "123456"])
        .succeeds()
        .stdout_only("123456");
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
        .fails_with_code(1);
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

#[test]
fn sub_num_sci_negative() {
    new_ucmd!()
        .args(&["-1234 is %e", "-1234"])
        .succeeds()
        .stdout_only("-1234 is -1.234000e+03");
}

#[test]
fn sub_num_hex_float_lower() {
    new_ucmd!()
        .args(&["%a", ".875"])
        .succeeds()
        .stdout_only("0xep-4");
}

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
fn sub_any_asterisk_first_param_with_integer() {
    new_ucmd!()
        .args(&["|%*d|", "3", "0"])
        .succeeds()
        .stdout_only("|  0|");

    new_ucmd!()
        .args(&["|%*d|", "1", "0"])
        .succeeds()
        .stdout_only("|0|");

    new_ucmd!()
        .args(&["|%*d|", "0", "0"])
        .succeeds()
        .stdout_only("|0|");

    new_ucmd!()
        .args(&["|%*d|", "-1", "0"])
        .succeeds()
        .stdout_only("|0|");

    // Negative widths are left-aligned
    new_ucmd!()
        .args(&["|%*d|", "-3", "0"])
        .succeeds()
        .stdout_only("|0  |");
}

#[test]
fn sub_any_asterisk_second_param_with_integer() {
    new_ucmd!()
        .args(&["|%.*d|", "3", "10"])
        .succeeds()
        .stdout_only("|010|");

    new_ucmd!()
        .args(&["|%*.d|", "1", "10"])
        .succeeds()
        .stdout_only("|10|");

    new_ucmd!()
        .args(&["|%.*d|", "0", "10"])
        .succeeds()
        .stdout_only("|10|");

    new_ucmd!()
        .args(&["|%.*d|", "-1", "10"])
        .succeeds()
        .stdout_only("|10|");

    new_ucmd!()
        .args(&["|%.*d|", "-2", "10"])
        .succeeds()
        .stdout_only("|10|");

    new_ucmd!()
        .args(&["|%.*d|", &i64::MIN.to_string(), "10"])
        .succeeds()
        .stdout_only("|10|");

    new_ucmd!()
        .args(&["|%.*d|", &format!("-{}", u128::MAX), "10"])
        .fails_with_code(1)
        .stdout_is("|10|")
        .stderr_is(
            "printf: '-340282366920938463463374607431768211455': Numerical result out of range\n",
        );
}

#[test]
fn sub_any_specifiers() {
    // spell-checker:disable-next-line
    for format in ["%ztlhLji", "%0ztlhLji", "%0.ztlhLji"] {
        new_ucmd!().args(&[format, "3"]).succeeds().stdout_only("3");
    }
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
        .fails_with_code(1)
        .stdout_is("42.03 is a lot")
        .stderr_is("printf: '42.03x': value not completely converted\n");
}

#[test]
fn partial_integer() {
    new_ucmd!()
        .args(&["%d is %s", "42x23", "a lot"])
        .fails_with_code(1)
        .stdout_is("42 is a lot")
        .stderr_is("printf: '42x23': value not completely converted\n");

    new_ucmd!()
        .args(&["%d is not %s", "0xwa", "a lot"])
        .fails_with_code(1)
        .stdout_is("0 is not a lot")
        .stderr_is("printf: '0xwa': value not completely converted\n");
}

#[test]
fn unsigned_hex_negative_wraparound() {
    new_ucmd!()
        .args(&["%x", "-0b100"])
        .succeeds()
        .stdout_only("fffffffffffffffc");

    new_ucmd!()
        .args(&["%x", "-0100"])
        .succeeds()
        .stdout_only("ffffffffffffffc0");

    new_ucmd!()
        .args(&["%x", "-100"])
        .succeeds()
        .stdout_only("ffffffffffffff9c");

    new_ucmd!()
        .args(&["%x", "-0x100"])
        .succeeds()
        .stdout_only("ffffffffffffff00");

    new_ucmd!()
        .args(&["%x", "-92233720368547758150"])
        .fails_with_code(1)
        .stdout_is("ffffffffffffffff")
        .stderr_is("printf: '-92233720368547758150': Numerical result out of range\n");

    new_ucmd!()
        .args(&["%u", "-1002233720368547758150"])
        .fails_with_code(1)
        .stdout_is("18446744073709551615")
        .stderr_is("printf: '-1002233720368547758150': Numerical result out of range\n");
}

#[test]
fn test_overflow() {
    new_ucmd!()
        .args(&["%d", "36893488147419103232"])
        .fails_with_code(1)
        .stdout_is("9223372036854775807")
        .stderr_is("printf: '36893488147419103232': Numerical result out of range\n");

    new_ucmd!()
        .args(&["%d", "-36893488147419103232"])
        .fails_with_code(1)
        .stdout_is("-9223372036854775808")
        .stderr_is("printf: '-36893488147419103232': Numerical result out of range\n");

    new_ucmd!()
        .args(&["%u", "36893488147419103232"])
        .fails_with_code(1)
        .stdout_is("18446744073709551615")
        .stderr_is("printf: '36893488147419103232': Numerical result out of range\n");
}

#[test]
fn partial_char() {
    new_ucmd!()
        .args(&["%d", "'abc"])
        .succeeds()
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
        ("%#05x", "0x003"),
        ("%#05X", "0X003"),
        ("%3x", "  3"),
        ("%3X", "  3"),
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
fn format_spec_zero_fails() {
    // It is invalid to have the format spec
    for format in ["%0c", "%0s"] {
        new_ucmd!().args(&[format, "3"]).fails_with_code(1);
    }
}

#[test]
fn invalid_precision_tests() {
    // It is invalid to have length of output string greater than i32::MAX
    for format in ["%.*d", "%.*f"] {
        let expected_error = "printf: invalid precision: '2147483648'\n";
        new_ucmd!()
            .args(&[format, "2147483648", "0"])
            .fails()
            .stderr_is(expected_error);
    }
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
fn spaces_before_numbers_are_ignored() {
    new_ucmd!()
        .args(&["%*.*d", "   5", "  3", " 6"])
        .succeeds()
        .stdout_only("  006");
}

#[test]
fn float_with_zero_precision_should_pad() {
    new_ucmd!()
        .args(&["%03.0f", "-1"])
        .succeeds()
        .stdout_only("-01");
}

#[test]
fn float_non_finite() {
    new_ucmd!()
        .args(&[
            "%f %f %F %f %f %F",
            "nan",
            "-nan",
            "nan",
            "inf",
            "-inf",
            "inf",
        ])
        .succeeds()
        .stdout_only("nan -nan NAN inf -inf INF");
}

#[test]
fn float_zero_neg_zero() {
    new_ucmd!()
        .args(&["%f %f", "0.0", "-0.0"])
        .succeeds()
        .stdout_only("0.000000 -0.000000");
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
fn float_large_precision() {
    // Note: This does not match GNU coreutils output (0.100000000000000000001355252716 on x86),
    // as we parse and format using ExtendedBigDecimal, which provides arbitrary precision.
    new_ucmd!()
        .args(&["%.30f", "0.1"])
        .succeeds()
        .stdout_only("0.100000000000000000000000000000");
}

#[test]
fn float_non_finite_space_padding() {
    new_ucmd!()
        .args(&["% 5.2f|% 5.2f|% 5.2f|% 5.2f", "inf", "-inf", "nan", "-nan"])
        .succeeds()
        .stdout_only("  inf| -inf|  nan| -nan");
}

#[test]
fn float_non_finite_zero_padding() {
    // Zero-padding pads non-finite numbers with spaces.
    new_ucmd!()
        .args(&["%05.2f|%05.2f|%05.2f|%05.2f", "inf", "-inf", "nan", "-nan"])
        .succeeds()
        .stdout_only("  inf| -inf|  nan| -nan");
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

#[test]
fn float_arg_zero() {
    new_ucmd!()
        .args(&["%f", "0."])
        .succeeds()
        .stdout_only("0.000000");

    new_ucmd!()
        .args(&["%f", ".0"])
        .succeeds()
        .stdout_only("0.000000");

    new_ucmd!()
        .args(&["%f", ".0e100000"])
        .succeeds()
        .stdout_only("0.000000");
}

#[test]
fn float_arg_invalid() {
    // Just a dot fails.
    new_ucmd!()
        .args(&["%f", "."])
        .fails()
        .stdout_is("0.000000")
        .stderr_contains("expected a numeric value");

    new_ucmd!()
        .args(&["%f", "-."])
        .fails()
        .stdout_is("0.000000")
        .stderr_contains("expected a numeric value");

    // Just an exponent indicator fails.
    new_ucmd!()
        .args(&["%f", "e"])
        .fails()
        .stdout_is("0.000000")
        .stderr_contains("expected a numeric value");

    // No digit but only exponent fails
    new_ucmd!()
        .args(&["%f", ".e12"])
        .fails()
        .stdout_is("0.000000")
        .stderr_contains("expected a numeric value");

    // No exponent partially fails
    new_ucmd!()
        .args(&["%f", "123e"])
        .fails()
        .stdout_is("123.000000")
        .stderr_contains("value not completely converted");

    // Nothing past `0x` parses as zero
    new_ucmd!()
        .args(&["%f", "0x"])
        .fails()
        .stdout_is("0.000000")
        .stderr_contains("value not completely converted");

    new_ucmd!()
        .args(&["%f", "0x."])
        .fails()
        .stdout_is("0.000000")
        .stderr_contains("value not completely converted");

    new_ucmd!()
        .args(&["%f", "0xp12"])
        .fails()
        .stdout_is("0.000000")
        .stderr_contains("value not completely converted");
}

#[test]
fn float_arg_with_whitespace() {
    new_ucmd!()
        .args(&["%f", " \u{0020}\u{000d}\t\n0.000001"])
        .succeeds()
        .stdout_only("0.000001");

    new_ucmd!()
        .args(&["%f", "0.1 "])
        .fails()
        .stderr_contains("value not completely converted");

    // Unicode whitespace should not be allowed in a number
    new_ucmd!()
        .args(&["%f", "\u{2029}0.1"])
        .fails()
        .stderr_contains("expected a numeric value");

    // An input string with a whitespace special character that has
    // not already been expanded should fail.
    new_ucmd!()
        .args(&["%f", "\\t0.1"])
        .fails()
        .stderr_contains("expected a numeric value");
}

#[test]
fn mb_input() {
    let cases = vec![
        ("%04x\n", "\"á", "00e1\n"),
        ("%04x\n", "'á", "00e1\n"),
        ("%04x\n", "'\u{e1}", "00e1\n"),
        ("%i\n", "\"á", "225\n"),
        ("%i\n", "'á", "225\n"),
        ("%i\n", "'\u{e1}", "225\n"),
        ("%f\n", "'á", "225.000000\n"),
    ];
    for (format, arg, stdout) in cases {
        new_ucmd!()
            .args(&[format, arg])
            .succeeds()
            .stdout_only(stdout);
    }

    let cases = vec![
        ("%04x\n", "\"á=", "00e1\n", "="),
        ("%04x\n", "'á-", "00e1\n", "-"),
        ("%04x\n", "'á=-==", "00e1\n", "=-=="),
        ("%04x\n", "'á'", "00e1\n", "'"),
        ("%04x\n", "'\u{e1}++", "00e1\n", "++"),
        ("%04x\n", "''á'", "0027\n", "á'"),
        ("%i\n", "\"á=", "225\n", "="),
    ];
    for (format, arg, stdout, stderr) in cases {
        new_ucmd!()
            .args(&[format, arg])
            .succeeds()
            .stdout_is(stdout)
            .stderr_is(format!("printf: warning: {stderr}: character(s) following character constant have been ignored\n"));
    }

    for arg in ["\"", "'"] {
        new_ucmd!()
            .args(&["%04x\n", arg])
            .fails()
            .stderr_contains("expected a numeric value");
    }
}

#[test]
#[cfg(target_family = "unix")]
fn mb_invalid_unicode() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let cases = vec![
        ("%04x\n", b"\"\xe1", "00e1\n"),
        ("%04x\n", b"'\xe1", "00e1\n"),
        ("%i\n", b"\"\xe1", "225\n"),
        ("%i\n", b"'\xe1", "225\n"),
        ("%f\n", b"'\xe1", "225.000000\n"),
    ];
    for (format, arg, stdout) in cases {
        new_ucmd!()
            .arg(format)
            .arg(OsStr::from_bytes(arg))
            .succeeds()
            .stdout_only(stdout);
    }

    let cases = vec![
        (b"\"\xe1=".as_slice(), "="),
        (b"'\xe1-".as_slice(), "-"),
        (b"'\xe1=-==".as_slice(), "=-=="),
        (b"'\xe1'".as_slice(), "'"),
        // unclear if original or replacement character is better in stderr
        //(b"''\xe1'".as_slice(), "'�'"),
    ];
    for (arg, expected) in cases {
        new_ucmd!()
            .arg("%04x\n")
            .arg(OsStr::from_bytes(arg))
            .succeeds()
            .stdout_is("00e1\n")
            .stderr_is(format!("printf: warning: {expected}: character(s) following character constant have been ignored\n"));
    }
}

#[test]
fn positional_format_specifiers() {
    new_ucmd!()
        .args(&["%1$d%d-", "5", "10", "6", "20"])
        .succeeds()
        .stdout_only("55-1010-66-2020-");

    new_ucmd!()
        .args(&["%2$d%d-", "5", "10", "6", "20"])
        .succeeds()
        .stdout_only("105-206-");

    new_ucmd!()
        .args(&["%3$d%d-", "5", "10", "6", "20"])
        .succeeds()
        .stdout_only("65-020-");

    new_ucmd!()
        .args(&["%4$d%d-", "5", "10", "6", "20"])
        .succeeds()
        .stdout_only("205-");

    new_ucmd!()
        .args(&["%5$d%d-", "5", "10", "6", "20"])
        .succeeds()
        .stdout_only("05-");

    new_ucmd!()
        .args(&["%0$d%d-", "5", "10", "6", "20"])
        .fails_with_code(1)
        .stderr_only("printf: %0$: invalid conversion specification\n");

    new_ucmd!()
        .args(&[
            "Octal: %6$o, Int: %1$d, Float: %4$f, String: %2$s, Hex: %7$x, Scientific: %5$e, Char: %9$c, Unsigned: %3$u, Integer: %8$i",
            "42",          // 1$d - Int
            "hello",       // 2$s - String
            "100",         // 3$u - Unsigned
            "3.14159",     // 4$f - Float
            "0.00001",     // 5$e - Scientific
            "77",          // 6$o - Octal
            "255",         // 7$x - Hex
            "123",         // 8$i - Integer
            "A",           // 9$c - Char
        ])
        .succeeds()
        .stdout_only("Octal: 115, Int: 42, Float: 3.141590, String: hello, Hex: ff, Scientific: 1.000000e-05, Char: A, Unsigned: 100, Integer: 123");
}

#[test]
#[cfg(target_family = "unix")]
fn non_utf_8_input() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    // ISO-8859-1 encoded text
    // spell-checker:disable
    const INPUT_AND_OUTPUT: &[u8] =
        b"Swer an rehte g\xFCete wendet s\xEEn gem\xFCete, dem volget s\xE6lde und \xEAre.";
    // spell-checker:enable

    let os_str = OsStr::from_bytes(INPUT_AND_OUTPUT);

    new_ucmd!()
        .arg("%s")
        .arg(os_str)
        .succeeds()
        .stdout_only_bytes(INPUT_AND_OUTPUT);

    new_ucmd!()
        .arg(os_str)
        .succeeds()
        .stdout_only_bytes(INPUT_AND_OUTPUT);

    new_ucmd!()
        .arg("%d")
        .arg(os_str)
        .fails()
        .stderr_contains("expected a numeric value");
}

#[test]
fn test_emoji_formatting() {
    new_ucmd!()
        .args(&["Status: %s 🎯 Count: %d\n", "Success 🚀", "42"])
        .succeeds()
        .stdout_only("Status: Success 🚀 🎯 Count: 42\n");
}

#[test]
fn test_large_width_format() {
    // Test that extremely large width specifications fail gracefully with an error
    // rather than panicking. This tests the fix for the printf-surprise.sh GNU test.
    // When printf tries to format with a width of 20 million, it should return
    // an error message and exit code 1, not panic with exit code 101.
    let test_cases = [
        ("%20000000f", "0"),    // float formatting
        ("%10000000s", "test"), // string formatting
        ("%15000000d", "42"),   // integer formatting
    ];

    for (format, arg) in test_cases {
        new_ucmd!()
            .args(&[format, arg])
            .fails_with_code(1)
            .stderr_contains("write error")
            .stdout_is("");
    }
}
