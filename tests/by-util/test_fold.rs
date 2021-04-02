use crate::common::util::*;

#[test]
fn test_default_80_column_wrap() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .run()
        .stdout_is_fixture("lorem_ipsum_80_column.expected");
}

#[test]
fn test_40_column_hard_cutoff() {
    new_ucmd!()
        .args(&["-w", "40", "lorem_ipsum.txt"])
        .run()
        .stdout_is_fixture("lorem_ipsum_40_column_hard.expected");
}

#[test]
fn test_40_column_word_boundary() {
    new_ucmd!()
        .args(&["-s", "-w", "40", "lorem_ipsum.txt"])
        .run()
        .stdout_is_fixture("lorem_ipsum_40_column_word.expected");
}

#[test]
fn test_default_wrap_with_newlines() {
    new_ucmd!()
        .arg("lorem_ipsum_new_line.txt")
        .run()
        .stdout_is_fixture("lorem_ipsum_new_line_80_column.expected");
}

#[test]
fn test_should_preserve_empty_lines() {
    new_ucmd!().pipe_in("\n").succeeds().stdout_is("\n");

    new_ucmd!()
        .arg("-w1")
        .pipe_in("0\n1\n\n2\n\n\n")
        .succeeds()
        .stdout_is("0\n1\n\n2\n\n\n");
}

#[test]
fn test_word_boundary_split_should_preserve_empty_lines() {
    new_ucmd!()
        .arg("-s")
        .pipe_in("\n")
        .succeeds()
        .stdout_is("\n");

    new_ucmd!()
        .args(&["-w1", "-s"])
        .pipe_in("0\n1\n\n2\n\n\n")
        .succeeds()
        .stdout_is("0\n1\n\n2\n\n\n");
}
