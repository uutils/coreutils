#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "pwd";


#[test]
fn test_default() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let out = ucmd.run().stdout;

    let expected = at.root_dir();
    assert_eq!(out.trim_right(), expected);
}
