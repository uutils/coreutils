use crate::common::util::*;

#[test]
fn test_random_shuffle_len() {
    // check whether output is the same length as the input
    const FILE: &'static str = "default_unsorted_ints.expected";
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg("-R").arg(FILE).run().stdout;
    let expected = at.read(FILE);

    assert_eq!(result.len(), expected.len());
}

#[test]
fn test_random_shuffle_contains_all_lines() {
    // check whether lines of input are all in output
    const FILE: &'static str = "default_unsorted_ints.expected";
    let (at, mut ucmd) = at_and_ucmd!();
    let result = new_ucmd!().arg("-R").arg(FILE).run().stdout;

    let expected = at.read(FILE);

    let result_sorted = new_ucmd!().pipe_in(result).run().stdout;

    assert_eq!(result_sorted, expected);
}

#[test]
fn test_random_shuffle_contains_two_runs_not_the_same() {
    // check whether lines of input are all in output
    const FILE: &'static str = "default_unsorted_ints.expected";
    let result = new_ucmd!().arg("-R").arg(FILE).run().stdout;

    let unexpected = new_ucmd!().arg("-R").arg(FILE).run().stdout;

    assert_ne!(result, unexpected);
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
fn test_numeric_floats_with_nan() {
    test_helper("numeric_floats_with_nan", "-n");
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
fn test_ignore_case() {
    test_helper("ignore_case", "-f");
}

#[test]
fn test_dictionary_order() {
    test_helper("dictionary_order", "-d");
}

#[test]
fn test_dictionary_order2() {
    for non_dictionary_order2_param in vec!["-d"] {
        new_ucmd!()
            .pipe_in("a👦🏻aa	b\naaaa	b")
            .arg(non_dictionary_order2_param)
            .succeeds()
            .stdout_only("a👦🏻aa	b\naaaa	b\n");
    }
}

#[test]
fn test_non_printing_chars() {
    for non_printing_chars_param in vec!["-i"] {
        new_ucmd!()
            .pipe_in("a👦🏻aa	b\naaaa	b")
            .arg(non_printing_chars_param)
            .succeeds()
            .stdout_only("aaaa	b\na👦🏻aa	b\n");
    }
}

#[test]
fn test_exponents_positive_general_fixed() {
    for exponents_positive_general_param in vec!["-g"] {
        new_ucmd!()
            .pipe_in("100E6\n\n50e10\n+100000\n\n10000K78\n10E\n\n\n1000EDKLD\n\n\n100E6\n\n50e10\n+100000\n\n")
            .arg(exponents_positive_general_param)
            .succeeds()
            .stdout_only("\n\n\n\n\n\n\n\n10000K78\n1000EDKLD\n10E\n+100000\n+100000\n100E6\n100E6\n50e10\n50e10\n");
    }
}

#[test]
fn test_exponents_positive_numeric() {
    test_helper("exponents-positive-numeric", "-n");
}

#[test]
fn test_months_dedup() {
    test_helper("months-dedup", "-Mu");
}

#[test]
fn test_mixed_floats_ints_chars_numeric() {
    test_helper("mixed_floats_ints_chars_numeric", "-n");
}

#[test]
fn test_mixed_floats_ints_chars_numeric_unique() {
    test_helper("mixed_floats_ints_chars_numeric_unique", "-nu");
}

#[test]
fn test_mixed_floats_ints_chars_numeric_reverse() {
    test_helper("mixed_floats_ints_chars_numeric_unique_reverse", "-nur");
}

#[test]
fn test_mixed_floats_ints_chars_numeric_stable() {
    test_helper("mixed_floats_ints_chars_numeric_stable", "-ns");
}

#[test]
fn test_numeric_floats_and_ints2() {
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
fn test_numeric_floats2() {
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
fn test_numeric_floats_with_nan2() {
    test_helper("numeric-floats-with-nan2", "-n");
}

#[test]
fn test_human_block_sizes2() {
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
fn test_month_default2() {
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
fn test_default_unsorted_ints2() {
    let input = "9\n1909888\n000\n1\n2";
    new_ucmd!()
        .pipe_in(input)
        .succeeds()
        .stdout_only("000\n1\n1909888\n2\n9\n");
}

#[test]
fn test_numeric_unique_ints2() {
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
fn test_zero_terminated() {
    test_helper("zero-terminated", "-z");
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

#[test]
fn test_check_silent() {
    new_ucmd!()
        .arg("-C")
        .arg("check_fail.txt")
        .fails()
        .stdout_is("");
}

fn test_helper(file_name: &str, args: &str) {
    new_ucmd!()
        .arg(args)
        .arg(format!("{}{}", file_name, ".txt"))
        .succeeds()
        .stdout_is_fixture(format!("{}{}", file_name, ".expected"));
}
