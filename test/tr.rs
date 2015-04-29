use std::io::Write;
use std::process::{Command, Stdio};

static PROGNAME: &'static str = "./tr";

fn run(input: &str, args: &[&'static str]) -> Vec<u8> {
    let mut process = Command::new(PROGNAME)
                                   .args(args)
                                   .stdin(Stdio::piped())
                                   .stdout(Stdio::piped())
                                   .spawn()
                                   .unwrap_or_else(|e| panic!("{}", e));

    process.stdin.take().unwrap_or_else(|| panic!("Could not take child process stdin"))
        .write_all(input.as_bytes()).unwrap_or_else(|e| panic!("{}", e));

    let po = process.wait_with_output().unwrap_or_else(|e| panic!("{}", e));
    po.stdout
}

#[test]
fn test_toupper() {
    let out = run("!abcd!", &["a-z", "A-Z"]);
    assert_eq!(&out[..], b"!ABCD!");
}

#[test]
fn test_small_set2() {
    let out = run("@0123456789", &["0-9", "X"]);
    assert_eq!(&out[..], b"@XXXXXXXXXX");
}

#[test]
fn test_unicode() {
    let out = run("(,°□°）, ┬─┬", &[", ┬─┬", "╯︵┻━┻"]);
    assert_eq!(&out[..], "(╯°□°）╯︵┻━┻".as_bytes());
}

#[test]
fn test_delete() {
    let out = run("aBcD", &["-d", "a-z"]);
    assert_eq!(&out[..], b"BD");
}

#[test]
fn test_delete_complement() {
    let out = run("aBcD", &["-d", "-c", "a-z"]);
    assert_eq!(&out[..], b"ac");
}


