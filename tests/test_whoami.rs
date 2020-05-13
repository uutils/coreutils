use common::util::*;
use std::env;

#[test]
fn test_normal() {
    let result = TestScenario::new(util_name!()).ucmd().run();

    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    println!("env::var(CI).is_ok() = {}", env::var("CI").is_ok());
    
    for (key, value) in env::vars() {
        println!("{}: {}", key, value);
    }
    if env::var("USER").unwrap() == "runner" && result.stderr.contains("failed to get username") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }

    assert!(result.success);
    assert!(!result.stdout.trim().is_empty());
}

#[test]
fn test_normal_compare_id() {
    let result = TestScenario::new(util_name!()).ucmd().run();

    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    if env::var("USER").unwrap() == "runner" && result.stderr.contains("failed to get username") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    assert!(result.success);
    let id = TestScenario::new("id").ucmd().arg("-un").run();

    if env::var("USER").unwrap() == "runner" && id.stderr.contains("cannot find name for user ID") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    assert_eq!(result.stdout.trim(), id.stdout.trim());
}
