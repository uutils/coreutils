#![allow(unstable)]

use std::old_io::process::Command;
use std::old_io::File;
use std::string::String;

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

fn numeric_helper(test_num: isize) {
    let mut cmd = Command::new(PROGNAME);
    cmd.arg("-n");
    let po = match cmd.clone().arg(format!("{}{}{}", "numeric", test_num, ".txt")).output() {
        Ok(p) => p,
        Err(err) => panic!("{}", err),
    };

    let answer = match File::open(&Path::new(format!("{}{}{}", "numeric", test_num, ".ans")))
            .read_to_end() {
        Ok(answer) => answer,
        Err(err) => panic!("{}", err),
    };
    assert_eq!(String::from_utf8(po.output).unwrap(), String::from_utf8(answer).unwrap());
}
