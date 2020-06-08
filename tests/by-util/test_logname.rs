use crate::common::util::*;
use std::env;

#[test]
fn test_normal() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.run();
    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    println!("env::var(CI).is_ok() = {}", env::var("CI").is_ok());

    for (key, value) in env::vars() {
        println!("{}: {}", key, value);
    }
    if env::var("USER").is_ok()
        && env::var("USER").unwrap() == "runner"
        && result.stderr.contains("error: no login name")
    {
        // In the CI, some server are failing to return logname.
        // As seems to be a configuration issue, ignoring it
        return;
    }

    assert!(result.success);
    assert!(!result.stdout.trim().is_empty());
}
