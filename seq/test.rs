use std::io::process::Process;
use std::str;

#[test]
fn test_count_up() {
    let p = Process::output("build/seq", ["10".to_owned()]).unwrap();
    let out = str::from_utf8(p.output.as_slice().to_owned()).unwrap().into_owned();
    assert_eq!(out, "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n".to_owned());
}

#[test]
fn test_count_down() {
    let p = Process::output("build/seq", ["--".to_owned(), "5".to_owned(), "-1".to_owned(), "1".to_owned()]).unwrap();
    let out = str::from_utf8(p.output.as_slice().to_owned()).unwrap().into_owned();
    assert_eq!(out, "5\n4\n3\n2\n1\n".to_owned());
}

#[test]
fn test_separator_and_terminator() {
    let p = Process::output("build/seq", ["-s".to_owned(), ",".to_owned(), "-t".to_owned(), "!".to_owned(), "2".to_owned(), "6".to_owned()]).unwrap();
    let out = str::from_utf8(p.output.as_slice().to_owned()).unwrap().into_owned();
    assert_eq!(out, "2,3,4,5,6!".to_owned());
}

#[test]
fn test_equalize_widths() {
    let p = Process::output("build/seq", ["-w".to_owned(), "5".to_owned(), "10".to_owned()]).unwrap();
    let out = str::from_utf8(p.output.as_slice().to_owned()).unwrap().into_owned();
    assert_eq!(out, "05\n06\n07\n08\n09\n10\n".to_owned());
}
