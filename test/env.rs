use std::process::Command;
use std::str;

static PROGNAME: &'static str = "./env";

#[test]
fn test_single_name_value_pair() {
    let po = Command::new(PROGNAME)
        .arg("FOO=bar")
        .output()
        .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap();
    assert!(out.lines_any().any(|line| line == "FOO=bar"));
}

#[test]
fn test_multiple_name_value_pairs() {
    let po = Command::new(PROGNAME)
        .arg("FOO=bar")
        .arg("ABC=xyz")
        .output()
        .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap();
    assert_eq!(out.lines_any().filter(|&line| line == "FOO=bar" || line == "ABC=xyz").count(), 2);
}

#[test]
fn test_ignore_environment() {
    let po = Command::new(PROGNAME)
        .arg("-i")
        .output()
        .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap();
    assert_eq!(out, "");
}

#[test]
fn test_null_delimiter() {
    let po = Command::new(PROGNAME)
        .arg("-i")
        .arg("--null")
        .arg("FOO=bar")
        .arg("ABC=xyz")
        .output()
        .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap();
    assert_eq!(out, "FOO=bar\0ABC=xyz\0");
}

#[test]
fn test_unset_variable() {
    // This test depends on the HOME variable being pre-defined by the
    // default shell
    let po = Command::new(PROGNAME)
        .arg("-u")
        .arg("HOME")
        .output()
        .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap();
    assert_eq!(out.lines_any().any(|line| line.starts_with("HOME=")), false);
}
