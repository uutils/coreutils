use crate::common::util::*;

static INPUT: &'static str = "lorem_ipsum.txt";

#[test]
fn test_stdin_default() {
    new_ucmd!()
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("lorem_ipsum_default.expected");
}

#[test]
fn test_stdin_1_line_obsolete() {
    new_ucmd!()
        .args(&["-1"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("lorem_ipsum_1_line.expected");
}

#[test]
fn test_stdin_1_line() {
    new_ucmd!()
        .args(&["-n", "1"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("lorem_ipsum_1_line.expected");
}

#[test]
fn test_stdin_negative_23_line() {
    new_ucmd!()
        .args(&["-n", "-23"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("lorem_ipsum_1_line.expected");
}

#[test]
fn test_stdin_5_chars() {
    new_ucmd!()
        .args(&["-c", "5"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("lorem_ipsum_5_chars.expected");
}

#[test]
fn test_single_default() {
    new_ucmd!()
        .arg(INPUT)
        .run()
        .stdout_is_fixture("lorem_ipsum_default.expected");
}

#[test]
fn test_single_1_line_obsolete() {
    new_ucmd!()
        .args(&["-1", INPUT])
        .run()
        .stdout_is_fixture("lorem_ipsum_1_line.expected");
}

#[test]
fn test_single_1_line() {
    new_ucmd!()
        .args(&["-n", "1", INPUT])
        .run()
        .stdout_is_fixture("lorem_ipsum_1_line.expected");
}

#[test]
fn test_single_5_chars() {
    new_ucmd!()
        .args(&["-c", "5", INPUT])
        .run()
        .stdout_is_fixture("lorem_ipsum_5_chars.expected");
}

#[test]
fn test_verbose() {
    new_ucmd!()
        .args(&["-v", INPUT])
        .run()
        .stdout_is_fixture("lorem_ipsum_verbose.expected");
}

#[test]
fn test_zero_terminated() {
    new_ucmd!()
        .args(&["-z", "zero_terminated.txt"])
        .run()
        .stdout_is_fixture("zero_terminated.expected");
}

#[test]
#[ignore]
fn test_spams_newline() {
    new_ucmd!().pipe_in("a").succeeds().stdout_is("a\n");
}

#[test]
#[ignore]
fn test_unsupported_byte_syntax() {
    new_ucmd!()
        .args(&["-1c"])
        .pipe_in("abc")
        .fails()
        //GNU head returns "a"
        .stdout_is("")
        .stderr_is("head: error: Unrecognized option: \'1\'");
}

#[test]
#[ignore]
fn test_unsupported_line_syntax() {
    new_ucmd!()
        .args(&["-n", "2048m"])
        .pipe_in("a\n")
        .fails()
        //.stdout_is("a\n");  What GNU head returns.
        .stdout_is("")
        .stderr_is("head: error: invalid line count \'2048m\': invalid digit found in string");
}

#[test]
#[ignore]
fn test_unsupported_zero_terminated_syntax() {
    new_ucmd!()
        .args(&["-z -n 1"])
        .pipe_in("x\0y")
        .fails()
        //GNU Head returns "x\0"
        .stderr_is("head: error: Unrecognized option: \'z\'");
}

#[test]
#[ignore]
fn test_unsupported_zero_terminated_syntax_2() {
    new_ucmd!()
        .args(&["-z -n 2"])
        .pipe_in("x\0y")
        .fails()
        //GNU Head returns "x\0y"
        .stderr_is("head: error: Unrecognized option: \'z\'");
}

#[test]
#[ignore]
fn test_unsupported_negative_byte_syntax() {
    new_ucmd!()
        .args(&["--bytes=-2"])
        .pipe_in("a\n")
        .fails()
        //GNU Head returns ""
        .stderr_is("head: error: invalid byte count \'-2\': invalid digit found in string");
}

#[test]
#[ignore]
fn test_bug_in_negative_zero_lines() {
    new_ucmd!()
        .args(&["--lines=-0"])
        .pipe_in("a\nb\n")
        .succeeds()
        //GNU Head returns "a\nb\n"
        .stdout_is("");
}

#[test]
fn test_no_such_file_or_directory() {
    let result = new_ucmd!().arg("no_such_file.toml").run();

    assert_eq!(
        true,
        result
            .stderr
            .contains("cannot open 'no_such_file.toml' for reading: No such file or directory")
    )
}
