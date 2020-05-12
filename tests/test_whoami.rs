use common::util::*;
use std::env;

#[test]
fn test_normal() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.run();
    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    println!("env::var("CI").is_ok() = {}", env::var("CI").is_ok());
    if env::var("CI").is_ok() && result.stderr.contains("failed to get username") {
        return;
    }

    assert!(result.success);
    assert!(!result.stdout.trim().is_empty());
}

#[test]
fn test_normal_compare_id() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.run();

    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    if env::var("CI").is_ok() && result.stderr.contains("failed to get username") {
        return;
    }
    assert!(result.success);
    let ts = TestScenario::new("id");
    let id = ts.cmd("id").arg("-un").run();
    if env::var("CI").is_ok() && id.stderr.contains("cannot find name for user ID") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    assert_eq!(result.stdout.trim(), id.stdout.trim());
}
