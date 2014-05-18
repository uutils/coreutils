use std::io::process::Command;

fn run(input: &str, set1: &str, set2: &str) -> Vec<u8> {
    let mut process = Command::new("build/tr").arg(set1).arg(set2).spawn().unwrap();

    process.stdin.take_unwrap().write_str(input).unwrap();

    let po = match process.wait_with_output() {
        Ok(p) => p,
        Err(err) => fail!("{}", err),
    };
    po.output
}

#[test]
fn test_toupper() {
    let out = run("!abcd!", "a-z", "A-Z");
    assert_eq!(out.as_slice(), bytes!("!ABCD!"));
}

#[test]
fn test_small_set2() {
    let out = run("@0123456789", "0-9", "X");
    assert_eq!(out.as_slice(), bytes!("@XXXXXXXXXX"));
}

#[test]
fn test_unicode() {
    let out = run("(,°□°）, ┬─┬", ", ┬─┬", "╯︵┻━┻");
    assert_eq!(out.as_slice(), bytes!("(╯°□°）╯︵┻━┻"));
}


