#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "sort";


#[test]
fn test_numeric_floats_and_ints() {
    test_helper("numeric_floats_and_ints", &String::from("-n"));
}

#[test]
fn test_numeric_floats() {
    test_helper("numeric_floats", &String::from("-n"));
}

#[test]
fn test_numeric_unfixed_floats() {
    test_helper("numeric_unfixed_floats", &String::from("-n"));
}

#[test]
fn test_numeric_fixed_floats() {
    test_helper("numeric_fixed_floats", &String::from("-n"));
}

#[test]
fn test_numeric_unsorted_ints() {
    test_helper("numeric_unsorted_ints", &String::from("-n"));
}

#[test]
fn test_human_block_sizes() {
    test_helper("human_block_sizes", &String::from("-h"));
}

#[test]
fn test_month_default() {
    test_helper("month_default", &String::from("-M"));
}

fn test_helper(file_name: &str, args: &String) {
    let (at, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg(args);
    let out = ucmd.arg(format!("{}{}", file_name, ".txt")).run().stdout;

    let filename = format!("{}{}", file_name, ".expected");
    assert_eq!(out, at.read(&filename));
}
