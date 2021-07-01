use crate::common::util::*;

// Apparently some CI environments have configuration issues, e.g. with 'whoami' and 'id'.
// If we are running inside the CI and "needle" is in "stderr" skipping this test is
// considered okay. If we are not inside the CI this calls assert!(result.success).
//
// From the Logs: "Build (ubuntu-18.04, x86_64-unknown-linux-gnu, feat_os_unix, use-cross)"
// stderr: "whoami: failed to get username"
// Maybe: "adduser --uid 1001 username" can put things right?
fn skipping_test_is_okay(result: &CmdResult, needle: &str) -> bool {
    if !result.succeeded() {
        println!("result.stdout = {}", result.stdout_str());
        println!("result.stderr = {}", result.stderr_str());
        if is_ci() && result.stderr_str().contains(needle) {
            println!("test skipped:");
            return true;
        } else {
            result.success();
        }
    }
    false
}

#[test]
fn test_normal() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.run();

    // use std::env;
    // println!("env::var(CI).is_ok() = {}", env::var("CI").is_ok());
    // for (key, value) in env::vars() {
    //     println!("{}: {}", key, value);
    // }

    if skipping_test_is_okay(&result, "failed to get username") {
        return;
    }

    result.no_stderr();
    assert!(!result.stdout_str().trim().is_empty());
}

#[test]
#[cfg(not(windows))]
fn test_normal_compare_id() {
    let scene = TestScenario::new(util_name!());

    let result_ucmd = scene.ucmd().run();
    if skipping_test_is_okay(&result_ucmd, "failed to get username") {
        return;
    }

    let result_cmd = scene.cmd("id").arg("-un").run();
    if skipping_test_is_okay(&result_cmd, "cannot find name for user ID") {
        return;
    }

    assert_eq!(
        result_ucmd.stdout_str().trim(),
        result_cmd.stdout_str().trim()
    );
}
