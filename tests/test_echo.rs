use common::util::*;


#[test]
fn test_default() {
    //CmdResult.stdout_only(...) trims trailing newlines
    assert_eq!("hi\n", new_ucmd!().arg("hi").succeeds().no_stderr().stdout);
}

#[test]
fn test_no_trailing_newline() {
    //CmdResult.stdout_only(...) trims trailing newlines
    assert_eq!("hi", new_ucmd!().arg("-n").arg("hi").succeeds().no_stderr().stdout);
}

#[test]
fn test_escape_alert() {
    new_ucmd!().args(&["-e", "\\a"]).succeeds().stdout_only("\x07\n");
}

#[test]
fn test_escape_backslash() {
    new_ucmd!().args(&["-e", "\\\\"]).succeeds().stdout_only("\\\n");
}

#[test]
fn test_escape_backspace() {
    new_ucmd!().args(&["-e", "\\b"]).succeeds().stdout_only("\x08\n");
}

#[test]
fn test_escape_carriage_return() {
    new_ucmd!().args(&["-e", "\\r"]).succeeds().stdout_only("\r\n");
}

#[test]
fn test_escape_escape() {
    new_ucmd!().args(&["-e", "\\e"]).succeeds().stdout_only("\x1B\n");
}

#[test]
fn test_escape_form_feed() {
    new_ucmd!().args(&["-e", "\\f"]).succeeds().stdout_only("\x0C\n");
}

#[test]
fn test_escape_hex() {
    new_ucmd!().args(&["-e", "\\x41"]).succeeds().stdout_only("A");
}

#[test]
fn test_escape_newline() {
    new_ucmd!().args(&["-e", "\\na"]).succeeds().stdout_only("\na");
}

#[test]
fn test_escape_no_further_output() {
    new_ucmd!().args(&["-e", "a\\cb"]).succeeds().stdout_only("a\n");
}

#[test]
fn test_escape_octal() {
    new_ucmd!().args(&["-e", "\\0100"]).succeeds().stdout_only("@");
}

#[test]
fn test_escape_tab() {
    new_ucmd!().args(&["-e", "\\t"]).succeeds().stdout_only("\t\n");
}

#[test]
fn test_escape_vertical_tab() {
    new_ucmd!().args(&["-e", "\\v"]).succeeds().stdout_only("\x0B\n");
}

#[test]
fn test_disable_escapes() {
    let input_str = "\\a \\\\ \\b \\r \\e \\f \\x41 \\n a\\cb \\u0100 \\t \\v";
    new_ucmd!()
        .arg("-E")
        .arg(input_str)
        .succeeds()
        .stdout_only(format!("{}\n", input_str));
}
