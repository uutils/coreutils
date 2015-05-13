#![feature(path_ext)]

use std::fs::{remove_dir, PathExt};
use std::path::Path;
use std::process::{Command, Output};

static PROGNAME: &'static str = "./mkdir";
static TEST_DIR1: &'static str = "mkdir_test1";
static TEST_DIR2: &'static str = "mkdir_test2";
static TEST_DIR3: &'static str = "mkdir_test3";
static TEST_DIR4: &'static str = "mkdir_test4/mkdir_test4_1";
static TEST_DIR5: &'static str = "mkdir_test5/mkdir_test5_1";

fn run(args: &[&'static str]) -> Output {
    Command::new(PROGNAME)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("{}", e))
}

fn cleanup(dir: &'static str) {
    let p = Path::new(dir);
    if p.exists() {
        remove_dir(&p).unwrap();
    }
}

#[test]
fn test_mkdir_mkdir() {
    cleanup(TEST_DIR1);
    let exit_success = run(&[TEST_DIR1]).status.success();
    cleanup(TEST_DIR1);
    assert_eq!(exit_success, true);
}

#[test]
fn test_mkdir_dup_dir() {
    cleanup(TEST_DIR2);
    let exit_success = run(&[TEST_DIR2]).status.success();
    if !exit_success {
        cleanup(TEST_DIR2);
        panic!();
    }
    let exit_success2 = run(&[TEST_DIR2]).status.success();
    cleanup(TEST_DIR2);
    assert_eq!(exit_success2, false);
}

#[test]
fn test_mkdir_mode() {
    cleanup(TEST_DIR3);
    let exit_success = run(&["-m", "755", TEST_DIR3]).status.success();
    cleanup(TEST_DIR3);
    assert_eq!(exit_success, true);
}

#[test]
fn test_mkdir_parent() {
    cleanup(TEST_DIR4);
    let exit_success = run(&["-p", TEST_DIR4]).status.success();
    cleanup(TEST_DIR4);
    assert_eq!(exit_success, true);
}

#[test]
fn test_mkdir_no_parent() {
    cleanup(TEST_DIR5);
    let exit_success = run(&[TEST_DIR5]).status.success();
    cleanup(TEST_DIR5);
    assert_eq!(exit_success, false);
}
