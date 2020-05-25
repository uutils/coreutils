extern crate regex;

use self::regex::Regex;
use crate::common::util::*;

#[test]
fn test_date_email() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg("--rfc-email").run();
    assert!(result.success);
}

#[test]
fn test_date_email2() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg("-R").run();
    assert!(result.success);
}

#[test]
fn test_date_rfc_3339() {
    let scene = TestScenario::new(util_name!());

    let mut result = scene.ucmd().arg("--rfc-3339=ns").succeeds();

    // Check that the output matches the regexp
    let rfc_regexp = r"(\d+)-(0[1-9]|1[012])-(0[1-9]|[12]\d|3[01])\s([01]\d|2[0-3]):([0-5]\d):([0-5]\d|60)(\.\d+)?(([Zz])|([\+|\-]([01]\d|2[0-3])))";
    let re = Regex::new(rfc_regexp).unwrap();
    assert!(re.is_match(&result.stdout.trim()));

    result = scene.ucmd().arg("--rfc-3339=seconds").succeeds();

    // Check that the output matches the regexp
    let re = Regex::new(rfc_regexp).unwrap();
    assert!(re.is_match(&result.stdout.trim()));
}

#[test]
fn test_date_rfc_8601() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg("--iso-8601=ns").run();
    assert!(result.success);
}

#[test]
fn test_date_rfc_8601_second() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg("--iso-8601=second").run();
    assert!(result.success);
}

#[test]
fn test_date_utc() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg("--utc").run();
    assert!(result.success);
}

#[test]
fn test_date_universal() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg("--universal").run();
    assert!(result.success);
}

#[test]
fn test_date_format_y() {
    let scene = TestScenario::new(util_name!());

    let mut result = scene.ucmd().arg("+%Y").succeeds();

    assert!(result.success);
    let mut re = Regex::new(r"^\d{4}$").unwrap();
    assert!(re.is_match(&result.stdout.trim()));

    result = scene.ucmd().arg("+%y").succeeds();

    assert!(result.success);
    re = Regex::new(r"^\d{2}$").unwrap();
    assert!(re.is_match(&result.stdout.trim()));
}

#[test]
fn test_date_format_m() {
    let scene = TestScenario::new(util_name!());

    let mut result = scene.ucmd().arg("+%b").succeeds();

    assert!(result.success);
    let mut re = Regex::new(r"\S+").unwrap();
    assert!(re.is_match(&result.stdout.trim()));

    result = scene.ucmd().arg("+%m").succeeds();

    assert!(result.success);
    re = Regex::new(r"^\d{2}$").unwrap();
    assert!(re.is_match(&result.stdout.trim()));
}

#[test]
fn test_date_format_day() {
    let scene = TestScenario::new(util_name!());

    let mut result = scene.ucmd().arg("+%a").succeeds();

    assert!(result.success);
    let mut re = Regex::new(r"\S+").unwrap();
    assert!(re.is_match(&result.stdout.trim()));

    result = scene.ucmd().arg("+%A").succeeds();

    assert!(result.success);

    re = Regex::new(r"\S+").unwrap();
    assert!(re.is_match(&result.stdout.trim()));

    result = scene.ucmd().arg("+%u").succeeds();

    assert!(result.success);
    re = Regex::new(r"^\d{1}$").unwrap();
    assert!(re.is_match(&result.stdout.trim()));
}

#[test]
fn test_date_format_full_day() {
    let scene = TestScenario::new(util_name!());

    let result = scene.ucmd().arg("+'%a %Y-%m-%d'").succeeds();

    assert!(result.success);
    let re = Regex::new(r"\S+ \d{4}-\d{2}-\d{2}").unwrap();
    assert!(re.is_match(&result.stdout.trim()));
}
