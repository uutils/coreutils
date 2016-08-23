use common::util::*;


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
