// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use regex::Regex;
use std::env;
use uutests::new_ucmd;
use uutests::util::is_ci;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_normal() {
    let result = new_ucmd!().run();
    println!("env::var(CI).is_ok() = {}", env::var("CI").is_ok());

    for (key, value) in env::vars() {
        println!("{key}: {value}");
    }
    if (is_ci() || uucore::os::is_wsl()) && result.stderr_str().contains("no login name") {
        // ToDO: investigate WSL failure
        // In the CI, some server are failing to return logname.
        // As seems to be a configuration issue, ignoring it
        return;
    }

    result.success();
    assert!(!result.stdout_str().trim().is_empty());
}

#[test]
fn test_help() {
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .stdout_contains("Print user's login name");
}

#[test]
fn test_output_format() {
    let result = new_ucmd!().run();
    if (is_ci() || uucore::os::is_wsl()) && result.stderr_str().contains("no login name") {
        return;
    }
    result.success();
    assert!(
        Regex::new(r"^\w+\n$")
            .unwrap()
            .is_match(result.stdout_str()),
        "unexpected logname output: {:?}",
        result.stdout_str()
    );
}
