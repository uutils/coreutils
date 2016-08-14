use common::util::*;

static UTIL_NAME: &'static str = "true";
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_exit_code() {
    new_ucmd().succeeds();
}
