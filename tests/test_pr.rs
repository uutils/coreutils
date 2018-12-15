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
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["test_one_page.log"])
        .succeeds()
        .stdout_is_templated_fixture("test_one_page.log.expected", vec![("{last_modified_time}".to_string(), value)]);
}