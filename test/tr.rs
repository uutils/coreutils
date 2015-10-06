use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./tr";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_toupper() {
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.args(&["a-z", "A-Z"]), b"!abcd!");
    assert_eq!(result.stdout, "!ABCD!");
}

#[test]
fn test_small_set2() {
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.args(&["0-9", "X"]), b"@0123456789");
    assert_eq!(result.stdout, "@XXXXXXXXXX");
}

#[test]
fn test_unicode() {
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.args(&[", ┬─┬", "╯︵┻━┻"]),
                                 "(,°□°）, ┬─┬".as_bytes());
    assert_eq!(result.stdout, "(╯°□°）╯︵┻━┻");
}

#[test]
fn test_delete() {
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.args(&["-d", "a-z"]), b"aBcD");
    assert_eq!(result.stdout, "BD");
}

#[test]
fn test_delete_complement() {
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.args(&["-d", "-c", "a-z"]), b"aBcD");
    assert_eq!(result.stdout, "ac");
}
