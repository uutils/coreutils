use common::util::*;

static UTIL_NAME: &'static str = "unlink";

fn at_and_ucmd() -> (AtPath, UCommand) {
    let ts = TestScenario::new(UTIL_NAME);
    let ucmd = ts.ucmd();
    (ts.fixtures, ucmd)
}

fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_unlink_file() {
    let (at, mut ucmd) = at_and_ucmd();
    let file = "test_unlink_file";

    at.touch(file);

    let result = ucmd.arg(file).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.file_exists(file));
}

#[test]
fn test_unlink_multiple_files() {
    let (at, mut ucmd) = at_and_ucmd();
    let file_a = "test_unlink_multiple_file_a";
    let file_b = "test_unlink_multiple_file_b";

    at.touch(file_a);
    at.touch(file_b);

    let result = ucmd.arg(file_a).arg(file_b).run();
    assert_eq!(result.stderr,
               "unlink: error: extra operand: 'test_unlink_multiple_file_b'\nTry 'unlink --help' \
                for more information.\n");
    assert!(!result.success);
}

#[test]
fn test_unlink_directory() {
    let (at, mut ucmd) = at_and_ucmd();
    let dir = "test_unlink_empty_directory";

    at.mkdir(dir);

    let result = ucmd.arg(dir).run();
    assert_eq!(result.stderr,
               "unlink: error: cannot unlink 'test_unlink_empty_directory': Not a regular file \
                or symlink\n");
    assert!(!result.success);
}

#[test]
fn test_unlink_nonexistent() {
    let file = "test_unlink_nonexistent";

    let result = new_ucmd().arg(file).run();
    assert_eq!(result.stderr,
               "unlink: error: Cannot stat 'test_unlink_nonexistent': No such file or directory \
                (os error 2)\n");
    assert!(!result.success);
}
