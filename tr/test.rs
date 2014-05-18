use std::io::process::Command;

fn run(input: &str, args: &[&'static str]) -> Vec<u8> {
    let mut process = Command::new("build/tr").args(args).spawn().unwrap();

    process.stdin.take_unwrap().write_str(input).unwrap();

    let po = match process.wait_with_output() {
        Ok(p) => p,
        Err(err) => fail!("{}", err),
    };
    po.output
}

#[test]
fn test_toupper() {
    let out = run("!abcd!", ["a-z", "A-Z"]);
    assert_eq!(out.as_slice(), bytes!("!ABCD!"));
}

#[test]
fn test_small_set2() {
    let out = run("@0123456789", ["0-9", "X"]);
    assert_eq!(out.as_slice(), bytes!("@XXXXXXXXXX"));
}

#[test]
fn test_unicode() {
    let out = run("(,°□°）, ┬─┬", [", ┬─┬", "╯︵┻━┻"]);
    assert_eq!(out.as_slice(), bytes!("(╯°□°）╯︵┻━┻"));
}

#[test]
fn test_delete() {
    let out = run("aBcD", ["-d", "a-z"]);
    assert_eq!(out.as_slice(), bytes!("BD"));
}

#[test]
fn test_delete_complement() {
    let out = run("aBcD", ["-d", "-c", "a-z"]);
    assert_eq!(out.as_slice(), bytes!("ac"));
}


