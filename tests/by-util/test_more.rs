use crate::common::util::*;

#[test]
fn test_more_no_arg() {
    // stderr = more: Reading from stdin isn't supported yet.
    new_ucmd!().fails();
}

#[test]
fn test_more_dir_arg() {
    let result = new_ucmd!().arg(".").run();
    result.failure();
    const EXPECTED_ERROR_MESSAGE: &str =
        "more: '.' is a directory.\nTry 'more --help' for more information.";
    assert_eq!(result.stderr_str().trim(), EXPECTED_ERROR_MESSAGE);
}
