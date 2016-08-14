use common::util::*;

static UTIL_NAME: &'static str = "pwd";
fn at_and_ucmd() -> (AtPath, UCommand) {
    let ts = TestScenario::new(UTIL_NAME);
    let ucmd = ts.ucmd();
    (ts.fixtures, ucmd)
}

#[test]
fn test_default() {
    let (at, mut ucmd) = at_and_ucmd();
    ucmd.run().stdout_is(at.root_dir_resolved());
}
