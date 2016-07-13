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
    let file1 = "test_install_target_dir_file_a1";
    let file2 = "test_install_target_dir_file_a2";

    at.touch(file1);
    at.touch(file2);
    at.mkdir(dir);
    let result = ucmd.arg(file1).arg(file2).arg(dir).run();

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(at.file_exists(file1));
    assert!(at.file_exists(file2));
    assert!(at.file_exists(&format!("{}/{}", dir, file1)));
    assert!(at.file_exists(&format!("{}/{}", dir, file2)));
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
