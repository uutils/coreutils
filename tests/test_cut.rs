use common::util::*;

static UTIL_NAME: &'static str = "cut";
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

static INPUT: &'static str = "lists.txt";


#[test]
fn test_prefix() {
    new_ucmd().args(&["-c", "-10", INPUT]).run().stdout_is_fixture("lists_prefix.expected");
}

#[test]
fn test_char_range() {
    new_ucmd().args(&["-c", "4-10", INPUT]).run().stdout_is_fixture("lists_char_range.expected");
}

#[test]
fn test_column_to_end_of_line() {
    new_ucmd().args(&["-d", ":", "-f", "5-", INPUT]).run().stdout_is_fixture("lists_column_to_end_of_line.expected");
}

#[test]
fn test_specific_field() {
    new_ucmd().args(&["-d", " ", "-f", "3", INPUT]).run().stdout_is_fixture("lists_specific_field.expected");
}

#[test]
fn test_multiple_fields() {
    new_ucmd().args(&["-d", ":", "-f", "1,3", INPUT]).run().stdout_is_fixture("lists_multiple_fields.expected");
}

#[test]
fn test_tail() {
    new_ucmd().args(&["-d", ":", "--complement", "-f", "1", INPUT]).run().stdout_is_fixture("lists_tail.expected");
}

#[test]
fn test_change_delimiter() {
    new_ucmd()
        .args(&["-d", ":", "--complement", "--output-delimiter=#", "-f", "1", INPUT])
        .run().stdout_is_fixture("lists_change_delimiter.expected");
}
