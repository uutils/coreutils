use common::util::*;

utility_test!();

#[test]
fn test_default() {
    let (at, mut ucmd) = at_and_ucmd();
    ucmd.run().stdout_is(at.root_dir_resolved());
}
