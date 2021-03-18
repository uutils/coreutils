use crate::common::util::*;

#[test]
fn test_toupper() {
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
        .stdout_is("xyyde");
}

#[test]
fn test_set1_shorter_than_set2() {
    new_ucmd!()
        .args(&["ab", "xyz"])
        .pipe_in("abcde")
        .run()
        .stdout_is("xycde");
}

#[test]
fn test_truncate() {
    new_ucmd!()
        .args(&["-t", "abc", "xy"])
        .pipe_in("abcde")
        .run()
        .stdout_is("xycde");
}

#[test]
fn test_truncate_with_set1_shorter_than_set2() {
    new_ucmd!()
        .args(&["-t", "ab", "xyz"])
        .pipe_in("abcde")
        .run()
        .stdout_is("xycde");
}

#[test]
fn missing_args_fails() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.run();

    assert!(!result.success);
    assert!(result.stderr.contains("missing operand"));
}

#[test]
fn missing_required_second_arg_fails() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.args(&["foo"]).run();

    assert!(!result.success);
    assert!(result.stderr.contains("missing operand after"));
}

#[test]
fn test_interpret_backslash_escapes() {
    new_ucmd!()
        .args(&["abfnrtv", r"\a\b\f\n\r\t\v"])
        .pipe_in("abfnrtv")
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
