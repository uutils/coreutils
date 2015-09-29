use std::process::Command;
use std::str;
use util::*;

static PROGNAME: &'static str = "./readlink";
static GIBBERISH: &'static str = "supercalifragilisticexpialidocious";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_canonicalize() {
    let po = Command::new(PROGNAME)
                 .arg("-f")
                 .arg(".")
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap().trim_right();
    assert_eq!(out, current_directory());
}

#[test]
fn test_canonicalize_existing() {
    let po = Command::new(PROGNAME)
                 .arg("-e")
                 .arg(".")
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap().trim_right();
    assert_eq!(out, current_directory());
}

#[test]
fn test_canonicalize_missing() {
    let mut expected = current_directory();
    expected.push_str("/");
    expected.push_str(GIBBERISH);

    let po = Command::new(PROGNAME)
                 .arg("-m")
                 .arg(GIBBERISH)
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap().trim_right();
    assert_eq!(out, expected);
}

#[test]
fn test_long_redirection_to_current_dir() {
    // Create a 256-character path to current directory
    let dir = repeat_str("./", 128);
    let po = Command::new(PROGNAME)
                 .arg("-n")
                 .arg("-m")
                 .arg(dir)
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap();
    assert_eq!(out, current_directory());
}

#[test]
fn test_long_redirection_to_root() {
    // Create a 255-character path to root
    let dir = repeat_str("../", 85);
    let po = Command::new(PROGNAME)
                 .arg("-n")
                 .arg("-m")
                 .arg(dir)
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap();
    assert_eq!(out, "/");
}
