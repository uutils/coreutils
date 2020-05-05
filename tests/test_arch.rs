use common::util::*;

#[test]
fn test_arch() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.run();
    assert!(result.success);
}

#[test]
fn test_arch_help() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("--help").run();
    assert!(result.success);
    assert!(result.stdout.contains("architecture name"));
}
