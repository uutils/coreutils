//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

extern crate uucore;

use crate::common::util::*;

use self::uucore::entries::{Locate, Passwd};

extern crate pinky;
pub use self::pinky::*;

#[test]
fn test_capitalize() {
    assert_eq!("Zbnmasd", "zbnmasd".capitalize()); // spell-checker:disable-line
    assert_eq!("Abnmasd", "Abnmasd".capitalize()); // spell-checker:disable-line
    assert_eq!("1masd", "1masd".capitalize()); // spell-checker:disable-line
    assert_eq!("", "".capitalize());
}

#[test]
fn test_long_format() {
    let login = "root";
    let pw: Passwd = Passwd::locate(login).unwrap();
    let real_name = pw.user_info.replace('&', &pw.name.capitalize());
    let ts = TestScenario::new(util_name!());
    ts.ucmd().arg("-l").arg(login).succeeds().stdout_is(format!(
        "Login name: {:<28}In real life:  {}\nDirectory: {:<29}Shell:  {}\n\n",
        login, real_name, pw.user_dir, pw.user_shell
    ));

    ts.ucmd()
        .arg("-lb")
        .arg(login)
        .succeeds()
        .stdout_is(format!(
            "Login name: {:<28}In real life:  {1}\n\n",
            login, real_name
        ));
}

#[cfg(unix)]
#[test]
fn test_long_format_multiple_users() {
    let args = ["-l", "root", "root", "root"];
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
fn test_no_flag() {
    let ts = TestScenario::new(util_name!());
    let actual = ts.ucmd().succeeds().stdout_move_str();
    let expect = unwrap_or_return!(expected_result(&ts, &[])).stdout_move_str();
    let v_actual: Vec<&str> = actual.split_whitespace().collect();
    let v_expect: Vec<&str> = expect.split_whitespace().collect();
    assert_eq!(v_actual, v_expect);
}
