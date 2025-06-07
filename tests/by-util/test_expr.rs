// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore αbcdef ; (people) kkos
// spell-checker:ignore aabcccd aabcd aabd abbb abbbd abbcabc abbcac abbcbbbd abbcbd
// spell-checker:ignore abbccd abcabc abcac acabc andand bigcmp bignum emptysub
// spell-checker:ignore orempty oror

use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_no_arguments() {
    new_ucmd!()
        .fails_with_code(2)
        .usage_error("missing operand");
}

#[test]
fn test_simple_values() {
    // null or 0 => EXIT_VALUE == 1
    new_ucmd!().args(&[""]).fails_with_code(1).stdout_only("\n");
    new_ucmd!()
        .args(&["0"])
        .fails_with_code(1)
        .stdout_only("0\n");
    new_ucmd!()
        .args(&["00"])
        .fails_with_code(1)
        .stdout_only("00\n");
    new_ucmd!()
        .args(&["-0"])
        .fails_with_code(1)
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
        .fails_with_code(1)
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
        .fails_with_code(2)
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

    new_ucmd!().args(&["", "|", ""]).fails().stdout_only("0\n");

    new_ucmd!().args(&["", "|", "0"]).fails().stdout_only("0\n");

    new_ucmd!()
        .args(&["", "|", "00"])
        .fails()
        .stdout_only("0\n");

    new_ucmd!()
        .args(&["", "|", "-0"])
        .fails()
        .stdout_only("0\n");
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
        .fails()
        .stdout_only("0\n");

    new_ucmd!()
        .args(&["", "&", "a", "/", "5"])
        .fails()
        .stdout_only("0\n");

    new_ucmd!().args(&["", "&", "1"]).fails().stdout_only("0\n");

    new_ucmd!().args(&["", "&", ""]).fails().stdout_only("0\n");
}

