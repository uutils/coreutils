extern crate chrono;

use common::util::*;
use std::fs::metadata;
use test_pr::chrono::DateTime;
use test_pr::chrono::offset::Local;

fn file_last_modified_time(ucmd: &UCommand, path: &str) -> String {
    let tmp_dir_path = ucmd.get_full_fixture_path(path);
    let file_metadata = metadata(tmp_dir_path);
    return file_metadata.map(|i| {
        return i.modified().map(|x| {
            let datetime: DateTime<Local> = x.into();
            datetime.format("%b %d %H:%M %Y").to_string()
        }).unwrap_or(String::new());
    }).unwrap_or(String::new());
}


#[test]
fn test_without_any_options() {
    let test_file_path = "test_one_page.log";
    let expected_test_file_path = "test_one_page.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&[test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, vec![("{last_modified_time}".to_string(), value)]);
}

#[test]
fn test_with_numbering_option() {
    let test_file_path = "test_num_page.log";
    let expected_test_file_path = "test_num_page.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["-n", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, vec![("{last_modified_time}".to_string(), value)]);
}

#[test]
fn test_with_numbering_option_when_content_is_less_than_page() {
    let test_file_path = "test_num_page_less_content.log";
    let expected_test_file_path = "test_num_page_less_content.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["-n", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, vec![("{last_modified_time}".to_string(), value)]);
}

#[test]
fn test_with_numbering_option_with_number_width() {
    let test_file_path = "test_num_page.log";
    let expected_test_file_path = "test_num_page_2.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["-n", "2", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, vec![("{last_modified_time}".to_string(), value)]);
}

#[test]
fn test_with_header_option() {
    let test_file_path = "test_one_page.log";
    let expected_test_file_path = "test_one_page_header.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    let header = "new file";
    scenario
        .args(&["-h", header, test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, vec![
            ("{last_modified_time}".to_string(), value),
            ("{header}".to_string(), header.to_string())
        ]);
}

#[test]
fn test_with_long_header_option() {
    let test_file_path = "test_one_page.log";
    let expected_test_file_path = "test_one_page_header.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    let header = "new file";
    scenario
        .args(&["--header=new file", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, vec![
            ("{last_modified_time}".to_string(), value),
            ("{header}".to_string(), header.to_string())
        ]);
}

#[test]
fn test_with_double_space_option() {
    let test_file_path = "test_one_page.log";
    let expected_test_file_path = "test_one_page_double_line.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["-d", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, vec![
            ("{last_modified_time}".to_string(), value),
        ]);
}

#[test]
fn test_with_long_double_space_option() {
    let test_file_path = "test_one_page.log";
    let expected_test_file_path = "test_one_page_double_line.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["--double-space", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, vec![
            ("{last_modified_time}".to_string(), value),
        ]);
}

#[test]
fn test_with_first_line_number_option() {
    let test_file_path = "test_one_page.log";
    let expected_test_file_path = "test_one_page_first_line.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["-N", "5", "-n", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, vec![
            ("{last_modified_time}".to_string(), value),
        ]);
}

#[test]
fn test_with_first_line_number_long_option() {
    let test_file_path = "test_one_page.log";
    let expected_test_file_path = "test_one_page_first_line.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["--first-line-number=5", "-n", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, vec![
            ("{last_modified_time}".to_string(), value),
        ]);
}

#[test]
fn test_with_number_option_with_custom_separator_char() {
    let test_file_path = "test_num_page.log";
    let expected_test_file_path = "test_num_page_char.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["-nc", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, vec![
            ("{last_modified_time}".to_string(), value),
        ]);
}

#[test]
fn test_with_number_option_with_custom_separator_char_and_width() {
    let test_file_path = "test_num_page.log";
    let expected_test_file_path = "test_num_page_char_one.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["-nc1", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, vec![
            ("{last_modified_time}".to_string(), value),
        ]);
}