use crate::common::util::*;

#[test]
fn test_rejects_nan() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd().args(&["NaN"]).fails().stderr_only(format!(
        "{0}: invalid 'not-a-number' argument: 'NaN'\nTry '{1} {0} --help' for more information.",
        ts.util_name,
        ts.bin_path.to_string_lossy()
    ));
}

#[test]
fn test_rejects_non_floats() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd().args(&["foo"]).fails().stderr_only(&format!(
        "{0}: invalid floating point argument: 'foo'\nTry '{1} {0} --help' for more information.",
        ts.util_name,
        ts.bin_path.to_string_lossy()
    ));
}

// ---- Tests for the big integer based path ----

#[test]
fn test_count_up() {
    new_ucmd!()
        .args(&["10"])
        .run()
        .stdout_is("1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");
}

#[test]
fn test_count_down() {
    new_ucmd!()
        .args(&["--", "5", "-1", "1"])
        .run()
        .stdout_is("5\n4\n3\n2\n1\n");
    new_ucmd!()
        .args(&["5", "-1", "1"])
        .run()
        .stdout_is("5\n4\n3\n2\n1\n");
}

#[test]
fn test_separator_and_terminator() {
    new_ucmd!()
        .args(&["-s", ",", "-t", "!", "2", "6"])
        .run()
        .stdout_is("2,3,4,5,6!");
    new_ucmd!()
        .args(&["-s", ",", "2", "6"])
        .run()
        .stdout_is("2,3,4,5,6\n");
    new_ucmd!()
        .args(&["-s", "\n", "2", "6"])
        .run()
        .stdout_is("2\n3\n4\n5\n6\n");
    new_ucmd!()
        .args(&["-s", "\\n", "2", "6"])
        .run()
        .stdout_is("2\\n3\\n4\\n5\\n6\n");
}

#[test]
fn test_equalize_widths() {
    new_ucmd!()
        .args(&["-w", "5", "10"])
        .run()
        .stdout_is("05\n06\n07\n08\n09\n10\n");
}

#[test]
fn test_seq_wrong_arg() {
    new_ucmd!().args(&["-w", "5", "10", "33", "32"]).fails();
}

#[test]
fn test_zero_step() {
    new_ucmd!().args(&["10", "0", "32"]).fails();
}

#[test]
fn test_big_numbers() {
    new_ucmd!()
        .args(&[
            "1000000000000000000000000000",
            "1000000000000000000000000001",
        ])
        .succeeds()
        .stdout_only("1000000000000000000000000000\n1000000000000000000000000001\n");
}

// ---- Tests for the floating point based path ----

#[test]
fn test_count_up_floats() {
    new_ucmd!()
        .args(&["10.0"])
        .run()
        .stdout_is("1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");
}

#[test]
fn test_count_down_floats() {
    new_ucmd!()
        .args(&["--", "5", "-1.0", "1"])
        .run()
        .stdout_is("5.0\n4.0\n3.0\n2.0\n1.0\n");
    new_ucmd!()
        .args(&["5", "-1", "1.0"])
        .run()
        .stdout_is("5\n4\n3\n2\n1\n");
}

#[test]
fn test_separator_and_terminator_floats() {
    new_ucmd!()
        .args(&["-s", ",", "-t", "!", "2.0", "6"])
        .run()
        .stdout_is("2.0,3.0,4.0,5.0,6.0!");
}

#[test]
fn test_equalize_widths_floats() {
    new_ucmd!()
        .args(&["-w", "5", "10.0"])
        .run()
        .stdout_is("05\n06\n07\n08\n09\n10\n");
}

#[test]
fn test_seq_wrong_arg_floats() {
    new_ucmd!().args(&["-w", "5", "10.0", "33", "32"]).fails();
}

#[test]
fn test_zero_step_floats() {
    new_ucmd!().args(&["10.0", "0", "32"]).fails();
}

#[test]
fn test_preserve_negative_zero_start() {
    new_ucmd!()
        .args(&["-0", "1"])
        .succeeds()
        .stdout_is("-0\n1\n")
        .no_stderr();
}

#[test]
fn test_drop_negative_zero_end() {
    new_ucmd!()
        .args(&["1", "-1", "-0"])
        .succeeds()
        .stdout_is("1\n0\n")
        .no_stderr();
}

#[test]
fn test_width_scientific_notation() {
    new_ucmd!()
        .args(&["-w", "999", "1e3"])
        .succeeds()
        .stdout_is("0999\n1000\n")
        .no_stderr();
}

#[test]
fn test_width_negative_zero() {
    new_ucmd!()
        .args(&["-w", "-0", "1"])
        .succeeds()
        .stdout_is("-0\n01\n")
        .no_stderr();
}
