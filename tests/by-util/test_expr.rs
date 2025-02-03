// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore αbcdef ; (people) kkos

use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_no_arguments() {
    new_ucmd!()
        .fails()
        .code_is(2)
        .usage_error("missing operand");
}

#[test]
fn test_simple_values() {
    // null or 0 => EXIT_VALUE == 1
    new_ucmd!().args(&[""]).fails().code_is(1).stdout_only("\n");
    new_ucmd!()
        .args(&["0"])
        .fails()
        .code_is(1)
        .stdout_only("0\n");
    new_ucmd!()
        .args(&["00"])
        .fails()
        .code_is(1)
        .stdout_only("00\n");
    new_ucmd!()
        .args(&["-0"])
        .fails()
        .code_is(1)
        .stdout_only("-0\n");

    // non-null and non-0 => EXIT_VALUE = 0
    new_ucmd!().args(&["1"]).succeeds().stdout_only("1\n");
}

#[test]
fn test_simple_arithmetic() {
    new_ucmd!()
        .args(&["1", "+", "1"])
        .succeeds()
        .stdout_only("2\n");

    new_ucmd!()
        .args(&["1", "-", "1"])
        .fails()
        .code_is(1)
        .stdout_only("0\n");

    new_ucmd!()
        .args(&["3", "*", "2"])
        .succeeds()
        .stdout_only("6\n");

    new_ucmd!()
        .args(&["4", "/", "2"])
        .succeeds()
        .stdout_only("2\n");
}

#[test]
fn test_complex_arithmetic() {
    new_ucmd!()
        .args(&["9223372036854775807", "+", "9223372036854775807"])
        .succeeds()
        .stdout_only("18446744073709551614\n");

    new_ucmd!()
        .args(&[
            "92233720368547758076549841651981984981498415651",
            "%",
            "922337203685",
        ])
        .succeeds()
        .stdout_only("533691697086\n");

    new_ucmd!()
        .args(&[
            "92233720368547758076549841651981984981498415651",
            "*",
            "922337203685",
        ])
        .succeeds()
        .stdout_only("85070591730190566808700855121818604965830915152801178873935\n");

    new_ucmd!()
        .args(&[
            "92233720368547758076549841651981984981498415651",
            "-",
            "922337203685",
        ])
        .succeeds()
        .stdout_only("92233720368547758076549841651981984059161211966\n");

    new_ucmd!()
        .args(&["9", "/", "0"])
        .fails()
        .stderr_only("expr: division by zero\n");
}

#[test]
fn test_parenthesis() {
    new_ucmd!()
        .args(&["(", "1", "+", "1", ")", "*", "2"])
        .succeeds()
        .stdout_only("4\n");

    new_ucmd!()
        .args(&["1", "(", ")"])
        .fails()
        .code_is(2)
        .stderr_only("expr: syntax error: unexpected argument '('\n");
}

#[test]
fn test_or() {
    new_ucmd!()
        .args(&["0", "|", "foo"])
        .succeeds()
        .stdout_only("foo\n");

    new_ucmd!()
        .args(&["foo", "|", "bar"])
        .succeeds()
        .stdout_only("foo\n");

    new_ucmd!()
        .args(&["14", "|", "1"])
        .succeeds()
        .stdout_only("14\n");

    new_ucmd!()
        .args(&["-14", "|", "1"])
        .succeeds()
        .stdout_only("-14\n");

    new_ucmd!()
        .args(&["1", "|", "a", "/", "5"])
        .succeeds()
        .stdout_only("1\n");

    new_ucmd!()
        .args(&["foo", "|", "a", "/", "5"])
        .succeeds()
        .stdout_only("foo\n");

    new_ucmd!()
        .args(&["0", "|", "10", "/", "5"])
        .succeeds()
        .stdout_only("2\n");

    new_ucmd!()
        .args(&["12", "|", "9a", "+", "1"])
        .succeeds()
        .stdout_only("12\n");

    new_ucmd!().args(&["", "|", ""]).run().stdout_only("0\n");

    new_ucmd!().args(&["", "|", "0"]).run().stdout_only("0\n");

    new_ucmd!().args(&["", "|", "00"]).run().stdout_only("0\n");

    new_ucmd!().args(&["", "|", "-0"]).run().stdout_only("0\n");
}

#[test]
fn test_and() {
    new_ucmd!()
        .args(&["foo", "&", "1"])
        .succeeds()
        .stdout_only("foo\n");

    new_ucmd!()
        .args(&["14", "&", "1"])
        .succeeds()
        .stdout_only("14\n");

    new_ucmd!()
        .args(&["-14", "&", "1"])
        .succeeds()
        .stdout_only("-14\n");

    new_ucmd!()
        .args(&["-1", "&", "10", "/", "5"])
        .succeeds()
        .stdout_only("-1\n");

    new_ucmd!()
        .args(&["0", "&", "a", "/", "5"])
        .run()
        .stdout_only("0\n");

    new_ucmd!()
        .args(&["", "&", "a", "/", "5"])
        .run()
        .stdout_only("0\n");

    new_ucmd!().args(&["", "&", "1"]).run().stdout_only("0\n");

    new_ucmd!().args(&["", "&", ""]).run().stdout_only("0\n");
}

