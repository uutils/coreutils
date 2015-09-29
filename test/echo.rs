use std::process::Command;
use std::str;

static PROGNAME: &'static str = "./echo";

#[test]
fn test_default() {
    let po = Command::new(PROGNAME)
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap();
    assert_eq!(out, "\n");
}

#[test]
fn test_no_trailing_newline() {
    let po = Command::new(PROGNAME)
                 .arg("-n")
                 .arg("hello_world")
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap();
    assert_eq!(out, "hello_world");
}

#[test]
fn test_enable_escapes() {
    let po = Command::new(PROGNAME)
                 .arg("-e")
                 .arg("\\\\\\t\\r")
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap();
    assert_eq!(out, "\\\t\r\n");
}

#[test]
fn test_disable_escapes() {
    let po = Command::new(PROGNAME)
                 .arg("-E")
                 .arg("\\b\\c\\e")
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap();
    assert_eq!(out, "\\b\\c\\e\n");
}
