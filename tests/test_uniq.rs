use common::util::*;


static INPUT: &'static str = "sorted.txt";
static SKIP_CHARS: &'static str = "skip-chars.txt";
static SKIP_FIELDS: &'static str = "skip-fields.txt";
static SORTED_ZERO_TERMINATED: &'static str = "sorted-zero-terminated.txt";

#[test]
fn test_stdin_default() {
    new_ucmd!()
        .pipe_in_fixture(INPUT)
        .run().stdout_is_fixture("sorted-simple.expected");
}

#[test]
fn test_single_default() {
    new_ucmd!()
        .arg(INPUT)
        .run().stdout_is_fixture("sorted-simple.expected");
}

#[test]
fn test_stdin_counts() {
    new_ucmd!()
        .args(&["-c"]).pipe_in_fixture(INPUT)
        .run().stdout_is_fixture("sorted-counts.expected");
}

#[test]
fn test_stdin_skip_1_char() {
    new_ucmd!()
        .args(&["-s1"]).pipe_in_fixture(SKIP_CHARS)
        .run().stdout_is_fixture("skip-1-char.expected");
}

#[test]
fn test_stdin_skip_5_chars() {
    new_ucmd!()
        .args(&["-s5"]).pipe_in_fixture(SKIP_CHARS)
        .run().stdout_is_fixture("skip-5-chars.expected");
}

#[test]
fn test_stdin_skip_and_check_2_chars() {
    new_ucmd!()
        .args(&["-s3", "-w2"]).pipe_in_fixture(SKIP_CHARS)
        .run().stdout_is_fixture("skip-3-check-2-chars.expected");
}

#[test]
fn test_stdin_skip_1_field() {
    new_ucmd!()
        .args(&["-f2"]).pipe_in_fixture(SKIP_FIELDS)
        .run().stdout_is_fixture("skip-2-fields.expected");
}

#[test]
fn test_stdin_all_repeated() {
    new_ucmd!()
        .args(&["--all-repeated"]).pipe_in_fixture(INPUT)
        .run().stdout_is_fixture("sorted-all-repeated.expected");
}

#[test]
fn test_stdin_all_repeated_separate() {
    new_ucmd!()
        .args(&["--all-repeated", "separate"]).pipe_in_fixture(INPUT)
        .run().stdout_is_fixture("sorted-all-repeated-separate.expected");
}

#[test]
fn test_stdin_all_repeated_prepend() {
    new_ucmd!()
        .args(&["--all-repeated", "prepend"]).pipe_in_fixture(INPUT)
        .run().stdout_is_fixture("sorted-all-repeated-prepend.expected");
}

#[test]
fn test_stdin_unique_only() {
    new_ucmd!()
        .args(&["-u"]).pipe_in_fixture(INPUT)
        .run().stdout_is_fixture("sorted-unique-only.expected");
}

#[test]
fn test_stdin_repeated_only() {
    new_ucmd!()
        .args(&["-d"]).pipe_in_fixture(INPUT)
        .run().stdout_is_fixture("sorted-repeated-only.expected");
}

#[test]
fn test_stdin_ignore_case() {
    new_ucmd!()
        .args(&["-i"]).pipe_in_fixture(INPUT)
        .run().stdout_is_fixture("sorted-ignore-case.expected");
}

#[test]
fn test_stdin_zero_terminated() {
    new_ucmd!()
        .args(&["-z"]).pipe_in_fixture(SORTED_ZERO_TERMINATED)
        .run().stdout_is_fixture("sorted-zero-terminated.expected");
}
