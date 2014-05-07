use std::io::process::Process;
use std::io::fs::rmdir;

static exe: &'static str = "build/mkdir";
static test_dir1: &'static str = "tmp/mkdir_test1";
static test_dir2: &'static str = "tmp/mkdir_test2";
static test_dir3: &'static str = "tmp/mkdir_test3";
static test_dir4: &'static str = "tmp/mkdir_test4/mkdir_test4_1";
static test_dir5: &'static str = "tmp/mkdir_test5/mkdir_test5_1";

fn cleanup(dir: &'static str) {
    let d = dir.into_owned();
    let p = Path::new(d.into_owned());
    if p.exists() {
        rmdir(&p);
    }
}

#[test]
fn test_mkdir_mkdir() {
    cleanup(test_dir1);
    let prog = Process::status(exe.into_owned(), [test_dir1.into_owned()]);
    let exit_success = prog.unwrap().success();
    cleanup(test_dir1);
    assert_eq!(exit_success, true);
}

#[test]
fn test_mkdir_dup_dir() {
    cleanup(test_dir2);
    let prog = Process::status(exe.into_owned(), [test_dir2.into_owned()]);
    let exit_success = prog.unwrap().success();
    if !exit_success {
        cleanup(test_dir2);
        fail!();
    }
    let prog2 = Process::status(exe.into_owned(), [test_dir2.into_owned()]);
    let exit_success2 = prog2.unwrap().success();
    cleanup(test_dir2);
    assert_eq!(exit_success2, false);
}

#[test]
fn test_mkdir_mode() {
    cleanup(test_dir3);
    let prog = Process::status(exe.into_owned(), ["-m".to_owned(), "755".to_owned(), test_dir3.into_owned()]);
    let exit_success = prog.unwrap().success();
    cleanup(test_dir3);
    assert_eq!(exit_success, true);
}

#[test]
fn test_mkdir_parent() {
    cleanup(test_dir4);
    let prog = Process::status(exe.into_owned(), ["-p".to_owned(), test_dir4.into_owned()]);
    let exit_success = prog.unwrap().success();
    cleanup(test_dir4);
    assert_eq!(exit_success, true);
}

#[test]
fn test_mkdir_no_parent() {
    cleanup(test_dir5);
    let prog = Process::status(exe.into_owned(), [test_dir5.into_owned()]);
    let exit_success = prog.unwrap().success();
    cleanup(test_dir5);
    assert_eq!(exit_success, false);
}
