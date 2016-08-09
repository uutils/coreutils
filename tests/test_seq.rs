use common::util::*;

static UTIL_NAME: &'static str = "seq";
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_count_up() {
    let out = new_ucmd()
        .args(&["10"]).run().stdout;
    assert_eq!(out, "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");
}

#[test]
fn test_count_down() {
    let out = new_ucmd()
        .args(&["--", "5", "-1", "1"]).run().stdout;
    assert_eq!(out, "5\n4\n3\n2\n1\n");
}

#[test]
fn test_separator_and_terminator() {
    let out = new_ucmd()
        .args(&["-s", ",", "-t", "!", "2", "6"]).run().stdout;
    assert_eq!(out, "2,3,4,5,6!");
}

#[test]
fn test_equalize_widths() {
    let out = new_ucmd()
        .args(&["-w", "5", "10"]).run().stdout;
    assert_eq!(out, "05\n06\n07\n08\n09\n10\n");
}
