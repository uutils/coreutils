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
    let real_name = pw.user_info().replace("&", &pw.name().capitalize());
    new_ucmd!()
        .arg("-l")
        .arg(login)
        .succeeds()
        .stdout_is(format!(
            "Login name: {:<28}In real life:  {}\nDirectory: {:<29}Shell:  {}\n\n",
            login,
            real_name,
            pw.user_dir(),
            pw.user_shell()
        ));

    new_ucmd!()
        .arg("-lb")
        .arg(login)
        .succeeds()
        .stdout_is(format!(
            "Login name: {:<28}In real life:  {1}\n\n",
            login, real_name
        ));
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_long_format_multiple_users() {
    let args = ["-l", "root", "root", "root"];

    new_ucmd!()
        .args(&args)
        .succeeds()
        .stdout_is(expected_result(&args));
}

#[test]
fn test_long_format_wo_user() {
    // "no username specified; at least one must be specified when using -l"
    new_ucmd!().arg("-l").fails().code_is(1);
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_short_format_i() {
    // allow whitespace variation
    // * minor whitespace differences occur between platform built-in outputs; specifically, the number of trailing TABs may be variant
    let args = ["-i"];
    let actual = new_ucmd!().args(&args).succeeds().stdout_move_str();
    let expect = expected_result(&args);
    let v_actual: Vec<&str> = actual.split_whitespace().collect();
    let v_expect: Vec<&str> = expect.split_whitespace().collect();
    assert_eq!(v_actual, v_expect);
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_short_format_q() {
    // allow whitespace variation
    // * minor whitespace differences occur between platform built-in outputs; specifically, the number of trailing TABs may be variant
    let args = ["-q"];
    let actual = new_ucmd!().args(&args).succeeds().stdout_move_str();
    let expect = expected_result(&args);
    let v_actual: Vec<&str> = actual.split_whitespace().collect();
    let v_expect: Vec<&str> = expect.split_whitespace().collect();
    assert_eq!(v_actual, v_expect);
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_no_flag() {
    let actual = new_ucmd!().succeeds().stdout_move_str();
    let expect = expected_result(&[]);
    let v_actual: Vec<&str> = actual.split_whitespace().collect();
    let v_expect: Vec<&str> = expect.split_whitespace().collect();
    assert_eq!(v_actual, v_expect);
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
fn expected_result(args: &[&str]) -> String {
    #[cfg(target_os = "linux")]
    let util_name = util_name!();
    #[cfg(target_vendor = "apple")]
    let util_name = format!("g{}", util_name!());

    TestScenario::new(&util_name)
        .cmd_keepenv(util_name)
        .env("LANGUAGE", "C")
        .args(args)
        .succeeds()
        .stdout_move_str()
}