#[test]
fn test_index() {
    new_ucmd!()
        .args(&["index", "αbcdef", "x"])
        .fails_with_code(1)
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
        .fails_with_code(2)
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
        .fails_with_code(2)
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
fn test_regex_empty() {
    new_ucmd!().args(&["", ":", ""]).fails().stdout_only("0\n");
    new_ucmd!()
        .args(&["abc", ":", ""])
        .fails()
        .stdout_only("0\n");
}

#[test]
fn test_regex_trailing_backslash() {
    new_ucmd!()
        .args(&["\\", ":", "\\\\"])
        .succeeds()
        .stdout_only("1\n");
    new_ucmd!()
        .args(&["\\", ":", "\\"])
        .fails()
        .stderr_only("expr: Trailing backslash\n");
    new_ucmd!()
        .args(&["abc\\", ":", "abc\\\\"])
        .succeeds()
        .stdout_only("4\n");
    new_ucmd!()
        .args(&["abc\\", ":", "abc\\"])
        .fails()
        .stderr_only("expr: Trailing backslash\n");
}

#[test]
fn test_regex_caret() {
    new_ucmd!()
        .args(&["a^b", ":", "a^b"])
        .succeeds()
        .stdout_only("3\n");
    new_ucmd!()
        .args(&["a^b", ":", "a\\^b"])
        .succeeds()
        .stdout_only("3\n");
    new_ucmd!()
        .args(&["abc", ":", "^abc"])
        .succeeds()
        .stdout_only("3\n");
    new_ucmd!()
        .args(&["^abc", ":", "^^abc"])
        .succeeds()
        .stdout_only("4\n");
    new_ucmd!()
        .args(&["b", ":", "a\\|^b"])
        .succeeds()
        .stdout_only("1\n");
    new_ucmd!()
        .args(&["ab", ":", "\\(^a\\)b"])
        .succeeds()
        .stdout_only("a\n");
    new_ucmd!()
        .args(&["^abc", ":", "^abc"])
        .fails()
        .stdout_only("0\n");
    new_ucmd!()
        .args(&["^^^^^^^^^", ":", "^^^"])
        .succeeds()
        .stdout_only("2\n");
    new_ucmd!()
        .args(&["ab[^c]", ":", "ab[^c]"])
        .succeeds()
        .stdout_only("3\n"); // Matches "ab["
    new_ucmd!()
        .args(&["ab[^c]", ":", "ab\\[^c]"])
        .succeeds()
        .stdout_only("6\n");
    new_ucmd!()
        .args(&["[^a]", ":", "\\[^a]"])
        .succeeds()
        .stdout_only("4\n");
    new_ucmd!()
        .args(&["\\a", ":", "\\\\[^^]"])
        .succeeds()
        .stdout_only("2\n");
    // Patterns are anchored to the beginning of the pattern "^bc"
    new_ucmd!()
        .args(&["abc", ":", "bc"])
        .fails()
        .stdout_only("0\n");
    new_ucmd!()
        .args(&["^a", ":", "^^[^^]"])
        .succeeds()
        .stdout_only("2\n");
    new_ucmd!()
        .args(&["abc", ":", "ab[^c]"])
        .fails()
        .stdout_only("0\n");
}

#[test]
fn test_regex_dollar() {
    new_ucmd!()
        .args(&["a$b", ":", "a\\$b"])
        .succeeds()
        .stdout_only("3\n");
    new_ucmd!()
        .args(&["a", ":", "a$\\|b"])
        .succeeds()
        .stdout_only("1\n");
    new_ucmd!()
        .args(&["ab", ":", "a\\(b$\\)"])
        .succeeds()
        .stdout_only("b\n");
    new_ucmd!()
        .args(&["a$c", ":", "a$\\c"])
        .succeeds()
        .stdout_only("3\n");
    new_ucmd!()
        .args(&["$a", ":", "$a"])
        .succeeds()
        .stdout_only("2\n");
    new_ucmd!()
        .args(&["a", ":", "a$\\|b"])
        .succeeds()
        .stdout_only("1\n");
    new_ucmd!()
        .args(&["-5", ":", "-\\{0,1\\}[0-9]*$"])
        .succeeds()
        .stdout_only("2\n");
    new_ucmd!()
        .args(&["$", ":", "$"])
        .fails()
        .stdout_only("0\n");
    new_ucmd!()
        .args(&["a$", ":", "a$\\|b"])
        .fails()
        .stdout_only("0\n");
}

#[test]
fn test_regex_range_quantifier() {
    new_ucmd!()
        .args(&["a", ":", "a\\{1\\}"])
        .succeeds()
        .stdout_only("1\n");
    new_ucmd!()
        .args(&["aaaaaaaaaa", ":", "a\\{1,\\}"])
        .succeeds()
        .stdout_only("10\n");
    new_ucmd!()
        .args(&["aaa", ":", "a\\{,3\\}"])
        .succeeds()
        .stdout_only("3\n");
    new_ucmd!()
        .args(&["aa", ":", "a\\{1,3\\}"])
        .succeeds()
        .stdout_only("2\n");
    new_ucmd!()
        .args(&["aaaa", ":", "a\\{,\\}"])
        .succeeds()
        .stdout_only("4\n");
    new_ucmd!()
        .args(&["a", ":", "ab\\{,3\\}"])
        .succeeds()
        .stdout_only("1\n");
    new_ucmd!()
        .args(&["abbb", ":", "ab\\{,3\\}"])
        .succeeds()
        .stdout_only("4\n");
    new_ucmd!()
        .args(&["abcabc", ":", "\\(abc\\)\\{,\\}"])
        .succeeds()
        .stdout_only("abc\n");
    new_ucmd!()
        .args(&["a", ":", "a\\{,6\\}"])
        .succeeds()
        .stdout_only("1\n");
    new_ucmd!()
        .args(&["{abc}", ":", "\\{abc\\}"])
        .succeeds()
        .stdout_only("5\n");
    new_ucmd!()
        .args(&["a{bc}", ":", "a\\(\\{bc\\}\\)"])
        .succeeds()
        .stdout_only("{bc}\n");
    new_ucmd!()
        .args(&["{b}", ":", "a\\|\\{b\\}"])
        .succeeds()
        .stdout_only("3\n");
    new_ucmd!()
        .args(&["{", ":", "a\\|\\{"])
        .succeeds()
        .stdout_only("1\n");
    new_ucmd!()
        .args(&["{}}}", ":", "\\{\\}\\}\\}"])
        .succeeds()
        .stdout_only("4\n");
    new_ucmd!()
        .args(&["a{}}}", ":", "a\\{\\}\\}\\}"])
        .fails()
        .stderr_only("expr: Invalid content of \\{\\}\n");
    new_ucmd!()
        .args(&["ab", ":", "ab\\{\\}"])
        .fails()
        .stderr_only("expr: Invalid content of \\{\\}\n");
    new_ucmd!()
        .args(&["_", ":", "a\\{12345678901234567890\\}"])
        .fails()
        .stderr_only("expr: Regular expression too big\n");
    new_ucmd!()
        .args(&["_", ":", "a\\{12345678901234567890,\\}"])
        .fails()
        .stderr_only("expr: Regular expression too big\n");
    new_ucmd!()
        .args(&["_", ":", "a\\{,12345678901234567890\\}"])
        .fails()
        .stderr_only("expr: Regular expression too big\n");
    new_ucmd!()
        .args(&["_", ":", "a\\{1,12345678901234567890\\}"])
        .fails()
        .stderr_only("expr: Regular expression too big\n");
    new_ucmd!()
        .args(&["_", ":", "a\\{1,1234567890abcdef\\}"])
        .fails()
        .stderr_only("expr: Invalid content of \\{\\}\n");
}

#[test]
fn test_substr() {
    new_ucmd!()
        .args(&["substr", "abc", "1", "1"])
        .succeeds()
        .stdout_only("a\n");

    new_ucmd!()
        .args(&["abc", "substr", "1", "1"])
        .fails_with_code(2)
        .stderr_only("expr: syntax error: unexpected argument 'substr'\n");
}

#[test]
fn test_invalid_substr() {
    new_ucmd!()
        .args(&["substr", "abc", "0", "1"])
        .fails_with_code(1)
        .stdout_only("\n");

    new_ucmd!()
        .args(&["substr", "abc", &(usize::MAX.to_string() + "0"), "1"])
        .fails_with_code(1)
        .stdout_only("\n");

    new_ucmd!()
        .args(&["substr", "abc", "0", &(usize::MAX.to_string() + "0")])
        .fails_with_code(1)
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
            .fails_with_code(2)
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

#[test]
fn test_eager_evaluation() {
    new_ucmd!()
        .args(&["(", "1", "/", "0"])
        .fails()
        .stderr_contains("division by zero");
}

#[test]
fn test_long_input() {
    // Giving expr an arbitrary long expression should succeed rather than end with a segfault due to a stack overflow.
    #[cfg(not(windows))]
    const MAX_NUMBER: usize = 40000;
    #[cfg(not(windows))]
    const RESULT: &str = "800020000\n";

    // On windows there is 8192 characters input limit
    #[cfg(windows)]
    const MAX_NUMBER: usize = 1300; // 7993 characters (with spaces)
    #[cfg(windows)]
    const RESULT: &str = "845650\n";

    let mut args: Vec<String> = vec!["1".to_string()];

    for i in 2..=MAX_NUMBER {
        args.push('+'.to_string());
        args.push(i.to_string());
    }

    new_ucmd!().args(&args).succeeds().stdout_is(RESULT);
}

/// Regroup the testcases of the GNU test expr.pl
mod gnu_expr {
    use uutests::new_ucmd;
    use uutests::util::TestScenario;
    use uutests::util_name;

    #[test]
    fn test_a() {
        new_ucmd!()
            .args(&["5", "+", "6"])
            .succeeds()
            .stdout_only("11\n");
    }

    #[test]
    fn test_b() {
        new_ucmd!()
            .args(&["5", "-", "6"])
            .succeeds()
            .stdout_only("-1\n");
    }

    #[test]
    fn test_c() {
        new_ucmd!()
            .args(&["5", "*", "6"])
            .succeeds()
            .stdout_only("30\n");
    }

    #[test]
    fn test_d() {
        new_ucmd!()
            .args(&["100", "/", "6"])
            .succeeds()
            .stdout_only("16\n");
    }

    #[test]
    fn test_e() {
        new_ucmd!()
            .args(&["100", "%", "6"])
            .succeeds()
            .stdout_only("4\n");
    }

    #[test]
    fn test_f() {
        new_ucmd!()
            .args(&["3", "+", "-2"])
            .succeeds()
            .stdout_only("1\n");
    }

    #[test]
    fn test_g() {
        new_ucmd!()
            .args(&["-2", "+", "-2"])
            .succeeds()
            .stdout_only("-4\n");
    }

    #[test]
    fn test_opt1() {
        new_ucmd!()
            .args(&["--", "-11", "+", "12"])
            .succeeds()
            .stdout_only("1\n");
    }

    #[test]
    fn test_opt2() {
        new_ucmd!()
            .args(&["-11", "+", "12"])
            .succeeds()
            .stdout_only("1\n");
    }

    #[test]
    fn test_opt3() {
        new_ucmd!()
            .args(&["--", "-1", "+", "2"])
            .succeeds()
            .stdout_only("1\n");
    }

    #[test]
    fn test_opt4() {
        new_ucmd!()
            .args(&["-1", "+", "2"])
            .succeeds()
            .stdout_only("1\n");
    }

    #[test]
    fn test_opt5() {
        new_ucmd!()
            .args(&["--", "2", "+", "2"])
            .succeeds()
            .stdout_only("4\n");
    }

    #[test]
    fn test_paren1() {
        new_ucmd!()
            .args(&["(", "100", "%", "6", ")"])
            .succeeds()
            .stdout_only("4\n");
    }

    #[test]
    fn test_paren2() {
        new_ucmd!()
            .args(&["(", "100", "%", "6", ")", "-", "8"])
            .succeeds()
            .stdout_only("-4\n");
    }

    #[test]
    fn test_paren3() {
        new_ucmd!()
            .args(&["9", "/", "(", "100", "%", "6", ")", "-", "8"])
            .succeeds()
            .stdout_only("-6\n");
    }

    #[test]
    fn test_paren4() {
        new_ucmd!()
            .args(&["9", "/", "(", "(", "100", "%", "6", ")", "-", "8", ")"])
            .succeeds()
            .stdout_only("-2\n");
    }

    #[test]
    fn test_paren5() {
        new_ucmd!()
            .args(&["9", "+", "(", "100", "%", "6", ")"])
            .succeeds()
            .stdout_only("13\n");
    }

    #[test]
    fn test_0bang() {
        new_ucmd!()
            .args(&["00", "<", "0!"])
            .fails()
            .code_is(1)
            .stdout_only("0\n");
    }

    #[test]
    fn test_00() {
        new_ucmd!()
            .args(&["00"])
            .fails()
            .code_is(1)
            .stdout_only("00\n");
    }

    #[test]
    fn test_minus0() {
        new_ucmd!()
            .args(&["-0"])
            .fails()
            .code_is(1)
            .stdout_only("-0\n");
    }

    #[test]
    fn test_andand() {
        new_ucmd!()
            .args(&["0", "&", "1", "/", "0"])
            .fails()
            .code_is(1)
            .stdout_only("0\n");
    }

    #[test]
    fn test_oror() {
        new_ucmd!()
            .args(&["1", "|", "1", "/", "0"])
            .succeeds()
            .stdout_only("1\n");
    }

    #[test]
    fn test_orempty() {
        new_ucmd!()
            .args(&["", "|", ""])
            .fails()
            .code_is(1)
            .stdout_only("0\n");
    }

    #[test]
    fn test_fail_a() {
        new_ucmd!()
            .args(&["3", "+", "-"])
            .fails()
            .code_is(2)
            .no_stdout()
            .stderr_contains("non-integer argument");
    }

    #[test]
    fn test_bigcmp() {
        new_ucmd!()
            .args(&[
                "--",
                "-2417851639229258349412352",
                "<",
                "2417851639229258349412352",
            ])
            .succeeds()
            .stdout_only("1\n");
    }

    #[test]
    fn test_anchor() {
        new_ucmd!()
            .args(&["a\nb", ":", "a$"])
            .fails()
            .code_is(1)
            .stdout_only("0\n");
    }

    #[test]
    fn test_emptysub() {
        new_ucmd!()
            .args(&["a", ":", "\\(b\\)*"])
            .fails()
            .code_is(1)
            .stdout_only("\n");
    }

    #[test]
    fn test_bre1() {
        new_ucmd!()
            .args(&["abc", ":", "a\\(b\\)c"])
            .succeeds()
            .stdout_only("b\n");
    }

    #[test]
    fn test_bre2() {
        new_ucmd!()
            .args(&["a(", ":", "a("])
            .succeeds()
            .stdout_only("2\n");
    }

    #[test]
    fn test_bre3() {
        new_ucmd!()
            .args(&["_", ":", "a\\("])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("Unmatched ( or \\(");
    }

    #[test]
    fn test_bre4() {
        new_ucmd!()
            .args(&["_", ":", "a\\(b"])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("Unmatched ( or \\(");
    }

    #[test]
    fn test_bre5() {
        new_ucmd!()
            .args(&["a(b", ":", "a(b"])
            .succeeds()
            .stdout_only("3\n");
    }

    #[test]
    fn test_bre6() {
        new_ucmd!()
            .args(&["a)", ":", "a)"])
            .succeeds()
            .stdout_only("2\n");
    }

    #[test]
    fn test_bre7() {
        new_ucmd!()
            .args(&["_", ":", "a\\)"])
            .fails_with_code(2)
            .stderr_contains("Unmatched ) or \\)");
    }

    #[test]
    fn test_bre8() {
        new_ucmd!()
            .args(&["_", ":", "\\)"])
            .fails_with_code(2)
            .stderr_contains("Unmatched ) or \\)");
    }

    #[test]
    fn test_bre9() {
        new_ucmd!()
            .args(&["ab", ":", "a\\(\\)b"])
            .fails_with_code(1)
            .stdout_only("\n");
    }

    #[test]
    fn test_bre10() {
        new_ucmd!()
            .args(&["a^b", ":", "a^b"])
            .succeeds()
            .stdout_only("3\n");
    }

    #[test]
    fn test_bre11() {
        new_ucmd!()
            .args(&["a$b", ":", "a$b"])
            .succeeds()
            .stdout_only("3\n");
    }

    #[test]
    fn test_bre12() {
        new_ucmd!()
            .args(&["", ":", "\\($\\)\\(^\\)"])
            .fails_with_code(1)
            .stdout_only("\n");
    }

    #[test]
    fn test_bre13() {
        new_ucmd!()
            .args(&["b", ":", "a*\\(b$\\)c*"])
            .succeeds()
            .stdout_only("b\n");
    }

    #[test]
    fn test_bre14() {
        new_ucmd!()
            .args(&["X|", ":", "X\\(|\\)", ":", "(", "X|", ":", "X\\(|\\)", ")"])
            .succeeds()
            .stdout_only("1\n");
    }

    #[test]
    fn test_bre15() {
        new_ucmd!()
            .args(&["X*", ":", "X\\(*\\)", ":", "(", "X*", ":", "X\\(*\\)", ")"])
            .succeeds()
            .stdout_only("1\n");
    }

    #[test]
    fn test_bre16() {
        new_ucmd!()
            .args(&["abc", ":", "\\(\\)"])
            .fails_with_code(1)
            .stdout_only("\n");
    }

    #[test]
    fn test_bre17() {
        new_ucmd!()
            .args(&["{1}a", ":", "\\(\\{1\\}a\\)"])
            .succeeds()
            .stdout_only("{1}a\n");
    }

    #[test]
    fn test_bre18() {
        new_ucmd!()
            .args(&["X*", ":", "X\\(*\\)", ":", "^*"])
            .succeeds()
            .stdout_only("1\n");
    }

    #[test]
    fn test_bre19() {
        new_ucmd!()
            .args(&["{1}", ":", "\\{1\\}"])
            .succeeds()
            .stdout_only("3\n");
    }

    #[test]
    fn test_bre20() {
        new_ucmd!()
            .args(&["{", ":", "{"])
            .succeeds()
            .stdout_only("1\n");
    }

    #[test]
    fn test_bre21() {
        new_ucmd!()
            .args(&["abbcbd", ":", "a\\(b*\\)c\\1d"])
            .fails_with_code(1)
            .stdout_only("\n");
    }

    #[test]
    fn test_bre22() {
        new_ucmd!()
            .args(&["abbcbbbd", ":", "a\\(b*\\)c\\1d"])
            .fails_with_code(1)
            .stdout_only("\n");
    }

    #[test]
    fn test_bre23() {
        new_ucmd!()
            .args(&["abc", ":", "\\(.\\)\\1"])
            .fails_with_code(1)
            .stdout_only("\n");
    }

    #[test]
    fn test_bre24() {
        new_ucmd!()
            .args(&["abbccd", ":", "a\\(\\([bc]\\)\\2\\)*d"])
            .succeeds()
            .stdout_only("cc\n");
    }

    #[test]
    fn test_bre25() {
        new_ucmd!()
            .args(&["abbcbd", ":", "a\\(\\([bc]\\)\\2\\)*d"])
            .fails_with_code(1)
            .stdout_only("\n");
    }

    #[test]
    fn test_bre26() {
        new_ucmd!()
            .args(&["abbbd", ":", "a\\(\\(b\\)*\\2\\)*d"])
            .succeeds()
            .stdout_only("bbb\n");
    }

    #[test]
    fn test_bre27() {
        new_ucmd!()
            .args(&["aabcd", ":", "\\(a\\)\\1bcd"])
            .succeeds()
            .stdout_only("a\n");
    }

    #[test]
    fn test_bre28() {
        new_ucmd!()
            .args(&["aabcd", ":", "\\(a\\)\\1bc*d"])
            .succeeds()
            .stdout_only("a\n");
    }

    #[test]
    fn test_bre29() {
        new_ucmd!()
            .args(&["aabd", ":", "\\(a\\)\\1bc*d"])
            .succeeds()
            .stdout_only("a\n");
    }

    #[test]
    fn test_bre30() {
        new_ucmd!()
            .args(&["aabcccd", ":", "\\(a\\)\\1bc*d"])
            .succeeds()
            .stdout_only("a\n");
    }

    #[test]
    fn test_bre31() {
        new_ucmd!()
            .args(&["aabcccd", ":", "\\(a\\)\\1bc*[ce]d"])
            .succeeds()
            .stdout_only("a\n");
    }

    #[test]
    fn test_bre32() {
        new_ucmd!()
            .args(&["aabcccd", ":", "\\(a\\)\\1b\\(c\\)*cd"])
            .succeeds()
            .stdout_only("a\n");
    }

    #[test]
    fn test_bre33() {
        new_ucmd!()
            .args(&["a*b", ":", "a\\(*\\)b"])
            .succeeds()
            .stdout_only("*\n");
    }

    #[test]
    fn test_bre34() {
        new_ucmd!()
            .args(&["ab", ":", "a\\(**\\)b"])
            .fails_with_code(1)
            .stdout_only("\n");
    }

    #[test]
    fn test_bre35() {
        new_ucmd!()
            .args(&["ab", ":", "a\\(***\\)b"])
            .fails_with_code(1)
            .stdout_only("\n");
    }

    #[test]
    fn test_bre36() {
        new_ucmd!()
            .args(&["*a", ":", "*a"])
            .succeeds()
            .stdout_only("2\n");
    }

    #[test]
    fn test_bre37() {
        new_ucmd!()
            .args(&["a", ":", "**a"])
            .succeeds()
            .stdout_only("1\n");
    }

    #[test]
    fn test_bre38() {
        new_ucmd!()
            .args(&["a", ":", "***a"])
            .succeeds()
            .stdout_only("1\n");
    }

    #[test]
    fn test_bre39() {
        new_ucmd!()
            .args(&["ab", ":", "a\\{1\\}b"])
            .succeeds()
            .stdout_only("2\n");
    }

    #[test]
    fn test_bre40() {
        new_ucmd!()
            .args(&["ab", ":", "a\\{1,\\}b"])
            .succeeds()
            .stdout_only("2\n");
    }

    #[test]
    fn test_bre41() {
        new_ucmd!()
            .args(&["aab", ":", "a\\{1,2\\}b"])
            .succeeds()
            .stdout_only("3\n");
    }

    #[test]
    fn test_bre42() {
        new_ucmd!()
            .args(&["_", ":", "a\\{1"])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("Unmatched \\{");
    }

    #[test]
    fn test_bre43() {
        new_ucmd!()
            .args(&["_", ":", "a\\{1a"])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("Unmatched \\{");
    }

    #[test]
    fn test_bre44() {
        new_ucmd!()
            .args(&["_", ":", "a\\{1a\\}"])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("Invalid content of \\{\\}");
    }

    #[test]
    fn test_bre45() {
        new_ucmd!()
            .args(&["a", ":", "a\\{,2\\}"])
            .succeeds()
            .stdout_only("1\n");
    }

    #[test]
    fn test_bre46() {
        new_ucmd!()
            .args(&["a", ":", "a\\{,\\}"])
            .succeeds()
            .stdout_only("1\n");
    }

    #[test]
    fn test_bre47() {
        new_ucmd!()
            .args(&["_", ":", "a\\{1,x\\}"])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("Invalid content of \\{\\}");
    }

    #[test]
    fn test_bre48() {
        new_ucmd!()
            .args(&["_", ":", "a\\{1,x"])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("Unmatched \\{");
    }

    #[test]
    fn test_bre49() {
        new_ucmd!()
            .args(&["_", ":", "a\\{32768\\}"])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("Regular expression too big\n");
    }

    #[test]
    fn test_bre50() {
        new_ucmd!()
            .args(&["_", ":", "a\\{1,0\\}"])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("Invalid content of \\{\\}");
    }

    #[test]
    fn test_bre51() {
        new_ucmd!()
            .args(&["acabc", ":", ".*ab\\{0,0\\}c"])
            .succeeds()
            .stdout_only("2\n");
    }

    #[test]
    fn test_bre52() {
        new_ucmd!()
            .args(&["abcac", ":", "ab\\{0,1\\}c"])
            .succeeds()
            .stdout_only("3\n");
    }

    #[test]
    fn test_bre53() {
        new_ucmd!()
            .args(&["abbcac", ":", "ab\\{0,3\\}c"])
            .succeeds()
            .stdout_only("4\n");
    }

    #[test]
    fn test_bre54() {
        new_ucmd!()
            .args(&["abcac", ":", ".*ab\\{1,1\\}c"])
            .succeeds()
            .stdout_only("3\n");
    }

    #[test]
    fn test_bre55() {
        new_ucmd!()
            .args(&["abcac", ":", ".*ab\\{1,3\\}c"])
            .succeeds()
            .stdout_only("3\n");
    }

    #[test]
    fn test_bre56() {
        new_ucmd!()
            .args(&["abbcabc", ":", ".*ab\\{2,2\\}c"])
            .succeeds()
            .stdout_only("4\n");
    }

    #[test]
    fn test_bre57() {
        new_ucmd!()
            .args(&["abbcabc", ":", ".*ab\\{2,4\\}c"])
            .succeeds()
            .stdout_only("4\n");
    }

    #[test]
    fn test_bre58() {
        new_ucmd!()
            .args(&["aa", ":", "a\\{1\\}\\{1\\}"])
            .succeeds()
            .stdout_only("1\n");
    }

    #[test]
    fn test_bre59() {
        new_ucmd!()
            .args(&["aa", ":", "a*\\{1\\}"])
            .succeeds()
            .stdout_only("2\n");
    }

    #[test]
    fn test_bre60() {
        new_ucmd!()
            .args(&["aa", ":", "a\\{1\\}*"])
            .succeeds()
            .stdout_only("2\n");
    }

    #[test]
    fn test_bre61() {
        new_ucmd!()
            .args(&["acd", ":", "a\\(b\\)?c\\1d"])
            .fails_with_code(1)
            .stdout_only("\n");
    }

    #[test]
    fn test_bre62() {
        new_ucmd!()
            .args(&["--", "-5", ":", "-\\{0,1\\}[0-9]*$"])
            .succeeds()
            .stdout_only("2\n");
    }

    #[test]
    fn test_fail_c() {
        new_ucmd!()
            .args::<&str>(&[])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("missing operand")
            .stderr_contains("Try")
            .stderr_contains("for more information");
    }

    const BIG: &str = "98782897298723498732987928734";
    const BIG_P1: &str = "98782897298723498732987928735";
    const BIG_SUM: &str = "197565794597446997465975857469";
    const BIG_PROD: &str = "9758060798730154302876482828124348356960410232492450771490";

    #[test]
    fn test_bignum_add() {
        new_ucmd!()
            .args(&[BIG, "+", "1"])
            .succeeds()
            .stdout_only(format!("{BIG_P1}\n"));
    }

    #[test]
    fn test_bignum_add1() {
        new_ucmd!()
            .args(&[BIG, "+", BIG_P1])
            .succeeds()
            .stdout_only(format!("{BIG_SUM}\n"));
    }

    #[test]
    fn test_bignum_sub() {
        new_ucmd!()
            .args(&[BIG_P1, "-", "1"])
            .succeeds()
            .stdout_only(format!("{BIG}\n"));
    }

    #[test]
    fn test_bignum_sub1() {
        new_ucmd!()
            .args(&[BIG_SUM, "-", BIG])
            .succeeds()
            .stdout_only(format!("{BIG_P1}\n"));
    }

    #[test]
    fn test_bignum_mul() {
        new_ucmd!()
            .args(&[BIG_P1, "*", BIG])
            .succeeds()
            .stdout_only(format!("{BIG_PROD}\n"));
    }

    #[test]
    fn test_bignum_div() {
        new_ucmd!()
            .args(&[BIG_PROD, "/", BIG])
            .succeeds()
            .stdout_only(format!("{BIG_P1}\n"));
    }

    #[test]
    fn test_se0() {
        new_ucmd!()
            .args(&["9", "9"])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("syntax error: unexpected argument '9'");
    }

    #[test]
    fn test_se1() {
        new_ucmd!()
            .args(&["2", "a"])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("syntax error: unexpected argument 'a'");
    }

    #[test]
    fn test_se2() {
        new_ucmd!()
            .args(&["2", "+"])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("syntax error: missing argument after '+'");
    }

    #[test]
    fn test_se3() {
        new_ucmd!()
            .args(&["2", ":"])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("syntax error: missing argument after ':'");
    }

    #[test]
    fn test_se4() {
        new_ucmd!()
            .args(&["length"])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("syntax error: missing argument after 'length'");
    }

    #[test]
    fn test_se5() {
        new_ucmd!()
            .args(&["(", "2"])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("syntax error: expecting ')' after '2'");
    }

    #[test]
    fn test_se6() {
        new_ucmd!()
            .args(&["(", "2", "a"])
            .fails_with_code(2)
            .no_stdout()
            .stderr_contains("syntax error: expecting ')' instead of 'a'");
    }
}
