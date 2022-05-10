use crate::common::util::*;
use std::env;

#[test]
fn test_get_all() {
    let key = "KEY";
    env::set_var(key, "VALUE");
    assert_eq!(env::var(key), Ok("VALUE".to_string()));

    TestScenario::new(util_name!())
        .ucmd_keepenv()
        .succeeds()
        .stdout_contains("HOME=")
        .stdout_contains("KEY=VALUE");
}

#[test]
fn test_get_var() {
    let key = "KEY";
    env::set_var(key, "VALUE");
    assert_eq!(env::var(key), Ok("VALUE".to_string()));

    let result = TestScenario::new(util_name!())
        .ucmd_keepenv()
        .arg("KEY")
        .succeeds();

    assert!(!result.stdout_str().is_empty());
    assert_eq!(result.stdout_str().trim(), "VALUE");
}
