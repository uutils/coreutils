use std::{run};
use std::io::fs::rmdir;

static exe: &'static str = "build/mkdir";
static test_dir1: &'static str = "mkdir_test1";
static test_dir2: &'static str = "mkdir_test1/mkdir_test2";

fn main() {
    test_mkdir_mkdir();
    test_mkdir_dup_dir();
    test_mkdir_mode();
    test_mkdir_parent();
    test_mkdir_no_parent();
}

fn cleanup() {
    let dirs = [test_dir2, test_dir1];
    for d in dirs.iter() {
        let p = Path::new(d.into_owned());
        if p.exists() {
            rmdir(&p);
        }
    }
}

fn test_mkdir_mkdir() {
    cleanup();
    let prog = run::process_status(exe.into_owned(), [test_dir1.into_owned()]);
    let exit_success = prog.unwrap().success();
    cleanup();
    assert_eq!(exit_success, true);
}

fn test_mkdir_dup_dir() {
    cleanup();
    let prog = run::process_status(exe.into_owned(), [test_dir1.into_owned()]);
    let exit_success = prog.unwrap().success();
    if !exit_success {
        cleanup();
        fail!();
    }
    let prog2 = run::process_status(exe.into_owned(), [test_dir1.into_owned()]);
    let exit_success2 = prog2.unwrap().success();
    cleanup();
    assert_eq!(exit_success2, false);
}

fn test_mkdir_mode() {
    cleanup();
    let prog = run::process_status(exe.into_owned(), [~"-m", ~"755", test_dir1.into_owned()]);
    let exit_success = prog.unwrap().success();
    cleanup();
    assert_eq!(exit_success, true);
}

fn test_mkdir_parent() {
    cleanup();
    let prog = run::process_status(exe.into_owned(), [~"-p", test_dir2.into_owned()]);
    let exit_success = prog.unwrap().success();
    cleanup();
    assert_eq!(exit_success, true);
}

fn test_mkdir_no_parent() {
    cleanup();
    let prog = run::process_status(exe.into_owned(), [test_dir2.into_owned()]);
    let exit_success = prog.unwrap().success();
    cleanup();
    assert_eq!(exit_success, false);
}
