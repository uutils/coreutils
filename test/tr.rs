use std::io::process::Command;

static PROGNAME: &'static str = "./tr";

fn run(input: &str, args: &[&'static str]) -> Vec<u8> {
    let mut process = Command::new(PROGNAME).args(args).spawn().unwrap();

    process.stdin.take().unwrap().write_str(input).unwrap();

    let po = match process.wait_with_output() {
        Ok(p) => p,
        Err(err) => panic!("{}", err),
    };
    po.output
}

#[test]
fn test_toupper() {
    let out = run("!abcd!", &["a-z", "A-Z"]);
    assert_eq!(out.as_slice(), b"!ABCD!");
}

#[test]
fn test_small_set2() {
    let out = run("@0123456789", &["0-9", "X"]);
    assert_eq!(out.as_slice(), b"@XXXXXXXXXX");
}

#[test]
fn test_unicode() {
    let out = run("(,°□°）, ┬─┬", &[", ┬─┬", "╯︵┻━┻"]);
    assert_eq!(out.as_slice(), "(╯°□°）╯︵┻━┻".as_bytes());
}

#[test]
fn test_delete() {
    let out = run("aBcD", &["-d", "a-z"]);
    assert_eq!(out.as_slice(), b"BD");
}

#[test]
fn test_delete_complement() {
    let out = run("aBcD", &["-d", "-c", "a-z"]);
    assert_eq!(out.as_slice(), b"ac");
}


