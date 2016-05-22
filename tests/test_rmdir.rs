extern crate libc;

use common::util::*;

static UTIL_NAME: &'static str = "rmdir";

#[test]
fn test_rmdir_empty_directory_no_parents() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir = "test_rmdir_empty_no_parents";

    at.mkdir(dir);
    assert!(at.dir_exists(dir));

    let result = ucmd.arg(dir).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.dir_exists(dir));
}

#[test]
fn test_rmdir_empty_directory_with_parents() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir = "test_rmdir_empty/with/parents";

    at.mkdir_all(dir);
    assert!(at.dir_exists(dir));

    let result = ucmd.arg("-p").arg(dir).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.dir_exists(dir));
}

#[test]
fn test_rmdir_nonempty_directory_no_parents() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir = "test_rmdir_nonempty_no_parents";
    let file = "test_rmdir_nonempty_no_parents/foo";

    at.mkdir(dir);
    assert!(at.dir_exists(dir));

    at.touch(file);
    assert!(at.file_exists(file));

    let result = ucmd.arg(dir).run();
    assert_eq!(result.stderr,
               "rmdir: error: failed to remove 'test_rmdir_nonempty_no_parents': Directory not \
                empty\n");
    assert!(!result.success);

    assert!(at.dir_exists(dir));
}

#[test]
fn test_rmdir_nonempty_directory_with_parents() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir = "test_rmdir_nonempty/with/parents";
    let file = "test_rmdir_nonempty/with/parents/foo";

    at.mkdir_all(dir);
    assert!(at.dir_exists(dir));

    at.touch(file);
    assert!(at.file_exists(file));

    let result = ucmd.arg("-p").arg(dir).run();
    assert_eq!(result.stderr,
               "rmdir: error: failed to remove 'test_rmdir_nonempty/with/parents': Directory not \
                empty\nrmdir: error: failed to remove 'test_rmdir_nonempty/with': Directory not \
                empty\nrmdir: error: failed to remove 'test_rmdir_nonempty': Directory not \
                empty\n");
    assert!(!result.success);

    assert!(at.dir_exists(dir));
}

#[test]
fn test_rmdir_ignore_nonempty_directory_no_parents() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir = "test_rmdir_ignore_nonempty_no_parents";
    let file = "test_rmdir_ignore_nonempty_no_parents/foo";

    at.mkdir(dir);
    assert!(at.dir_exists(dir));

    at.touch(file);
    assert!(at.file_exists(file));

    let result = ucmd.arg("--ignore-fail-on-non-empty").arg(dir).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(at.dir_exists(dir));
}

#[test]
fn test_rmdir_ignore_nonempty_directory_with_parents() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir = "test_rmdir_ignore_nonempty/with/parents";
    let file = "test_rmdir_ignore_nonempty/with/parents/foo";

    at.mkdir_all(dir);
    assert!(at.dir_exists(dir));

    at.touch(file);
    assert!(at.file_exists(file));

    let result = ucmd.arg("--ignore-fail-on-non-empty").arg("-p").arg(dir).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(at.dir_exists(dir));
}
