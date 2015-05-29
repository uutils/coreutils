#![feature(path_ext)]

extern crate libc;

use std::fs::PathExt;
use std::path::Path;
use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./rm";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_rm_one_file() {
    let file = "test_rm_one_file";

    touch(file);

    let result = run(Command::new(PROGNAME).arg(file));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(file).exists());
}

#[test]
fn test_rm_multiple_files() {
    let file_a = "test_rm_multiple_file_a";
    let file_b = "test_rm_multiple_file_b";

    touch(file_a);
    touch(file_b);

    let result = run(Command::new(PROGNAME).arg(file_a).arg(file_b));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(file_a).exists());
    assert!(!Path::new(file_b).exists());
}

#[test]
fn test_rm_interactive() {
    let file_a = "test_rm_interactive_file_a";
    let file_b = "test_rm_interactive_file_b";

    touch(file_a);
    touch(file_b);

    let result1 = run_piped_stdin(Command::new(PROGNAME).arg("-i").arg(file_a).arg(file_b), b"n");

    assert!(result1.success);

    assert!(Path::new(file_a).exists());
    assert!(Path::new(file_b).exists());

    let result2 = run_piped_stdin(Command::new(PROGNAME).arg("-i").arg(file_a).arg(file_b), b"Yesh");

    assert!(result2.success);

    assert!(!Path::new(file_a).exists());
    assert!(Path::new(file_b).exists());
}

#[test]
fn test_rm_force() {
    let file_a = "test_rm_force_a";
    let file_b = "test_rm_force_b";

    let result = run(Command::new(PROGNAME).arg("-f").arg(file_a).arg(file_b));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(file_a).exists());
    assert!(!Path::new(file_b).exists());
}

#[test]
fn test_rm_empty_directory() {
    let dir = "test_rm_empty_directory";

    mkdir(dir);

    let result = run(Command::new(PROGNAME).arg("-d").arg(dir));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(dir).exists());
}

#[test]
fn test_rm_recursive() {
    let dir = "test_rm_recursive_directory";
    let file_a = "test_rm_recursive_directory/test_rm_recursive_file_a";
    let file_b = "test_rm_recursive_directory/test_rm_recursive_file_b";

    mkdir(dir);
    touch(file_a);
    touch(file_b);

    let result = run(Command::new(PROGNAME).arg("-r").arg(dir));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(dir).exists());
    assert!(!Path::new(file_a).exists());
    assert!(!Path::new(file_b).exists());
}

#[test]
fn test_rm_errors() {
    let dir = "test_rm_errors_directory";
    let file_a = "test_rm_errors_directory/test_rm_errors_file_a";
    let file_b = "test_rm_errors_directory/test_rm_errors_file_b";

    mkdir(dir);
    touch(file_a);
    touch(file_b);

    // $ rm test_rm_errors_directory
    // rm: error: could not remove directory 'test_rm_errors_directory' (did you mean to pass '-r'?)
    let result = run(Command::new(PROGNAME).arg(dir));
    assert_eq!(result.stderr,
        "rm: error: could not remove directory 'test_rm_errors_directory' (did you mean to pass '-r'?)\n");
    assert!(!result.success);
}

#[test]
fn test_rm_verbose() {
    let file_a = "test_rm_verbose_file_a";
    let file_b = "test_rm_verbose_file_b";

    touch(file_a);
    touch(file_b);

    let result = run(Command::new(PROGNAME).arg("-v").arg(file_a).arg(file_b));
    assert_empty_stderr!(result);
    assert_eq!(result.stdout,
        format!("removed '{}'\nremoved '{}'\n", file_a, file_b));
    assert!(result.success);
}
