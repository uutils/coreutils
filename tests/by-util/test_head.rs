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
#[ignore]
fn test_spams_newline() {
    //this test is does not mirror what GNU does
    new_ucmd!().pipe_in("a").succeeds().stdout_is("a\n");
}

#[test]
fn test_byte_syntax() {
    new_ucmd!()
        .args(&["-1c"])
        .pipe_in("abc")
        .run()
        .stdout_is("a");
}

#[test]
fn test_line_syntax() {
    new_ucmd!()
        .args(&["-n", "2048m"])
        .pipe_in("a\n")
        .run()
        .stdout_is("a\n");
}

#[test]
fn test_zero_terminated_syntax() {
    new_ucmd!()
        .args(&["-z", "-n", "1"])
        .pipe_in("x\0y")
        .run()
        .stdout_is("x\0");
}

#[test]
fn test_zero_terminated_syntax_2() {
    new_ucmd!()
        .args(&["-z", "-n", "2"])
        .pipe_in("x\0y")
        .run()
        .stdout_is("x\0y");
}

#[test]
fn test_negative_byte_syntax() {
    new_ucmd!()
        .args(&["--bytes=-2"])
        .pipe_in("a\n")
        .run()
        .stdout_is("");
}

#[test]
fn test_negative_zero_lines() {
    new_ucmd!()
        .args(&["--lines=-0"])
        .pipe_in("a\nb\n")
        .succeeds()
        .stdout_is("a\nb\n");
}
#[test]
fn test_negative_zero_bytes() {
    new_ucmd!()
        .args(&["--bytes=-0"])
        .pipe_in("qwerty")
        .succeeds()
        .stdout_is("qwerty");
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

// there was a bug not caught by previous tests
// where for negative n > 3, the total amount of lines
// was correct, but it would eat from the second line
#[test]
fn test_sequence_fixture() {
    new_ucmd!()
        .args(&["-n", "-10", "sequence"])
        .run()
        .stdout_is_fixture("sequence.expected");
}
#[test]
fn test_file_backwards() {
    new_ucmd!()
        .args(&["-c", "-10", "lorem_ipsum.txt"])
        .run()
        .stdout_is_fixture("lorem_ipsum_backwards_file.expected");
}

#[test]
fn test_zero_terminated() {
    new_ucmd!()
        .args(&["-z", "zero_terminated.txt"])
        .run()
        .stdout_is_fixture("zero_terminated.expected");
}

#[test]
fn test_obsolete_extras() {
    new_ucmd!()
        .args(&["-5zv"])
        .pipe_in("1\02\03\04\05\06")
        .succeeds()
        .stdout_is("==> standard input <==\n1\02\03\04\05\0");
}
