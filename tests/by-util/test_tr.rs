// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore lowre punct aabbaa aabbcc aabc abbb abbbcddd abcc abcdefabcdef abcdefghijk abcdefghijklmn abcdefghijklmnop ABCDEFGHIJKLMNOPQRS abcdefghijklmnopqrstuvwxyz ABCDEFGHIJKLMNOPQRSTUVWXYZ ABCDEFZZ abcxyz ABCXYZ abcxyzabcxyz ABCXYZABCXYZ acbdef alnum amzamz AMZXAMZ bbbd cclass cefgm cntrl compl dabcdef dncase fooclass Gzabcdefg PQRST upcase wxyzz xdigit XXXYYY xycde xyyye xyyz xyzzzzxyzzzz ZABCDEF Zamz Cdefghijkl Cdefghijklmn asdfqqwweerr qwerr asdfqwer qwer aassddffqwer asdfqwer
use uutests::at_and_ucmd;
use uutests::new_ucmd;

#[cfg(unix)]
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_invalid_input() {
    new_ucmd!()
        .args(&["1", "1", "<", "."])
        .fails_with_code(1)
        .stderr_contains("tr: extra operand '<'");
    #[cfg(unix)]
    {
        let cmd = new_ucmd!()
            .args(&["1", "1"])
            // will test "tr 1 1 < ."
            .set_stdin(std::process::Stdio::from(std::fs::File::open(".").unwrap()))
            .fails_with_code(1);
        if std::env::var("UUTESTS_WASM_RUNNER").is_ok() {
            // On WASI the fluent translation key may appear instead of the
            // translated text, but the OS error string is still present.
            cmd.stderr_contains("Is a directory");
        } else {
            cmd.stderr_contains("tr: read error: Is a directory");
        }
    }
}

#[test]
fn test_to_upper() {
    new_ucmd!()
        .args(&["a-z", "A-Z"])
        .pipe_in("!abcd!")
        .succeeds()
        .stdout_is("!ABCD!");
}

#[test]
fn test_small_set2() {
    new_ucmd!()
        .args(&["0-9", "X"])
        .pipe_in("@0123456789")
        .succeeds()
        .stdout_is("@XXXXXXXXXX");
}

#[test]
fn test_invalid_unicode() {
    new_ucmd!()
        .args(&["-dc", "abc"])
        .pipe_in([0o200, b'a', b'b', b'c'])
        .succeeds()
        .stdout_is("abc");
}

#[test]
fn test_delete() {
    new_ucmd!()
        .args(&["-d", "a-z"])
        .pipe_in("aBcD")
        .succeeds()
        .stdout_is("BD");
}

#[test]
fn test_delete_graph_and_print_match_gnu() {
    let input = *b" A!\t\n";
    new_ucmd!()
        .args(&["-d", "[:graph:]"])
        .pipe_in(input)
        .succeeds()
        .stdout_is_bytes(*b" \t\n");

    new_ucmd!()
        .args(&["-d", "[:print:]"])
        .pipe_in(input)
        .succeeds()
        .stdout_is_bytes(*b"\t\n");
}

#[test]
fn test_delete_complement_graph_and_print_match_gnu() {
    let input = *b" A!\t\n";
    new_ucmd!()
        .args(&["-d", "-c", "[:graph:]"])
        .pipe_in(input)
        .succeeds()
        .stdout_is_bytes(*b"A!");

    new_ucmd!()
        .args(&["-d", "-c", "[:print:]"])
        .pipe_in(input)
        .succeeds()
        .stdout_is_bytes(*b" A!");
}

#[test]
fn test_delete_afterwards_is_not_flag() {
    new_ucmd!()
        .args(&["a-z", "-d"])
        .pipe_in("aBcD")
        .succeeds()
        .stdout_is("-BdD");
}

#[test]
fn test_delete_multi() {
    new_ucmd!()
        .args(&["-d", "-d", "a-z"])
        .pipe_in("aBcD")
        .succeeds()
        .stdout_is("BD");
}

#[test]
fn test_delete_late() {
    new_ucmd!()
        .args(&["-d", "a-z", "-d"])
        .fails()
        .stderr_contains("extra operand '-d'");
}

#[test]
fn test_delete_complement() {
    new_ucmd!()
        .args(&["-d", "-c", "a-z"])
        .pipe_in("aBcD")
        .succeeds()
        .stdout_is("ac");
}

#[test]
fn test_delete_complement_2() {
    new_ucmd!()
        .args(&["-d", "-C", "0-9"])
        .pipe_in("Phone: 01234 567890")
        .succeeds()
        .stdout_is("01234567890");
    new_ucmd!()
        .args(&["-d", "--complement", "0-9"])
        .pipe_in("Phone: 01234 567890")
        .succeeds()
        .stdout_is("01234567890");
}

#[test]
fn test_complement1() {
    new_ucmd!()
        .args(&["-c", "a", "X"])
        .pipe_in("ab")
        .succeeds()
        .stdout_is("aX");
}

#[test]
fn test_complement_afterwards_is_not_flag() {
    new_ucmd!()
        .args(&["a", "X", "-c"])
        .fails()
        .stderr_contains("extra operand '-c'");
}

#[test]
fn test_complement2() {
    new_ucmd!()
        .args(&["-c", "0-9", "x"])
        .pipe_in("Phone: 01234 567890")
        .succeeds()
        .stdout_is("xxxxxxx01234x567890");
}

#[test]
fn test_complement3() {
    new_ucmd!()
        .args(&["-c", "abcdefgh", "123"])
        .pipe_in("the cat and the bat")
        .succeeds()
        .stdout_is("3he3ca33a3d33he3ba3");
}

#[test]
fn test_complement4() {
    // $ echo -n '0x1y2z3' | tr -c '0-@' '*-~'
    // 0~1~2~3
    new_ucmd!()
        .args(&["-c", "0-@", "*-~"])
        .pipe_in("0x1y2z3")
        .succeeds()
        .stdout_is("0~1~2~3");
}

#[test]
fn test_complement5() {
    // $ echo -n '0x1y2z3' | tr -c '\0-@' '*-~'
    // 0a1b2c3
    new_ucmd!()
        .args(&["-c", r"\0-@", "*-~"])
        .pipe_in("0x1y2z3")
        .succeeds()
        .stdout_is("0a1b2c3");
}

#[test]
fn test_complement_multi_early() {
    new_ucmd!()
        .args(&["-c", "-c", "a", "X"])
        .pipe_in("ab")
        .succeeds()
        .stdout_is("aX");
}

