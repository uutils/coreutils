extern crate regex;
use self::regex::Regex;
use crate::common::util::*;

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

#[test]
fn test_uptime_since() {
    let scene = TestScenario::new(util_name!());

    let result = scene.ucmd().arg("--since").succeeds();

    println!("stdout = {}", result.stdout);
    println!("stderr = {}", result.stderr);

    assert!(result.success);
    let re = Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}").unwrap();
    assert!(re.is_match(&result.stdout.trim()));
}
