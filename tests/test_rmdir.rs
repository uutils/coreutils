use common::util::*;


#[test]
fn test_rmdir_empty_directory_no_parents() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rmdir_empty_no_parents";

    at.mkdir(dir);
    assert!(at.dir_exists(dir));

    ucmd.arg(dir).succeeds().no_stderr();

    assert!(!at.dir_exists(dir));
}

#[test]
fn test_rmdir_empty_directory_with_parents() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rmdir_empty/with/parents";

    at.mkdir_all(dir);
    assert!(at.dir_exists(dir));

    ucmd.arg("-p").arg(dir).succeeds().no_stderr();

    assert!(!at.dir_exists(dir));
}

#[test]
fn test_rmdir_nonempty_directory_no_parents() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rmdir_nonempty_no_parents";
    let file = "test_rmdir_nonempty_no_parents/foo";

    at.mkdir(dir);
    assert!(at.dir_exists(dir));

    at.touch(file);
    assert!(at.file_exists(file));

    ucmd.arg(dir).fails()
        .stderr_is("rmdir: error: failed to remove 'test_rmdir_nonempty_no_parents': Directory not \
                empty\n");

    assert!(at.dir_exists(dir));
}

#[test]
fn test_rmdir_nonempty_directory_with_parents() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rmdir_nonempty/with/parents";
    let file = "test_rmdir_nonempty/with/parents/foo";

    at.mkdir_all(dir);
    assert!(at.dir_exists(dir));

    at.touch(file);
    assert!(at.file_exists(file));

    ucmd.arg("-p").arg(dir).fails()
        .stderr_is(
               "rmdir: error: failed to remove 'test_rmdir_nonempty/with/parents': Directory not \
                empty\nrmdir: error: failed to remove 'test_rmdir_nonempty/with': Directory not \
                empty\nrmdir: error: failed to remove 'test_rmdir_nonempty': Directory not \
                empty\n");

    assert!(at.dir_exists(dir));
}

#[test]
fn test_rmdir_ignore_nonempty_directory_no_parents() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rmdir_ignore_nonempty_no_parents";
    let file = "test_rmdir_ignore_nonempty_no_parents/foo";

    at.mkdir(dir);
    assert!(at.dir_exists(dir));

    at.touch(file);
    assert!(at.file_exists(file));

    ucmd.arg("--ignore-fail-on-non-empty").arg(dir).succeeds().no_stderr();

    assert!(at.dir_exists(dir));
}

#[test]
fn test_rmdir_ignore_nonempty_directory_with_parents() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rmdir_ignore_nonempty/with/parents";
    let file = "test_rmdir_ignore_nonempty/with/parents/foo";

    at.mkdir_all(dir);
    assert!(at.dir_exists(dir));

    at.touch(file);
    assert!(at.file_exists(file));

    ucmd.arg("--ignore-fail-on-non-empty").arg("-p").arg(dir).succeeds().no_stderr();

    assert!(at.dir_exists(dir));
}
