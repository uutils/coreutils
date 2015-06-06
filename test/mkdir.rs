use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./mkdir";
static TEST_DIR1: &'static str = "mkdir_test1";
static TEST_DIR2: &'static str = "mkdir_test2";
static TEST_DIR3: &'static str = "mkdir_test3";
static TEST_DIR4: &'static str = "mkdir_test4/mkdir_test4_1";
static TEST_DIR5: &'static str = "mkdir_test5/mkdir_test5_1";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_mkdir_mkdir() {
    let mut cmd = Command::new(PROGNAME);
    let exit_success = run(&mut cmd.arg(TEST_DIR1)).success;
    cleanup(TEST_DIR1);
    assert_eq!(exit_success, true);
}

#[test]
fn test_mkdir_dup_dir() {
    let mut cmd = Command::new(PROGNAME);
    let exit_success = run(&mut cmd.arg(TEST_DIR2)).success;
    if !exit_success {
        cleanup(TEST_DIR2);
        panic!();
    }
    let exit_success2 = run(&mut cmd.arg(TEST_DIR2)).success;
    cleanup(TEST_DIR2);
    assert_eq!(exit_success2, false);
}

#[test]
fn test_mkdir_mode() {
    let mut cmd = Command::new(PROGNAME);
    let exit_success = run(&mut cmd.arg("-m").arg("755").arg(TEST_DIR3)).success;
    cleanup(TEST_DIR3);
    assert_eq!(exit_success, true);
}

#[test]
fn test_mkdir_parent() {
    let mut cmd = Command::new(PROGNAME);
    let exit_success = run(&mut cmd.arg("-p").arg(TEST_DIR4)).success;
    cleanup(TEST_DIR4);
    assert_eq!(exit_success, true);
}

#[test]
fn test_mkdir_no_parent() {
    let mut cmd = Command::new(PROGNAME);
    let exit_success = run(&mut cmd.arg(TEST_DIR5)).success;
    cleanup(TEST_DIR5);
    assert_eq!(exit_success, false);
}
