use std::io::process::Command;
use std::str;

static PROGNAME: &'static str = "./seq";

#[test]
fn test_count_up() {
    let p = Command::new(PROGNAME).args(["10"]).output().unwrap();
    let out = str::from_utf8(p.output.as_slice()).unwrap();
    assert_eq!(out, "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");
}

#[test]
fn test_count_down() {
    let p = Command::new(PROGNAME).args(["--", "5", "-1", "1"]).output().unwrap();
    let out = str::from_utf8(p.output.as_slice()).unwrap();
    assert_eq!(out, "5\n4\n3\n2\n1\n");
}

#[test]
fn test_separator_and_terminator() {
    let p = Command::new(PROGNAME).args(["-s", ",", "-t", "!", "2", "6"]).output().unwrap();
    let out = str::from_utf8(p.output.as_slice()).unwrap();
    assert_eq!(out, "2,3,4,5,6!");
}

#[test]
fn test_equalize_widths() {
    let p = Command::new(PROGNAME).args(["-w", "5", "10"]).output().unwrap();
    let out = str::from_utf8(p.output.as_slice()).unwrap();
    assert_eq!(out, "05\n06\n07\n08\n09\n10\n");
}
