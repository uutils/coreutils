use crate::common::util::*;
use std::env;

#[test]
fn test_groups() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.run();
    println!("result.stdout {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    if env::var("USER").is_ok()
        && env::var("USER").unwrap() == "runner"
        && result.stdout.trim().is_empty()
    {
        // In the CI, some server are failing to return the group.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    assert!(result.success);
    assert!(!result.stdout.trim().is_empty());
}

#[test]
fn test_groups_arg() {
    // get the username with the "id -un" command
    let result = TestScenario::new("id").ucmd_keepenv().arg("-un").run();
    println!("result.stdout {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    let s1 = String::from(result.stdout.trim());
    if s1.parse::<f64>().is_ok() {
        // In the CI, some server are failing to return id -un.
        // So, if we are getting a uid, just skip this test
        // As seems to be a configuration issue, ignoring it
        return;
    }

    println!("result.stdout {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    assert!(result.success);
    assert!(!result.stdout.is_empty());
    let username = result.stdout.trim();

    // call groups with the user name to check that we
    // are getting something
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg(username).run();
    println!("result.stdout {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    assert!(result.success);
    assert!(!result.stdout.is_empty());
}
