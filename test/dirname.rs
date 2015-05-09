use std::process::Command;
use std::str;

static PROGNAME: &'static str = "./dirname";

#[test]
fn test_path_with_trailing_slashes() {
    let dir = "/root/alpha/beta/gamma/delta/epsilon/omega//";
    let po = Command::new(PROGNAME)
        .arg(dir)
        .output()
        .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap().trim_right();
    assert_eq!(out, "/root/alpha/beta/gamma/delta/epsilon");
}

#[test]
fn test_path_without_trailing_slashes() {
    let dir = "/root/alpha/beta/gamma/delta/epsilon/omega";
    let po = Command::new(PROGNAME)
        .arg(dir)
        .output()
        .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap().trim_right();
    assert_eq!(out, "/root/alpha/beta/gamma/delta/epsilon");
}
