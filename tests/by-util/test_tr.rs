// spell-checker:ignore aabbaa aabbcc aabc abbb abcc abcdefabcdef abcdefghijk abcdefghijklmn abcdefghijklmnop ABCDEFGHIJKLMNOPQRS abcdefghijklmnopqrstuvwxyz ABCDEFGHIJKLMNOPQRSTUVWXYZ ABCDEFZZ abcxyz ABCXYZ abcxyzabcxyz ABCXYZABCXYZ acbdef alnum amzamz AMZXAMZ bbbd cclass cefgm cntrl compl dabcdef dncase Gzabcdefg PQRST upcase wxyzz xdigit xycde xyyye xyyz xyzzzzxyzzzz ZABCDEF Zamz Cdefghijkl Cdefghijklmn
use crate::common::util::*;

#[test]
fn test_to_upper() {
    new_ucmd!()
        .args(&["a-z", "A-Z"])
        .pipe_in("!abcd!")
        .run()
        .stdout_is("!ABCD!");
}

#[test]
fn test_small_set2() {
    new_ucmd!()
        .args(&["0-9", "X"])
        .pipe_in("@0123456789")
        .run()
        .stdout_is("@XXXXXXXXXX");
}

#[test]
fn test_unicode() {
    new_ucmd!()
        .args(&[", ┬─┬", "╯︵┻━┻"])
        .pipe_in("(,°□°）, ┬─┬")
        .run()
        .stdout_is("(╯°□°）╯︵┻━┻");
}

#[test]
fn test_delete() {
    new_ucmd!()
        .args(&["-d", "a-z"])
        .pipe_in("aBcD")
        .run()
        .stdout_is("BD");
}

#[test]
fn test_delete_complement() {
    new_ucmd!()
        .args(&["-d", "-c", "a-z"])
        .pipe_in("aBcD")
        .run()
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
        .run()
        .stdout_is("aX");
}

#[test]
fn test_complement2() {
    new_ucmd!()
        .args(&["-c", "0-9", "x"])
        .pipe_in("Phone: 01234 567890")
        .run()
        .stdout_is("xxxxxxx01234x567890");
}

#[test]
fn test_complement3() {
    new_ucmd!()
        .args(&["-c", "abcdefgh", "123"]) // spell-checker:disable-line
        .pipe_in("the cat and the bat")
        .run()
        .stdout_is("3he3ca33a3d33he3ba3"); // spell-checker:disable-line
}

#[test]
fn test_complement4() {
    // $ echo -n '0x1y2z3' | tr -c '0-@' '*-~'
    // 0~1~2~3
    new_ucmd!()
        .args(&["-c", "0-@", "*-~"])
        .pipe_in("0x1y2z3")
        .run()
        .stdout_is("0~1~2~3");
}

#[test]
fn test_complement5() {
    // $ echo -n '0x1y2z3' | tr -c '\0-@' '*-~'
    // 0a1b2c3
    new_ucmd!()
        .args(&["-c", r"\0-@", "*-~"])
        .pipe_in("0x1y2z3")
        .run()
        .stdout_is("0a1b2c3");
}

#[test]
fn test_squeeze() {
    new_ucmd!()
        .args(&["-s", "a-z"])
        .pipe_in("aaBBcDcc")
        .run()
        .stdout_is("aBBcDc");
}

#[test]
fn test_squeeze_complement() {
    new_ucmd!()
        .args(&["-sc", "a-z"])
        .pipe_in("aaBBcDcc")
        .run()
        .stdout_is("aaBcDcc");
}

#[test]
fn test_squeeze_complement_two_sets() {
    new_ucmd!()
        .args(&["-sc", "a", "_"])
        .pipe_in("test a aa with 3 ___ spaaaces +++") // spell-checker:disable-line
        .run()
        .stdout_is("_a_aa_aaa_");
}

#[test]
fn test_translate_and_squeeze() {
    new_ucmd!()
        .args(&["-s", "x", "y"])
        .pipe_in("xx")
        .run()
        .stdout_is("y");
}

#[test]
fn test_translate_and_squeeze_multiple_lines() {
    new_ucmd!()
        .args(&["-s", "x", "y"])
        .pipe_in("xxaax\nxaaxx") // spell-checker:disable-line
        .run()
        .stdout_is("yaay\nyaay"); // spell-checker:disable-line
}

