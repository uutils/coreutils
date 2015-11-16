#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "sort";


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
    let (at, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg(args);
    let out = ucmd.arg(format!("{}{}", file_name, ".txt")).run().stdout;

    let filename = format!("{}{}", file_name, ".ans");
    assert_eq!(out, at.read(&filename));
}
