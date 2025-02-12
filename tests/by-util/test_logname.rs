// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::env;
use uutests::new_ucmd;
use uutests::util::{is_ci, TestScenario};
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_normal() {
    let result = new_ucmd!().run();
    println!("env::var(CI).is_ok() = {}", env::var("CI").is_ok());

    for (key, value) in env::vars() {
        println!("{key}: {value}");
    }
    if (is_ci() || uucore::os::is_wsl_1()) && result.stderr_str().contains("no login name") {
        // ToDO: investigate WSL failure
        // In the CI, some server are failing to return logname.
        // As seems to be a configuration issue, ignoring it
        return;
    }

    result.success();
    assert!(!result.stdout_str().trim().is_empty());
}
