use crate::common::util::*;

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
fn escaped_hex() {
    new_ucmd!().args(&["\\x41"]).succeeds().stdout_only("A");
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
fn sub_b_string_ignore_subs() {
    new_ucmd!()
        .args(&["hello %b", "world %% %i"])
        .succeeds()
        .stdout_only("hello world %% %i");
}

#[test]
fn sub_char() {
    new_ucmd!()
        .args(&["the letter %c", "A"])
        .succeeds()
        .stdout_only("the letter A");
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
fn sub_num_float_round() {
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
fn sub_any_specifiers_after_second_param() {
    new_ucmd!()
        .args(&["%0.0ztlhLji", "3"]) //spell-checker:disable-line
        .succeeds()
        .stdout_only("3");
}

#[test]
fn stop_after_additional_escape() {
    new_ucmd!()
        .args(&["A%sC\\cD%sF", "B", "E"]) //spell-checker:disable-line
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
