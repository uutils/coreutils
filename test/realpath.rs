use std::process::Command;
use std::str;
use util::*;

static PROGNAME: &'static str = "./realpath";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_current_directory() {
    let po = Command::new(PROGNAME)
                 .arg(".")
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap().trim_right();
    assert_eq!(out, current_directory());
}

#[test]
fn test_long_redirection_to_current_dir() {
    // Create a 256-character path to current directory
    let dir = repeat_str("./", 128);
    let po = Command::new(PROGNAME)
                 .arg(dir)
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap().trim_right();
    assert_eq!(out, current_directory());
}

#[test]
fn test_long_redirection_to_root() {
    // Create a 255-character path to root
    let dir = repeat_str("../", 85);
    let po = Command::new(PROGNAME)
                 .arg(dir)
                 .output()
                 .unwrap_or_else(|err| panic!("{}", err));

    let out = str::from_utf8(&po.stdout[..]).unwrap().trim_right();
    assert_eq!(out, "/");
}
