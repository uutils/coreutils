use std::process::Command;
use std::str;

static PROGNAME: &'static str = "./basename";

#[test]
fn test_directory() {
    let dir = "/root/alpha/beta/gamma/delta/epsilon/omega/";
    let po = Command::new(PROGNAME)
                 .arg(dir)
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap().trim_right();
    assert_eq!(out, "omega");
}

#[test]
fn test_file() {
    let file = "/etc/passwd";
    let po = Command::new(PROGNAME)
                 .arg(file)
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap().trim_right();
    assert_eq!(out, "passwd");
}

#[test]
fn test_remove_suffix() {
    let path = "/usr/local/bin/reallylongexecutable.exe";
    let po = Command::new(PROGNAME)
                 .arg(path)
                 .arg(".exe")
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap().trim_right();
    assert_eq!(out, "reallylongexecutable");
}

#[test]
fn test_dont_remove_suffix() {
    let path = "/foo/bar/baz";
    let po = Command::new(PROGNAME)
                 .arg(path)
                 .arg("baz")
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap().trim_right();
    assert_eq!(out, "baz");
}