#[test]
fn test_complement_multi_middle() {
    new_ucmd!()
        .args(&["-c", "a", "-c", "X"])
        .fails()
        .stderr_contains("tr: extra operand 'X'");
}

#[test]
fn test_complement_multi_late() {
    new_ucmd!()
        .args(&["-c", "a", "X", "-c"])
        .fails()
        .stderr_contains("tr: extra operand '-c'");
}

#[test]
fn test_squeeze() {
    new_ucmd!()
        .args(&["-s", "a-z"])
        .pipe_in("aaBBcDcc")
        .succeeds()
        .stdout_is("aBBcDc");
}

#[test]
fn test_squeeze_multi() {
    new_ucmd!()
        .args(&["-ss", "-s", "a-z"])
        .pipe_in("aaBBcDcc")
        .succeeds()
        .stdout_is("aBBcDc");
}

#[test]
fn test_squeeze_complement() {
    new_ucmd!()
        .args(&["-sc", "a-z"])
        .pipe_in("aaBBcDcc")
        .succeeds()
        .stdout_is("aaBcDcc");
}

#[test]
fn test_squeeze_complement_multi() {
    new_ucmd!()
        .args(&["-scsc", "a-z"]) // spell-checker:disable-line
        .pipe_in("aaBBcDcc")
        .succeeds()
        .stdout_is("aaBcDcc");
}

#[test]
fn test_squeeze_complement_two_sets() {
    new_ucmd!()
        .args(&["-sc", "a", "_"])
        .pipe_in("test a aa with 3 ___ spaaaces +++") // spell-checker:disable-line
        .succeeds()
        .stdout_is("_a_aa_aaa_");
}

#[test]
fn test_translate_and_squeeze() {
    new_ucmd!()
        .args(&["-s", "x", "y"])
        .pipe_in("xx")
        .succeeds()
        .stdout_is("y");
}

#[test]
fn test_translate_and_squeeze_multiple_lines() {
    new_ucmd!()
        .args(&["-s", "x", "y"])
        .pipe_in("xxaax\nxaaxx") // spell-checker:disable-line
        .succeeds()
        .stdout_is("yaay\nyaay"); // spell-checker:disable-line
}

#[test]
fn test_delete_and_squeeze_one_set() {
    new_ucmd!()
        .args(&["-ds", "a-z"])
        .fails()
        .stderr_contains("missing operand after 'a-z'")
        .stderr_contains("Two strings must be given when deleting and squeezing.");
}

#[test]
fn test_delete_and_squeeze() {
    new_ucmd!()
        .args(&["-ds", "a-z", "A-Z"])
        .pipe_in("abBcB")
        .succeeds()
        .stdout_is("B");
}

#[test]
fn test_delete_and_squeeze_complement() {
    new_ucmd!()
        .args(&["-dsc", "a-z", "A-Z"])
        .pipe_in("abBcB")
        .succeeds()
        .stdout_is("abc");
}

#[test]
fn test_delete_and_squeeze_complement_squeeze_set2() {
    new_ucmd!()
        .args(&["-dsc", "abX", "XYZ"])
        .pipe_in("abbbcdddXXXYYY")
        .succeeds()
        .stdout_is("abbbX");
}

#[test]
fn test_set1_longer_than_set2() {
    new_ucmd!()
        .args(&["abc", "xy"])
        .pipe_in("abcde")
        .succeeds()
        .stdout_is("xyyde"); // spell-checker:disable-line
}

#[test]
fn test_set1_shorter_than_set2() {
    new_ucmd!()
        .args(&["ab", "xyz"])
        .pipe_in("abcde")
        .succeeds()
        .stdout_is("xycde");
}

#[test]
fn test_truncate() {
    // echo -n "abcde" | tr -t "abc" "xy"
    new_ucmd!()
        .args(&["-t", "abc", "xy"])
        .pipe_in("abcde")
        .succeeds()
        .stdout_is("xycde");
}

#[test]
fn test_truncate_multi() {
    new_ucmd!()
        .args(&["-tt", "-t", "abc", "xy"])
        .pipe_in("abcde")
        .succeeds()
        .stdout_is("xycde");
}

#[test]
fn test_truncate_with_set1_shorter_than_set2() {
    new_ucmd!()
        .args(&["-t", "ab", "xyz"])
        .pipe_in("abcde")
        .succeeds()
        .stdout_is("xycde");
}

#[test]
fn test_truncate_applies_before_complement_with_class() {
    new_ucmd!()
        .args(&["-ct", "[:digit:]", "X"])
        .ignore_stdin_write_error()
        .pipe_in("A")
        .fails()
        .stderr_contains("when translating with complemented character classes,\nstring2 must map all characters in the domain to one");
}

#[test]
fn missing_args_fails() {
    let (_, mut ucmd) = at_and_ucmd!();
    ucmd.fails().stderr_contains("missing operand");
}

#[test]
fn missing_required_second_arg_fails() {
    let (_, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["foo"])
        .fails()
        .stderr_contains("missing operand after");
}

#[test]
fn test_interpret_backslash_escapes() {
    new_ucmd!()
        .args(&["abfnrtv", r"\a\b\f\n\r\t\v"]) // spell-checker:disable-line
        .pipe_in("abfnrtv") // spell-checker:disable-line
        .succeeds()
        .stdout_is("\u{7}\u{8}\u{c}\n\r\t\u{b}");
}

#[test]
fn test_interpret_unrecognized_backslash_escape_as_character() {
    new_ucmd!()
        .args(&["qcz+=~-", r"\q\c\z\+\=\~\-"])
        .pipe_in("qcz+=~-")
        .succeeds()
        .stdout_is("qcz+=~-");
}

#[test]
fn test_interpret_single_octal_escape() {
    new_ucmd!()
        .args(&["X", r"\015"])
        .pipe_in("X")
        .succeeds()
        .stdout_is("\r");
}

#[test]
fn test_interpret_one_and_two_digit_octal_escape() {
    new_ucmd!()
        .args(&["XYZ", r"\0\11\77"])
        .pipe_in("XYZ")
        .succeeds()
        .stdout_is("\0\t?");
}

#[test]
fn test_octal_escape_is_at_most_three_digits() {
    new_ucmd!()
        .args(&["XY", r"\0156"])
        .pipe_in("XY")
        .succeeds()
        .stdout_is("\r6");
}

