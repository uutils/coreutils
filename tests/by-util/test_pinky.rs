// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(not(target_os = "openbsd"))]
use uucore::entries::{Locate, Passwd};
use uutests::new_ucmd;
#[cfg(not(target_os = "openbsd"))]
use uutests::util::{TestScenario, expected_result};
#[cfg(not(target_os = "openbsd"))]
use uutests::{unwrap_or_return, util_name};

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_long_format() {
    use pinky::Capitalize;

    let login = "root";
    let pw: Passwd = Passwd::locate(login).unwrap();
    let user_info = pw.user_info.unwrap_or_default();
    let user_dir = pw.user_dir.unwrap_or_default();
    let user_shell = pw.user_shell.unwrap_or_default();
    let real_name = user_info.replace('&', &pw.name.capitalize());
    let ts = TestScenario::new(util_name!());
    ts.ucmd().arg("-l").arg(login).succeeds().stdout_is(format!(
        "Login name: {login:<28}In real life:  {real_name}\nDirectory: {user_dir:<29}Shell:  {user_shell}\n\n"
    ));

    ts.ucmd()
        .arg("-lb")
        .arg(login)
        .succeeds()
        .stdout_is(format!(
            "Login name: {login:<28}In real life:  {real_name}\n\n"
        ));
}

#[cfg(unix)]
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_long_format_multiple_users() {
    // multiple instances of one account we know exists,
    // the account of the test runner,
    // and an account that (probably) doesn't exist
    let runner = std::env::var("USER").unwrap_or_default();
    let args = ["-l", "root", "root", "root", &runner, "no_such_user"];
    let ts = TestScenario::new(util_name!());
    let expect = unwrap_or_return!(expected_result(&ts, &args));

    ts.ucmd()
        .args(&args)
        .succeeds()
        .stdout_is(expect.stdout_str())
        .stderr_is(expect.stderr_str());
}

#[test]
fn test_long_format_wo_user() {
    // "no username specified; at least one must be specified when using -l"
    new_ucmd!().arg("-l").fails();
}

#[cfg(unix)]
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_short_format_i() {
    // allow whitespace variation
    // * minor whitespace differences occur between platform built-in outputs; specifically, the number of trailing TABs may be variant
    let args = ["-i"];
    let ts = TestScenario::new(util_name!());
    let actual = ts.ucmd().args(&args).succeeds().stdout_move_str();
    let expect = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
    let v_actual: Vec<&str> = actual.split_whitespace().collect();
    let v_expect: Vec<&str> = expect.split_whitespace().collect();
    assert_eq!(v_actual, v_expect);
}

#[cfg(unix)]
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_lookup() {
    let args = ["--lookup"];
    let ts = TestScenario::new(util_name!());
    let actual = ts.ucmd().args(&args).succeeds().stdout_move_str();
    let expect = unwrap_or_return!(expected_result(&ts, &[])).stdout_move_str();
    let v_actual: Vec<&str> = actual.split_whitespace().collect();
    let v_expect: Vec<&str> = expect.split_whitespace().collect();
    // The "Idle" field (index 3 in header) contains a dynamic time value that can change
    // between when the two commands run (e.g., "00:09" vs "00:10"), causing flaky tests.
    // We filter out values matching the idle time pattern (HH:MM format) to avoid race conditions.
    // Header: ["Login", "Name", "TTY", "Idle", "When", "Where"]
    fn filter_idle_times(v: &[&str]) -> Vec<String> {
        v.iter()
            .enumerate()
            .filter(|(i, s)| {
                // Skip the "Idle" header at index 3
                if *i == 3 {
                    return false;
                }
                // Skip any value that looks like an idle time (HH:MM format like "00:09")
                // These appear after the header in user data rows
                if *i >= 6 && s.len() == 5 && s.chars().nth(2) == Some(':') {
                    let chars: Vec<char> = s.chars().collect();
                    if chars[0].is_ascii_digit()
                        && chars[1].is_ascii_digit()
                        && chars[3].is_ascii_digit()
                        && chars[4].is_ascii_digit()
                    {
                        return false;
                    }
                }
                true
            })
            .map(|(_, s)| (*s).to_string())
            .collect()
    }
    let v_actual_filtered = filter_idle_times(&v_actual);
    let v_expect_filtered = filter_idle_times(&v_expect);
    assert_eq!(v_actual_filtered, v_expect_filtered);
}

#[cfg(unix)]
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_short_format_q() {
    // allow whitespace variation
    // * minor whitespace differences occur between platform built-in outputs; specifically, the number of trailing TABs may be variant
    let args = ["-q"];
    let ts = TestScenario::new(util_name!());
    let actual = ts.ucmd().args(&args).succeeds().stdout_move_str();
    let expect = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
    let v_actual: Vec<&str> = actual.split_whitespace().collect();
    let v_expect: Vec<&str> = expect.split_whitespace().collect();
    assert_eq!(v_actual, v_expect);
}

#[cfg(unix)]
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_no_flag() {
    let ts = TestScenario::new(util_name!());
    let actual = ts.ucmd().succeeds().stdout_move_str();
    let expect = unwrap_or_return!(expected_result(&ts, &[])).stdout_move_str();
    let v_actual: Vec<&str> = actual.split_whitespace().collect();
    let v_expect: Vec<&str> = expect.split_whitespace().collect();
    assert_eq!(v_actual, v_expect);
}
