// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use uutests::new_ucmd;
#[cfg(unix)]
use uutests::unwrap_or_return;
#[cfg(unix)]
use uutests::util::expected_result;
use uutests::util::{is_ci, whoami, TestScenario};
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
#[cfg(unix)]
fn test_normal() {
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
fn test_normal_compare_id() {
    let ts = TestScenario::new("id");
    let id_un = unwrap_or_return!(expected_result(&ts, &["-un"]));
    if id_un.succeeded() {
        new_ucmd!().succeeds().stdout_is(id_un.stdout_str());
    } else if is_ci() && id_un.stderr_str().contains("cannot find name for user ID") {
        println!("test skipped:");
    } else {
        id_un.success();
    }
}

#[test]
fn test_normal_compare_env() {
    let whoami = whoami();
    if whoami == "nobody" {
        println!("test skipped:");
    } else if !is_ci() {
        new_ucmd!().succeeds().stdout_is(format!("{whoami}\n"));
    } else {
        println!("test skipped:");
    }
}

#[test]
fn test_succeeds_on_all_platforms() {
    new_ucmd!().succeeds().no_stderr();
}