#[test]
fn test_non_octal_digit_ends_escape() {
    new_ucmd!()
        .args(&["rust", r"\08\11956"])
        .pipe_in("rust")
        .succeeds()
        .stdout_is("\08\t9");
}

#[test]
fn test_interpret_backslash_at_eol_literally() {
    new_ucmd!()
        .args(&["X", r"\"])
        .pipe_in("X")
        .succeeds()
        .stdout_is("\\");
}

#[test]
fn test_more_than_2_sets() {
    new_ucmd!().args(&["'abcdef'", "'a'", "'b'"]).fails();
}

#[test]
fn basic_translation_works() {
    // echo -n "abcdefabcdef" | tr "dabcdef"  "xyz"
    new_ucmd!()
        .args(&["abcdef", "xyz"])
        .pipe_in("abcdefabcdef")
        .succeeds()
        .stdout_is("xyzzzzxyzzzz");
}

#[test]
fn alnum_overrides_translation_to_fallback_1() {
    // echo -n "abcdefghijklmnopqrstuvwxyz" | tr "abc[:alpha:]" "xyz"
    new_ucmd!()
        .args(&["abc[:alpha:]", "xyz"])
        .pipe_in("abcdefghijklmnopqrstuvwxyz")
        .succeeds()
        .stdout_is("zzzzzzzzzzzzzzzzzzzzzzzzzz");
}

#[test]
fn alnum_overrides_translation_to_fallback_2() {
    // echo -n "abcdefghijklmnopqrstuvwxyz" | tr "[:alpha:]abc" "xyz"
    new_ucmd!()
        .args(&["[:alpha:]abc", "xyz"])
        .pipe_in("abcdefghijklmnopqrstuvwxyz")
        .succeeds()
        .stdout_is("zzzzzzzzzzzzzzzzzzzzzzzzzz");
}

#[test]
fn overrides_translation_pair_if_repeats() {
    // echo -n 'aaa' | tr "aaa" "xyz"
    new_ucmd!()
        .args(&["aaa", "xyz"])
        .pipe_in("aaa")
        .succeeds()
        .stdout_is("zzz");
}

#[test]
fn uppercase_conversion_works_1() {
    // echo -n 'abcdefghijklmnopqrstuvwxyz' | tr "abcdefghijklmnopqrstuvwxyz" "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    new_ucmd!()
        .args(&["abcdefghijklmnopqrstuvwxyz", "ABCDEFGHIJKLMNOPQRSTUVWXYZ"])
        .pipe_in("abcdefghijklmnopqrstuvwxyz")
        .succeeds()
        .stdout_is("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
}

#[test]
fn uppercase_conversion_works_2() {
    // echo -n 'abcdefghijklmnopqrstuvwxyz' | tr "a-z" "A-Z"
    new_ucmd!()
        .args(&["a-z", "A-Z"])
        .pipe_in("abcdefghijklmnopqrstuvwxyz")
        .succeeds()
        .stdout_is("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
}

#[test]
fn uppercase_conversion_works_3() {
    // echo -n 'abcdefghijklmnopqrstuvwxyz' | tr "[:lower:]" "[:upper:]"
    new_ucmd!()
        .args(&["[:lower:]", "[:upper:]"])
        .pipe_in("abcdefghijklmnopqrstuvwxyz")
        .succeeds()
        .stdout_is("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
}

#[test]
fn translate_complement_set_in_order() {
    // echo -n '01234' | tr -c '@-~' ' -^'
    new_ucmd!()
        .args(&["-c", "@-~", " -^"])
        .pipe_in("01234")
        .succeeds()
        .stdout_is("PQRST");
}

#[test]
fn alpha_expands_uppercase_lowercase() {
    // echo -n "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz" | tr "[:alpha:]" " -_"
    new_ucmd!()
        .args(&["[:alpha:]", " -_"])
        .pipe_in("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz")
        .succeeds()
        .stdout_is(r##" !"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRS"##);
}

#[test]
fn alnum_expands_number_uppercase_lowercase() {
    // echo -n "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz" | tr "[:alnum:]" " -_"
    new_ucmd!()
        .args(&["[:alnum:]", " -_"])
        .pipe_in("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz")
        .succeeds()
        .stdout_is(r##" !"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\]"##);
}

#[test]
fn tr_translate_range_to_repeat_class_zero() {
    new_ucmd!()
        .args(&["abcd", "[]*]"])
        .pipe_in("abcd")
        .succeeds()
        .stdout_is("]]]]");
}

#[test]
fn tr_translate_range_to_repeat_class() {
    new_ucmd!()
        .args(&["rst", "[%*]uvw"])
        .pipe_in("rst")
        .succeeds()
        .stdout_is("uvw");
}

#[test]
fn tr_translate_empty_set1_noop() {
    new_ucmd!()
        .args(&["", "[.*]"])
        .pipe_in("rst")
        .succeeds()
        .stdout_is("rst");
}

#[test]
fn tr_truncate_set1_longer_than_set2() {
    // # Test --truncate-set1 behavior when string1 is longer than string2
    new_ucmd!()
        .args(&["-t", "mnop", "pq"])
        .pipe_in("mnopq")
        .succeeds()
        .stdout_is("pqopq");
}

#[test]
fn tr_bsd_set1_longer_extends_last_char() {
    // # Test bsd behavior (the default) when string1 is longer than string2
    new_ucmd!()
        .args(&["mnop", "pq"])
        .pipe_in("mnopq")
        .succeeds()
        .stdout_is("pqqqq");
}

#[test]
fn tr_posix_repeat_class_extension() {
    // # Do it the posix way
    new_ucmd!()
        .args(&["mnop", "p[q*]"])
        .pipe_in("mnopq")
        .succeeds()
        .stdout_is("pqqqq");
}

#[test]
fn tr_squeeze_range_to_special_chars() {
    new_ucmd!()
        .args(&["-s", "a-p", "%[.*]$"])
        .pipe_in("abcdefghijklmnop")
        .succeeds()
        .stdout_is("%.$");
}

#[test]
fn tr_squeeze_range_to_repeat_class() {
    new_ucmd!()
        .args(&["-s", "a-p", "[.*]$"])
        .pipe_in("abcdefghijklmnop")
        .succeeds()
        .stdout_is(".$");
}

#[test]
fn tr_squeeze_range_to_class_prefix() {
    new_ucmd!()
        .args(&["-s", "a-p", "%[.*]"])
        .pipe_in("abcdefghijklmnop")
        .succeeds()
        .stdout_is("%.");
}

#[test]
fn tr_squeeze_char_class_alnum() {
    new_ucmd!()
        .args(&["-s", "[p-z]"])
        .pipe_in("ppqqrr")
        .succeeds()
        .stdout_is("pqr");
}

#[test]
fn tr_squeeze_explicit_range() {
    new_ucmd!()
        .args(&["-s", "[p-r]"])
        .pipe_in("ppqqrr")
        .succeeds()
        .stdout_is("pqr");
}

#[test]
fn tr_squeeze_partial_range() {
    new_ucmd!()
        .args(&["-s", "[p-q]"])
        .pipe_in("ppqqrr")
        .succeeds()
        .stdout_is("pqrr");
}

#[test]
fn tr_squeeze_tail_range() {
    new_ucmd!()
        .args(&["-s", "[q-r]"])
        .pipe_in("ppqqrr")
        .succeeds()
        .stdout_is("ppqr");
}

#[test]
fn tr_squeeze_nul_range() {
    new_ucmd!()
        .args(&["-s", r"[\0-\5]"])
        .pipe_in(
            "\u{0}\u{0}a\u{1}\u{1}b\u{2}\u{2}\u{2}c\u{3}\u{3}\u{3}d\u{4}\u{4}\u{4}\u{4}e\u{5}\u{5}",
        )
        .succeeds()
        .stdout_is("\u{0}a\u{1}b\u{2}c\u{3}d\u{4}e\u{5}");
}

#[test]
fn tr_delete_equivalence_class_open() {
    // # tests of delete
    new_ucmd!()
        .args(&["-d", "[=[=]"])
        .pipe_in("[[[[[[[]]]]]]]]")
        .succeeds()
        .stdout_is("]]]]]]]]");
}

#[test]
fn tr_delete_equivalence_class_close() {
    new_ucmd!()
        .args(&["-d", "[=]=]"])
        .pipe_in("[[[[[[[]]]]]]]]")
        .succeeds()
        .stdout_is("[[[[[[[");
}

#[test]
fn tr_delete_xdigit_all() {
    new_ucmd!()
        .args(&["-d", "[:xdigit:]"])
        .pipe_in("0123456789acbdefABCDEF")
        .succeeds()
        .no_output();
}

#[test]
fn tr_delete_xdigit_leaves_non_hex() {
    new_ucmd!()
        .args(&["-d", "[:xdigit:]"])
        .pipe_in("w0x1y2z3456789acbdefABCDEFz")
        .succeeds()
        .stdout_is("wxyzz");
}

#[test]
fn tr_delete_digit_all() {
    new_ucmd!()
        .args(&["-d", "[:digit:]"])
        .pipe_in("0123456789")
        .succeeds()
        .no_output();
}

#[test]
fn tr_delete_digit_leaves_alpha() {
    new_ucmd!()
        .args(&["-d", "[:digit:]"])
        .pipe_in("a0b1c2d3e4f5g6h7i8j9k")
        .succeeds()
        .stdout_is("abcdefghijk");
}

#[test]
fn tr_delete_lower_all() {
    new_ucmd!()
        .args(&["-d", "[:lower:]"])
        .pipe_in("abcdefghijklmnopqrstuvwxyz")
        .succeeds()
        .no_output();
}

#[test]
fn tr_delete_upper_all() {
    new_ucmd!()
        .args(&["-d", "[:upper:]"])
        .pipe_in("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .succeeds()
        .no_output();
}

#[test]
fn tr_delete_alpha_all() {
    new_ucmd!()
        .args(&["-d", "[:lower:][:upper:]"])
        .pipe_in("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .succeeds()
        .no_output();
}

#[test]
fn tr_delete_alnum_all() {
    new_ucmd!()
        .args(&["-d", "[:alpha:]"])
        .pipe_in("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .succeeds()
        .no_output();
}

#[test]
fn tr_delete_space_class() {
    new_ucmd!()
        .args(&["-d", "[:alnum:]"])
        .pipe_in("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789")
        .succeeds()
        .no_output();
}

#[test]
fn tr_delete_complement_alnum() {
    new_ucmd!()
        .args(&["-d", "[:alnum:]"])
        .pipe_in(".abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.")
        .succeeds()
        .stdout_is("..");
}

#[test]
fn tr_delete_complement_alpha() {
    new_ucmd!()
        .args(&["-ds", "[:alnum:]", "."])
        .pipe_in(".abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.")
        .succeeds()
        .stdout_is(".");
}

#[test]
fn tr_squeeze_complement_alnum_to_newline_bsd() {
    // # The classic example, with string2 BSD-style
    new_ucmd!()
        .args(&["-cs", "[:alnum:]", "\n"])
        .pipe_in("The quick brown fox jumps over the lazy dog.")
        .succeeds()
        .stdout_is("The\nquick\nbrown\nfox\njumps\nover\nthe\nlazy\ndog\n");
}

#[test]
fn tr_squeeze_complement_alnum_to_newline_posix() {
    // # The classic example, POSIX-style
    new_ucmd!()
        .args(&["-cs", "[:alnum:]", "[\n*]"])
        .pipe_in("The big black fox jumped over the fence.")
        .succeeds()
        .stdout_is("The\nbig\nblack\nfox\njumped\nover\nthe\nfence\n");
}

#[test]
fn tr_delete_squeeze_combined() {
    new_ucmd!()
        .args(&["-ds", "q", "p"])
        .pipe_in("ppqqpp")
        .succeeds()
        .stdout_is("p");
}

#[test]
fn tr_delete_squeeze_xdigit() {
    new_ucmd!()
        .args(&["-ds", "[:xdigit:]", "Z"])
        .pipe_in("ZZ0123456789acbdefABCDEFZZ")
        .succeeds()
        .stdout_is("Z");
}

#[test]
fn tr_translate_alnum_to_rot13_plus_space() {
    // # Try some data with 8th bit set in case something is mistakenly
    // # sign-extended.
    new_ucmd!()
        .arg("-ds")
        .args(&["\\350", "\\345"])
        .pipe_in([0o300, 0o301, 0o377, 0o345, 0o345, 0o350, 0o345])
        .succeeds()
        .stdout_is_bytes([0o300, 0o301, 0o377, 0o345]);
}

#[test]
fn tr_squeeze_alnum_remove_spaces() {
    new_ucmd!()
        .args(&["-s", "abcdefghijklmn", "[:*016]"])
        .pipe_in("abcdefghijklmnop")
        .succeeds()
        .stdout_is(":op");
}

#[test]
fn tr_translate_digits_to_letters() {
    new_ucmd!()
        .args(&["-d", "a-z"])
        .pipe_in("abc $code")
        .succeeds()
        .stdout_is(" $");
}

#[test]
fn tr_translate_control_chars() {
    new_ucmd!()
        .args(&["-ds", "a-z", "$."])
        .pipe_in("a.b.c $$$$code\\")
        .succeeds()
        .stdout_is(". $\\");
}

#[test]
fn tr_translate_single_char_range() {
    // # Make sure that a-a is accepted.
    new_ucmd!()
        .args(&["a-a", "z"])
        .pipe_in("abc")
        .succeeds()
        .stdout_is("zbc");
}

#[test]
fn tr_translate_with_null_byte() {
    //  {ERR=>"$prog: when not truncating set1, string2 must be non-empty\n"}],
    new_ucmd!()
        .args(&["a", ""])
        .pipe_in("")
        .fails()
        .stderr_is("tr: when not truncating set1, string2 must be non-empty\n");
}

#[test]
fn tr_translate_lower_to_upper() {
    new_ucmd!()
        .args(&["[:lower:]", "[:upper:]"])
        .pipe_in("abcxyzABCXYZ")
        .succeeds()
        .stdout_is("ABCXYZABCXYZ");
}

#[test]
fn tr_translate_upper_to_lower() {
    new_ucmd!()
        .args(&["[:upper:]", "[:lower:]"])
        .pipe_in("abcxyzABCXYZ")
        .succeeds()
        .stdout_is("abcxyzabcxyz");
}

#[test]
fn tr_repeat_complement_char_class() {
    new_ucmd!()
        .args(&["a[=*2][=c=]", "xyyz"])
        .pipe_in("a=c")
        .succeeds()
        .stdout_is("xyz");
}

#[test]
fn tr_repeat_class_in_set2_basic() {
    new_ucmd!()
        .args(&["[:*3][:digit:]", "a-m"])
        .pipe_in(":1239")
        .succeeds()
        .stdout_is("cefgm");
}

#[test]
fn tr_repeat_class_extends_set2() {
    new_ucmd!()
        .args(&["a[b*512]c", "1[x*]2"])
        .pipe_in("abc")
        .succeeds()
        .stdout_is("1x2");
}

#[test]
fn tr_repeat_class_with_squeeze() {
    new_ucmd!()
        .args(&["a[b*513]c", "1[x*]2"])
        .pipe_in("abc")
        .succeeds()
        .stdout_is("1x2");
}

#[test]
fn tr_translate_overlap_repeat() {
    // # Another couple octal repeat count tests.
    //  {ERR=>"$prog: invalid repeat count '08' in [c*n] construct\n"}],
    new_ucmd!()
        .args(&["[b*08]", "[x*]"])
        .pipe_in("")
        .fails()
        .stderr_is("tr: invalid repeat count '08' in [c*n] construct\n");
}

#[test]
fn tr_translate_overlap_repeat_squeeze() {
    new_ucmd!()
        .args(&["[b*010]cd", "[a*7]BC[x*]"])
        .pipe_in("bcd")
        .succeeds()
        .stdout_is("BCx");
}

#[test]
fn octal_repeat_count_test() {
    //below will result in 8'x' and 4'y' as octal 010 = decimal 8
    new_ucmd!()
        .args(&["ABCdefghijkl", "[x*010]Y"])
        .pipe_in("ABCdefghijklmn12")
        .succeeds()
        .stdout_is("xxxxxxxxYYYYmn12");
}

#[test]
fn non_octal_repeat_count_test() {
    //below will result in 10'x' and 2'y' as the 10 does not have 0 prefix
    new_ucmd!()
        .args(&["ABCdefghijkl", "[x*10]Y"])
        .pipe_in("ABCdefghijklmn12")
        .succeeds()
        .stdout_is("xxxxxxxxxxYYmn12");
}

#[test]
fn tr_translate_with_escape_sequence() {
    new_ucmd!()
        .args(&[r"a\-z", "A-Z"])
        .pipe_in("abc-z")
        .succeeds()
        .stdout_is("AbcBC");
}

#[test]
fn tr_translate_octal_backslash() {
    new_ucmd!()
        .args(&["a\u{055}b", "def"])
        .pipe_in("a\u{055}b")
        .succeeds()
        .stdout_is("def");
}

#[test]
// Fails on Windows because it will not separate '\' and 'x' as separate arguments
#[cfg(unix)]
fn tr_translate_backslash_at_end() {
    //  {ERR=>"$prog: warning: an unescaped backslash at end of "
    //   . "string is not portable\n"}],
    new_ucmd!()
        .args(&[r"\", "x"])
        .pipe_in(r"\")
        .succeeds()
        .stdout_is("x")
        .stderr_is("tr: warning: an unescaped backslash at end of string is not portable\n");
}

#[test]
fn tr_ross_delete_no_squeeze() {
    // # From Ross
    //  {ERR=>$map_all_to_1}],
    new_ucmd!()
        .args(&["-cs", "[:upper:]", "X[Y*]"])
        .pipe_in("")
        .fails()
        .stderr_is("tr: when translating with complemented character classes,\nstring2 must map all characters in the domain to one\n");
}

#[test]
fn tr_ross_delete_with_squeeze() {
    //  {ERR=>$map_all_to_1}],
    new_ucmd!()
        .args(&["-cs", "[:cntrl:]", "X[Y*]"])
        .pipe_in("")
        .fails()
        .stderr_is("tr: when translating with complemented character classes,\nstring2 must map all characters in the domain to one\n");
}

#[test]
fn tr_ross_translate_complement() {
    new_ucmd!()
        .args(&["-cs", "[:upper:]", "[X*]"])
        .pipe_in("AMZamz123.-+AMZ")
        .succeeds()
        .stdout_is("AMZXAMZ");
}

#[test]
fn tr_ross_translate_complement_squeeze() {
    new_ucmd!()
        .args(&["-cs", "[:upper:][:digit:]", "[Z*]"])
        .pipe_in("")
        .succeeds()
        .no_output();
}

#[test]
fn tr_ross_delete_complement() {
    new_ucmd!()
        .args(&["-dcs", "[:lower:]", "n-rs-z"])
        .pipe_in("amzAMZ123.-+amz")
        .succeeds()
        .stdout_is("amzamz");
}

#[test]
fn tr_ross_delete_complement_squeeze() {
    new_ucmd!()
        .args(&["-ds", "[:xdigit:]", "[:alnum:]"])
        .pipe_in(".ZABCDEFGzabcdefg.0123456788899.GG")
        .succeeds()
        .stdout_is(".ZGzg..G");
}

#[test]
fn tr_ross_translate_overlong() {
    new_ucmd!()
        .args(&["-dcs", "[:alnum:]", "[:digit:]"])
        .pipe_in("")
        .succeeds()
        .no_output();
}

#[test]
fn tr_ross_translate_squeeze_overlong() {
    new_ucmd!()
        .args(&["-dc", "[:lower:]"])
        .pipe_in("")
        .succeeds()
        .no_output();
}

#[test]
fn tr_ross_translate_squeeze_delete() {
    new_ucmd!()
        .args(&["-dc", "[:upper:]"])
        .pipe_in("")
        .succeeds()
        .no_output();
}

#[test]
fn tr_error_empty_equivalence_class() {
    // # Ensure that these fail.
    // # Prior to 2.0.20, each would evoke a failed assertion.
    //  {ERR=>"$prog: missing equivalence class character '[==]'\n"}],
    new_ucmd!()
        .args(&["[==]", "x"])
        .pipe_in("")
        .fails()
        .stderr_is("tr: missing equivalence class character '[==]'\n");
}

#[test]
fn check_too_many_chars_in_eq() {
    new_ucmd!()
        .args(&["-d", "[=aa=]"])
        .pipe_in("")
        .fails()
        .stderr_contains("aa: equivalence class operand must be a single character\n");
}

#[test]
fn tr_error_empty_char_class() {
    //  {ERR=>"$prog: missing character class name '[::]'\n"}],
    new_ucmd!()
        .args(&["[::]", "x"])
        .pipe_in("")
        .fails()
        .stderr_is("tr: missing character class name '[::]'\n");
}

#[test]
fn tr_error_invalid_char_class() {
    new_ucmd!()
        .args(&["[:fooclass:]", "x"])
        .pipe_in("")
        .fails()
        .stderr_is("tr: invalid character class 'fooclass'\n");
}

#[test]
fn tr_error_repeat_in_set1() {
    new_ucmd!()
        .args(&["[a*]", "a"])
        .pipe_in("")
        .fails()
        .stderr_is("tr: the [c*] repeat construct may not appear in string1\n");
}

#[test]
fn tr_translate_repeat_in_set2() {
    new_ucmd!()
        .args(&["a", "[a*][a*]"])
        .pipe_in("")
        .fails()
        .stderr_is("tr: only one [c*] repeat construct may appear in string2\n");
}

#[test]
fn tr_translate_repeat_octal() {
    // # Weird repeat counts.
    new_ucmd!()
        .args(&["abc", r"[b*\9]"])
        .pipe_in("abcd")
        .succeeds()
        .stdout_is("[b*d");
}

#[test]
fn tr_translate_repeat_zero_count() {
    new_ucmd!()
        .args(&["abc", "[b*0]"])
        .pipe_in("abcd")
        .succeeds()
        .stdout_is("bbbd");
}

#[test]
fn tr_translate_repeat_multiple_zeros() {
    new_ucmd!()
        .args(&["abc", "[b*00000000000000000000]"])
        .pipe_in("abcd")
        .succeeds()
        .stdout_is("bbbd");
}

#[test]
fn tr_translate_repeat_complement() {
    new_ucmd!()
        .args(&["-c", "[a*65536]\n", "[b*]"])
        .pipe_in("abcd")
        .succeeds()
        .stdout_is("abbb");
}

#[test]
fn tr_translate_repeat_x_complement() {
    new_ucmd!()
        .args(&["-C", "[a*65536]\n", "[b*]"])
        .pipe_in("abcd")
        .succeeds()
        .stdout_is("abbb");
}

#[test]
fn tr_fowler_translate_basic() {
    // # From Glenn Fowler.
    new_ucmd!()
        .args(&["ah", "-H"])
        .pipe_in("aha")
        .succeeds()
        .stdout_is("-H-");
}

#[test]
fn tr_translate_no_abort_on_long_input() {
    // # Up to coreutils-6.9, this would provoke a failed assertion.
    new_ucmd!()
        .args(&["-c", "a", "[b*256]"])
        .pipe_in("abc")
        .succeeds()
        .stdout_is("abb");
}

#[test]
fn test_delete_flag_takes_only_one_operand() {
    // gnu tr -d fails with more than 1 argument
    new_ucmd!().args(&["-d", "a", "p"]).fails().stderr_contains(
        "extra operand 'p'\nOnly one string may be given when deleting without squeezing repeats.",
    );
}

#[test]
fn test_truncate_flag_fails_with_more_than_two_operand() {
    new_ucmd!()
        .args(&["-t", "a", "b", "c"])
        .fails()
        .stderr_contains("extra operand 'c'");
}

#[test]
fn test_squeeze_flag_fails_with_more_than_two_operand() {
    new_ucmd!()
        .args(&["-s", "a", "b", "c"])
        .fails()
        .stderr_contains("extra operand 'c'");
}

#[test]
fn test_complement_flag_fails_with_more_than_two_operand() {
    new_ucmd!()
        .args(&["-c", "a", "b", "c"])
        .fails()
        .stderr_contains("extra operand 'c'");
}

#[test]
fn check_regression_class_space() {
    // This invocation checks:
    // 1. that the [:space:] class has exactly 6 characters,
    // 2. that the [:space:] class contains at least the given 6 characters (and therefore no other characters), and
    // 3. that the given characters occur in exactly this order.
    new_ucmd!()
        .args(&["[:space:][:upper:]", "123456[:lower:]"])
        // 0x0B = "\v" ("VERTICAL TAB")
        // 0x0C = "\f" ("FEED FORWARD")
        .pipe_in("A\t\n\u{0B}\u{0C}\r B")
        .succeeds()
        .no_stderr()
        .stdout_only("a123456b");
}

#[test]
fn check_regression_class_blank() {
    // This invocation checks:
    // 1. that the [:blank:] class has exactly 2 characters,
    // 2. that the [:blank:] class contains at least the given 2 characters (and therefore no other characters), and
    // 3. that the given characters occur in exactly this order.
    new_ucmd!()
        .args(&["[:blank:][:upper:]", "12[:lower:]"])
        .pipe_in("A\t B")
        .succeeds()
        .no_stderr()
        .stdout_only("a12b");
}

// Check regression found in https://github.com/uutils/coreutils/issues/6163
#[test]
fn check_regression_issue_6163_no_match() {
    new_ucmd!()
        .args(&["-c", "-t", "Y", "Z"])
        .pipe_in("X\n")
        .succeeds()
        .no_stderr()
        .stdout_only("X\n");
}

#[test]
fn check_regression_issue_6163_match() {
    new_ucmd!()
        .args(&["-c", "-t", "Y", "Z"])
        .pipe_in("\0\n")
        .succeeds()
        .no_stderr()
        .stdout_only("Z\n");
}

#[test]
fn check_ignore_truncate_when_deleting_and_squeezing() {
    new_ucmd!()
        .args(&["-dts", "asdf", "qwe"])
        .pipe_in("asdfqqwweerr\n")
        .succeeds()
        .no_stderr()
        .stdout_only("qwerr\n");
}

#[test]
fn check_ignore_truncate_when_deleting() {
    new_ucmd!()
        .args(&["-dt", "asdf"])
        .pipe_in("asdfqwer\n")
        .succeeds()
        .no_stderr()
        .stdout_only("qwer\n");
}

#[test]
fn check_ignore_truncate_when_squeezing() {
    new_ucmd!()
        .args(&["-ts", "asdf"])
        .pipe_in("aassddffqwer\n")
        .succeeds()
        .no_stderr()
        .stdout_only("asdfqwer\n");
}

#[test]
fn check_disallow_blank_in_set2_when_translating() {
    new_ucmd!().args(&["-t", "1234", "[:blank:]"]).fails();
}

#[test]
fn check_class_in_set2_must_be_matched_in_set1() {
    new_ucmd!().args(&["-t", "1[:upper:]", "[:upper:]"]).fails();
}

#[test]
fn check_class_in_set2_must_be_matched_in_set1_right_length_check() {
    new_ucmd!()
        .args(&["-t", "a-z[:upper:]", "abcdefghijklmnopqrstuvwxyz[:upper:]"])
        .succeeds();
}

#[test]
fn check_set1_longer_set2_ends_in_class() {
    new_ucmd!().args(&["[:lower:]a", "[:upper:]"]).fails();
}

#[test]
fn check_set1_longer_set2_ends_in_class_with_trunc() {
    new_ucmd!()
        .args(&["-t", "[:lower:]a", "[:upper:]"])
        .succeeds();
}

#[test]
fn check_complement_2_unique_in_set2() {
    let x226 = "x".repeat(226);

    // [y*] is expanded tp "y" here
    let arg = x226 + "[y*]xxx";
    new_ucmd!().args(&["-c", "[:upper:]", arg.as_str()]).fails();
}

#[test]
fn check_complement_1_unique_in_set2() {
    let x226 = "x".repeat(226);

    // [y*] is expanded to "" here
    let arg = x226 + "[y*]xxxx";
    new_ucmd!()
        .args(&["-c", "[:upper:]", arg.as_str()])
        .succeeds();
}

#[test]
fn check_complement_set2_too_big() {
    let x231 = "x".repeat(231);
    let x230 = &x231[..230];

    // The complement of [:upper:] expands to 230 characters,
    // putting more characters in set2 should fail.
    new_ucmd!().args(&["-c", "[:upper:]", x230]).succeeds();
    new_ucmd!()
        .args(&["-c", "[:upper:]", x231.as_str()])
        .fails()
        .stderr_contains("when translating with complemented character classes,\nstring2 must map all characters in the domain to one");
}

#[test]
#[cfg(unix)]
#[cfg_attr(wasi_runner, ignore = "WASI: argv/filenames must be valid UTF-8")]
fn test_truncate_non_utf8_set() {
    let stdin = b"\x01amp\xfe\xff";
    let set1 = OsStr::from_bytes(b"a\xfe\xffz"); // spell-checker:disable-line
    let set2 = OsStr::from_bytes(b"01234");

    new_ucmd!()
        .arg(set1)
        .arg(set2)
        .pipe_in(*stdin)
        .succeeds()
        .stdout_is_bytes(b"\x010mp12");
}

#[test]
#[cfg(unix)]
fn test_unescaped_backslash_warning_false_positive() {
    // Was erroneously printing this warning (even though the backslash was escaped):
    // "tr: warning: an unescaped backslash at end of string is not portable"
    new_ucmd!()
        .args(&["-d", r"\\"])
        .pipe_in(r"a\b\c\")
        .succeeds()
        .stdout_only("abc");
    new_ucmd!()
        .args(&["-d", r"\\\\"])
        .pipe_in(r"a\b\c\")
        .succeeds()
        .stdout_only("abc");
    new_ucmd!()
        .args(&["-d", r"\\\\\\"])
        .pipe_in(r"a\b\c\")
        .succeeds()
        .stdout_only("abc");
}

#[test]
#[cfg(unix)]
fn test_trailing_backslash() {
    new_ucmd!()
        .args(&["-d", r"\"])
        .pipe_in(r"a\b\c\")
        .succeeds()
        .stderr_is("tr: warning: an unescaped backslash at end of string is not portable\n")
        .stdout_is("abc");
    new_ucmd!()
        .args(&["-d", r"\\\"])
        .pipe_in(r"a\b\c\")
        .succeeds()
        .stderr_is("tr: warning: an unescaped backslash at end of string is not portable\n")
        .stdout_is("abc");
    new_ucmd!()
        .args(&["-d", r"\\\\\"])
        .pipe_in(r"a\b\c\")
        .succeeds()
        .stderr_is("tr: warning: an unescaped backslash at end of string is not portable\n")
        .stdout_is("abc");
}

#[test]
fn test_multibyte_octal_sequence() {
    new_ucmd!()
        .args(&["-d", r"\501"])
        .pipe_in("(1Ł)")
        .succeeds()
        .stderr_is("tr: warning: the ambiguous octal escape \\501 is being interpreted as the 2-byte sequence \\050, 1\n")
        .stdout_is("Ł)");
}

#[test]
fn test_octal_warning_still_fires_after_a_bad_sequence() {
    // The set fails on the character class, but the ambiguous octal escape
    // after it must still be warned about: the whole set is parsed even when
    // an earlier sequence is bad.
    // No `pipe_in`: the sets are rejected before stdin is ever read, so
    // handing the child input only races its exit.
    new_ucmd!()
        .args(&[r"[:foo:]\400", "y"])
        .fails()
        .stderr_is(
            "tr: warning: the ambiguous octal escape \\400 is being interpreted as the 2-byte sequence \\040, 0\n\
             tr: invalid character class 'foo'\n",
        );
}

#[test]
fn test_backwards_range() {
    new_ucmd!()
        .args(&["-d", r"\046-\048"])
        .pipe_in("")
        .fails()
        .stderr_only(
            r"tr: range-endpoints of '&-\004' are in reverse collating sequence order
",
        );
}

#[test]
fn test_non_digit_repeat() {
    new_ucmd!()
        .args(&["a", "[b*c]"])
        .pipe_in("")
        .fails()
        .stderr_only("tr: invalid repeat count 'c' in [c*n] construct\n");
}

#[test]
#[cfg(unix)]
#[cfg_attr(wasi_runner, ignore = "WASI: argv/filenames must be valid UTF-8")]
fn test_octal_escape_ambiguous_followed_by_non_utf8() {
    // This case does not trigger the panic
    let set1 = OsStr::from_bytes(b"\\501a");
    new_ucmd!()
        .arg("-d")
        .arg(set1)
        .pipe_in("(1a)")
        .succeeds()
        .stderr_contains("warning: the ambiguous octal escape")
        .stdout_is(")");

    // An user is not supposed to use this invalid utf8 set but
    // we would need to make the command more error-proof still
    let set1 = OsStr::from_bytes(b"\\501\xff");
    new_ucmd!()
        .arg("-d")
        .arg(set1)
        .pipe_in([b'(', b'1', 0xff, b')'])
        .succeeds()
        .stderr_contains("warning: invalid utf8 sequence");
}

#[cfg(target_os = "linux")]
#[test]
fn test_failed_write_is_reported() {
    new_ucmd!()
        .pipe_in("hello")
        .args(&["e", "a"])
        .set_stdout(std::fs::File::create("/dev/full").unwrap())
        .fails()
        .stderr_is("tr: write error: No space left on device\n");
}

#[test]
#[cfg_attr(wasi_runner, ignore = "WASI: no pipe/signal support")]
fn test_broken_pipe_no_error() {
    new_ucmd!()
        .args(&["e", "a"])
        .pipe_in("hello".repeat(100))
        .run_stdout_starts_with(b"")
        .fails_silently();
}

#[cfg(unix)]
#[test]
fn test_stdin_is_socket() {
    use std::io::Write as _;

    let (fd1, fd2) = rustix::net::socketpair(
        rustix::net::AddressFamily::UNIX,
        rustix::net::SocketType::STREAM,
        rustix::net::SocketFlags::empty(),
        None,
    )
    .unwrap();
    std::fs::File::from(fd1).write_all(b"::").unwrap();
    new_ucmd!()
        .args(&[":", ";"])
        .set_stdin(fd2)
        .succeeds()
        .stdout_is(";;");
}

#[cfg(all(feature = "feat_diagnostics", not(wasi_runner)))]
mod diagnostics {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn test_snippet_points_at_the_misspelled_class() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["w[:lowre:]w", "x"])
            .fails_with_code(1);

        // The whole report: the `tr: ` prefix of the plain form, the argument
        // list echoed back, a caret covering `[:lowre:]` only — not the whole
        // set — and the list of valid class names.
        assert_eq!(
            result.stderr_as_displayed(),
            "\
tr: invalid character class 'lowre'
   ╭─[ tr:1:5 ]
   │
 1 │ tr w[:lowre:]w x
   │     ─────────
   │
   │ Help: classes are alnum, alpha, blank, cntrl, digit, graph, lower, print, punct, space, upper and xdigit
───╯"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_snippet_points_at_the_backwards_range() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["qw[y-b]", "x"])
            .fails_with_code(1);

        // The caret covers `y-b` alone, not the letters around the range.
        assert_eq!(
            result.stderr_as_displayed(),
            "\
tr: range-endpoints of 'y-b' are in reverse collating sequence order
   ╭─[ tr:1:7 ]
   │
 1 │ tr qw[y-b] x
   │       ─┬─
   │        ╰─── did you mean 'b-y'?
   │
   │ Help: a range goes from the lower character to the higher one, as in a-z
───╯"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_snippet_points_at_the_repeat_count() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["wxy", "[z*4k]"])
            .fails_with_code(1);

        assert_eq!(
            result.stderr_as_displayed(),
            "\
tr: invalid repeat count '4k' in [c*n] construct
   ╭─[ tr:1:8 ]
   │
 1 │ tr wxy [z*4k]
   │        ──────
   │
   │ Help: [c*N] repeats c N times, [c*] pads SET2 to the length of SET1
───╯"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_snippet_underlines_the_whole_set_when_the_set_is_at_fault() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["w[q*]", "xyz"])
            .fails_with_code(1);

        // Nothing inside the set is wrong on its own, so all of it is marked.
        assert_eq!(
            result.stderr_as_displayed(),
            "\
tr: the [c*] repeat construct may not appear in string1
   ╭─[ tr:1:4 ]
   │
 1 │ tr w[q*] xyz
   │    ─────
   │
   │ Help: [c*N] repeats c N times, [c*] pads SET2 to the length of SET1
───╯"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_snippet_points_into_the_second_set() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["[:lower:]", "q[=we=]"])
            .fails_with_code(1);

        // The set at fault is the second operand, not the first.
        assert_eq!(
            result.stderr_as_displayed(),
            "\
tr: we: equivalence class operand must be a single character
   ╭─[ tr:1:15 ]
   │
 1 │ tr [:lower:] q[=we=]
   │               ──────
   │
   │ Help: [=c=] stands for every character equivalent to c
───╯"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_snippet_ignores_the_program_name_as_a_set() {
        // The second set reads exactly like the program name; the caret must
        // stay on the operand, not drift onto `tr` at the start of the line.
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["-c", "[:alpha:]", "tr"])
            .fails_with_code(1);

        assert_eq!(
            result.stderr_as_displayed(),
            "\
tr: when translating with complemented character classes,
string2 must map all characters in the domain to one
   ╭─[ tr:1:17 ]
   │
 1 │ tr -c [:alpha:] tr
   │                 ─┬
   │                  ╰── only one character may be complemented to
───╯"
        );
    }

    #[test]
    fn test_plain_message_when_stderr_is_a_pipe() {
        // The test harness pipes stderr, so the report must not appear.
        new_ucmd!()
            .args(&["w[:lowre:]w", "x"])
            .fails_with_code(1)
            .stderr_only("tr: invalid character class 'lowre'\n");
    }
}
