use common::util::*;

static UTIL_NAME: &'static str = "printf";

fn expect_stdout(input: Vec<&str>, expected: &str) {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let results = ucmd.args(&input).run();
    // assert_empty_stderr!(result);
    // assert!(result.success);
    assert_eq!(expected, results.stdout);
}

#[test]
fn basic_literal() {
    expect_stdout(vec!["hello world"], "hello world");
}

#[test]
fn escaped_tab() {
    expect_stdout(vec!["hello\\t world"], "hello\t world");
}

#[test]
fn escaped_newline() {
    expect_stdout(vec!["hello\\n world"], "hello\n world");
}

#[test]
fn escaped_slash() {
    expect_stdout(vec!["hello\\\\ world"], "hello\\ world");
}

#[test]
fn escaped_hex() {
    expect_stdout(vec!["\\x41"], "A");
}

#[test]
fn escaped_octal() {
    expect_stdout(vec!["\\101"], "A");
}

#[test]
fn escaped_unicode_fourdigit() {
    expect_stdout(vec!["\\u0125"], "ĥ");
}

#[test]
fn escaped_unicode_eightdigit() {
    expect_stdout(vec!["\\U00000125"], "ĥ");
}

#[test]
fn escaped_percent_sign() {
    expect_stdout(vec!["hello%% world"], "hello% world");
}

#[test]
fn escaped_unrecognized() {
    expect_stdout(vec!["c\\d"], "c\\d");
}

#[test]
fn sub_string() {
    expect_stdout(vec!["hello %s", "world"], "hello world");
}

#[test]
fn sub_multifield() {
    expect_stdout(vec!["%s %s", "hello", "world"], "hello world");
}

#[test]
fn sub_repeat_formatstr() {
    expect_stdout(vec!["%s.", "hello", "world"], "hello.world.");
}

#[test]
fn sub_string_ignore_escapes() {
    expect_stdout(vec!["hello %s", "\\tworld"], "hello \\tworld");
}

#[test]
fn sub_bstring_handle_escapes() {
    expect_stdout(vec!["hello %b", "\\tworld"], "hello \tworld");
}

#[test]
fn sub_bstring_ignore_subs() {
    expect_stdout(vec!["hello %b", "world %% %i"], "hello world %% %i");
}

#[test]
fn sub_char() {
    expect_stdout(vec!["the letter %c", "A"], "the letter A");
}

#[test]
fn sub_num_int() {
    expect_stdout(vec!["twenty is %i", "20"], "twenty is 20");
}

#[test]
fn sub_num_int_minwidth() {
    expect_stdout(vec!["twenty is %1i", "20"], "twenty is 20");
}

#[test]
fn sub_num_int_neg() {
    expect_stdout(vec!["neg. twenty is %i", "-20"], "neg. twenty is -20");
}

#[test]
fn sub_num_int_oct_in() {
    expect_stdout(vec!["twenty is %i", "024"], "twenty is 20");
}

#[test]
fn sub_num_int_oct_in_neg() {
    expect_stdout(vec!["neg. twenty is %i", "-024"], "neg. twenty is -20");
}

#[test]
fn sub_num_int_hex_in() {
    expect_stdout(vec!["twenty is %i", "0x14"], "twenty is 20");
}

#[test]
fn sub_num_int_hex_in_neg() {
    expect_stdout(vec!["neg. twenty is %i", "-0x14"], "neg. twenty is -20");
}

#[test]
fn sub_num_int_charconst_in() {
    expect_stdout(vec!["ninetyseven is %i", "'a"], "ninetyseven is 97");
}

#[test]
fn sub_num_uint() {
    expect_stdout(vec!["twenty is %u", "20"], "twenty is 20");
}

#[test]
fn sub_num_octal() {
    expect_stdout(vec!["twenty in octal is %o", "20"], "twenty in octal is 24");
}

#[test]
fn sub_num_hex_lower() {
    expect_stdout(vec!["thirty in hex is %x", "30"], "thirty in hex is 1e");
}

