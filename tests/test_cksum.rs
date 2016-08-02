use common::util::*;

static UTIL_NAME: &'static str = "cksum";
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_single_file() {
    new_ucmd().arg("lorem_ipsum.txt")
        .succeeds().stdout_is_fixture("single_file.expected");
}

#[test]
fn test_multiple_files() {
    new_ucmd()
        .arg("lorem_ipsum.txt")
        .arg("alice_in_wonderland.txt")
        .succeeds().stdout_is_fixture("multiple_files.expected");
}

#[test]
fn test_stdin() {
    new_ucmd()
        .pipe_in_fixture("lorem_ipsum.txt")
        .succeeds().stdout_is_fixture("stdin.expected");
}
