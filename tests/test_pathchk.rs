use common::util::*;

static UTIL_NAME: &'static str = "pathchk";
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_default_mode() {
    // test the default mode

    // accept some reasonable default
    new_ucmd().args(&["abc/def"]).succeeds().no_stdout();

    // fail on long inputs
    new_ucmd().args(&[repeat_str("test", 20000)]).fails().no_stdout();
}
