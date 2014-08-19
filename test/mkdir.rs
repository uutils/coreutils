use std::io::process::Command;
use std::io::fs::rmdir;

static exe: &'static str = "./mkdir";
static test_dir1: &'static str = "mkdir_test1";
static test_dir2: &'static str = "mkdir_test2";
static test_dir3: &'static str = "mkdir_test3";
static test_dir4: &'static str = "mkdir_test4/mkdir_test4_1";
static test_dir5: &'static str = "mkdir_test5/mkdir_test5_1";

fn cleanup(dir: &'static str) {
    let d = dir.into_string();
    let p = Path::new(d.into_string());
    if p.exists() {
        rmdir(&p).unwrap();
    }
}

#[test]
fn test_mkdir_mkdir() {
    cleanup(test_dir1);
    let prog = Command::new(exe).arg(test_dir1).status();
    let exit_success = prog.unwrap().success();
    cleanup(test_dir1);
    assert_eq!(exit_success, true);
}

#[test]
fn test_mkdir_dup_dir() {
    cleanup(test_dir2);
    let prog = Command::new(exe).arg(test_dir2).status();
    let exit_success = prog.unwrap().success();
    if !exit_success {
        cleanup(test_dir2);
        fail!();
    }
    let prog2 = Command::new(exe).arg(test_dir2).status();
    let exit_success2 = prog2.unwrap().success();
    cleanup(test_dir2);
    assert_eq!(exit_success2, false);
}

#[test]
fn test_mkdir_mode() {
    cleanup(test_dir3);
    let prog = Command::new(exe).arg("-m").arg("755").arg(test_dir3).status();
    let exit_success = prog.unwrap().success();
    cleanup(test_dir3);
    assert_eq!(exit_success, true);
}

#[test]
fn test_mkdir_parent() {
    cleanup(test_dir4);
    let prog = Command::new(exe).arg("-p").arg(test_dir4).status();
    let exit_success = prog.unwrap().success();
    cleanup(test_dir4);
    assert_eq!(exit_success, true);
}

#[test]
fn test_mkdir_no_parent() {
    cleanup(test_dir5);
    let prog = Command::new(exe).arg(test_dir5).status();
    let exit_success = prog.unwrap().success();
    cleanup(test_dir5);
    assert_eq!(exit_success, false);
}
