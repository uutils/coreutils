use crate::common::util::*;
extern crate tempfile;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_sync_default() {
    new_ucmd!().succeeds();
}

#[test]
fn test_sync_incorrect_arg() {
    new_ucmd!().arg("--foo").fails();
}

#[test]
fn test_sync_fs() {
    let temporary_directory = tempdir().unwrap();
    let temporary_path = fs::canonicalize(temporary_directory.path()).unwrap();
    new_ucmd!()
        .arg("--file-system")
        .arg(&temporary_path)
        .succeeds();
}

#[test]
fn test_sync_data() {
    // Todo add a second arg
    let temporary_directory = tempdir().unwrap();
    let temporary_path = fs::canonicalize(temporary_directory.path()).unwrap();
    new_ucmd!().arg("--data").arg(&temporary_path).succeeds();
}

#[test]
fn test_sync_no_existing_files() {
    new_ucmd!()
        .arg("--data")
        .arg("do-no-exist")
        .fails()
        .stderr_contains("cannot stat");
}