#[test]
fn sub_num_hex_upper() {
    expect_stdout(vec!["thirty in hex is %X", "30"], "thirty in hex is 1E");
}

#[test]
fn sub_num_float() {
    expect_stdout(vec!["twenty is %f", "20"], "twenty is 20.000000");
}

#[test]
fn sub_num_float_round() {
    expect_stdout(vec!["two is %f", "1.9999995"], "two is 2.000000");
}

#[test]
fn sub_num_sci_lower() {
    expect_stdout(vec!["twenty is %e", "20"], "twenty is 2.000000e+01");
}

#[test]
fn sub_num_sci_upper() {
    expect_stdout(vec!["twenty is %E", "20"], "twenty is 2.000000E+01");
}

#[test]
fn sub_num_sci_trunc() {
    expect_stdout(vec!["pi is ~ %e", "3.1415926535"], "pi is ~ 3.141593e+00");
}

#[test]
fn sub_num_dec_trunc() {
    expect_stdout(vec!["pi is ~ %g", "3.1415926535"], "pi is ~ 3.141593");
}

#[cfg_attr(not(feature="test_unimplemented"),ignore)]
#[test]
fn sub_num_hex_float_lower() {
    expect_stdout(vec!["%a", ".875"], "0xep-4");
}

#[cfg_attr(not(feature="test_unimplemented"),ignore)]
#[test]
fn sub_num_hex_float_upper() {
    expect_stdout(vec!["%A", ".875"], "0XEP-4");
}

#[test]
fn sub_minwidth() {
    expect_stdout(vec!["hello %7s", "world"], "hello   world");
}

#[test]
fn sub_minwidth_negative() {
    expect_stdout(vec!["hello %-7s", "world"], "hello world  ");
}

#[test]
fn sub_str_max_chars_input() {
    expect_stdout(vec!["hello %7.2s", "world"], "hello      wo");
}

#[test]
fn sub_int_decimal() {
    expect_stdout(vec!["%0.i", "11"], "11");
}

#[test]
fn sub_int_leading_zeroes() {
    expect_stdout(vec!["%.4i", "11"], "0011");
}

#[test]
fn sub_int_leading_zeroes_prio() {
    expect_stdout(vec!["%5.4i", "11"], " 0011");
}

#[test]
fn sub_float_dec_places() {
    expect_stdout(vec!["pi is ~ %.11f", "3.1415926535"],
                  "pi is ~ 3.14159265350");
}

#[test]
fn sub_float_hex_in() {
    expect_stdout(vec!["%f", "0xF1.1F"], "241.121094");
}

#[test]
fn sub_float_no_octal_in() {
    expect_stdout(vec!["%f", "077"], "77.000000");
}

#[test]
fn sub_any_asterisk_firstparam() {
    expect_stdout(vec!["%*i", "3", "11", "4", "12"], " 11  12");
}

#[test]
fn sub_any_asterisk_second_param() {
    expect_stdout(vec!["%.*i", "3", "11", "4", "12"], "0110012");
}

#[test]
fn sub_any_asterisk_both_params() {
    expect_stdout(vec!["%*.*i", "4", "3", "11", "5", "4", "12"], " 011 0012");
}

#[test]
fn sub_any_asterisk_octal_arg() {
    expect_stdout(vec!["%.*i", "011", "12345678"], "012345678");
}

#[test]
fn sub_any_asterisk_hex_arg() {
    expect_stdout(vec!["%.*i", "0xA", "123456789"], "0123456789");
}

#[test]
fn sub_any_specifiers_no_params() {
    expect_stdout(vec!["%ztlhLji", "3"], "3");
}

#[test]
fn sub_any_specifiers_after_first_param() {
    expect_stdout(vec!["%0ztlhLji", "3"], "3");
}

#[test]
fn sub_any_specifiers_after_period() {
    expect_stdout(vec!["%0.ztlhLji", "3"], "3");
}

#[test]
fn sub_any_specifiers_after_second_param() {
    expect_stdout(vec!["%0.0ztlhLji", "3"], "3");
}
