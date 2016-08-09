use common::util::*;

static UTIL_NAME: &'static str = "link";

fn at_and_ucmd() -> (AtPath, UCommand) {
    let ts = TestScenario::new(UTIL_NAME);
    let ucmd = ts.ucmd();
    (ts.fixtures, ucmd)
}

#[test]
fn test_link_existing_file() {
    let (at, mut ucmd) = at_and_ucmd();
    let file = "test_link_existing_file";
    let link = "test_link_existing_file_link";

    at.touch(file);
    at.write(file, "foobar");
    assert!(at.file_exists(file));

    let result = ucmd.args(&[file, link]).run();

    assert_empty_stderr!(result);
    assert!(result.success);
    assert!(at.file_exists(file));
    assert!(at.file_exists(link));
    assert_eq!(at.read(file), at.read(link));
}

#[test]
fn test_link_no_circular() {
    let (at, mut ucmd) = at_and_ucmd();
    let link = "test_link_no_circular";

    let result = ucmd.args(&[link, link]).run();
    assert_eq!(result.stderr,
               "link: error: No such file or directory (os error 2)\n");
    assert!(!result.success);
    assert!(!at.file_exists(link));
}

#[test]
fn test_link_nonexistent_file() {
    let (at, mut ucmd) = at_and_ucmd();
    let file = "test_link_nonexistent_file";
    let link = "test_link_nonexistent_file_link";

    let result = ucmd.args(&[file, link]).run();
    assert_eq!(result.stderr,
               "link: error: No such file or directory (os error 2)\n");
    assert!(!result.success);
    assert!(!at.file_exists(file));
    assert!(!at.file_exists(link));
}
