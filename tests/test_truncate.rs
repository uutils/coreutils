use common::util::*;
use std::io::{Seek, SeekFrom, Write};

static UTIL_NAME: &'static str = "truncate";
fn at_and_ucmd() -> (AtPath, UCommand) {
    let ts = TestScenario::new(UTIL_NAME);
    let ucmd = ts.ucmd();
    (ts.fixtures, ucmd)
}

static TFILE1: &'static str = "truncate_test_1";
static TFILE2: &'static str = "truncate_test_2";

#[test]
fn test_increase_file_size() {
    let (at, mut ucmd) = at_and_ucmd();
    let mut file = at.make_file(TFILE1);
    assert!(ucmd.args(&["-s", "+5K", TFILE1]).run().success);

    file.seek(SeekFrom::End(0)).unwrap();
    assert!(file.seek(SeekFrom::Current(0)).unwrap() == 5 * 1024);
}

#[test]
fn test_decrease_file_size() {
    let (at, mut ucmd) = at_and_ucmd();
    let mut file = at.make_file(TFILE2);
    file.write_all(b"1234567890").unwrap();
    assert!(ucmd.args(&["--size=-4", TFILE2]).run().success);
    file.seek(SeekFrom::End(0)).unwrap();
    assert!(file.seek(SeekFrom::Current(0)).unwrap() == 6);
}
