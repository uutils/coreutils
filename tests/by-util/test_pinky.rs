extern crate uucore;

use crate::common::util::*;

use self::uucore::entries::{Locate, Passwd};

extern crate pinky;
pub use self::pinky::*;

#[test]
fn test_capitalize() {
    assert_eq!("Zbnmasd", "zbnmasd".capitalize());
    assert_eq!("Abnmasd", "Abnmasd".capitalize());
    assert_eq!("1masd", "1masd".capitalize());
    assert_eq!("", "".capitalize());
}

#[test]
fn test_long_format() {
    let ulogin = "root";
    let pw: Passwd = Passwd::locate(ulogin).unwrap();
    let real_name = pw.user_info().replace("&", &pw.name().capitalize());
    new_ucmd!().arg("-l").arg(ulogin).run().stdout_is(format!(
        "Login name: {:<28}In real life:  {}\nDirectory: {:<29}Shell:  {}\n\n",
        ulogin,
        real_name,
        pw.user_dir(),
        pw.user_shell()
    ));

    new_ucmd!().arg("-lb").arg(ulogin).run().stdout_is(format!(
        "Login name: {:<28}In real life:  {1}\n\n",
        ulogin, real_name
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn test_long_format_multiple_users() {
    let scene = TestScenario::new(util_name!());

    let expected = scene
        .cmd_keepenv(util_name!())
        .env("LANGUAGE", "C")
        .arg("-l")
        .arg("root")
        .arg("root")
        .arg("root")
        .succeeds();

    scene
        .ucmd()
        .arg("-l")
        .arg("root")
        .arg("root")
        .arg("root")
        .succeeds()
        .stdout_is(expected.stdout_str());
}

#[test]
fn test_long_format_wo_user() {
    // "no username specified; at least one must be specified when using -l"
    new_ucmd!().arg("-l").fails().code_is(1);
}

#[cfg(target_os = "linux")]
#[test]
fn test_short_format_i() {
    // allow whitespace variation
    // * minor whitespace differences occur between platform built-in outputs; specifically, the number of trailing TABs may be variant
    let args = ["-i"];
    let actual = TestScenario::new(util_name!())
        .ucmd()
        .args(&args)
        .succeeds()
        .stdout_move_str();
    let expect = expected_result(&args);
    let v_actual: Vec<&str> = actual.split_whitespace().collect();
    let v_expect: Vec<&str> = expect.split_whitespace().collect();
    assert_eq!(v_actual, v_expect);
}

#[cfg(target_os = "linux")]
#[test]
fn test_short_format_q() {
    // allow whitespace variation
    // * minor whitespace differences occur between platform built-in outputs; specifically, the number of trailing TABs may be variant
    let args = ["-q"];
    let actual = TestScenario::new(util_name!())
        .ucmd()
        .args(&args)
        .succeeds()
        .stdout_move_str();
    let expect = expected_result(&args);
    let v_actual: Vec<&str> = actual.split_whitespace().collect();
    let v_expect: Vec<&str> = expect.split_whitespace().collect();
    assert_eq!(v_actual, v_expect);
}

#[cfg(target_os = "linux")]
#[test]
fn test_no_flag() {
    let scene = TestScenario::new(util_name!());

    let actual = scene.ucmd().succeeds().stdout_move_str();
    let expect = scene
        .cmd_keepenv(util_name!())
        .env("LANGUAGE", "C")
        .succeeds()
        .stdout_move_str();

    let v_actual: Vec<&str> = actual.split_whitespace().collect();
    let v_expect: Vec<&str> = expect.split_whitespace().collect();
    assert_eq!(v_actual, v_expect);
}

#[cfg(target_os = "linux")]
fn expected_result(args: &[&str]) -> String {
    TestScenario::new(util_name!())
        .cmd_keepenv(util_name!())
        .env("LANGUAGE", "C")
        .args(args)
        .run()
        .stdout_move_str()
}
