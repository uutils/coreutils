use std::io::Write;

#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "tail";

static INPUT: &'static str = "foobar.txt";

static BIG: &'static str = "big.txt";

static BIG_EXPECTED: &'static str = "big_single_big_args.expected";

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

const BIG_LINES: usize = 1_000_000;
const BIG_N_ARG: usize = 100_000;

fn generate_big_test_files(at: &AtPath) {
    let mut big_input = at.make_file(BIG);
    for i in 0..BIG_LINES {
        write!(&mut big_input, "Line {}\n", i).expect("Could not write to BIG file");
    }
    big_input.flush().expect("Could not flush BIG file");

    let mut big_expected = at.make_file(BIG_EXPECTED);
    for i in (BIG_LINES - BIG_N_ARG)..BIG_LINES {
        write!(&mut big_expected, "Line {}\n", i).expect("Could not write to BIG_EXPECTED file");
    }
    big_expected.flush().expect("Could not flush BIG_EXPECTED file");
}

fn cleanup_big_test_files(at: &AtPath) {
    at.cleanup(BIG);
    at.cleanup(BIG_EXPECTED);
}

#[test]
fn test_single_big_args() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    generate_big_test_files(&at);
    let result = ucmd.arg(BIG).arg("-n").arg(format!("{}", BIG_N_ARG)).run();
    assert_eq!(result.stdout, at.read(BIG_EXPECTED));
    cleanup_big_test_files(&at);
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
