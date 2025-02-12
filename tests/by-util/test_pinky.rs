// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use pinky::Capitalize;
#[cfg(not(target_os = "openbsd"))]
use uucore::entries::{Locate, Passwd};
use uutests::new_ucmd;
use uutests::unwrap_or_return;
#[cfg(target_os = "openbsd")]
use uutests::util::TestScenario;
#[cfg(not(target_os = "openbsd"))]
use uutests::util::{expected_result, TestScenario};
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_capitalize() {
    assert_eq!("Zbnmasd", "zbnmasd".capitalize()); // spell-checker:disable-line
    assert_eq!("Abnmasd", "Abnmasd".capitalize()); // spell-checker:disable-line
    assert_eq!("1masd", "1masd".capitalize()); // spell-checker:disable-line
    assert_eq!("", "".capitalize());
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_long_format() {
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
