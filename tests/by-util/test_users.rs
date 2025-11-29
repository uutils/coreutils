// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::new_ucmd;
#[cfg(any(target_vendor = "apple", target_os = "linux"))]
use uutests::{unwrap_or_return, util::TestScenario, util::expected_result, util_name};

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_users_no_arg() {
    new_ucmd!().succeeds();
}

#[test]
#[cfg(any(target_vendor = "apple", target_os = "linux"))]
fn test_users_check_name() {
    let ts = TestScenario::new(util_name!());
    let expected_stdout = unwrap_or_return!(expected_result(&ts, &[])).stdout_move_str();
    ts.ucmd().succeeds().stdout_is(expected_stdout);
}

#[test]
#[cfg(target_os = "openbsd")]
fn test_users_check_name_openbsd() {
    new_ucmd!()
        .args(&["openbsd_utmp"])
        .succeeds()
        .stdout_contains("test");
}
