use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./cut";
static INPUT: &'static str = "lists.txt";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_prefix() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.args(&["-c", "-10", INPUT]));
    assert_eq!(result.stdout, get_file_contents("lists_prefix.expected"));
}

#[test]
fn test_char_range() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.args(&["-c", "4-10", INPUT]));
    assert_eq!(result.stdout, get_file_contents("lists_char_range.expected"));
}

#[test]
fn test_column_to_end_of_line() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.args(&["-d", ":", "-f", "5-", INPUT]));
    assert_eq!(result.stdout, get_file_contents("lists_column_to_end_of_line.expected"));
}

#[test]
fn test_specific_field() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.args(&["-d", " ", "-f", "3", INPUT]));
    assert_eq!(result.stdout, get_file_contents("lists_specific_field.expected"));
}

#[test]
fn test_multiple_fields() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.args(&["-d", ":", "-f", "1,3", INPUT]));
    assert_eq!(result.stdout, get_file_contents("lists_multiple_fields.expected"));
}

#[test]
fn test_tail() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.args(&["-d", ":", "--complement", "-f", "1", INPUT]));
    assert_eq!(result.stdout, get_file_contents("lists_tail.expected"));
}

#[test]
fn test_change_delimiter() {
    let mut cmd = Command::new(PROGNAME);
    let result = run(&mut cmd.args(&["-d", ":", "--complement", "--output-delimiter=#", "-f", "1", INPUT]));
    assert_eq!(result.stdout, get_file_contents("lists_change_delimiter.expected"));
}
