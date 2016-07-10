/*
 * This file is part of the uutils coreutils package.
 * (c) Smigle00 (smigle00 [at] gmail [dot] com)

 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use common::util::*;

static UTIL_NAME: &'static str = "remove";

#[test]
fn test_remove_one_file() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let file = "test_remove_one_file";

    at.touch(file);

    let result = ucmd.arg(file).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.file_exists(file));
}

#[test]
fn test_remove_multiple_files() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let file_a = "test_remove_multiple_file_a";
    let file_b = "test_remove_multiple_file_b";

    at.touch(file_a);
    at.touch(file_b);

    let result = ucmd.arg(file_a).arg(file_b).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.file_exists(file_a));
    assert!(!at.file_exists(file_b));
}

#[test]
fn test_remove_single_empty_directory() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir = "test_remove_single_empty_directory";

    at.mkdir(dir);

    let result = ucmd.arg(dir).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.dir_exists(dir));
}

#[test]
fn test_remove_multiple_empty_directories() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir1 = "test_remove_empty_directory_1";
    let dir2 = "test_remove_empty_directory_2";

    at.mkdir(dir1);
    at.mkdir(dir2);

    let result = ucmd.arg(dir1).arg(dir2).run();
    assert!(result.success);
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.dir_exists(dir1));
    assert!(!at.dir_exists(dir2));
}

#[test]
fn test_remove_errors() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let non_exist_file = "non-existing-file";
    let dir       = "test_remove_errors_directory";
    let file_1    = "test_remove_errors_directory/test_remove_errors_file_1";

    // Test when file does not exist in file system
    // $ rmdir non-existing-file
    // remove: error: failed to remove 'non-existing-file': No such file or directory
    let result = ucmd.arg(non_exist_file).run();
    assert_eq!(result.stderr,
               "remove: error: failed to remove 'non-existing-file': No such file or directory\n");
    assert!(!result.success);
    assert!(!at.file_exists(non_exist_file));

    let (at, mut ucmd) = testing(UTIL_NAME);
    at.mkdir(dir);
    at.touch(file_1);

    // Test to remove non-empty directory
    // $ remove test_remove_errors_directory
    // remove: error: failed to remove 'test_remove_errors_directory' directory: Directory not empty (os error 39)
    let result = ucmd.arg(dir).run();
    assert_eq!(result.stderr,
               "remove: error: failed to remove 'test_remove_errors_directory' directory: Directory not empty (os error 39)\n");
    assert!(!result.success);
    assert!(at.dir_exists(dir));
}
