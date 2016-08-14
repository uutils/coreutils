use common::util::*;

static UTIL_NAME: &'static str = "paste";
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_combine_pairs_of_lines() {
    new_ucmd()
        .args(&["-s", "-d", "\t\n", "html_colors.txt"])
        .run()
        .stdout_is_fixture("html_colors.expected");
}
