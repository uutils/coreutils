use std::io::Write;

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

#[test]
fn test_single_big_args() {
    const FILE: &'static str = "single_big_args.txt";
    const EXPECTED_FILE: &'static str = "single_big_args_expected.txt";
    const LINES: usize = 1_000_000;
    const N_ARG: usize = 100_000;

    let (at, mut ucmd) = testing(UTIL_NAME);

    let mut big_input = at.make_scoped_file(FILE);
    for i in 0..LINES {
        write!(&mut big_input, "Line {}\n", i).expect("Could not write to FILE");
    }
    big_input.flush().expect("Could not flush FILE");

    let mut big_expected = at.make_scoped_file(EXPECTED_FILE);
    for i in (LINES - N_ARG)..LINES {
        write!(&mut big_expected, "Line {}\n", i).expect("Could not write to EXPECTED_FILE");
    }
    big_expected.flush().expect("Could not flush EXPECTED_FILE");

    let result = ucmd.arg(FILE).arg("-n").arg(format!("{}", N_ARG)).run();
    assert_eq!(result.stdout, at.read(EXPECTED_FILE));
}

#[test]
fn test_bytes_single() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.arg("-c").arg("10").arg(INPUT).run();
    assert_eq!(result.stdout, at.read("foobar_bytes_single.expected"));
}

#[test]
fn test_bytes_stdin() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.arg("-c").arg("13").run_piped_stdin(at.read(INPUT));
    assert_eq!(result.stdout, at.read("foobar_bytes_stdin.expected"));
}
