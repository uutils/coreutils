// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) Sdivide

use chrono::{DateTime, Duration, Utc};
use std::fs::metadata;
use uutests::new_ucmd;
use uutests::util::{TestScenario, UCommand};
use uutests::util_name;

const DATE_TIME_FORMAT: &str = "%b %d %H:%M %Y";

fn file_last_modified_time(ucmd: &UCommand, path: &str) -> String {
    let tmp_dir_path = ucmd.get_full_fixture_path(path);
    let file_metadata = metadata(tmp_dir_path);
    file_metadata
        .map(|i| {
            i.modified()
                .map(|x| {
                    let date_time: DateTime<Utc> = x.into();
                    date_time.format(DATE_TIME_FORMAT).to_string()
                })
                .unwrap_or_default()
        })
        .unwrap_or_default()
}

fn all_minutes(from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<String> {
    let to = to + Duration::try_minutes(1).unwrap();
    let mut vec = vec![];
    let mut current = from;
    while current < to {
        vec.push(current.format(DATE_TIME_FORMAT).to_string());
        current += Duration::try_minutes(1).unwrap();
    }
    vec
}

fn valid_last_modified_template_vars(from: DateTime<Utc>) -> Vec<Vec<(String, String)>> {
    all_minutes(from, Utc::now())
        .into_iter()
        .map(|time| vec![("{last_modified_time}".to_string(), time)])
        .collect()
}

#[test]
fn test_invalid_flag() {
    new_ucmd!()
        .arg("--invalid-argument")
        .fails()
        .no_stdout()
        .code_is(1);
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
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
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
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
}

#[test]
fn test_with_long_header_option() {
    let test_file_path = "test_one_page.log";
    let expected_test_file_path = "test_one_page_header.log.expected";
    let header = "new file";
    for args in [&["-h", header][..], &["--header=new file"][..]] {
        let mut scenario = new_ucmd!();
        let value = file_last_modified_time(&scenario, test_file_path);
        scenario
            .args(args)
            .arg(test_file_path)
            .succeeds()
            .stdout_is_templated_fixture(
                expected_test_file_path,
                &[("{last_modified_time}", &value), ("{header}", header)],
            );
    }
}

#[test]
fn test_with_double_space_option() {
    let test_file_path = "test_one_page.log";
    let expected_test_file_path = "test_one_page_double_line.log.expected";
    for arg in ["-d", "--double-space"] {
        let mut scenario = new_ucmd!();
        let value = file_last_modified_time(&scenario, test_file_path);
        scenario
            .args(&[arg, test_file_path])
            .succeeds()
            .stdout_is_templated_fixture(
                expected_test_file_path,
                &[("{last_modified_time}", &value)],
            );
    }
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
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
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
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
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
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
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
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
}

#[test]
fn test_with_valid_page_ranges() {
    let test_file_path = "test_num_page.log";
    let mut scenario = new_ucmd!();
    scenario
        .args(&["--pages=20:5", test_file_path])
        .fails()
        .stderr_is("pr: invalid --pages argument '20:5'\n")
        .stdout_is("");
    new_ucmd!()
        .args(&["--pages=1:5", test_file_path])
        .succeeds();
    new_ucmd!().args(&["--pages=1", test_file_path]).succeeds();
    new_ucmd!()
        .args(&["--pages=-1:5", test_file_path])
        .fails()
        .stderr_is("pr: invalid --pages argument '-1:5'\n")
        .stdout_is("");
    new_ucmd!()
        .args(&["--pages=1:-5", test_file_path])
        .fails()
        .stderr_is("pr: invalid --pages argument '1:-5'\n")
        .stdout_is("");
    new_ucmd!()
        .args(&["--pages=5:1", test_file_path])
        .fails()
        .stderr_is("pr: invalid --pages argument '5:1'\n")
        .stdout_is("");
}

#[test]
fn test_with_page_range() {
    let test_file_path = "test.log";
    let expected_test_file_path = "test_page_range_1.log.expected";
    let expected_test_file_path1 = "test_page_range_2.log.expected";
    for arg in ["--pages=15", "+15"] {
        let mut scenario = new_ucmd!();
        let value = file_last_modified_time(&scenario, test_file_path);
        scenario
            .args(&[arg, test_file_path])
            .succeeds()
            .stdout_is_templated_fixture(
                expected_test_file_path,
                &[("{last_modified_time}", &value)],
            );
    }
    for arg in ["--pages=15:17", "+15:17"] {
        let mut scenario = new_ucmd!();
        let value = file_last_modified_time(&scenario, test_file_path);
        scenario
            .args(&[arg, test_file_path])
            .succeeds()
            .stdout_is_templated_fixture(
                expected_test_file_path1,
                &[("{last_modified_time}", &value)],
            );
    }
}

#[test]
fn test_with_no_header_trailer_option() {
    let test_file_path = "test_one_page.log";
    let expected_test_file_path = "test_one_page_no_ht.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["-t", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
}

#[test]
fn test_with_page_length_option() {
    let test_file_path = "test.log";
    for (arg, expected) in [
        ("100", "test_page_length.log.expected"),
        ("5", "test_page_length1.log.expected"),
    ] {
        let mut scenario = new_ucmd!();
        let value = file_last_modified_time(&scenario, test_file_path);
        scenario
            .args(&["--pages=2:3", "-l", arg, "-n", test_file_path])
            .succeeds()
            .stdout_is_templated_fixture(expected, &[("{last_modified_time}", &value)]);
    }
}

#[test]
fn test_with_suppress_error_option() {
    let test_file_path = "test_num_page.log";
    let mut scenario = new_ucmd!();
    scenario
        .args(&["--pages=20:5", "-r", test_file_path])
        .fails()
        .stderr_is("")
        .stdout_is("");
}

#[test]
fn test_with_stdin() {
    let expected_file_path = "stdin.log.expected";
    let mut scenario = new_ucmd!();
    let start = Utc::now();
    scenario
        .pipe_in_fixture("stdin.log")
        .args(&["--pages=1:2", "-n", "-"])
        .run()
        .stdout_is_templated_fixture_any(
            expected_file_path,
            &valid_last_modified_template_vars(start),
        );
}

#[test]
fn test_with_column() {
    let test_file_path = "column.log";
    let expected_test_file_path = "column.log.expected";
    for arg in ["-3", "--column=3"] {
        let mut scenario = new_ucmd!();
        let value = file_last_modified_time(&scenario, test_file_path);
        scenario
            .args(&["--pages=3:5", arg, "-n", test_file_path])
            .succeeds()
            .stdout_is_templated_fixture(
                expected_test_file_path,
                &[("{last_modified_time}", &value)],
            );
    }
}

#[test]
fn test_with_column_across_option() {
    let test_file_path = "column.log";
    let expected_test_file_path = "column_across.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&["--pages=3:5", "--column=3", "-a", "-n", test_file_path])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
}

#[test]
fn test_with_column_across_option_and_column_separator() {
    let test_file_path = "column.log";
    for (arg, expected) in [
        ("-s|", "column_across_sep.log.expected"),
        ("-Sdivide", "column_across_sep1.log.expected"),
    ] {
        let mut scenario = new_ucmd!();
        let value = file_last_modified_time(&scenario, test_file_path);
        scenario
            .args(&["--pages=3:5", "--column=3", arg, "-a", "-n", test_file_path])
            .succeeds()
            .stdout_is_templated_fixture(expected, &[("{last_modified_time}", &value)]);
    }
}

#[test]
fn test_with_mpr() {
    let test_file_path = "column.log";
    let test_file_path1 = "hosts.log";
    let expected_test_file_path = "mpr.log.expected";
    let expected_test_file_path1 = "mpr1.log.expected";
    let expected_test_file_path2 = "mpr2.log.expected";
    let start = Utc::now();
    new_ucmd!()
        .args(&["--pages=1:2", "-m", "-n", test_file_path, test_file_path1])
        .succeeds()
        .stdout_is_templated_fixture_any(
            expected_test_file_path,
            &valid_last_modified_template_vars(start),
        );

    let start = Utc::now();
    new_ucmd!()
        .args(&["--pages=2:4", "-m", "-n", test_file_path, test_file_path1])
        .succeeds()
        .stdout_is_templated_fixture_any(
            expected_test_file_path1,
            &valid_last_modified_template_vars(start),
        );

    let start = Utc::now();
    new_ucmd!()
        .args(&[
            "--pages=1:2",
            "-l",
            "100",
            "-n",
            "-m",
            test_file_path,
            test_file_path1,
            test_file_path,
        ])
        .succeeds()
        .stdout_is_templated_fixture_any(
            expected_test_file_path2,
            &valid_last_modified_template_vars(start),
        );
}

#[test]
fn test_with_mpr_and_column_options() {
    let test_file_path = "column.log";
    new_ucmd!()
        .args(&["--column=2", "-m", "-n", test_file_path])
        .fails()
        .stderr_is("pr: cannot specify number of columns when printing in parallel\n")
        .stdout_is("");

    new_ucmd!()
        .args(&["-a", "-m", "-n", test_file_path])
        .fails()
        .stderr_is("pr: cannot specify both printing across and printing in parallel\n")
        .stdout_is("");
}

#[test]
fn test_with_offset_space_option() {
    let test_file_path = "column.log";
    let expected_test_file_path = "column_spaces_across.log.expected";
    let mut scenario = new_ucmd!();
    let value = file_last_modified_time(&scenario, test_file_path);
    scenario
        .args(&[
            "-o",
            "5",
            "--pages=3:5",
            "--column=3",
            "-a",
            "-n",
            test_file_path,
        ])
        .succeeds()
        .stdout_is_templated_fixture(expected_test_file_path, &[("{last_modified_time}", &value)]);
}

#[test]
fn test_with_pr_core_utils_tests() {
    let test_cases = vec![
        ("", vec!["0Ft"], vec!["0F"], 0),
        ("", vec!["0Fnt"], vec!["0F"], 0),
        ("+3", vec!["0Ft"], vec!["3-0F"], 0),
        ("+3 -f", vec!["0Ft"], vec!["3f-0F"], 0),
        ("-a -3", vec!["0Ft"], vec!["a3-0F"], 0),
        ("-a -3 -f", vec!["0Ft"], vec!["a3f-0F"], 0),
        ("-a -3 -f", vec!["0Fnt"], vec!["a3f-0F"], 0),
        ("+3 -a -3 -f", vec!["0Ft"], vec!["3a3f-0F"], 0),
        ("-l 24", vec!["FnFn"], vec!["l24-FF"], 0),
        ("-W 20 -l24 -f", vec!["tFFt-ll"], vec!["W20l24f-ll"], 0),
    ];

    for test_case in test_cases {
        let (flags, input_file, expected_file, return_code) = test_case;
        let mut scenario = new_ucmd!();
        let input_file_path = input_file.first().unwrap();
        let test_file_path = expected_file.first().unwrap();
        let value = file_last_modified_time(&scenario, input_file_path);
        let mut arguments: Vec<&str> = flags
            .split(' ')
            .filter(|i| i.trim() != "")
            .collect::<Vec<&str>>();

        arguments.extend(input_file.clone());

        let scenario_with_args = scenario.args(&arguments);

        let scenario_with_expected_status = if return_code == 0 {
            scenario_with_args.succeeds()
        } else {
            scenario_with_args.fails()
        };

        scenario_with_expected_status.stdout_is_templated_fixture(
            test_file_path,
            &[
                ("{last_modified_time}", &value),
                ("{file_name}", input_file_path),
            ],
        );
    }
}

#[test]
fn test_with_join_lines_option() {
    let test_file_1 = "hosts.log";
    let test_file_2 = "test.log";
    let expected_file_path = "joined.log.expected";
    let mut scenario = new_ucmd!();
    let start = Utc::now();
    scenario
        .args(&["+1:2", "-J", "-m", test_file_1, test_file_2])
        .run()
        .stdout_is_templated_fixture_any(
            expected_file_path,
            &valid_last_modified_template_vars(start),
        );
}

#[test]
fn test_value_for_number_lines() {
    // *5 is of the form [SEP[NUMBER]] so is accepted and succeeds
    new_ucmd!().args(&["-n", "*5", "test.log"]).succeeds();

    // a is of the form [SEP[NUMBER]] so is accepted and succeeds
    new_ucmd!().args(&["-n", "a", "test.log"]).succeeds();

    // foo5.txt is of not the form [SEP[NUMBER]] so is not used as value.
    // Therefore, pr tries to access the file, which does not exist.
    new_ucmd!().args(&["-n", "foo5.txt", "test.log"]).fails();
}

#[test]
fn test_help() {
    new_ucmd!().arg("--help").succeeds();
}

#[test]
fn test_version() {
    new_ucmd!().arg("--version").succeeds();
}
