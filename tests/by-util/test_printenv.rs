use crate::common::util::TestScenario;
use std::env;

#[test]
fn test_get_all() {
    let key = "KEY";
    env::set_var(key, "VALUE");
    assert_eq!(env::var(key), Ok("VALUE".to_string()));

    TestScenario::new(util_name!())
        .ucmd()
        .keep_env()
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
        .ucmd()
        .keep_env()
        .arg("KEY")
        .succeeds();

    assert!(!result.stdout_str().is_empty());
    assert_eq!(result.stdout_str().trim(), "VALUE");
}

#[test]
fn test_ignore_equal_var() {
    let scene = TestScenario::new(util_name!());
    // tested by gnu/tests/misc/printenv.sh
    let result = scene.ucmd().env("a=b", "c").arg("a=b").fails();

    assert!(result.stdout_str().is_empty());
}
