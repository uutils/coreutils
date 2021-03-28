// ToDO: add more tests

use crate::common::util::*;

#[test]
fn test_shuf_echo_and_input_range_not_allowed() {
    let result = new_ucmd!().args(&["-e", "0", "-i", "0-2"]).run();

    assert!(!result.success);
    assert!(result
        .stderr
        .contains("The argument '--input-range <LO-HI>' cannot be used with '--echo <ARG>...'"));
}

#[test]
fn test_shuf_input_range_and_file_not_allowed() {
    let result = new_ucmd!().args(&["-i", "0-9", "file"]).run();

    assert!(!result.success);
    assert!(result
        .stderr
        .contains("The argument '<file>' cannot be used with '--input-range <LO-HI>'"));
}

#[test]
fn test_shuf_invalid_input_range_one() {
    let result = new_ucmd!().args(&["-i", "0"]).run();

    assert!(!result.success);
    assert!(result.stderr.contains("invalid input range"));
}

#[test]
fn test_shuf_invalid_input_range_two() {
    let result = new_ucmd!().args(&["-i", "a-9"]).run();

    assert!(!result.success);
    assert!(result.stderr.contains("invalid input range: 'a'"));
}

#[test]
fn test_shuf_invalid_input_range_three() {
    let result = new_ucmd!().args(&["-i", "0-b"]).run();

    assert!(!result.success);
    assert!(result.stderr.contains("invalid input range: 'b'"));
}

#[test]
fn test_shuf_invalid_input_line_count() {
    let result = new_ucmd!().args(&["-n", "a"]).run();

    assert!(!result.success);
    assert!(result.stderr.contains("invalid line count: 'a'"));
}
