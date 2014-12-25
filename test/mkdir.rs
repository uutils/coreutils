use std::io::process::Command;
use std::io::fs::{rmdir, PathExtensions};
use std::borrow::ToOwned;

static EXE: &'static str = "./mkdir";
static TEST_DIR1: &'static str = "mkdir_test1";
static TEST_DIR2: &'static str = "mkdir_test2";
static TEST_DIR3: &'static str = "mkdir_test3";
static TEST_DIR4: &'static str = "mkdir_test4/mkdir_test4_1";
static TEST_DIR5: &'static str = "mkdir_test5/mkdir_test5_1";

fn cleanup(dir: &'static str) {
    let d = dir.to_owned();
    let p = Path::new(d.to_owned());
    if p.exists() {
        rmdir(&p).unwrap();
    }
}

#[test]
fn test_mkdir_mkdir() {
    cleanup(TEST_DIR1);
    let prog = Command::new(EXE).arg(TEST_DIR1).status();
    let exit_success = prog.unwrap().success();
    cleanup(TEST_DIR1);
    assert_eq!(exit_success, true);
}

#[test]
fn test_mkdir_dup_dir() {
    cleanup(TEST_DIR2);
    let prog = Command::new(EXE).arg(TEST_DIR2).status();
    let exit_success = prog.unwrap().success();
    if !exit_success {
        cleanup(TEST_DIR2);
        panic!();
    }
    let prog2 = Command::new(EXE).arg(TEST_DIR2).status();
    let exit_success2 = prog2.unwrap().success();
    cleanup(TEST_DIR2);
    assert_eq!(exit_success2, false);
}

#[test]
fn test_mkdir_mode() {
    cleanup(TEST_DIR3);
    let prog = Command::new(EXE).arg("-m").arg("755").arg(TEST_DIR3).status();
    let exit_success = prog.unwrap().success();
    cleanup(TEST_DIR3);
    assert_eq!(exit_success, true);
}

#[test]
fn test_mkdir_parent() {
    cleanup(TEST_DIR4);
    let prog = Command::new(EXE).arg("-p").arg(TEST_DIR4).status();
    let exit_success = prog.unwrap().success();
    cleanup(TEST_DIR4);
    assert_eq!(exit_success, true);
}

#[test]
fn test_mkdir_no_parent() {
    cleanup(TEST_DIR5);
    let prog = Command::new(EXE).arg(TEST_DIR5).status();
    let exit_success = prog.unwrap().success();
    cleanup(TEST_DIR5);
    assert_eq!(exit_success, false);
}
