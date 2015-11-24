#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "head";

static INPUT: &'static str = "lorem_ipsum.txt";


#[test]
fn test_stdin_default() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.run_piped_stdin(at.read(INPUT));
    assert_eq!(result.stdout, at.read("lorem_ipsum_default.expected"));
}

#[test]
fn test_stdin_1_line_obsolete() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-1"])
                     .run_piped_stdin(at.read(INPUT));
    assert_eq!(result.stdout, at.read("lorem_ipsum_1_line.expected"));
}

#[test]
fn test_stdin_1_line() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-n", "1"])
                     .run_piped_stdin(at.read(INPUT));
    assert_eq!(result.stdout, at.read("lorem_ipsum_1_line.expected"));
}

#[test]
fn test_stdin_5_chars() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-c", "5"])
                     .run_piped_stdin(at.read(INPUT));
    assert_eq!(result.stdout, at.read("lorem_ipsum_5_chars.expected"));
}

#[test]
fn test_single_default() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.arg(INPUT).run();
    assert_eq!(result.stdout, at.read("lorem_ipsum_default.expected"));
}

#[test]
fn test_single_1_line_obsolete() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-1", INPUT]).run();
    assert_eq!(result.stdout, at.read("lorem_ipsum_1_line.expected"));
}

#[test]
fn test_single_1_line() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-n", "1", INPUT]).run();
    assert_eq!(result.stdout, at.read("lorem_ipsum_1_line.expected"));
}

#[test]
fn test_single_5_chars() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-c", "5", INPUT]).run();
    assert_eq!(result.stdout, at.read("lorem_ipsum_5_chars.expected"));
}

#[test]
fn test_verbose() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-v", INPUT]).run();
    assert_eq!(result.stdout, at.read("lorem_ipsum_verbose.expected"));
}
