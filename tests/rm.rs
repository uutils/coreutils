#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "rm";

#[test]
fn test_rm_one_file() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let file = "test_rm_one_file";

    at.touch(file);

    let result = ucmd.arg(file).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.file_exists(file));
}

#[test]
fn test_rm_multiple_files() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let file_a = "test_rm_multiple_file_a";
    let file_b = "test_rm_multiple_file_b";

    at.touch(file_a);
    at.touch(file_b);

    let result = ucmd.arg(file_a).arg(file_b).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.file_exists(file_a));
    assert!(!at.file_exists(file_b));
}

#[test]
fn test_rm_interactive() {
    let ts = TestSet::new(UTIL_NAME);
    let at = &ts.fixtures;

    let file_a = "test_rm_interactive_file_a";
    let file_b = "test_rm_interactive_file_b";

    at.touch(file_a);
    at.touch(file_b);

    let result1 = ts.util_cmd()
                    .arg("-i")
                    .arg(file_a)
                    .arg(file_b)
                    .run_piped_stdin("n");

    assert!(result1.success);

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));

    let result2 = ts.util_cmd()
                    .arg("-i")
                    .arg(file_a)
                    .arg(file_b)
                    .run_piped_stdin("Yesh");

    assert!(result2.success);

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
}

#[test]
fn test_rm_force() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let file_a = "test_rm_force_a";
    let file_b = "test_rm_force_b";

    let result = ucmd.arg("-f")
                     .arg(file_a)
                     .arg(file_b)
                     .run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.file_exists(file_a));
    assert!(!at.file_exists(file_b));
}

#[test]
fn test_rm_empty_directory() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir = "test_rm_empty_directory";

    at.mkdir(dir);

    let result = ucmd.arg("-d").arg(dir).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.dir_exists(dir));
}

#[test]
fn test_rm_recursive() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir = "test_rm_recursive_directory";
    let file_a = "test_rm_recursive_directory/test_rm_recursive_file_a";
    let file_b = "test_rm_recursive_directory/test_rm_recursive_file_b";

    at.mkdir(dir);
    at.touch(file_a);
    at.touch(file_b);

    let result = ucmd.arg("-r").arg(dir).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.dir_exists(dir));
    assert!(!at.file_exists(file_a));
    assert!(!at.file_exists(file_b));
}

#[test]
fn test_rm_errors() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir = "test_rm_errors_directory";
    let file_a = "test_rm_errors_directory/test_rm_errors_file_a";
    let file_b = "test_rm_errors_directory/test_rm_errors_file_b";

    at.mkdir(dir);
    at.touch(file_a);
    at.touch(file_b);

    // $ rm test_rm_errors_directory
    // rm: error: could not remove directory 'test_rm_errors_directory' (did you mean to pass '-r'?)
    let result = ucmd.arg(dir).run();
    assert_eq!(result.stderr,
               "rm: error: could not remove directory 'test_rm_errors_directory' (did you mean \
                to pass '-r'?)\n");
    assert!(!result.success);
}

#[test]
fn test_rm_verbose() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let file_a = "test_rm_verbose_file_a";
    let file_b = "test_rm_verbose_file_b";

    at.touch(file_a);
    at.touch(file_b);

    let result = ucmd.arg("-v").arg(file_a).arg(file_b).run();
    assert_empty_stderr!(result);
    assert_eq!(result.stdout,
               format!("removed '{}'\nremoved '{}'\n", file_a, file_b));
    assert!(result.success);
}
