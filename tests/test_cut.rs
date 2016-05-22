use common::util::*;

static UTIL_NAME: &'static str = "cut";

static INPUT: &'static str = "lists.txt";


#[test]
fn test_prefix() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-c", "-10", INPUT]).run();
    assert_eq!(result.stdout, at.read("lists_prefix.expected"));
}

#[test]
fn test_char_range() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-c", "4-10", INPUT]).run();
    assert_eq!(result.stdout, at.read("lists_char_range.expected"));
}

#[test]
fn test_column_to_end_of_line() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-d", ":", "-f", "5-", INPUT]).run();
    assert_eq!(result.stdout,
               at.read("lists_column_to_end_of_line.expected"));
}

#[test]
fn test_specific_field() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-d", " ", "-f", "3", INPUT]).run();
    assert_eq!(result.stdout, at.read("lists_specific_field.expected"));
}

#[test]
fn test_multiple_fields() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-d", ":", "-f", "1,3", INPUT]).run();
    assert_eq!(result.stdout, at.read("lists_multiple_fields.expected"));
}

#[test]
fn test_tail() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-d", ":", "--complement", "-f", "1", INPUT]).run();
    assert_eq!(result.stdout, at.read("lists_tail.expected"));
}

#[test]
fn test_change_delimiter() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let result = ucmd.args(&["-d", ":", "--complement", "--output-delimiter=#", "-f", "1", INPUT])
                     .run();
    assert_eq!(result.stdout, at.read("lists_change_delimiter.expected"));
}
