use crate::common::util::*;

// Apparently some CI environments have configuration issues, e.g. with 'whoami' and 'id'.
// If we are running inside the CI and "needle" is in "stderr" skipping this test is
// considered okay. If we are not inside the CI this calls assert!(result.success).
//
// From the Logs: "Build (ubuntu-18.04, x86_64-unknown-linux-gnu, feat_os_unix, use-cross)"
// stderr: "whoami: cannot find name for user ID 1001"
// Maybe: "adduser --uid 1001 username" can put things right?
// stderr = id: Could not find uid 1001: No such id: 1001
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

fn return_whoami_username() -> String {
    let scene = TestScenario::new("whoami");
    let result = scene.cmd("whoami").run();
    if skipping_test_is_okay(&result, "whoami: cannot find name for user ID") {
        println!("test skipped:");
        return String::from("");
    }

    result.stdout_str().trim().to_string()
}

#[test]
fn test_id() {
    let scene = TestScenario::new(util_name!());

    let result = scene.ucmd().arg("-u").succeeds();
    let uid = result.stdout_str().trim();

    let result = scene.ucmd().run();
    if skipping_test_is_okay(&result, "Could not find uid") {
        return;
    }

    // Verify that the id found by --user/-u exists in the list
    result.stdout_contains(uid);
}

#[test]
fn test_id_from_name() {
    let username = return_whoami_username();
    if username.is_empty() {
        return;
    }

    let scene = TestScenario::new(util_name!());
    let result = scene.ucmd().arg(&username).run();
    if skipping_test_is_okay(&result, "Could not find uid") {
        return;
    }

    let uid = result.stdout_str().trim();

    let result = scene.ucmd().run();
    if skipping_test_is_okay(&result, "Could not find uid") {
        return;
    }

    result
        // Verify that the id found by --user/-u exists in the list
        .stdout_contains(uid)
        // Verify that the username found by whoami exists in the list
        .stdout_contains(username);
}

#[test]
fn test_id_name_from_id() {
    let result = new_ucmd!().arg("-nu").run();

    let username_id = result.stdout_str().trim();

    let username_whoami = return_whoami_username();
    if username_whoami.is_empty() {
        return;
    }

    assert_eq!(username_id, username_whoami);
}

#[test]
fn test_id_group() {
    let scene = TestScenario::new(util_name!());

    let mut result = scene.ucmd().arg("-g").succeeds();
    let s1 = result.stdout_str().trim();
    assert!(s1.parse::<u64>().is_ok());

    result = scene.ucmd().arg("--group").succeeds();
    let s1 = result.stdout_str().trim();
    assert!(s1.parse::<u64>().is_ok());

    #[cfg(any(target_vendor = "apple", target_os = "linux"))]
    for flag in &["-g", "--group"] {
        new_ucmd!()
            .arg(flag)
            .succeeds()
            .stdout_is(expected_result(&[flag], false));
    }
}

#[test]
#[cfg(any(target_vendor = "apple", target_os = "linux"))]
fn test_id_groups() {
    let scene = TestScenario::new(util_name!());
    for g_flag in &["-G", "--groups"] {
        scene
            .ucmd()
            .arg(g_flag)
            .succeeds()
            .stdout_is(expected_result(&[g_flag], false));
        for &r_flag in &["-r", "--real"] {
            let args = [g_flag, r_flag];
            scene
                .ucmd()
                .args(&args)
                .succeeds()
                .stdout_is(expected_result(&args, false));
        }
    }
}

#[test]
fn test_id_user() {
    let scene = TestScenario::new(util_name!());

    let result = scene.ucmd().arg("-u").succeeds();
    let s1 = result.stdout_str().trim();
    assert!(s1.parse::<u64>().is_ok());

    let result = scene.ucmd().arg("--user").succeeds();
    let s1 = result.stdout_str().trim();
    assert!(s1.parse::<u64>().is_ok());

    #[cfg(any(target_vendor = "apple", target_os = "linux"))]
    for flag in &["-u", "--user"] {
        new_ucmd!()
            .arg(flag)
            .succeeds()
            .stdout_is(expected_result(&[flag], false));
    }
}

#[test]
fn test_id_pretty_print() {
    let username = return_whoami_username();
    if username.is_empty() {
        return;
    }

    let scene = TestScenario::new(util_name!());
    let result = scene.ucmd().arg("-p").run();
    if result.stdout_str().trim().is_empty() {
        // this fails only on: "MinRustV (ubuntu-latest, feat_os_unix)"
        // `rustc 1.40.0 (73528e339 2019-12-16)`
        // run: /home/runner/work/coreutils/coreutils/target/debug/coreutils id -p
        // thread 'test_id::test_id_pretty_print' panicked at 'Command was expected to succeed.
        // stdout =
        // stderr = ', tests/common/util.rs:157:13
        println!("test skipped:");
        return;
    }

    result.success().stdout_contains(username);
}

#[test]
fn test_id_password_style() {
    let username = return_whoami_username();
    if username.is_empty() {
        return;
    }

    let result = new_ucmd!().arg("-P").succeeds();

    assert!(result.stdout_str().starts_with(&username));
}

#[test]
#[cfg(any(target_vendor = "apple", target_os = "linux"))]
fn test_id_default_format() {
    let scene = TestScenario::new(util_name!());
    // -ugG
    for flag in &["--name", "--real"] {
        scene
            .ucmd()
            .arg(flag)
            .fails()
            .stderr_is(expected_result(&[flag], true));
        for &opt in &["--user", "--group", "--groups"] {
            if is_ci() && *flag == "--name" {
                // '--name' does not work on CICD ubuntu-16/ubuntu-18
                // id: cannot find name for user ID 1001
                // id: cannot find name for group ID 116
                println!("test skipped");
                continue;
            }
            let args = [opt, flag];
            scene
                .ucmd()
                .args(&args)
                .succeeds()
                .stdout_is(expected_result(&args, false));
        }
    }
}

#[test]
#[cfg(any(target_vendor = "apple", target_os = "linux"))]
fn test_id_zero() {
    for z_flag in &["-z", "--zero"] {
        for &opt in &["-n", "--name", "-r", "--real"] {
            let args = [opt, z_flag];
            new_ucmd!()
                .args(&args)
                .fails()
                .stderr_is(expected_result(&args, true));
        }
        for &opt in &["-u", "--user", "-g", "--group", "-G", "--groups"] {
            let args = [opt, z_flag];
            new_ucmd!()
                .args(&args)
                .succeeds()
                .stdout_is(expected_result(&args, false));
        }
    }
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
fn expected_result(args: &[&str], exp_fail: bool) -> String {
    #[cfg(target_os = "linux")]
    let util_name = util_name!();
    #[cfg(target_vendor = "apple")]
    let util_name = format!("g{}", util_name!());

    let result = if !exp_fail {
        TestScenario::new(&util_name)
            .cmd_keepenv(util_name)
            .env("LANGUAGE", "C")
            .args(args)
            .succeeds()
            .stdout_move_str()
    } else {
        TestScenario::new(&util_name)
            .cmd_keepenv(util_name)
            .env("LANGUAGE", "C")
            .args(args)
            .fails()
            .stderr_move_str()
    };
    return if cfg!(target_os = "macos") && result.starts_with("gid") {
        result[1..].to_string()
    } else {
        result
    };
}
