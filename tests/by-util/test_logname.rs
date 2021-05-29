use crate::common::util::*;
use std::env;

#[test]
fn test_normal() {
    let result = new_ucmd!().run();
    println!("env::var(CI).is_ok() = {}", env::var("CI").is_ok());

    for (key, value) in env::vars() {
        println!("{}: {}", key, value);
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
