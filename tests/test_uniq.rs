use common::util::*;

static UTIL_NAME: &'static str = "uniq";

static INPUT: &'static str = "sorted.txt";
static SKIP_CHARS: &'static str = "skip-chars.txt";
static SKIP_FIELDS: &'static str = "skip-fields.txt";

#[test]
fn test_stdin_default() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.run_piped_stdin(at.read(INPUT));
    assert_eq!(result.stdout, at.read("sorted-simple.expected"));
}

#[test]
fn test_single_default() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.arg(INPUT).run();
    assert_eq!(result.stdout, at.read("sorted-simple.expected"));
}

#[test]
fn test_stdin_counts() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-c"]).run_piped_stdin(at.read(INPUT));
    assert_eq!(result.stdout, at.read("sorted-counts.expected"));
}

#[test]
fn test_stdin_skip_1_char() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-s1"]).run_piped_stdin(at.read(SKIP_CHARS));
    assert_eq!(result.stdout, at.read("skip-1-char.expected"));
}

#[test]
fn test_stdin_skip_5_chars() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-s5"]).run_piped_stdin(at.read(SKIP_CHARS));
    assert_eq!(result.stdout, at.read("skip-5-chars.expected"));
}

#[test]
fn test_stdin_skip_and_check_2_chars() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-s3", "-w2"]).run_piped_stdin(at.read(SKIP_CHARS));
    assert_eq!(result.stdout, at.read("skip-3-check-2-chars.expected"));
}

#[test]
fn test_stdin_skip_1_field() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-f2"]).run_piped_stdin(at.read(SKIP_FIELDS));
    assert_eq!(result.stdout, at.read("skip-2-fields.expected"));
}

#[test]
fn test_stdin_all_repeated() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["--all-repeated"]).run_piped_stdin(at.read(INPUT));
    assert_eq!(result.stdout, at.read("sorted-all-repeated.expected"));
}

#[test]
fn test_stdin_all_repeated_separate() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["--all-repeated", "separate"]).run_piped_stdin(at.read(INPUT));
    assert_eq!(result.stdout, at.read("sorted-all-repeated-separate.expected"));
}

#[test]
fn test_stdin_all_repeated_prepend() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["--all-repeated", "prepend"]).run_piped_stdin(at.read(INPUT));
    assert_eq!(result.stdout, at.read("sorted-all-repeated-prepend.expected"));
}

#[test]
fn test_stdin_unique_only() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-u"]).run_piped_stdin(at.read(INPUT));
    assert_eq!(result.stdout, at.read("sorted-unique-only.expected"));
}

#[test]
fn test_stdin_repeated_only() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-d"]).run_piped_stdin(at.read(INPUT));
    assert_eq!(result.stdout, at.read("sorted-repeated-only.expected"));
}
