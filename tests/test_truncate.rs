use common::util::*;
use std::io::{Seek, SeekFrom, Write};


static TFILE1: &'static str = "truncate_test_1";
static TFILE2: &'static str = "truncate_test_2";

#[test]
fn test_increase_file_size() {
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(TFILE1);
    ucmd.args(&["-s", "+5K", TFILE1]).succeeds();

    file.seek(SeekFrom::End(0)).unwrap();
    assert!(file.seek(SeekFrom::Current(0)).unwrap() == 5 * 1024);
}

#[test]
fn test_decrease_file_size() {
    let (at, mut ucmd) = at_and_ucmd!();
    let mut file = at.make_file(TFILE2);
    file.write_all(b"1234567890").unwrap();
    ucmd.args(&["--size=-4", TFILE2]).succeeds();
    file.seek(SeekFrom::End(0)).unwrap();
    assert!(file.seek(SeekFrom::Current(0)).unwrap() == 6);
}
