use common::util::*;
use std::env;

#[test]
fn test_get_all() {
    let key = "KEY";
    env::set_var(key, "VALUE");
    assert_eq!(env::var(key), Ok("VALUE".to_string()));

    let result = TestScenario::new(util_name!()).ucmd_keepenv().run();
    assert!(result.success);
    assert!(result.stdout.contains("HOME="));
    assert!(result.stdout.contains("KEY=VALUE"));
}

#[test]
fn test_get_var() {
    let key = "KEY";
    env::set_var(key, "VALUE");
    assert_eq!(env::var(key), Ok("VALUE".to_string()));

    let result = TestScenario::new(util_name!())
        .ucmd_keepenv()
        .arg("KEY")
        .run();

    assert!(result.success);
    assert!(!result.stdout.is_empty());
    assert!(result.stdout.trim() == "VALUE");
}
