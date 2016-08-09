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
    let out = ucmd.run().stdout;

    let expected = at.root_dir_resolved();
    assert_eq!(out.trim_right(), expected);
}
