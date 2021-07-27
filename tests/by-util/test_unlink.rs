use crate::common::util::*;

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
    let ts = TestScenario::new(util_name!());

    let (at, mut ucmd) = (ts.fixtures.clone(), ts.ucmd());
    let file_a = "test_unlink_multiple_file_a";
    let file_b = "test_unlink_multiple_file_b";

    at.touch(file_a);
    at.touch(file_b);

    ucmd.arg(file_a).arg(file_b).fails().stderr_is(&format!(
        "{0}: extra operand: 'test_unlink_multiple_file_b'\nTry `{1} {0} --help` for more information.",
        ts.util_name,
        ts.bin_path.to_string_lossy()
    ));
}

#[test]
fn test_unlink_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_unlink_empty_directory";

    at.mkdir(dir);

    ucmd.arg(dir).fails().stderr_is(
        "unlink: cannot unlink 'test_unlink_empty_directory': Not a regular file \
         or symlink\n",
    );
}

#[test]
fn test_unlink_nonexistent() {
    let file = "test_unlink_nonexistent";

    new_ucmd!().arg(file).fails().stderr_is(
        "unlink: Cannot stat 'test_unlink_nonexistent': No such file or directory \
         (os error 2)\n",
    );
}
