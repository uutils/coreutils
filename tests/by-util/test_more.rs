use crate::common::util::*;

#[test]
fn test_more_no_arg() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.run();
    assert!(!result.success);
}

#[test]
fn test_more_dir_arg() {
    let (_, mut ucmd) = at_and_ucmd!();
    ucmd.arg(".");
    let result = ucmd.run();
    assert!(!result.success);
    const EXPECTED_ERROR_MESSAGE: &str =
        "more: '.' is a directory.\nTry 'more --help' for more information.";
    assert_eq!(result.stderr.trim(), EXPECTED_ERROR_MESSAGE);
}
