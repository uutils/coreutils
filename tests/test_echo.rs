use common::util::*;

static UTIL_NAME: &'static str = "echo";

fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_default() {
    assert_eq!(new_ucmd()
        .run().stdout, "\n");
}

#[test]
fn test_no_trailing_newline() {
    new_ucmd()
        .arg("-n")
        .arg("hello_world")
        .run()
        .stdout_is("hello_world");
}

#[test]
fn test_enable_escapes() {
    new_ucmd()
        .arg("-e")
        .arg("\\\\\\t\\r")
        .run()
        .stdout_is("\\\t\r\n");
}

#[test]
fn test_disable_escapes() {
    new_ucmd()
        .arg("-E")
        .arg("\\b\\c\\e")
        .run()
        .stdout_is("\\b\\c\\e\n");
}
