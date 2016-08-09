use common::util::*;

static UTIL_NAME: &'static str = "tr";

fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_toupper() {
    let result = new_ucmd()
        .args(&["a-z", "A-Z"]).run_piped_stdin("!abcd!");
    assert_eq!(result.stdout, "!ABCD!");
}

#[test]
fn test_small_set2() {
    let result = new_ucmd()
        .args(&["0-9", "X"]).run_piped_stdin("@0123456789");
    assert_eq!(result.stdout, "@XXXXXXXXXX");
}

#[test]
fn test_unicode() {
    let result = new_ucmd()
        .args(&[", ┬─┬", "╯︵┻━┻"])
                     .run_piped_stdin("(,°□°）, ┬─┬".as_bytes());
    assert_eq!(result.stdout, "(╯°□°）╯︵┻━┻");
}

#[test]
fn test_delete() {
    let result = new_ucmd()
        .args(&["-d", "a-z"]).run_piped_stdin("aBcD");
    assert_eq!(result.stdout, "BD");
}

#[test]
fn test_delete_complement() {
    let result = new_ucmd()
        .args(&["-d", "-c", "a-z"]).run_piped_stdin("aBcD");
    assert_eq!(result.stdout, "ac");
}
