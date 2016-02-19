#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "tr";



#[test]
fn test_toupper() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["a-z", "A-Z"]).run_piped_stdin("!abcd!");
    assert_eq!(result.stdout, "!ABCD!");
}

#[test]
fn test_small_set2() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["0-9", "X"]).run_piped_stdin("@0123456789");
    assert_eq!(result.stdout, "@XXXXXXXXXX");
}

#[test]
fn test_unicode() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&[", ┬─┬", "╯︵┻━┻"])
                     .run_piped_stdin("(,°□°）, ┬─┬".as_bytes());
    assert_eq!(result.stdout, "(╯°□°）╯︵┻━┻");
}

#[test]
fn test_delete() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-d", "a-z"]).run_piped_stdin("aBcD");
    assert_eq!(result.stdout, "BD");
}

#[test]
fn test_delete_complement() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-d", "-c", "a-z"]).run_piped_stdin("aBcD");
    assert_eq!(result.stdout, "ac");
}
