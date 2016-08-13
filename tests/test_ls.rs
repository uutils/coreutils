use common::util::*;

static UTIL_NAME: &'static str = "ls";
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_ls_ls() {
    new_ucmd().succeeds();
}
