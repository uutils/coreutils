use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;

static PROGNAME: &'static str = "./sort";

#[test]
fn numeric1() {
    numeric_helper(1);
}

#[test]
fn numeric2() {
    numeric_helper(2);
}

#[test]
fn numeric3() {
    numeric_helper(3);
}

#[test]
fn numeric4() {
    numeric_helper(4);
}

#[test]
fn numeric5() {
    numeric_helper(5);
}

#[test]
fn numeric6() {
    numeric_helper(6);
}

#[test]
fn human1() {
    test_helper(&String::from("human1"), &String::from("-H"));
}

fn numeric_helper(test_num: isize) {
    test_helper(&format!("numeric{}", test_num), &String::from("-n"))
}

fn test_helper(file_name: &String, args: &String) {
    let mut cmd = Command::new(PROGNAME);
    cmd.arg(args);
    let po = match cmd.arg(format!("{}{}", file_name, ".txt")).output() {
        Ok(p) => p,
        Err(err) => panic!("{}", err)
    };

    let filename = format!("{}{}", file_name, ".ans");
    let mut f = File::open(Path::new(&filename)).unwrap_or_else(|err| {
        panic!("{}", err)
    });
    let mut answer = vec!();
    match f.read_to_end(&mut answer) {
        Ok(_) => {},
        Err(err) => panic!("{}", err)
    }
    assert_eq!(String::from_utf8(po.stdout).unwrap(), String::from_utf8(answer).unwrap());
}
