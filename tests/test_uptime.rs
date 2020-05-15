use common::util::*;

#[test]
fn test_uptime() {
    let result = TestScenario::new(util_name!()).ucmd_keepenv().run();

    println!("stdout = {}", result.stdout);
    println!("stderr = {}", result.stderr);

    assert!(result.success);
    assert!(result.stdout.contains("load average:"));
    assert!(result.stdout.contains(" up "));
    // Don't check for users as it doesn't show in some CI
}
