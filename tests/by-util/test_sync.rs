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
fn test_sync_fs_without_files_falls_back_to_full_sync() {
    new_ucmd!().arg("--file-system").succeeds();
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
#[test]
fn test_sync_no_permission_dir() {
    use std::os::unix::fs::PermissionsExt as _;
    use uutests::util::TestScenario;
    use uutests::util_name;

    // Root bypasses permission checks, so a mode-0000 directory can
    // still be opened and the command succeeds instead of failing.
    // The test is meaningless as root; skip it (same pattern as
    // test_chcon.rs).
    if rustix::process::geteuid().is_root() {
        println!("test skipped: root can open any directory");
        return;
    }

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let dir = "foo";
    at.mkdir_all(dir);
    let path = at.subdir.as_path().join(dir);
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o0);
    fs::set_permissions(&path, perms).unwrap();

    let result = ts.ucmd().arg("--data").arg(dir).fails();
    result.stderr_contains("sync: error opening 'foo': Permission denied");
    let result = ts.ucmd().arg(dir).fails();
    result.stderr_contains("sync: error opening 'foo': Permission denied");
}

#[cfg(not(windows))]
#[test]
fn test_sync_no_permission_file() {
    use std::os::unix::fs::PermissionsExt as _;
    use uutests::util::TestScenario;
    use uutests::util_name;

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let f = "file";
    at.touch(f);
    let path = at.subdir.as_path().join(f);
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o200);
    fs::set_permissions(&path, perms).unwrap();

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

// Regression tests for issue #14591: `sync FILE` with no -d/-f flags must
// fsync each operand like GNU (MODE_FILE), not fall back to a global sync()
// that ignores the operands. The core trick is an operand that can be
// opened but not synced: only a real per-file fsync fails on it, while a
// global sync() would succeed and exit 0.
#[test]
fn test_sync_file_operands_fsync_each_file() {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // /dev/null opens fine, but fsync fails on it with EINVAL.
        // One device is enough to prove per-file handling.
        // Linux/Android only: on macOS (XNU) fsync on a character
        // device returns 0 (spec_fsync: `if (v_type == VCHR) return 0`),
        // so /dev/null would NOT fail there and this test would be
        // vacuous. macOS is covered by the missing-file test instead.
        new_ucmd!()
            .arg("/dev/null")
            .fails_with_code(1)
            .stderr_contains("error syncing");
    }
    #[cfg(windows)]
    {
        // NUL is the Windows counterpart of /dev/null, but it fails
        // earlier: `sync NUL` errors at OPEN ("No such file or
        // directory") before fsync is reached, so assert the open
        // error instead. This still proves per-file processing: without
        // the fix, `sync NUL` exits 0 (global sync ignores the
        // operand). (The read-only test below is the deterministic
        // Windows case.)
        new_ucmd!()
            .arg("NUL")
            .fails_with_code(1)
            .stderr_contains("error opening");
    }
}

#[test]
fn test_sync_file_operands_no_flags() {
    // Happy path, no flags, on every platform: a regular file syncs fine.
    let temporary_directory = tempdir().unwrap();
    let file = temporary_directory.path().join("file.txt");
    fs::write(&file, "content").unwrap();
    new_ucmd!().arg(&file).succeeds();
}

#[test]
fn test_sync_file_operands_multiple_no_flags() {
    // Several files, no flags: each one is synced.
    let temporary_directory = tempdir().unwrap();
    let temp_path = temporary_directory.path();
    let file1 = temp_path.join("file1.txt");
    let file2 = temp_path.join("file2.txt");
    fs::write(&file1, "content1").unwrap();
    fs::write(&file2, "content2").unwrap();
    new_ucmd!().arg(&file1).arg(&file2).succeeds();
}

#[test]
fn test_sync_file_operands_one_bad_one_good() {
    // One good file and one that cannot be synced: exit 1, and the error
    // names the bad one. Proves files are handled one by one.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let temporary_directory = tempdir().unwrap();
        let good = temporary_directory.path().join("good.txt");
        fs::write(&good, "content").unwrap();
        new_ucmd!()
            .arg(&good)
            .arg("/dev/null")
            .fails_with_code(1)
            .stderr_contains("error syncing")
            .stderr_contains("/dev/null");
    }
    #[cfg(target_os = "macos")]
    {
        // macOS has no /dev/null-style fsync failure (XNU returns 0 for
        // character devices), so the bad operand here is a missing file:
        // it still proves files are handled one by one, and the error
        // names the bad operand.
        let temporary_directory = tempdir().unwrap();
        let good = temporary_directory.path().join("good.txt");
        fs::write(&good, "content").unwrap();
        let bad = temporary_directory.path().join("missing.txt");
        new_ucmd!()
            .arg(&good)
            .arg(&bad)
            .fails_with_code(1)
            .stderr_contains("error opening")
            .stderr_contains("missing.txt");
    }
    #[cfg(windows)]
    {
        let temporary_directory = tempdir().unwrap();
        let good = temporary_directory.path().join("good.txt");
        fs::write(&good, "content").unwrap();
        let bad = temporary_directory.path().join("read-only.txt");
        fs::write(&bad, "content").unwrap();
        let mut perms = fs::metadata(&bad).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&bad, perms).unwrap();
        new_ucmd!()
            .arg(&good)
            .arg(&bad)
            .fails_with_code(1)
            .stderr_contains("read-only.txt");
    }
}

#[cfg(windows)]
#[test]
fn test_sync_file_operands_read_only_file() {
    // Windows only: a read-only file cannot be opened for write, and the
    // Windows fsync path needs a write handle. With per-file handling
    // this fails; a global sync would silently exit 0.
    let temporary_directory = tempdir().unwrap();
    let file = temporary_directory.path().join("read-only.txt");
    fs::write(&file, "content").unwrap();
    let mut perms = fs::metadata(&file).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&file, perms).unwrap();
    new_ucmd!()
        .arg(&file)
        .fails_with_code(1)
        .stderr_contains("read-only.txt");
}

#[test]
fn test_sync_file_operands_missing_file_no_flags() {
    // A missing file, no flags: the operand is reported, not ignored.
    new_ucmd!()
        .arg("do-not-exist")
        .fails()
        .stderr_contains("error opening");
}

#[cfg(target_vendor = "apple")]
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

#[test]
fn test_sync_multiple_nonexistent_files() {
    let result = new_ucmd!().arg("--data").arg("bad1").arg("bad2").fails();

    result.stderr_is("sync: error opening 'bad1': No such file or directory\nsync: error opening 'bad2': No such file or directory\n");
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
        .arg(at.plus("test-fifo"))
        .timeout(Duration::from_secs(2))
        .fails()
        .stderr_contains("error syncing")
        .stderr_contains("test-fifo")
        .stderr_contains("Invalid input");
}
