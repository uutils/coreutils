#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "dircolors";

#[test]
fn test1() {
    test_helper("test1", "gnome");
}

#[test]
fn test_keywords() {
    test_helper("keywords", "");
}

#[test]
fn test_internal_db() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-p");
    let out = ucmd.run().stdout;
    let filename = "internal.expected";
    assert_eq!(out, at.read(filename));
}

#[test]
fn test_bash_default() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-b");
    let out = ucmd.env("TERM", "screen").run().stdout;
    let filename = "bash_def.expected";
    assert_eq!(out, at.read(filename));
}

#[test]
fn test_csh_default() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-c");
    let out = ucmd.env("TERM", "screen").run().stdout;
    let filename = "csh_def.expected";
    assert_eq!(out, at.read(filename));
}

#[test]
fn test_no_env() {
    // no SHELL and TERM
    let (_, mut ucmd) = testing(UTIL_NAME);
    ucmd.fails();
}

#[test]
fn test_exclusive_option() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-cp");
    ucmd.fails();
}

fn test_helper(file_name: &str, term: &str) {
    let (at, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-c").env("TERM", term);
    let out = ucmd.arg(format!("{}.txt", file_name)).run().stdout;
    let filename = format!("{}.csh.expected", file_name);
    assert_eq!(out, at.read(&filename));

    let (at, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-b").env("TERM", term);
    let out = ucmd.arg(format!("{}.txt", file_name)).run().stdout;
    let filename = format!("{}.sh.expected", file_name);
    assert_eq!(out, at.read(&filename));
}
