// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_get_all() {
    TestScenario::new(util_name!())
        .ucmd()
        .env("HOME", "FOO")
        .env("KEY", "VALUE")
        .succeeds()
        .stdout_contains("HOME=FOO")
        .stdout_contains("KEY=VALUE");
}

#[test]
fn test_get_var() {
    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("KEY", "VALUE")
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
