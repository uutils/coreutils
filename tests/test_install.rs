extern crate libc;
extern crate time;
extern crate kernel32;
extern crate winapi;
extern crate filetime;

use self::filetime::*;
use common::util::*;

static UTIL_NAME: &'static str = "install";

#[test]
fn test_install_help() {
    let (at, mut ucmd) = testing(UTIL_NAME);

    let result = ucmd.arg("--help").run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(result.stdout.contains("Usage:"));
}

#[test]
fn test_install_basic() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir = "test_install_target_dir_dir_a";
    let file = "test_install_target_dir_file_a";

    at.touch(file);
    at.mkdir(dir);
    let result = ucmd.arg(file).arg(dir).run();

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.file_exists(file));
    assert!(at.file_exists(&format!("{}/{}", dir, file)));
}

#[test]
fn test_install_unimplemented_arg() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir = "test_install_target_dir_dir_b";
    let file = "test_install_target_dir_file_b";
    let context_arg = "--context";

    at.touch(file);
    at.mkdir(dir);
    let result = ucmd.arg(context_arg).arg(file).arg(dir).run();

    assert!(!result.success);
    assert!(result.stderr.contains("Unimplemented"));

    assert!(!at.file_exists(&format!("{}/{}", dir, file)));
}
