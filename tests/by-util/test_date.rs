extern crate regex;

use self::regex::Regex;
use crate::common::util::*;
#[cfg(all(unix, not(target_os = "macos")))]
use rust_users::*;

#[test]
fn test_date_email() {
    let (_, mut ucmd) = at_and_ucmd!();
    ucmd.arg("--rfc-email").succeeds();
}

#[test]
fn test_date_email2() {
    let (_, mut ucmd) = at_and_ucmd!();
    ucmd.arg("-R").succeeds();
}

#[test]
fn test_date_rfc_3339() {
    let scene = TestScenario::new(util_name!());

    let mut result = scene.ucmd().arg("--rfc-3339=ns").succeeds();

    // Check that the output matches the regexp
    let rfc_regexp = r"(\d+)-(0[1-9]|1[012])-(0[1-9]|[12]\d|3[01])\s([01]\d|2[0-3]):([0-5]\d):([0-5]\d|60)(\.\d+)?(([Zz])|([\+|\-]([01]\d|2[0-3])))";
    let re = Regex::new(rfc_regexp).unwrap();
    assert!(re.is_match(&result.stdout_str().trim()));

    result = scene.ucmd().arg("--rfc-3339=seconds").succeeds();

    // Check that the output matches the regexp
    let re = Regex::new(rfc_regexp).unwrap();
    assert!(re.is_match(&result.stdout_str().trim()));
}

#[test]
fn test_date_rfc_8601() {
    let (_, mut ucmd) = at_and_ucmd!();
    ucmd.arg("--iso-8601=ns").succeeds();
}

#[test]
fn test_date_rfc_8601_second() {
    let (_, mut ucmd) = at_and_ucmd!();
    ucmd.arg("--iso-8601=second").succeeds();
}

#[test]
fn test_date_utc() {
    let (_, mut ucmd) = at_and_ucmd!();
    ucmd.arg("--utc").succeeds();
}

#[test]
fn test_date_universal() {
    let (_, mut ucmd) = at_and_ucmd!();
   ucmd.arg("--universal").succeeds();
}

#[test]
fn test_date_format_y() {
    let scene = TestScenario::new(util_name!());

    let mut result = scene.ucmd().arg("+%Y").succeeds();

    let mut re = Regex::new(r"^\d{4}$").unwrap();
    assert!(re.is_match(&result.stdout_str().trim()));

    result = scene.ucmd().arg("+%y").succeeds();

    re = Regex::new(r"^\d{2}$").unwrap();
    assert!(re.is_match(&result.stdout_str().trim()));
}

#[test]
fn test_date_format_m() {
    let scene = TestScenario::new(util_name!());

    let mut result = scene.ucmd().arg("+%b").succeeds();

    let mut re = Regex::new(r"\S+").unwrap();
    assert!(re.is_match(&result.stdout_str().trim()));

    result = scene.ucmd().arg("+%m").succeeds();

    re = Regex::new(r"^\d{2}$").unwrap();
    assert!(re.is_match(&result.stdout_str().trim()));
}

#[test]
fn test_date_format_day() {
    let scene = TestScenario::new(util_name!());

    let mut result = scene.ucmd().arg("+%a").succeeds();

    let mut re = Regex::new(r"\S+").unwrap();
    assert!(re.is_match(&result.stdout_str().trim()));

    result = scene.ucmd().arg("+%A").succeeds();

    re = Regex::new(r"\S+").unwrap();
    assert!(re.is_match(&result.stdout_str().trim()));

    result = scene.ucmd().arg("+%u").succeeds();

    re = Regex::new(r"^\d{1}$").unwrap();
    assert!(re.is_match(&result.stdout_str().trim()));
}

#[test]
fn test_date_format_full_day() {
    let scene = TestScenario::new(util_name!());

    let result = scene.ucmd().arg("+'%a %Y-%m-%d'").succeeds();

    let re = Regex::new(r"\S+ \d{4}-\d{2}-\d{2}").unwrap();
    assert!(re.is_match(&result.stdout_str().trim()));
}

#[test]
#[cfg(all(unix, not(target_os = "macos")))]
fn test_date_set_valid() {
    if get_effective_uid() == 0 {
        let (_, mut ucmd) = at_and_ucmd!();
        let result = ucmd
            .arg("--set")
            .arg("2020-03-12 13:30:00+08:00")
            .succeeds();
        result.no_stdout().no_stderr();
    }
}

#[test]
#[cfg(any(windows, all(unix, not(target_os = "macos"))))]
fn test_date_set_invalid() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg("--set").arg("123abcd").fails();
    let result = result.no_stdout();
    assert!(result.stderr_str().starts_with("date: invalid date "));
}

#[test]
#[cfg(all(unix, not(target_os = "macos")))]
fn test_date_set_permissions_error() {
    if !(get_effective_uid() == 0 || is_wsl()) {
        let (_, mut ucmd) = at_and_ucmd!();
        let result = ucmd.arg("--set").arg("2020-03-11 21:45:00+08:00").fails();
        let result = result.no_stdout();
        assert!(result.stderr_str().starts_with("date: cannot set date: "));
    }
}

#[test]
#[cfg(target_os = "macos")]
fn test_date_set_mac_unavailable() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg("--set").arg("2020-03-11 21:45:00+08:00").fails();
    let result = result.no_stdout();
    assert!(result
        .stderr_str()
        .starts_with("date: setting the date is not supported by macOS"));
}

#[test]
#[cfg(all(unix, not(target_os = "macos")))]
/// TODO: expected to fail currently; change to succeeds() when required.
fn test_date_set_valid_2() {
    if get_effective_uid() == 0 {
        let (_, mut ucmd) = at_and_ucmd!();
        let result = ucmd
            .arg("--set")
            .arg("Sat 20 Mar 2021 14:53:01 AWST")
            .fails();
        let result = result.no_stdout();
        assert!(result.stderr_str().starts_with("date: invalid date "));
    }
}

#[test]
#[cfg(all(unix, not(target_os = "macos")))]
/// TODO: expected to fail currently; change to succeeds() when required.
fn test_date_set_valid_3() {
    if get_effective_uid() == 0 {
        let (_, mut ucmd) = at_and_ucmd!();
        let result = ucmd
            .arg("--set")
            .arg("Sat 20 Mar 2021 14:53:01") // Local timezone
            .fails();
        let result = result.no_stdout();
        assert!(result.stderr_str().starts_with("date: invalid date "));
    }
}

#[test]
#[cfg(all(unix, not(target_os = "macos")))]
/// TODO: expected to fail currently; change to succeeds() when required.
fn test_date_set_valid_4() {
    if get_effective_uid() == 0 {
        let (_, mut ucmd) = at_and_ucmd!();
        let result = ucmd
            .arg("--set")
            .arg("2020-03-11 21:45:00") // Local timezone
            .fails();
        let result = result.no_stdout();
        assert!(result.stderr_str().starts_with("date: invalid date "));
    }
}
