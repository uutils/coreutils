extern crate libc;

use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./link";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_link_existing_file() {
    let file = "test_link_existing_file";
    let link = "test_link_existing_file_link";

    touch(file);
    set_file_contents(file, "foobar");
    assert!(file_exists(file));

    let result = run(Command::new(PROGNAME).args(&[file, link]));
    assert_empty_stderr!(result);
    assert!(result.success);
    assert!(file_exists(file));
    assert!(file_exists(link));
    assert_eq!(get_file_contents(file), get_file_contents(link));
}

#[test]
fn test_link_no_circular() {
    let link = "test_link_no_circular";

    let result = run(Command::new(PROGNAME).args(&[link, link]));
    assert_eq!(result.stderr, "link: error: No such file or directory (os error 2)\n");
    assert!(!result.success);
    assert!(!file_exists(link));
}

#[test]
fn test_link_nonexistent_file() {
    let file = "test_link_nonexistent_file";
    let link = "test_link_nonexistent_file_link";

    let result = run(Command::new(PROGNAME).args(&[file, link]));
    assert_eq!(result.stderr, "link: error: No such file or directory (os error 2)\n");
    assert!(!result.success);
    assert!(!file_exists(file));
    assert!(!file_exists(link));
}
