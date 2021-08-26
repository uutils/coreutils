use crate::common::util::*;

const DIR: &str = "dir";
const DIR_FILE: &str = "dir/file";
const NESTED_DIR: &str = "dir/ect/ory";
const NESTED_DIR_FILE: &str = "dir/ect/ory/file";

#[cfg(windows)]
const NOT_FOUND: &str = "The system cannot find the file specified.";
#[cfg(not(windows))]
const NOT_FOUND: &str = "No such file or directory";

#[cfg(windows)]
const NOT_EMPTY: &str = "The directory is not empty.";
#[cfg(not(windows))]
const NOT_EMPTY: &str = "Directory not empty";

#[cfg(windows)]
const NOT_A_DIRECTORY: &str = "The directory name is invalid.";
#[cfg(not(windows))]
const NOT_A_DIRECTORY: &str = "Not a directory";

#[test]
fn test_rmdir_empty_directory_no_parents() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir(DIR);

    ucmd.arg(DIR).succeeds().no_stderr();

    assert!(!at.dir_exists(DIR));
}

#[test]
fn test_rmdir_empty_directory_with_parents() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir_all(NESTED_DIR);

    ucmd.arg("-p").arg(NESTED_DIR).succeeds().no_stderr();

    assert!(!at.dir_exists(NESTED_DIR));
    assert!(!at.dir_exists(DIR));
}

#[test]
fn test_rmdir_nonempty_directory_no_parents() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir(DIR);
    at.touch(DIR_FILE);

    ucmd.arg(DIR)
        .fails()
        .stderr_is(format!("rmdir: failed to remove 'dir': {}", NOT_EMPTY));

    assert!(at.dir_exists(DIR));
}

#[test]
fn test_rmdir_nonempty_directory_with_parents() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir_all(NESTED_DIR);
    at.touch(NESTED_DIR_FILE);

    ucmd.arg("-p").arg(NESTED_DIR).fails().stderr_is(format!(
        "rmdir: failed to remove 'dir/ect/ory': {}",
        NOT_EMPTY
    ));

    assert!(at.dir_exists(NESTED_DIR));
}

#[test]
fn test_rmdir_ignore_nonempty_directory_no_parents() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir(DIR);
    at.touch(DIR_FILE);

    ucmd.arg("--ignore-fail-on-non-empty")
        .arg(DIR)
        .succeeds()
        .no_stderr();

    assert!(at.dir_exists(DIR));
}

#[test]
fn test_rmdir_ignore_nonempty_directory_with_parents() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir_all(NESTED_DIR);
    at.touch(NESTED_DIR_FILE);

    ucmd.arg("--ignore-fail-on-non-empty")
        .arg("-p")
        .arg(NESTED_DIR)
        .succeeds()
        .no_stderr();

    assert!(at.dir_exists(NESTED_DIR));
}

#[test]
fn test_rmdir_not_a_directory() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("file");

    ucmd.arg("--ignore-fail-on-non-empty")
        .arg("file")
        .fails()
        .no_stdout()
        .stderr_is(format!(
            "rmdir: failed to remove 'file': {}",
            NOT_A_DIRECTORY
        ));
}

#[test]
fn test_verbose_single() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir(DIR);

    ucmd.arg("-v")
        .arg(DIR)
        .succeeds()
        .no_stderr()
        .stdout_is("rmdir: removing directory, 'dir'\n");
}

#[test]
fn test_verbose_multi() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir(DIR);

    ucmd.arg("-v")
        .arg("does_not_exist")
        .arg(DIR)
        .fails()
        .stdout_is(
            "rmdir: removing directory, 'does_not_exist'\n\
             rmdir: removing directory, 'dir'\n",
        )
        .stderr_is(format!(
            "rmdir: failed to remove 'does_not_exist': {}",
            NOT_FOUND
        ));
}

#[test]
fn test_verbose_nested_failure() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir_all(NESTED_DIR);
    at.touch("dir/ect/file");

    ucmd.arg("-pv")
        .arg(NESTED_DIR)
        .fails()
        .stdout_is(
            "rmdir: removing directory, 'dir/ect/ory'\n\
             rmdir: removing directory, 'dir/ect'\n",
        )
        .stderr_is(format!("rmdir: failed to remove 'dir/ect': {}", NOT_EMPTY));
}

#[cfg(unix)]
#[test]
fn test_rmdir_ignore_nonempty_no_permissions() {
    use std::fs;

    let (at, mut ucmd) = at_and_ucmd!();

    // We make the *parent* dir read-only to prevent deleting the dir in it.
    at.mkdir_all("dir/ect/ory");
    at.touch("dir/ect/ory/file");
    let dir_ect = at.plus("dir/ect");
    let mut perms = fs::metadata(&dir_ect).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&dir_ect, perms.clone()).unwrap();

    // rmdir should now get a permissions error that it interprets as
    // a non-empty error.
    ucmd.arg("--ignore-fail-on-non-empty")
        .arg("dir/ect/ory")
        .succeeds()
        .no_stderr();

    assert!(at.dir_exists("dir/ect/ory"));

    // Politely restore permissions for cleanup
    perms.set_readonly(false);
    fs::set_permissions(&dir_ect, perms).unwrap();
}

#[test]
fn test_rmdir_remove_symlink_file() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("file");
    at.symlink_file("file", "fl");

    ucmd.arg("fl/").fails().stderr_is(format!(
        "rmdir: failed to remove 'fl/': {}",
        NOT_A_DIRECTORY
    ));
}

// This behavior is known to happen on Linux but not all Unixes
#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_rmdir_remove_symlink_dir() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("dir");
    at.symlink_dir("dir", "dl");

    ucmd.arg("dl/")
        .fails()
        .stderr_is("rmdir: failed to remove 'dl/': Symbolic link not followed");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_rmdir_remove_symlink_dangling() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.symlink_dir("dir", "dl");

    ucmd.arg("dl/")
        .fails()
        .stderr_is("rmdir: failed to remove 'dl/': Symbolic link not followed");
}
