#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "tail";

static INPUT: &'static str = "foobar.txt";


#[test]
fn test_stdin_default() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.run_piped_stdin(at.read(INPUT));
    assert_eq!(result.stdout, at.read("foobar_stdin_default.expected"));
}

#[test]
fn test_single_default() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.arg(INPUT).run();
    assert_eq!(result.stdout, at.read("foobar_single_default.expected"));
}