#[test]
fn test_index() {
    new_ucmd!()
        .args(&["index", "αbcdef", "x"])
        .fails()
        .code_is(1)
        .stdout_only("0\n");
    new_ucmd!()
        .args(&["index", "αbcdef", "α"])
        .succeeds()
        .stdout_only("1\n");
    new_ucmd!()
        .args(&["index", "αbc_δef", "δ"])
        .succeeds()
        .stdout_only("5\n");
    new_ucmd!()
        .args(&["index", "αbc_δef", "δf"])
        .succeeds()
        .stdout_only("5\n");
    new_ucmd!()
        .args(&["index", "αbcdef", "fb"])
        .succeeds()
        .stdout_only("2\n");
    new_ucmd!()
        .args(&["index", "αbcdef", "f"])
        .succeeds()
        .stdout_only("6\n");
    new_ucmd!()
        .args(&["index", "αbcdef_f", "f"])
        .succeeds()
        .stdout_only("6\n");

    new_ucmd!()
        .args(&["αbcdef", "index", "α"])
        .fails()
        .code_is(2)
        .stderr_only("expr: syntax error: unexpected argument 'index'\n");
}

#[test]
fn test_length_fail() {
    new_ucmd!().args(&["length", "αbcdef", "1"]).fails();
}

#[test]
fn test_length() {
    new_ucmd!()
        .args(&["length", "abcdef"])
        .succeeds()
        .stdout_only("6\n");

    new_ucmd!()
        .args(&["abcdef", "length"])
        .fails()
        .code_is(2)
        .stderr_only("expr: syntax error: unexpected argument 'length'\n");
}

#[test]
fn test_length_mb() {
    new_ucmd!()
        .args(&["length", "αbcdef"])
        .succeeds()
        .stdout_only("6\n");
}

#[test]
fn test_regex() {
    // FixME: [2022-12-19; rivy] test disabled as it currently fails due to 'oniguruma' bug (see GH:kkos/oniguruma/issues/279)
    // new_ucmd!()
    //     .args(&["a^b", ":", "a^b"])
    //     .succeeds()
    //     .stdout_only("3\n");
    new_ucmd!()
        .args(&["a^b", ":", "a\\^b"])
        .succeeds()
        .stdout_only("3\n");
    new_ucmd!()
        .args(&["a$b", ":", "a\\$b"])
        .succeeds()
        .stdout_only("3\n");
    new_ucmd!()
        .args(&["-5", ":", "-\\{0,1\\}[0-9]*$"])
        .succeeds()
        .stdout_only("2\n");
    new_ucmd!()
        .args(&["abc", ":", "bc"])
        .fails()
        .stdout_only("0\n");
}

#[test]
fn test_substr() {
    new_ucmd!()
        .args(&["substr", "abc", "1", "1"])
        .succeeds()
        .stdout_only("a\n");

    new_ucmd!()
        .args(&["abc", "substr", "1", "1"])
        .fails()
        .code_is(2)
        .stderr_only("expr: syntax error: unexpected argument 'substr'\n");
}

#[test]
fn test_invalid_substr() {
    new_ucmd!()
        .args(&["substr", "abc", "0", "1"])
        .fails()
        .code_is(1)
        .stdout_only("\n");

    new_ucmd!()
        .args(&["substr", "abc", &(usize::MAX.to_string() + "0"), "1"])
        .fails()
        .code_is(1)
        .stdout_only("\n");

    new_ucmd!()
        .args(&["substr", "abc", "0", &(usize::MAX.to_string() + "0")])
        .fails()
        .code_is(1)
        .stdout_only("\n");
}

#[test]
fn test_escape() {
    new_ucmd!().args(&["+", "1"]).succeeds().stdout_only("1\n");

    new_ucmd!()
        .args(&["1", "+", "+", "1"])
        .succeeds()
        .stdout_only("2\n");

    new_ucmd!()
        .args(&["2", "*", "+", "3"])
        .succeeds()
        .stdout_only("6\n");

    new_ucmd!()
        .args(&["(", "1", ")", "+", "1"])
        .succeeds()
        .stdout_only("2\n");
}

#[test]
fn test_invalid_syntax() {
    let invalid_syntaxes = [["12", "12"], ["12", "|"], ["|", "12"]];

    for invalid_syntax in invalid_syntaxes {
        new_ucmd!()
            .args(&invalid_syntax)
            .fails()
            .code_is(2)
            .stderr_contains("syntax error");
    }
}

#[test]
fn test_num_str_comparison() {
    new_ucmd!()
        .args(&["1a", "<", "1", "+", "1"])
        .succeeds()
        .stdout_is("1\n");
}
