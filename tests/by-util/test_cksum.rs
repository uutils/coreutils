// spell-checker:ignore (words) asdf

use crate::common::util::*;

#[test]
fn test_single_file() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_is_fixture("single_file.expected");
}

#[test]
fn test_multiple_files() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .arg("alice_in_wonderland.txt")
        .succeeds()
        .stdout_is_fixture("multiple_files.expected");
}

#[test]
fn test_stdin() {
    new_ucmd!()
        .pipe_in_fixture("lorem_ipsum.txt")
        .succeeds()
        .stdout_is_fixture("stdin.expected");
}

#[test]
fn test_empty() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("a");

    ucmd.arg("a")
        .succeeds()
        .no_stderr()
        .normalized_newlines_stdout_is("4294967295 0 a\n");
}

#[test]
fn test_arg_overrides_stdin() {
    let (at, mut ucmd) = at_and_ucmd!();
    let input = "foobarfoobar"; // spell-checker:disable-line

    at.touch("a");

    ucmd.arg("a")
        .pipe_in(input.as_bytes())
        // the command might have exited before all bytes have been pipe in.
        // in that case, we don't care about the error (broken pipe)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stderr()
        .normalized_newlines_stdout_is("4294967295 0 a\n");
}

#[test]
fn test_invalid_file() {
    let ts = TestScenario::new(util_name!());
    let at = ts.fixtures.clone();

    let folder_name = "asdf";

    // First check when file doesn't exist
    ts.ucmd()
        .arg(folder_name)
        .fails()
        .no_stdout()
        .stderr_contains("cksum: asdf: No such file or directory");

    // Then check when the file is of an invalid type
    at.mkdir(folder_name);
    ts.ucmd()
        .arg(folder_name)
        .succeeds()
        .stdout_only("4294967295 0 asdf\n");
}

// Make sure crc is correct for files larger than 32 bytes
// but <128 bytes (1 fold pclmul) // spell-checker:disable-line
#[test]
fn test_crc_for_bigger_than_32_bytes() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("chars.txt").succeeds();

    let mut stdout_split = result.stdout_str().split(' ');

    let cksum: i64 = stdout_split.next().unwrap().parse().unwrap();
    let bytes_cnt: i64 = stdout_split.next().unwrap().parse().unwrap();

    assert_eq!(cksum, 586_047_089);
    assert_eq!(bytes_cnt, 16);
}

#[test]
fn test_stdin_larger_than_128_bytes() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("larger_than_2056_bytes.txt").succeeds();

    let mut stdout_split = result.stdout_str().split(' ');

    let cksum: i64 = stdout_split.next().unwrap().parse().unwrap();
    let bytes_cnt: i64 = stdout_split.next().unwrap().parse().unwrap();

    assert_eq!(cksum, 945_881_979);
    assert_eq!(bytes_cnt, 2058);
}
