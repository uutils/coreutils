use crate::common::util::*;
use std::io::{Seek, SeekFrom, Write};

static TFILE1: &'static str = "truncate_test_1";
static TFILE2: &'static str = "truncate_test_2";

#[test]
fn test_increase_file_size() {
    let expected = 5 * 1024;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(TFILE1);
    ucmd.args(&["-s", "+5K", TFILE1]).succeeds();

    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.seek(SeekFrom::Current(0)).unwrap();
    assert!(
        expected == actual,
        "expected '{}' got '{}'",
        expected,
        actual
    );
}

#[test]
fn test_increase_file_size_kb() {
    let expected = 5 * 1000;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(TFILE1);
    ucmd.args(&["-s", "+5KB", TFILE1]).succeeds();

    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.seek(SeekFrom::Current(0)).unwrap();
    assert!(
        expected == actual,
        "expected '{}' got '{}'",
        expected,
        actual
    );
}

#[test]
fn test_reference() {
    let expected = 5 * 1000;
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let mut file = at.make_file(TFILE2);

    scene.ucmd().arg("-s").arg("+5KB").arg(TFILE1).run();

    scene
        .ucmd()
        .arg("--reference")
        .arg(TFILE1)
        .arg(TFILE2)
        .run();

    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.seek(SeekFrom::Current(0)).unwrap();
    assert!(
        expected == actual,
        "expected '{}' got '{}'",
        expected,
        actual
    );
}

#[test]
fn test_decrease_file_size() {
    let expected = 6;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(TFILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size=-4", TFILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.seek(SeekFrom::Current(0)).unwrap();
    assert!(
        expected == actual,
        "expected '{}' got '{}'",
        expected,
        actual
    );
}

#[test]
fn test_space_in_size() {
    let expected = 4;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(TFILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size", " 4", TFILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.seek(SeekFrom::Current(0)).unwrap();
    assert!(
        expected == actual,
        "expected '{}' got '{}'",
        expected,
        actual
    );
}

#[test]
fn test_failed() {
    new_ucmd!().fails();
}

#[test]
fn test_failed_2() {
    let (_at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&[TFILE1]).fails();
}

#[test]
fn test_failed_incorrect_arg() {
    let (_at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-s", "+5A", TFILE1]).fails();
}

#[test]
fn test_at_most_shrinks() {
    let expected = 4;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(TFILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size", "<4", TFILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.seek(SeekFrom::Current(0)).unwrap();
    assert!(
        expected == actual,
        "expected '{}' got '{}'",
        expected,
        actual
    );
}

#[test]
fn test_at_most_no_change() {
    let expected = 10;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(TFILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size", "<40", TFILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.seek(SeekFrom::Current(0)).unwrap();
    assert!(
        expected == actual,
        "expected '{}' got '{}'",
        expected,
        actual
    );
}

#[test]
fn test_at_least_grows() {
    let expected = 15;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(TFILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size", ">15", TFILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.seek(SeekFrom::Current(0)).unwrap();
    assert!(
        expected == actual,
        "expected '{}' got '{}'",
        expected,
        actual
    );
}

#[test]
fn test_at_least_no_change() {
    let expected = 10;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(TFILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size", ">4", TFILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.seek(SeekFrom::Current(0)).unwrap();
    assert!(
        expected == actual,
        "expected '{}' got '{}'",
        expected,
        actual
    );
}

#[test]
fn test_round_down() {
    let expected = 8;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(TFILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size", "/4", TFILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.seek(SeekFrom::Current(0)).unwrap();
    assert!(
        expected == actual,
        "expected '{}' got '{}'",
        expected,
        actual
    );
}

#[test]
fn test_round_up() {
    let expected = 12;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(TFILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size", "*4", TFILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    let actual = file.seek(SeekFrom::Current(0)).unwrap();
    assert!(
        expected == actual,
        "expected '{}' got '{}'",
        expected,
        actual
    );
}

#[test]
fn test_size_and_reference() {
    let expected = 15;
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file1 = at.make_file(TFILE1);
    let mut file2 = at.make_file(TFILE2);
    file1.write_all(b"1234567890").unwrap();
    ucmd.args(&["--reference", TFILE1, "--size", "+5", TFILE2])
        .succeeds();
    file2.seek(SeekFrom::End(0)).unwrap();
    let actual = file2.seek(SeekFrom::Current(0)).unwrap();
    assert!(
        expected == actual,
        "expected '{}' got '{}'",
        expected,
        actual
    );
}

#[test]
fn test_invalid_numbers() {
    // TODO For compatibility with GNU, `truncate -s 0X` should cause
    // the same error as `truncate -s 0X file`, but currently it returns
    // a different error.
    new_ucmd!().args(&["-s", "0X", "file"]).fails().stderr_contains("Invalid number: ‘0X’");
    new_ucmd!().args(&["-s", "0XB", "file"]).fails().stderr_contains("Invalid number: ‘0XB’");
    new_ucmd!().args(&["-s", "0B", "file"]).fails().stderr_contains("Invalid number: ‘0B’");
}

#[test]
fn test_reference_file_not_found() {
    new_ucmd!()
        .args(&["-r", "a", "b"])
        .fails()
        .stderr_contains("cannot stat 'a': No such file or directory");
}