#[test]
fn test_delete_and_squeeze() {
    new_ucmd!()
        .args(&["-ds", "a-z", "A-Z"])
        .pipe_in("abBcB")
        .run()
        .stdout_is("B");
}

#[test]
fn test_delete_and_squeeze_complement() {
    new_ucmd!()
        .args(&["-dsc", "a-z", "A-Z"])
        .pipe_in("abBcB")
        .run()
        .stdout_is("abc");
}

#[test]
fn test_set1_longer_than_set2() {
    new_ucmd!()
        .args(&["abc", "xy"])
        .pipe_in("abcde")
        .run()
        .stdout_is("xyyde"); // spell-checker:disable-line
}

#[test]
fn test_set1_shorter_than_set2() {
    new_ucmd!()
        .args(&["ab", "xyz"])
        .pipe_in("abcde")
        .run()
        .stdout_is("xycde"); // spell-checker:disable-line
}

#[test]
fn test_truncate() {
    // echo -n "abcde" | tr -t "abc" "xy"
    new_ucmd!()
        .args(&["-t", "abc", "xy"])
        .pipe_in("abcde")
        .run()
        .stdout_is("xycde"); // spell-checker:disable-line
}

#[test]
fn test_truncate_with_set1_shorter_than_set2() {
    new_ucmd!()
        .args(&["-t", "ab", "xyz"])
        .pipe_in("abcde")
        .run()
        .stdout_is("xycde"); // spell-checker:disable-line
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
fn check_against_gnu_tr_tests() {
    // ['1', qw(abcd '[]*]'),   {IN=>'abcd'}, {OUT=>']]]]'}],
    new_ucmd!()
        .args(&["abcd", "[]*]"])
        .pipe_in("abcd")
        .succeeds()
        .stdout_is("]]]]");
}

#[test]
fn check_against_gnu_tr_tests_2() {
    // ['2', qw(abc '[%*]xyz'), {IN=>'abc'}, {OUT=>'xyz'}],
    new_ucmd!()
        .args(&["abc", "[%*]xyz"])
        .pipe_in("abc")
        .succeeds()
        .stdout_is("xyz");
}

#[test]
fn check_against_gnu_tr_tests_3() {
    // ['3', qw('' '[.*]'),     {IN=>'abc'}, {OUT=>'abc'}],
    new_ucmd!()
        .args(&["", "[.*]"])
        .pipe_in("abc")
        .succeeds()
        .stdout_is("abc");
}

#[test]
fn check_against_gnu_tr_tests_4() {
    // # Test --truncate-set1 behavior when string1 is longer than string2
    // ['4', qw(-t abcd xy), {IN=>'abcde'}, {OUT=>'xycde'}],
    new_ucmd!()
        .args(&["-t", "abcd", "xy"])
        .pipe_in("abcde")
        .succeeds()
        .stdout_is("xycde");
}

#[test]
fn check_against_gnu_tr_tests_5() {
    // # Test bsd behavior (the default) when string1 is longer than string2
    // ['5', qw(abcd xy), {IN=>'abcde'}, {OUT=>'xyyye'}],
    new_ucmd!()
        .args(&["abcd", "xy"])
        .pipe_in("abcde")
        .succeeds()
        .stdout_is("xyyye");
}

#[test]
fn check_against_gnu_tr_tests_6() {
    // # Do it the posix way
    // ['6', qw(abcd 'x[y*]'), {IN=>'abcde'}, {OUT=>'xyyye'}],
    new_ucmd!()
        .args(&["abcd", "x[y*]"])
        .pipe_in("abcde")
        .succeeds()
        .stdout_is("xyyye");
}

#[test]
fn check_against_gnu_tr_tests_7() {
    // ['7', qw(-s a-p '%[.*]$'), {IN=>'abcdefghijklmnop'}, {OUT=>'%.$'}],
    new_ucmd!()
        .args(&["-s", "a-p", "%[.*]$"])
        .pipe_in("abcdefghijklmnop")
        .succeeds()
        .stdout_is("%.$");
}

#[test]
fn check_against_gnu_tr_tests_8() {
    // ['8', qw(-s a-p '[.*]$'), {IN=>'abcdefghijklmnop'}, {OUT=>'.$'}],
    new_ucmd!()
        .args(&["-s", "a-p", "[.*]$"])
        .pipe_in("abcdefghijklmnop")
        .succeeds()
        .stdout_is(".$");
}

#[test]
fn check_against_gnu_tr_tests_9() {
    // ['9', qw(-s a-p '%[.*]'), {IN=>'abcdefghijklmnop'}, {OUT=>'%.'}],
    new_ucmd!()
        .args(&["-s", "a-p", "%[.*]"])
        .pipe_in("abcdefghijklmnop")
        .succeeds()
        .stdout_is("%.");
}

#[test]
fn check_against_gnu_tr_tests_a() {
    // ['a', qw(-s '[a-z]'), {IN=>'aabbcc'}, {OUT=>'abc'}],
    new_ucmd!()
        .args(&["-s", "[a-z]"])
        .pipe_in("aabbcc")
        .succeeds()
        .stdout_is("abc");
}

#[test]
fn check_against_gnu_tr_tests_b() {
    // ['b', qw(-s '[a-c]'), {IN=>'aabbcc'}, {OUT=>'abc'}],
    new_ucmd!()
        .args(&["-s", "[a-c]"])
        .pipe_in("aabbcc")
        .succeeds()
        .stdout_is("abc");
}

#[test]
fn check_against_gnu_tr_tests_c() {
    // ['c', qw(-s '[a-b]'), {IN=>'aabbcc'}, {OUT=>'abcc'}],
    new_ucmd!()
        .args(&["-s", "[a-b]"])
        .pipe_in("aabbcc")
        .succeeds()
        .stdout_is("abcc");
}

#[test]
fn check_against_gnu_tr_tests_d() {
    // ['d', qw(-s '[b-c]'), {IN=>'aabbcc'}, {OUT=>'aabc'}],
    new_ucmd!()
        .args(&["-s", "[b-c]"])
        .pipe_in("aabbcc")
        .succeeds()
        .stdout_is("aabc");
}

#[test]
#[ignore = "I cannot tell if this means that tr preserve the octal representation?"]
fn check_against_gnu_tr_tests_e() {
    // ['e', qw(-s '[\0-\5]'), {IN=>"\0\0a\1\1b\2\2\2c\3\3\3d\4\4\4\4e\5\5"}, {OUT=>"\0a\1b\2c\3d\4e\5"}],
    new_ucmd!()
        .args(&["-s", r"[\0-\5]"])
        .pipe_in(
            "\u{0}\u{0}a\u{1}\u{1}b\u{2}\u{2}\u{2}c\u{3}\u{3}\u{3}d\u{4}\u{4}\u{4}\u{4}e\u{5}\u{5}",
        )
        .succeeds()
        .stdout_is("\u{0}a\u{1}b\u{2}c\u{3}d\u{4}e\u{5}");
}

#[test]
fn check_against_gnu_tr_tests_f() {
    // # tests of delete
    // ['f', qw(-d '[=[=]'), {IN=>'[[[[[[[]]]]]]]]'}, {OUT=>']]]]]]]]'}],
    new_ucmd!()
        .args(&["-d", "[=[=]"])
        .pipe_in("[[[[[[[]]]]]]]]")
        .succeeds()
        .stdout_is("]]]]]]]]");
}

#[test]
fn check_against_gnu_tr_tests_g() {
    // ['g', qw(-d '[=]=]'), {IN=>'[[[[[[[]]]]]]]]'}, {OUT=>'[[[[[[['}],
    new_ucmd!()
        .args(&["-d", "[=]=]"])
        .pipe_in("[[[[[[[]]]]]]]]")
        .succeeds()
        .stdout_is("[[[[[[[");
}

#[test]
fn check_against_gnu_tr_tests_h() {
    // ['h', qw(-d '[:xdigit:]'), {IN=>'0123456789acbdefABCDEF'}, {OUT=>''}],
    new_ucmd!()
        .args(&["-d", "[:xdigit:]"])
        .pipe_in("0123456789acbdefABCDEF")
        .succeeds()
        .stdout_is("");
}

#[test]
fn check_against_gnu_tr_tests_i() {
    // ['i', qw(-d '[:xdigit:]'), {IN=>'w0x1y2z3456789acbdefABCDEFz'}, {OUT=>'wxyzz'}],
    new_ucmd!()
        .args(&["-d", "[:xdigit:]"])
        .pipe_in("w0x1y2z3456789acbdefABCDEFz")
        .succeeds()
        .stdout_is("wxyzz");
}

#[test]
fn check_against_gnu_tr_tests_j() {
    // ['j', qw(-d '[:digit:]'), {IN=>'0123456789'}, {OUT=>''}],
    new_ucmd!()
        .args(&["-d", "[:digit:]"])
        .pipe_in("0123456789")
        .succeeds()
        .stdout_is("");
}

#[test]
fn check_against_gnu_tr_tests_k() {
    // ['k', qw(-d '[:digit:]'), {IN=>'a0b1c2d3e4f5g6h7i8j9k'}, {OUT=>'abcdefghijk'}],
    new_ucmd!()
        .args(&["-d", "[:digit:]"])
        .pipe_in("a0b1c2d3e4f5g6h7i8j9k")
        .succeeds()
        .stdout_is("abcdefghijk");
}

#[test]
fn check_against_gnu_tr_tests_l() {
    // ['l', qw(-d '[:lower:]'), {IN=>'abcdefghijklmnopqrstuvwxyz'}, {OUT=>''}],
    new_ucmd!()
        .args(&["-d", "[:lower:]"])
        .pipe_in("abcdefghijklmnopqrstuvwxyz")
        .succeeds()
        .stdout_is("");
}

#[test]
fn check_against_gnu_tr_tests_m() {
    // ['m', qw(-d '[:upper:]'), {IN=>'ABCDEFGHIJKLMNOPQRSTUVWXYZ'}, {OUT=>''}],
    new_ucmd!()
        .args(&["-d", "[:upper:]"])
        .pipe_in("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .succeeds()
        .stdout_is("");
}

#[test]
fn check_against_gnu_tr_tests_n() {
    // ['n', qw(-d '[:lower:][:upper:]'), {IN=>'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ'}, {OUT=>''}],
    new_ucmd!()
        .args(&["-d", "[:lower:][:upper:]"])
        .pipe_in("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .succeeds()
        .stdout_is("");
}

#[test]
fn check_against_gnu_tr_tests_o() {
    // ['o', qw(-d '[:alpha:]'), {IN=>'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ'}, {OUT=>''}],
    new_ucmd!()
        .args(&["-d", "[:alpha:]"])
        .pipe_in("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .succeeds()
        .stdout_is("");
}

#[test]
fn check_against_gnu_tr_tests_p() {
    // ['p', qw(-d '[:alnum:]'), {IN=>'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'}, {OUT=>''}],
    new_ucmd!()
        .args(&["-d", "[:alnum:]"])
        .pipe_in("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789")
        .succeeds()
        .stdout_is("");
}

#[test]
fn check_against_gnu_tr_tests_q() {
    // ['q', qw(-d '[:alnum:]'), {IN=>'.abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.'}, {OUT=>'..'}],
    new_ucmd!()
        .args(&["-d", "[:alnum:]"])
        .pipe_in(".abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.")
        .succeeds()
        .stdout_is("..");
}

#[test]
fn check_against_gnu_tr_tests_r() {
    // ['r', qw(-ds '[:alnum:]' .),
    //  {IN=>'.abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.'},
    //  {OUT=>'.'}],
    new_ucmd!()
        .args(&["-ds", "[:alnum:]", "."])
        .pipe_in(".abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.")
        .succeeds()
        .stdout_is(".");
}

#[test]
fn check_against_gnu_tr_tests_s() {
    // # The classic example, with string2 BSD-style
    // ['s', qw(-cs '[:alnum:]' '\n'),
    //  {IN=>'The big black fox jumped over the fence.'},
    //  {OUT=>"The\nbig\nblack\nfox\njumped\nover\nthe\nfence\n"}],
    new_ucmd!()
        .args(&["-cs", "[:alnum:]", "\n"])
        .pipe_in("The big black fox jumped over the fence.")
        .succeeds()
        .stdout_is("The\nbig\nblack\nfox\njumped\nover\nthe\nfence\n");
}

#[test]
fn check_against_gnu_tr_tests_t() {
    // # The classic example, POSIX-style
    // ['t', qw(-cs '[:alnum:]' '[\n*]'),
    //  {IN=>'The big black fox jumped over the fence.'},
    //  {OUT=>"The\nbig\nblack\nfox\njumped\nover\nthe\nfence\n"}],
    new_ucmd!()
        .args(&["-cs", "[:alnum:]", "[\n*]"])
        .pipe_in("The big black fox jumped over the fence.")
        .succeeds()
        .stdout_is("The\nbig\nblack\nfox\njumped\nover\nthe\nfence\n");
}

#[test]
fn check_against_gnu_tr_tests_u() {
    // ['u', qw(-ds b a), {IN=>'aabbaa'}, {OUT=>'a'}],
    new_ucmd!()
        .args(&["-ds", "b", "a"])
        .pipe_in("aabbaa")
        .succeeds()
        .stdout_is("a");
}

#[test]
fn check_against_gnu_tr_tests_v() {
    // ['v', qw(-ds '[:xdigit:]' Z), {IN=>'ZZ0123456789acbdefABCDEFZZ'}, {OUT=>'Z'}],
    new_ucmd!()
        .args(&["-ds", "[:xdigit:]", "Z"])
        .pipe_in("ZZ0123456789acbdefABCDEFZZ")
        .succeeds()
        .stdout_is("Z");
}

#[test]
fn check_against_gnu_tr_tests_w() {
    // # Try some data with 8th bit set in case something is mistakenly
    // # sign-extended.
    // ['w', qw(-ds '\350' '\345'),
    //  {IN=>"\300\301\377\345\345\350\345"},
    //  {OUT=>"\300\301\377\345"}],
    new_ucmd!()
        .args(&["-ds", "\u{350}", "\u{345}"])
        .pipe_in("\u{300}\u{301}\u{377}\u{345}\u{345}\u{350}\u{345}")
        .succeeds()
        .stdout_is("\u{300}\u{301}\u{377}\u{345}");
}

#[test]
fn check_against_gnu_tr_tests_x() {
    // ['x', qw(-s abcdefghijklmn '[:*016]'),
    //  {IN=>'abcdefghijklmnop'}, {OUT=>':op'}],
    new_ucmd!()
        .args(&["-s", "abcdefghijklmn", "[:*016]"])
        .pipe_in("abcdefghijklmnop")
        .succeeds()
        .stdout_is(":op");
}

#[test]
fn check_against_gnu_tr_tests_y() {
    // ['y', qw(-d a-z), {IN=>'abc $code'}, {OUT=>' $'}],
    new_ucmd!()
        .args(&["-d", "a-z"])
        .pipe_in("abc $code")
        .succeeds()
        .stdout_is(" $");
}

#[test]
fn check_against_gnu_tr_tests_z() {
    // ['z', qw(-ds a-z '$.'), {IN=>'a.b.c $$$$code\\'}, {OUT=>'. $\\'}],
    new_ucmd!()
        .args(&["-ds", "a-z", "$."])
        .pipe_in("a.b.c $$$$code\\")
        .succeeds()
        .stdout_is(". $\\");
}

#[test]
fn check_against_gnu_tr_tests_range_a_a() {
    // # Make sure that a-a is accepted.
    // ['range-a-a', qw(a-a z), {IN=>'abc'}, {OUT=>'zbc'}],
    new_ucmd!()
        .args(&["a-a", "z"])
        .pipe_in("abc")
        .succeeds()
        .stdout_is("zbc");
}

#[test]
fn check_against_gnu_tr_tests_null() {
    // ['null', qw(a ''), {IN=>''}, {OUT=>''}, {EXIT=>1},
    //  {ERR=>"$prog: when not truncating set1, string2 must be non-empty\n"}],
    new_ucmd!()
        .args(&["a", ""])
        .pipe_in("")
        .fails()
        .stderr_is("tr: when not truncating set1, string2 must be non-empty\n");
}

#[test]
fn check_against_gnu_tr_tests_upcase() {
    // ['upcase', qw('[:lower:]' '[:upper:]'),
    //  {IN=>'abcxyzABCXYZ'},
    //  {OUT=>'ABCXYZABCXYZ'}],
    new_ucmd!()
        .args(&["[:lower:]", "[:upper:]"])
        .pipe_in("abcxyzABCXYZ")
        .succeeds()
        .stdout_is("ABCXYZABCXYZ");
}

#[test]
fn check_against_gnu_tr_tests_dncase() {
    // ['dncase', qw('[:upper:]' '[:lower:]'),
    //  {IN=>'abcxyzABCXYZ'},
    //  {OUT=>'abcxyzabcxyz'}],
    new_ucmd!()
        .args(&["[:upper:]", "[:lower:]"])
        .pipe_in("abcxyzABCXYZ")
        .succeeds()
        .stdout_is("abcxyzabcxyz");
}

#[test]
fn check_against_gnu_tr_tests_rep_cclass() {
    // ['rep-cclass', qw('a[=*2][=c=]' xyyz), {IN=>'a=c'}, {OUT=>'xyz'}],
    new_ucmd!()
        .args(&["a[=*2][=c=]", "xyyz"])
        .pipe_in("a=c")
        .succeeds()
        .stdout_is("xyz");
}

#[test]
fn check_against_gnu_tr_tests_rep_1() {
    // ['rep-1', qw('[:*3][:digit:]' a-m), {IN=>':1239'}, {OUT=>'cefgm'}],
    new_ucmd!()
        .args(&["[:*3][:digit:]", "a-m"])
        .pipe_in(":1239")
        .succeeds()
        .stdout_is("cefgm");
}

#[test]
fn check_against_gnu_tr_tests_rep_2() {
    // ['rep-2', qw('a[b*512]c' '1[x*]2'), {IN=>'abc'}, {OUT=>'1x2'}],
    new_ucmd!()
        .args(&["a[b*512]c", "1[x*]2"])
        .pipe_in("abc")
        .succeeds()
        .stdout_is("1x2");
}

#[test]
fn check_against_gnu_tr_tests_rep_3() {
    // ['rep-3', qw('a[b*513]c' '1[x*]2'), {IN=>'abc'}, {OUT=>'1x2'}],
    new_ucmd!()
        .args(&["a[b*513]c", "1[x*]2"])
        .pipe_in("abc")
        .succeeds()
        .stdout_is("1x2");
}

#[test]
fn check_against_gnu_tr_tests_o_rep_1() {
    // # Another couple octal repeat count tests.
    // ['o-rep-1', qw('[b*08]' '[x*]'), {IN=>''}, {OUT=>''}, {EXIT=>1},
    //  {ERR=>"$prog: invalid repeat count '08' in [c*n] construct\n"}],
    new_ucmd!()
        .args(&["[b*08]", "[x*]"])
        .pipe_in("")
        .fails()
        .stderr_is("tr: invalid repeat count '08' in [c*n] construct\n");
}

#[test]
fn check_against_gnu_tr_tests_o_rep_2() {
    // ['o-rep-2', qw('[b*010]cd' '[a*7]BC[x*]'), {IN=>'bcd'}, {OUT=>'BCx'}],
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
fn check_against_gnu_tr_tests_esc() {
    // ['esc', qw('a\-z' A-Z), {IN=>'abc-z'}, {OUT=>'AbcBC'}],
    new_ucmd!()
        .args(&[r"a\-z", "A-Z"])
        .pipe_in("abc-z")
        .succeeds()
        .stdout_is("AbcBC");
}

#[test]
fn check_against_gnu_tr_tests_bs_055() {
    // ['bs-055', qw('a\055b' def), {IN=>"a\055b"}, {OUT=>'def'}],
    new_ucmd!()
        .args(&["a\u{055}b", "def"])
        .pipe_in("a\u{055}b")
        .succeeds()
        .stdout_is("def");
}

#[test]
#[ignore = "Failing in Windows because it will not separate '\' and 'x' as separate arguments"]
fn check_against_gnu_tr_tests_bs_at_end() {
    // ['bs-at-end', qw('\\' x), {IN=>"\\"}, {OUT=>'x'},
    //  {ERR=>"$prog: warning: an unescaped backslash at end of "
    //   . "string is not portable\n"}],
    new_ucmd!()
        .args(&[r"\", "x"])
        .pipe_in(r"\")
        .succeeds()
        .stdout_is("x")
        .stderr_is("tr: warning: an unescaped backslash at end of string is not portable");
}

#[test]
#[ignore = "not sure why GNU bails here. `[Y*]` should be able to generate all the mapping"]
fn check_against_gnu_tr_tests_ross_0a() {
    // # From Ross
    // ['ross-0a', qw(-cs '[:upper:]' 'X[Y*]'), {IN=>''}, {OUT=>''}, {EXIT=>1},
    //  {ERR=>$map_all_to_1}],
    new_ucmd!()
        .args(&["-cs", "[:upper:]", "X[Y*]"])
        .pipe_in("")
        .fails()
        .stderr_is("tr: when translating with complemented character classes,\nstring2 must map all characters in the domain to one");
}

#[test]
#[ignore = "not sure why GNU bails here. `[Y*]` should be able to generate all the mapping"]
fn check_against_gnu_tr_tests_ross_0b() {
    // ['ross-0b', qw(-cs '[:cntrl:]' 'X[Y*]'), {IN=>''}, {OUT=>''}, {EXIT=>1},
    //  {ERR=>$map_all_to_1}],
    new_ucmd!()
        .args(&["-cs", "[:cntrl:]", "X[Y*]"])
        .pipe_in("")
        .fails()
        .stderr_is("tr: when translating with complemented character classes,\nstring2 must map all characters in the domain to one");
}

#[test]
fn check_against_gnu_tr_tests_ross_1a() {
    // ['ross-1a', qw(-cs '[:upper:]' '[X*]'),
    //  {IN=>'AMZamz123.-+AMZ'}, {OUT=>'AMZXAMZ'}],
    new_ucmd!()
        .args(&["-cs", "[:upper:]", "[X*]"])
        .pipe_in("AMZamz123.-+AMZ")
        .succeeds()
        .stdout_is("AMZXAMZ");
}

#[test]
fn check_against_gnu_tr_tests_ross_1b() {
    // ['ross-1b', qw(-cs '[:upper:][:digit:]' '[Z*]'), {IN=>''}, {OUT=>''}],
    new_ucmd!()
        .args(&["-cs", "[:upper:][:digit:]", "[Z*]"])
        .pipe_in("")
        .succeeds()
        .stdout_is("");
}

#[test]
fn check_against_gnu_tr_tests_ross_2() {
    // ['ross-2', qw(-dcs '[:lower:]' n-rs-z),
    //  {IN=>'amzAMZ123.-+amz'}, {OUT=>'amzamz'}],
    new_ucmd!()
        .args(&["-dcs", "[:lower:]", "n-rs-z"])
        .pipe_in("amzAMZ123.-+amz")
        .succeeds()
        .stdout_is("amzamz");
}

#[test]
fn check_against_gnu_tr_tests_ross_3() {
    // ['ross-3', qw(-ds '[:xdigit:]' '[:alnum:]'),
    //  {IN=>'.ZABCDEFGzabcdefg.0123456788899.GG'}, {OUT=>'.ZGzg..G'}],
    new_ucmd!()
        .args(&["-ds", "[:xdigit:]", "[:alnum:]"])
        .pipe_in(".ZABCDEFGzabcdefg.0123456788899.GG")
        .succeeds()
        .stdout_is(".ZGzg..G");
}

#[test]
fn check_against_gnu_tr_tests_ross_4() {
    // ['ross-4', qw(-dcs '[:alnum:]' '[:digit:]'), {IN=>''}, {OUT=>''}],
    new_ucmd!()
        .args(&["-dcs", "[:alnum:]", "[:digit:]"])
        .pipe_in("")
        .succeeds()
        .stdout_is("");
}

#[test]
fn check_against_gnu_tr_tests_ross_5() {
    // ['ross-5', qw(-dc '[:lower:]'), {IN=>''}, {OUT=>''}],
    new_ucmd!()
        .args(&["-dc", "[:lower:]"])
        .pipe_in("")
        .succeeds()
        .stdout_is("");
}

#[test]
fn check_against_gnu_tr_tests_ross_6() {
    // ['ross-6', qw(-dc '[:upper:]'), {IN=>''}, {OUT=>''}],
    new_ucmd!()
        .args(&["-dc", "[:upper:]"])
        .pipe_in("")
        .succeeds()
        .stdout_is("");
}

#[test]
fn check_against_gnu_tr_tests_empty_eq() {
    // # Ensure that these fail.
    // # Prior to 2.0.20, each would evoke a failed assertion.
    // ['empty-eq', qw('[==]' x), {IN=>''}, {OUT=>''}, {EXIT=>1},
    //  {ERR=>"$prog: missing equivalence class character '[==]'\n"}],
    new_ucmd!()
        .args(&["[==]", "x"])
        .pipe_in("")
        .fails()
        .stderr_is("tr: missing equivalence class character '[==]'\n");
}

#[test]
fn check_against_gnu_tr_tests_empty_cc() {
    // ['empty-cc', qw('[::]' x), {IN=>''}, {OUT=>''}, {EXIT=>1},
    //  {ERR=>"$prog: missing character class name '[::]'\n"}],
    new_ucmd!()
        .args(&["[::]", "x"])
        .pipe_in("")
        .fails()
        .stderr_is("tr: missing character class name '[::]'\n");
}

#[test]
fn check_against_gnu_tr_tests_repeat_bs_9() {
    // # Weird repeat counts.
    // ['repeat-bs-9', qw(abc '[b*\9]'), {IN=>'abcd'}, {OUT=>'[b*d'}],
    new_ucmd!()
        .args(&["abc", r"[b*\9]"])
        .pipe_in("abcd")
        .succeeds()
        .stdout_is("[b*d");
}

#[test]
fn check_against_gnu_tr_tests_repeat_0() {
    // ['repeat-0', qw(abc '[b*0]'), {IN=>'abcd'}, {OUT=>'bbbd'}],
    new_ucmd!()
        .args(&["abc", "[b*0]"])
        .pipe_in("abcd")
        .succeeds()
        .stdout_is("bbbd");
}

#[test]
fn check_against_gnu_tr_tests_repeat_zeros() {
    // ['repeat-zeros', qw(abc '[b*00000000000000000000]'),
    //  {IN=>'abcd'}, {OUT=>'bbbd'}],
    new_ucmd!()
        .args(&["abc", "[b*00000000000000000000]"])
        .pipe_in("abcd")
        .succeeds()
        .stdout_is("bbbd");
}

#[test]
fn check_against_gnu_tr_tests_repeat_compl() {
    // ['repeat-compl', qw(-c '[a*65536]\n' '[b*]'), {IN=>'abcd'}, {OUT=>'abbb'}],
    new_ucmd!()
        .args(&["-c", "[a*65536]\n", "[b*]"])
        .pipe_in("abcd")
        .succeeds()
        .stdout_is("abbb");
}

#[test]
fn check_against_gnu_tr_tests_repeat_x_c() {
    // ['repeat-xC', qw(-C '[a*65536]\n' '[b*]'), {IN=>'abcd'}, {OUT=>'abbb'}],
    new_ucmd!()
        .args(&["-C", "[a*65536]\n", "[b*]"])
        .pipe_in("abcd")
        .succeeds()
        .stdout_is("abbb");
}

#[test]
#[ignore = "I think either clap-rs or uutils is parsing the '-H' as an argument..."]
fn check_against_gnu_tr_tests_fowler_1() {
    // # From Glenn Fowler.
    // ['fowler-1', qw(ah -H), {IN=>'aha'}, {OUT=>'-H-'}],
    new_ucmd!()
        .args(&["ah", "-H"])
        .pipe_in("aha")
        .succeeds()
        .stdout_is("-H-");
}

#[test]
fn check_against_gnu_tr_tests_no_abort_1() {
    // # Up to coreutils-6.9, this would provoke a failed assertion.
    // ['no-abort-1', qw(-c a '[b*256]'), {IN=>'abc'}, {OUT=>'abb'}],
    new_ucmd!()
        .args(&["-c", "a", "[b*256]"])
        .pipe_in("abc")
        .succeeds()
        .stdout_is("abb");
}
