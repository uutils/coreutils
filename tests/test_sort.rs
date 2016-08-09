use common::util::*;

static UTIL_NAME: &'static str = "sort";

fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_numeric_floats_and_ints() {
    test_helper("numeric_floats_and_ints", "-n");
}

#[test]
fn test_numeric_floats() {
    test_helper("numeric_floats", "-n");
}

#[test]
fn test_numeric_unfixed_floats() {
    test_helper("numeric_unfixed_floats", "-n");
}

#[test]
fn test_numeric_fixed_floats() {
    test_helper("numeric_fixed_floats", "-n");
}

#[test]
fn test_numeric_unsorted_ints() {
    test_helper("numeric_unsorted_ints", "-n");
}

#[test]
fn test_human_block_sizes() {
    test_helper("human_block_sizes", "-h");
}

#[test]
fn test_month_default() {
    test_helper("month_default", "-M");
}

#[test]
fn test_month_stable() {
    test_helper("month_stable", "-Ms");
}

#[test]
fn test_default_unsorted_ints() {
    test_helper("default_unsorted_ints", "");
}

#[test]
fn test_numeric_unique_ints() {
    test_helper("numeric_unsorted_ints_unique", "-nu");
}

#[test]
fn test_version() {
    test_helper("version", "-V");
}

#[test]
fn test_multiple_files() {
    new_ucmd()
        .arg("-n")
        .arg("multiple_files1.txt")
        .arg("multiple_files2.txt")
        .succeeds().stdout_is_fixture("multiple_files.expected");
}

#[test]
fn test_check() {
    new_ucmd()
        .arg("-c")
        .arg("check_fail.txt")
        .fails().stdout_is("sort: disorder in line 4\n");

    new_ucmd()
        .arg("-c")
        .arg("multiple_files.expected")
        .succeeds().stdout_is("");
}

fn test_helper(file_name: &str, args: &str) {
    new_ucmd().arg(args).arg(format!("{}{}", file_name, ".txt"))
        .succeeds().stdout_is_fixture(format!("{}{}", file_name, ".expected"));
}
