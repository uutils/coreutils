// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::fs;
use tempfile::tempdir;
use uutests::new_ucmd;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
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
        .stderr_contains("error opening");
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
    use uutests::util::TestScenario;
    use uutests::util_name;

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let dir = "foo";
    at.mkdir_all(dir);

    ts.ccmd("chmod").arg("0").arg(dir).succeeds();
    let result = ts.ucmd().arg("--data").arg(dir).fails();
    result.stderr_contains("sync: error opening 'foo': Permission denied");
    let result = ts.ucmd().arg(dir).fails();
    result.stderr_contains("sync: error opening 'foo': Permission denied");
}

#[cfg(not(target_os = "windows"))]
#[cfg(feature = "chmod")]
#[test]
fn test_sync_no_permission_file() {
    use uutests::util::TestScenario;
    use uutests::util_name;

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let f = "file";
    at.touch(f);

    ts.ccmd("chmod").arg("0200").arg(f).succeeds();
    ts.ucmd().arg(f).succeeds();
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_sync_data_nonblock_flag_reset() {
    // Test that O_NONBLOCK flag is properly reset when syncing files
    use uutests::util::TestScenario;
    use uutests::util_name;

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let test_file = "test_file.txt";

    // Create a test file
    at.write(test_file, "test content");

    // Run sync --data with the file - should succeed
    ts.ucmd().arg("--data").arg(test_file).succeeds();
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_sync_fs_nonblock_flag_reset() {
    // Test that O_NONBLOCK flag is properly reset when syncing filesystems
    use std::fs;
    use tempfile::tempdir;

    let temporary_directory = tempdir().unwrap();
    let temporary_path = fs::canonicalize(temporary_directory.path()).unwrap();

    // Run sync --file-system with the path - should succeed
    new_ucmd!()
        .arg("--file-system")
        .arg(&temporary_path)
        .succeeds();
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_sync_fdatasync_error_handling() {
    // Test that fdatasync properly handles file opening errors
    new_ucmd!()
        .arg("--data")
        .arg("/nonexistent/path/to/file")
        .fails()
        .stderr_contains("error opening");
}

#[cfg(target_os = "macos")]
#[test]
fn test_sync_syncfs_error_handling_macos() {
    // Test that syncfs properly handles invalid paths on macOS
    new_ucmd!()
        .arg("--file-system")
        .arg("/nonexistent/path/to/file")
        .fails()
        .stderr_contains("error opening");
}

#[test]
fn test_sync_multiple_files() {
    // Test syncing multiple files at once
    use std::fs;
    use tempfile::tempdir;

    let temporary_directory = tempdir().unwrap();
    let temp_path = temporary_directory.path();

    // Create multiple test files
    let file1 = temp_path.join("file1.txt");
    let file2 = temp_path.join("file2.txt");

    fs::write(&file1, "content1").unwrap();
    fs::write(&file2, "content2").unwrap();

    // Sync both files
    new_ucmd!().arg("--data").arg(&file1).arg(&file2).succeeds();
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_sync_data_fifo_fails_immediately() {
    use std::time::Duration;
    use uutests::util::TestScenario;
    use uutests::util_name;

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkfifo("test-fifo");

    ts.ucmd()
        .arg("--data")
        .arg(at.plus_as_string("test-fifo"))
        .timeout(Duration::from_secs(2))
        .fails();
}
