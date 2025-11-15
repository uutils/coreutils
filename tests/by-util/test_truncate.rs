// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (words) RFILE

use std::io::{Seek, SeekFrom, Write};
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

static FILE1: &str = "truncate_test_1";
static FILE2: &str = "truncate_test_2";

#[test]
fn test_increase_file_size() {
    let expected = 5 * 1024;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(FILE1);
    ucmd.args(&["-s", "+5K", FILE1]).succeeds();

    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.stream_position().unwrap();
    assert_eq!(expected, actual, "expected '{expected}' got '{actual}'");
}

#[test]
fn test_increase_file_size_kb() {
    let expected = 5 * 1000;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(FILE1);
    ucmd.args(&["-s", "+5KB", FILE1]).succeeds();

    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.stream_position().unwrap();
    assert_eq!(expected, actual, "expected '{expected}' got '{actual}'");
}

#[test]
fn test_reference() {
    let expected = 5 * 1000;
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let mut file = at.make_file(FILE2);

    // manpage: "A FILE argument that does not exist is created."
    scene.ucmd().arg("-s").arg("+5KB").arg(FILE1).succeeds();

    scene
        .ucmd()
        .arg("--reference")
        .arg(FILE1)
        .arg(FILE2)
        .succeeds();

    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.stream_position().unwrap();
    assert_eq!(expected, actual, "expected '{expected}' got '{actual}'");
}

