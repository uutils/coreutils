extern crate regex;
use self::regex::Regex;
use crate::common::util::*;

#[test]
fn test_uptime() {
    TestScenario::new(util_name!())
        .ucmd_keepenv()
        .succeeds()
        .stdout_contains("load average:")
        .stdout_contains(" up ");

    // Don't check for users as it doesn't show in some CI
}

#[test]
fn test_uptime_since() {
    let re = Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}").unwrap();

    new_ucmd!().arg("--since").succeeds().stdout_matches(&re);
}

#[test]
fn test_failed() {
    new_ucmd!().arg("will-fail").fails();
}
