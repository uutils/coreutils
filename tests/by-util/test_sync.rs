use crate::common::util::TestScenario;
extern crate tempfile;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

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

#[test]
fn test_sync_data_but_not_file() {
    new_ucmd!()
        .arg("--data")
        .fails()
        .stderr_contains("sync: --data needs at least one argument");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[cfg(feature = "chmod")]
#[test]
fn test_sync_no_permission_dir() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let dir = "foo";
    at.mkdir_all(dir);

    ts.ccmd("chmod").arg("0").arg(dir).succeeds();
    let result = ts.ucmd().arg("--data").arg(dir).fails();
    result.stderr_contains("sync: cannot stat 'foo': Permission denied");
    let result = ts.ucmd().arg(dir).fails();
    result.stderr_contains("sync: cannot stat 'foo': Permission denied");
}

#[cfg(not(target_os = "windows"))]
#[cfg(feature = "chmod")]
#[test]
fn test_sync_no_permission_file() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let f = "file";
    at.touch(f);

    ts.ccmd("chmod").arg("0200").arg(f).succeeds();
    ts.ucmd().arg(f).succeeds();
}
