// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//spell-checker: ignore coreutil

use uutests::new_ucmd;
use uutests::unwrap_or_return;
use uutests::util::{check_coreutil_version, expected_result, whoami, TestScenario};
use uutests::util_name;

const VERSION_MIN_MULTIPLE_USERS: &str = "8.31"; // this feature was introduced in GNU's coreutils 8.31

#[test]
#[cfg(unix)]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
#[cfg(unix)]
fn test_groups() {
    let ts = TestScenario::new(util_name!());
    let result = ts.ucmd().run();
    let exp_result = unwrap_or_return!(expected_result(&ts, &[]));

    result
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str())
        .code_is(exp_result.code());
}

#[test]
#[cfg(unix)]
fn test_groups_username() {
    let test_users = [&whoami()[..]];

    let ts = TestScenario::new(util_name!());
    let result = ts.ucmd().args(&test_users).run();
    let exp_result = unwrap_or_return!(expected_result(&ts, &test_users));

    result
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str())
        .code_is(exp_result.code());
}

#[test]
#[cfg(unix)]
fn test_groups_username_multiple() {
    unwrap_or_return!(check_coreutil_version(
        util_name!(),
        VERSION_MIN_MULTIPLE_USERS
    ));
    let test_users = ["root", "man", "postfix", "sshd", &whoami()];

    let ts = TestScenario::new(util_name!());
    let result = ts.ucmd().args(&test_users).run();
    let exp_result = unwrap_or_return!(expected_result(&ts, &test_users));

    result
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str())
        .code_is(exp_result.code());
}
