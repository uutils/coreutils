use common::util::*;

#[test]
fn test_normal() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.run();
    assert!(result.success);
    assert!(!result.stdout.trim().is_empty());
}

#[test]
fn test_normal_compare_id() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.run();
    assert!(result.success);
    let ts = TestScenario::new("id");
    let id = ts.cmd("id").arg("-un").run();
    assert_eq!(result.stdout.trim(), id.stdout.trim());
}