#[test]
fn test_decrease_file_size() {
    let expected = 6;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(FILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size=-4", FILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.stream_position().unwrap();
    assert_eq!(expected, actual, "expected '{expected}' got '{actual}'");
}

#[test]
fn test_space_in_size() {
    let expected = 4;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(FILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size", " 4", FILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.stream_position().unwrap();
    assert_eq!(expected, actual, "expected '{expected}' got '{actual}'");
}

#[test]
fn test_failed() {
    new_ucmd!().fails();
}

#[test]
fn test_failed_2() {
    let (_at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&[FILE1]).fails();
}

#[test]
fn test_failed_incorrect_arg() {
    let (_at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-s", "+5A", FILE1]).fails();
}

#[test]
fn test_at_most_shrinks() {
    let expected = 4;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(FILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size", "<4", FILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.stream_position().unwrap();
    assert_eq!(expected, actual, "expected '{expected}' got '{actual}'");
}

#[test]
fn test_at_most_no_change() {
    let expected = 10;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(FILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size", "<40", FILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.stream_position().unwrap();
    assert_eq!(expected, actual, "expected '{expected}' got '{actual}'");
}

#[test]
fn test_at_least_grows() {
    let expected = 15;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(FILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size", ">15", FILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.stream_position().unwrap();
    assert_eq!(expected, actual, "expected '{expected}' got '{actual}'");
}

#[test]
fn test_at_least_no_change() {
    let expected = 10;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(FILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size", ">4", FILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.stream_position().unwrap();
    assert_eq!(expected, actual, "expected '{expected}' got '{actual}'");
}

#[test]
fn test_round_down() {
    let expected = 8;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(FILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size", "/4", FILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.stream_position().unwrap();
    assert_eq!(expected, actual, "expected '{expected}' got '{actual}'");
}

#[test]
fn test_round_up() {
    let expected = 12;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(FILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size", "%4", FILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.stream_position().unwrap();
    assert_eq!(expected, actual, "expected '{expected}' got '{actual}'");
}

#[test]
fn test_size_and_reference() {
    let expected = 15;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file1 = at.make_file(FILE1);
    let mut file2 = at.make_file(FILE2);
    file1.write_all(b"1234567890").unwrap();
    ucmd.args(&["--reference", FILE1, "--size", "+5", FILE2])
        .succeeds();
    file2.seek(SeekFrom::End(0)).unwrap();
    let actual = file2.stream_position().unwrap();
    assert_eq!(expected, actual, "expected '{expected}' got '{actual}'");
}

#[test]
fn test_error_filename_only() {
    // truncate: you must specify either '--size' or '--reference'
    new_ucmd!()
        .args(&["file"])
        .fails_with_code(1)
        .stderr_contains("error: the following required arguments were not provided:");
}

#[test]
fn test_invalid_option() {
    // truncate: cli parsing error returns 1
    new_ucmd!()
        .args(&["--this-arg-does-not-exist"])
        .fails_with_code(1);
}

#[test]
fn test_invalid_numbers() {
    new_ucmd!()
        .args(&["-s", "0X", "file"])
        .fails()
        .stderr_contains("Invalid number: '0X'");
    new_ucmd!()
        .args(&["-s", "0XB", "file"])
        .fails()
        .stderr_contains("Invalid number: '0XB'");
    new_ucmd!()
        .args(&["-s", "0B", "file"])
        .fails()
        .stderr_contains("Invalid number: '0B'");
}

#[test]
fn test_reference_file_not_found() {
    new_ucmd!()
        .args(&["-r", "a", "b"])
        .fails()
        .stderr_contains("cannot stat 'a': No such file or directory");
}

#[test]
fn test_reference_with_size_file_not_found() {
    new_ucmd!()
        .args(&["-r", "a", "-s", "+1", "b"])
        .fails()
        .stderr_contains("cannot stat 'a': No such file or directory");
}

#[test]
fn test_truncate_bytes_size() {
    new_ucmd!()
        .args(&["--no-create", "--size", "K", "file"])
        .succeeds();
    new_ucmd!()
        .args(&["--size", "1024R", "file"])
        .fails_with_code(1)
        .stderr_only("truncate: Invalid number: '1024R': Value too large for defined data type\n");
    new_ucmd!()
        .args(&["--size", "1Y", "file"])
        .fails_with_code(1)
        .stderr_only("truncate: Invalid number: '1Y': Value too large for defined data type\n");
}

/// Test that truncating a non-existent file creates that file.
#[test]
fn test_new_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let filename = "new_file_that_does_not_exist_yet";
    ucmd.args(&["-s", "8", filename])
        .succeeds()
        .no_stdout()
        .no_stderr();
    assert!(at.file_exists(filename));
    assert_eq!(at.read_bytes(filename), vec![b'\0'; 8]);
}

/// Test that truncating a non-existent file creates that file, even in reference-mode.
#[test]
fn test_new_file_reference() {
    let (at, mut ucmd) = at_and_ucmd!();
    let mut old_file = at.make_file(FILE1);
    old_file.write_all(b"1234567890").unwrap();
    let filename = "new_file_that_does_not_exist_yet";
    ucmd.args(&["-r", FILE1, filename])
        .succeeds()
        .no_stdout()
        .no_stderr();
    assert!(at.file_exists(filename));
    assert_eq!(at.read_bytes(filename), vec![b'\0'; 10]);
}

/// Test that truncating a non-existent file creates that file, even in size-and-reference-mode.
#[test]
fn test_new_file_size_and_reference() {
    let (at, mut ucmd) = at_and_ucmd!();
    let mut old_file = at.make_file(FILE1);
    old_file.write_all(b"1234567890").unwrap();
    let filename = "new_file_that_does_not_exist_yet";
    ucmd.args(&["-s", "+3", "-r", FILE1, filename])
        .succeeds()
        .no_stdout()
        .no_stderr();
    assert!(at.file_exists(filename));
    assert_eq!(at.read_bytes(filename), vec![b'\0'; 13]);
}

/// Test for not creating a non-existent file.
#[test]
fn test_new_file_no_create_size_only() {
    let (at, mut ucmd) = at_and_ucmd!();
    let filename = "new_file_that_does_not_exist_yet";
    ucmd.args(&["-s", "8", "-c", filename])
        .succeeds()
        .no_stdout()
        .no_stderr();
    assert!(!at.file_exists(filename));
}

/// Test for not creating a non-existent file.
#[test]
fn test_new_file_no_create_reference_only() {
    let (at, mut ucmd) = at_and_ucmd!();
    let mut old_file = at.make_file(FILE1);
    old_file.write_all(b"1234567890").unwrap();
    let filename = "new_file_that_does_not_exist_yet";
    ucmd.args(&["-r", FILE1, "-c", filename])
        .succeeds()
        .no_stdout()
        .no_stderr();
    assert!(!at.file_exists(filename));
}

/// Test for not creating a non-existent file.
#[test]
fn test_new_file_no_create_size_and_reference() {
    let (at, mut ucmd) = at_and_ucmd!();
    let mut old_file = at.make_file(FILE1);
    old_file.write_all(b"1234567890").unwrap();
    let filename = "new_file_that_does_not_exist_yet";
    ucmd.args(&["-r", FILE1, "-s", "+8", "-c", filename])
        .succeeds()
        .no_stdout()
        .no_stderr();
    assert!(!at.file_exists(filename));
}

#[test]
fn test_division_by_zero_size_only() {
    new_ucmd!()
        .args(&["-s", "/0", "file"])
        .fails()
        .no_stdout()
        .stderr_contains("division by zero");
    new_ucmd!()
        .args(&["-s", "%0", "file"])
        .fails()
        .no_stdout()
        .stderr_contains("division by zero");
}

#[test]
fn test_division_by_zero_reference_and_size() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.make_file(FILE1);
    ucmd.args(&["-r", FILE1, "-s", "/0", "file"])
        .fails()
        .no_stdout()
        .stderr_contains("division by zero");

    let (at, mut ucmd) = at_and_ucmd!();
    at.make_file(FILE1);
    ucmd.args(&["-r", FILE1, "-s", "%0", "file"])
        .fails()
        .no_stdout()
        .stderr_contains("division by zero");
}

#[test]
fn test_no_such_dir() {
    new_ucmd!()
        .args(&["-s", "0", "a/b"])
        .fails()
        .no_stdout()
        .stderr_contains("cannot open 'a/b' for writing: No such file or directory");
}

/// Test that truncate with a relative size less than 0 is not an error.
#[test]
fn test_underflow_relative_size() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-s-1", FILE1])
        .succeeds()
        .no_stdout()
        .no_stderr();
    assert!(at.file_exists(FILE1));
    assert!(at.read_bytes(FILE1).is_empty());
}

#[test]
fn test_negative_size_with_space() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-s", "-1", FILE1])
        .succeeds()
        .no_stdout()
        .no_stderr();
    assert!(at.file_exists(FILE1));
    assert!(at.read_bytes(FILE1).is_empty());
}

#[cfg(not(windows))]
#[test]
fn test_fifo_error_size_only() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkfifo("fifo");
    ucmd.args(&["-s", "0", "fifo"])
        .fails()
        .no_stdout()
        .stderr_contains("cannot open 'fifo' for writing: No such device or address");
}

#[cfg(not(windows))]
#[test]
fn test_fifo_error_reference_file_only() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkfifo("fifo");
    at.make_file("reference_file");
    ucmd.args(&["-r", "reference_file", "fifo"])
        .fails()
        .no_stdout()
        .stderr_contains("cannot open 'fifo' for writing: No such device or address");
}

#[cfg(not(windows))]
#[test]
fn test_fifo_error_reference_and_size() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkfifo("fifo");
    at.make_file("reference_file");
    ucmd.args(&["-r", "reference_file", "-s", "+0", "fifo"])
        .fails()
        .no_stdout()
        .stderr_contains("cannot open 'fifo' for writing: No such device or address");
}

#[test]
#[cfg(target_os = "linux")]
fn test_truncate_non_utf8_paths() {
    use std::os::unix::ffi::OsStrExt;
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let file_name = std::ffi::OsStr::from_bytes(b"test_\xFF\xFE.txt");
    at.write(&file_name.to_string_lossy(), "test content");

    // Test that truncate can handle non-UTF-8 filenames
    ts.ucmd().arg("-s").arg("10").arg(file_name).succeeds();
}
