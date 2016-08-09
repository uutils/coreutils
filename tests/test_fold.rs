use common::util::*;

static UTIL_NAME: &'static str = "fold";
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_default_80_column_wrap() {
    new_ucmd()
        .arg("lorem_ipsum.txt")
        .run()
        .stdout_is_fixture("lorem_ipsum_80_column.expected");
}

#[test]
fn test_40_column_hard_cutoff() {
    new_ucmd()
        .arg("-w")
        .arg("40")
        .arg("lorem_ipsum.txt")
        .run()
        .stdout_is_fixture("lorem_ipsum_40_column_hard.expected");
}

#[test]
fn test_40_column_word_boundary() {
    new_ucmd()
        .arg("-s")
        .arg("-w")
        .arg("40")
        .arg("lorem_ipsum.txt")
        .run()
        .stdout_is_fixture("lorem_ipsum_40_column_word.expected");
}
