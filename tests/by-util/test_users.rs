// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_users_no_arg() {
    new_ucmd!().succeeds();
}

#[test]
#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[ignore = "issue #3219"]
fn test_users_check_name() {
    #[cfg(target_os = "linux")]
    let util_name = util_name!();
    #[cfg(target_vendor = "apple")]
    let util_name = &format!("g{}", util_name!());

    let expected = TestScenario::new(util_name)
        .cmd(util_name)
        .env("LC_ALL", "C")
        .succeeds()
        .stdout_move_str();

    new_ucmd!().succeeds().stdout_is(&expected);
}

#[test]
#[cfg(target_os = "openbsd")]
fn test_users_check_name_openbsd() {
    new_ucmd!()
        .args(&["openbsd_utmp"])
        .run()
        .stdout_contains("test");
}
