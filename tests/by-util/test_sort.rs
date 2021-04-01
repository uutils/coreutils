use crate::common::util::*;

#[test]
fn test_numeric_floats_and_ints() {
    for numeric_sort_param in vec!["-n", "--numeric-sort"] {
        let input = "1.444\n8.013\n1\n-8\n1.04\n-1";
        new_ucmd!()
            .arg(numeric_sort_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("-8\n-1\n1\n1.04\n1.444\n8.013\n");
    }
}

#[test]
fn test_numeric_floats() {
    for numeric_sort_param in vec!["-n", "--numeric-sort"] {
        let input = "1.444\n8.013\n1.58590\n-8.90880\n1.040000000\n-.05";
        new_ucmd!()
            .arg(numeric_sort_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("-8.90880\n-.05\n1.040000000\n1.444\n1.58590\n8.013\n");
    }
}

#[test]
fn test_numeric_floats_with_nan() {
    for numeric_sort_param in vec!["-n", "--numeric-sort"] {
        let input = "1.444\n1.0/0.0\n1.58590\n-8.90880\n1.040000000\n-.05";
        new_ucmd!()
            .arg(numeric_sort_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("-8.90880\n-.05\n1.0/0.0\n1.040000000\n1.444\n1.58590\n");
    }
}

#[test]
fn test_numeric_unfixed_floats() {
    test_helper("numeric_fixed_floats", "-n");
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
    for human_numeric_sort_param in vec!["-h", "--human-numeric-sort"] {
        let input = "8981K\n909991M\n-8T\n21G\n0.8M";
        new_ucmd!()
            .arg(human_numeric_sort_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("-8T\n0.8M\n8981K\n21G\n909991M\n");
    }
}

#[test]
fn test_month_default() {
    for month_sort_param in vec!["-M", "--month-sort"] {
        let input = "JAn\nMAY\n000may\nJun\nFeb";
        new_ucmd!()
            .arg(month_sort_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("000may\nJAn\nFeb\nMAY\nJun\n");
    }
}

#[test]
fn test_month_stable() {
    test_helper("month_stable", "-Ms");
}

#[test]
fn test_default_unsorted_ints() {
    let input = "9\n1909888\n000\n1\n2";
    new_ucmd!()
        .pipe_in(input)
        .succeeds()
        .stdout_only("000\n1\n1909888\n2\n9\n");
}

#[test]
fn test_numeric_unique_ints() {
    for numeric_unique_sort_param in vec!["-nu"] {
        let input = "9\n9\n8\n1\n";
        new_ucmd!()
            .arg(numeric_unique_sort_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("1\n8\n9\n");
    }
}

#[test]
fn test_keys_open_ended() {
    let input = "aa bb cc\ndd aa ff\ngg aa cc\n";
    new_ucmd!()
        .args(&["-k", "2.2"])
        .pipe_in(input)
        .succeeds()
        .stdout_only("gg aa cc\ndd aa ff\naa bb cc\n");
}

#[test]
fn test_keys_closed_range() {
    let input = "aa bb cc\ndd aa ff\ngg aa cc\n";
    new_ucmd!()
        .args(&["-k", "2.2,2.2"])
        .pipe_in(input)
        .succeeds()
        .stdout_only("dd aa ff\ngg aa cc\naa bb cc\n");
}

#[test]
fn test_keys_multiple_ranges() {
    let input = "aa bb cc\ndd aa ff\ngg aa cc\n";
    new_ucmd!()
        .args(&["-k", "2,2", "-k", "3,3"])
        .pipe_in(input)
        .succeeds()
        .stdout_only("gg aa cc\ndd aa ff\naa bb cc\n");
}

#[test]
fn test_keys_custom_separator() {
    let input = "aaxbbxcc\nddxaaxff\nggxaaxcc\n";
    new_ucmd!()
        .args(&["-k", "2.2,2.2", "-t", "x"])
        .pipe_in(input)
        .succeeds()
        .stdout_only("ddxaaxff\nggxaaxcc\naaxbbxcc\n");
}

#[test]
fn test_keys_invalid_field_letter() {
    let input = "foo";
    new_ucmd!()
        .args(&["-k", "1.1n"])
        .pipe_in(input)
        .fails()
        .stderr_only(
        "sort: error: failed to parse character index for key `1.1n`: invalid digit found in string",
    );
}

#[test]
fn test_keys_invalid_field_zero() {
    let input = "foo";
    new_ucmd!()
        .args(&["-k", "0.1"])
        .pipe_in(input)
        .fails()
        .stderr_only("sort: error: field index was 0");
}

#[test]
fn test_version() {
    test_helper("version", "-V");
}

#[test]
fn test_ignore_case() {
    test_helper("ignore_case", "-f");
}

#[test]
fn test_dictionary_order() {
    test_helper("dictionary_order", "-d");
}

#[test]
fn test_multiple_files() {
    new_ucmd!()
        .arg("-n")
        .arg("multiple_files1.txt")
        .arg("multiple_files2.txt")
        .succeeds()
        .stdout_only_fixture("multiple_files.expected");
}

#[test]
fn test_merge_interleaved() {
    new_ucmd!()
        .arg("-m")
        .arg("merge_ints_interleaved_1.txt")
        .arg("merge_ints_interleaved_2.txt")
        .arg("merge_ints_interleaved_3.txt")
        .succeeds()
        .stdout_only_fixture("merge_ints_interleaved.expected");
}

#[test]
fn test_merge_unique() {
    new_ucmd!()
        .arg("-m")
        .arg("--unique")
        .arg("merge_ints_interleaved_1.txt")
        .arg("merge_ints_interleaved_2.txt")
        .arg("merge_ints_interleaved_3.txt")
        .arg("merge_ints_interleaved_3.txt")
        .arg("merge_ints_interleaved_2.txt")
        .arg("merge_ints_interleaved_1.txt")
        .succeeds()
        .stdout_only_fixture("merge_ints_interleaved.expected");
}

#[test]
fn test_merge_reversed() {
    new_ucmd!()
        .arg("-m")
        .arg("--reverse")
        .arg("merge_ints_reversed_1.txt")
        .arg("merge_ints_reversed_2.txt")
        .arg("merge_ints_reversed_3.txt")
        .succeeds()
        .stdout_only_fixture("merge_ints_reversed.expected");
}

#[test]
fn test_pipe() {
    // TODO: issue 1608 reports a panic when we attempt to read from stdin,
    // which was closed by the other side of the pipe. This test does not
    // protect against regressions in that case; we should add one at some
    // point.
    new_ucmd!()
        .pipe_in("one\ntwo\nfour")
        .succeeds()
        .stdout_is("four\none\ntwo\n")
        .stderr_is("");
}

#[test]
fn test_check() {
    new_ucmd!()
        .arg("-c")
        .arg("check_fail.txt")
        .fails()
        .stdout_is("sort: disorder in line 4\n");

    new_ucmd!()
        .arg("-c")
        .arg("multiple_files.expected")
        .succeeds()
        .stdout_is("");
}

fn test_helper(file_name: &str, args: &str) {
    new_ucmd!()
        .arg(args)
        .arg(format!("{}{}", file_name, ".txt"))
        .succeeds()
        .stdout_is_fixture(format!("{}{}", file_name, ".expected"));
}
