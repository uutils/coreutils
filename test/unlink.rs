extern crate libc;

use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./unlink";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_unlink_file() {
    let file = "test_unlink_file";

    touch(file);

    let result = run(Command::new(PROGNAME).arg(file));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!file_exists(file));
}

#[test]
fn test_unlink_multiple_files() {
    let file_a = "test_unlink_multiple_file_a";
    let file_b = "test_unlink_multiple_file_b";

    touch(file_a);
    touch(file_b);

    let result = run(Command::new(PROGNAME).arg(file_a).arg(file_b));
    assert_eq!(result.stderr,
        "unlink: error: extra operand: 'test_unlink_multiple_file_b'\nTry 'unlink --help' for more information.\n");
    assert!(!result.success);
}

#[test]
fn test_unlink_directory() {
    let dir = "test_unlink_empty_directory";

    mkdir(dir);

    let result = run(Command::new(PROGNAME).arg(dir));
    assert_eq!(result.stderr,
        "unlink: error: cannot unlink 'test_unlink_empty_directory': Not a regular file or symlink\n");
    assert!(!result.success);
}

#[test]
fn test_unlink_nonexistent() {
    let file = "test_unlink_nonexistent";

    let result = run(Command::new(PROGNAME).arg(file));
    assert_eq!(result.stderr,
        "unlink: error: Cannot stat 'test_unlink_nonexistent': No such file or directory (os error 2)\n");
    assert!(!result.success);
}
