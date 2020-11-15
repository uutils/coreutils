use crate::common::util::*;

use std::fs;
extern crate tempfile;
use self::tempfile::tempdir;

#[test]
fn test_sync_default() {
    let result = new_ucmd!().run();
    assert!(result.success);
}

#[test]
fn test_sync_incorrect_arg() {
    new_ucmd!().arg("--foo").fails();
}

#[test]
fn test_sync_fs() {
    let temporary_directory = tempdir().unwrap();
    let temporary_path = fs::canonicalize(temporary_directory.path()).unwrap();
    let result = new_ucmd!().arg("--file-system").arg(&temporary_path).run();
    assert!(result.success);
}

#[test]
fn test_sync_data() {
    // Todo add a second arg
    let temporary_directory = tempdir().unwrap();
    let temporary_path = fs::canonicalize(temporary_directory.path()).unwrap();
    let result = new_ucmd!().arg("--data").arg(&temporary_path).run();
    assert!(result.success);
}

#[test]
fn test_sync_no_existing_files() {
    let result = new_ucmd!().arg("--data").arg("do-no-exist").fails();
    assert!(result.stderr.contains("error: cannot stat"));
}
