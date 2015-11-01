extern crate libc;

use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./rmdir";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_rmdir_empty_directory_no_parents() {
    let dir = "test_rmdir_empty_no_parents";

    mkdir(dir);
    assert!(dir_exists(dir));

    let result = run(Command::new(PROGNAME).arg(dir));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!dir_exists(dir));
}

#[test]
fn test_rmdir_empty_directory_with_parents() {
    let dir = "test_rmdir_empty/with/parents";

    mkdir_all(dir);
    assert!(dir_exists(dir));

    let result = run(Command::new(PROGNAME).arg("-p").arg(dir));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!dir_exists(dir));
}

#[test]
fn test_rmdir_nonempty_directory_no_parents() {
    let dir = "test_rmdir_nonempty_no_parents";
    let file = "test_rmdir_nonempty_no_parents/foo";

    mkdir(dir);
    assert!(dir_exists(dir));

    touch(file);
    assert!(file_exists(file));

    let result = run(Command::new(PROGNAME).arg(dir));
    assert_eq!(result.stderr,
               "rmdir: error: failed to remove 'test_rmdir_nonempty_no_parents': Directory not empty\n");
    assert!(!result.success);

    assert!(dir_exists(dir));
}

#[test]
fn test_rmdir_nonempty_directory_with_parents() {
    let dir = "test_rmdir_nonempty/with/parents";
    let file = "test_rmdir_nonempty/with/parents/foo";

    mkdir_all(dir);
    assert!(dir_exists(dir));

    touch(file);
    assert!(file_exists(file));

    let result = run(Command::new(PROGNAME).arg("-p").arg(dir));
    assert_eq!(result.stderr,
               "rmdir: error: failed to remove 'test_rmdir_nonempty/with/parents': Directory not empty\n\
               rmdir: error: failed to remove 'test_rmdir_nonempty/with': Directory not empty\n\
               rmdir: error: failed to remove 'test_rmdir_nonempty': Directory not empty\n");
    assert!(!result.success);

    assert!(dir_exists(dir));
}

#[test]
fn test_rmdir_ignore_nonempty_directory_no_parents() {
    let dir = "test_rmdir_ignore_nonempty_no_parents";
    let file = "test_rmdir_ignore_nonempty_no_parents/foo";

    mkdir(dir);
    assert!(dir_exists(dir));

    touch(file);
    assert!(file_exists(file));

    let result = run(Command::new(PROGNAME).arg("--ignore-fail-on-non-empty").arg(dir));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(dir_exists(dir));
}

#[test]
fn test_rmdir_ignore_nonempty_directory_with_parents() {
    let dir = "test_rmdir_ignore_nonempty/with/parents";
    let file = "test_rmdir_ignore_nonempty/with/parents/foo";

    mkdir_all(dir);
    assert!(dir_exists(dir));

    touch(file);
    assert!(file_exists(file));

    let result = run(Command::new(PROGNAME).arg("--ignore-fail-on-non-empty").arg("-p").arg(dir));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(dir_exists(dir));
}
