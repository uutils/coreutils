use crate::common::util::*;

#[test]
fn test_more_no_arg() {
    // Reading from stdin is now supported, so this must succeed
    if atty::is(atty::Stream::Stdout) {
        new_ucmd!().succeeds();
    } else {
    }
}

#[test]
fn test_more_dir_arg() {
    // Run the test only if there's a valid terminal, else do nothing
    // Maybe we could capture the error, i.e. "Device not found" in that case
    // but I am leaving this for later
    if atty::is(atty::Stream::Stdout) {
        new_ucmd!()
            .arg(".")
            .fails()
            .usage_error("'.' is a directory.");
    } else {
    }
}
