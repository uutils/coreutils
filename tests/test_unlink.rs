use common::util::*;


#[test]
fn test_unlink_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_unlink_file";

    at.touch(file);

    ucmd.arg(file).succeeds().no_stderr();

    assert!(!at.file_exists(file));
}

#[test]
fn test_unlink_multiple_files() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_unlink_multiple_file_a";
    let file_b = "test_unlink_multiple_file_b";

    at.touch(file_a);
    at.touch(file_b);

    ucmd.arg(file_a).arg(file_b).fails()
        .stderr_is("unlink: error: extra operand: 'test_unlink_multiple_file_b'\nTry 'unlink --help' \
                for more information.\n");
}

#[test]
fn test_unlink_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_unlink_empty_directory";

    at.mkdir(dir);

    ucmd.arg(dir).fails()
        .stderr_is("unlink: error: cannot unlink 'test_unlink_empty_directory': Not a regular file \
                or symlink\n");
}

#[test]
fn test_unlink_nonexistent() {
    let file = "test_unlink_nonexistent";

    new_ucmd!().arg(file).fails()
        .stderr_is("unlink: error: Cannot stat 'test_unlink_nonexistent': No such file or directory \
                (os error 2)\n");
}
