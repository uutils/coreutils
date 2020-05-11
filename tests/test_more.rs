use common::util::*;

#[test]
fn test_more_no_arg() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.run();
    assert!(!result.success);
}
