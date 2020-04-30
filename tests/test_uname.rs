use common::util::*;

#[test]
fn test_uname_compatible() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("-a").run();
    assert!(result.success);
}

#[test]
fn test_uname_name() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("-n").run();
    assert!(result.success);
}
