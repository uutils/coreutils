use common::util::*;

static UTIL_NAME: &'static str = "sort";

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
    let (at, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-n");
    ucmd.arg("multiple_files1.txt");
    ucmd.arg("multiple_files2.txt");
    let res = ucmd.run();
    assert_eq!(res.success, true);
    assert_eq!(res.stdout, at.read("multiple_files.expected"));
}

#[test]
fn test_check() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-c");
    let res = ucmd.arg("check_fail.txt").run();

    assert_eq!(res.success, false);
    assert_eq!(res.stdout, "sort: disorder in line 4\n");

    let (_, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-c");
    let res = ucmd.arg("multiple_files.expected").run();

    assert_eq!(res.success, true);
    assert_eq!(res.stdout, "");
}

fn test_helper(file_name: &str, args: &str) {
    let (at, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg(args);
    let res = ucmd.arg(format!("{}{}", file_name, ".txt")).run();

    assert_eq!(res.success, true);

    let filename = format!("{}{}", file_name, ".expected");
    assert_eq!(res.stdout, at.read(&filename));
}
