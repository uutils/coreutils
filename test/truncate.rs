use std::fs::{File, remove_file};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::process::Command;

static PROGNAME: &'static str = "./truncate";
static TFILE1: &'static str = "truncate_test_1";
static TFILE2: &'static str = "truncate_test_2";

fn make_file(name: &str) -> File {
    match File::create(Path::new(name)) {
        Ok(f) => f,
        Err(_) => panic!()
    }
}

#[test]
fn test_increase_file_size() {
    let mut file = make_file(TFILE1);
    if !Command::new(PROGNAME).args(&["-s", "+5K", TFILE1]).status().unwrap().success() {
        panic!();
    }
    file.seek(SeekFrom::End(0)).unwrap();
    if file.seek(SeekFrom::Current(0)).unwrap() != 5 * 1024 {
        panic!();
    }
    remove_file(Path::new(TFILE1)).unwrap();
}

#[test]
fn test_decrease_file_size() {
    let mut file = make_file(TFILE2);
    file.write_all(b"1234567890").unwrap();
    if !Command::new(PROGNAME).args(&["--size=-4", TFILE2]).status().unwrap().success() {
        panic!();
    }
    file.seek(SeekFrom::End(0)).unwrap();
    if file.seek(SeekFrom::Current(0)).unwrap() != 6 {
        println!("{:?}", file.seek(SeekFrom::Current(0)));
        panic!();
    }
    remove_file(Path::new(TFILE2)).unwrap();
}
