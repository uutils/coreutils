use crate::common::util::*;

// Apparently some CI environments have configuration issues, e.g. with 'whoami' and 'id'.
//
// From the Logs: "Build (ubuntu-18.04, x86_64-unknown-linux-gnu, feat_os_unix, use-cross)"
//    whoami: cannot find name for user ID 1001
// id --name: cannot find name for user ID 1001
// id --name: cannot find name for group ID 116
//
// However, when running "id" from within "/bin/bash" it looks fine:
// id: "uid=1001(runner) gid=118(docker) groups=118(docker),4(adm),101(systemd-journal)"
// whoami: "runner"
//

fn whoami() -> String {
    // Use environment variable to get current user instead of invoking `whoami`
    // and fall back to user "nobody" on error.
    std::env::var("USER").unwrap_or_else(|e| {
        println!("warning: {}, using \"nobody\" instead", e);
        "nobody".to_string()
    })
}

#[test]
#[cfg(unix)]
fn test_id_no_specified_user() {
    let result = new_ucmd!().run();
    let expected_result = expected_result(&[]);
    let mut exp_stdout = expected_result.stdout_str().to_string();

    // uu_id does not support selinux context. Remove 'context' part from exp_stdout:
    let context_offset = expected_result
        .stdout_str()
        .find(" context")
        .unwrap_or(exp_stdout.len());
    exp_stdout.replace_range(context_offset.., "\n");

    result
        .stdout_is(exp_stdout)
        .stderr_is(expected_result.stderr_str())
        .code_is(expected_result.code());
}

#[test]
#[cfg(unix)]
fn test_id_single_user() {
    let test_users = [&whoami()[..]];

    let scene = TestScenario::new(util_name!());
    let mut exp_result = expected_result(&test_users);
    scene
        .ucmd()
        .args(&test_users)
        .run()
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str())
        .code_is(exp_result.code());

    // u/g/G z/n
    for &opt in &["--user", "--group", "--groups"] {
        let mut args = vec![opt];
        args.extend_from_slice(&test_users);
        exp_result = expected_result(&args);
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str())
            .code_is(exp_result.code());
        args.push("--zero");
        exp_result = expected_result(&args);
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str())
            .code_is(exp_result.code());
        args.push("--name");
        exp_result = expected_result(&args);
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str())
            .code_is(exp_result.code());
        args.pop();
        exp_result = expected_result(&args);
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str())
            .code_is(exp_result.code());
    }
}

#[test]
#[cfg(unix)]
fn test_id_single_user_non_existing() {
    let args = &["hopefully_non_existing_username"];
    let result = new_ucmd!().args(args).run();
    let expected_result = expected_result(args);
    result
        .stdout_is(expected_result.stdout_str())
        .stderr_is(expected_result.stderr_str())
        .code_is(expected_result.code());
}

#[test]
#[cfg(unix)]
fn test_id_name() {
    let scene = TestScenario::new(util_name!());
    for &opt in &["--user", "--group", "--groups"] {
        let args = [opt, "--name"];
        let result = scene.ucmd().args(&args).run();
        let expected_result = expected_result(&args);
        result
            .stdout_is(expected_result.stdout_str())
            .stderr_is(expected_result.stderr_str())
            .code_is(expected_result.code());

        if opt == "--user" {
            assert_eq!(result.stdout_str().trim_end(), whoami());
        }
    }
}

#[test]
#[cfg(unix)]
fn test_id_real() {
    let scene = TestScenario::new(util_name!());
    for &opt in &["--user", "--group", "--groups"] {
        let args = [opt, "--real"];
        let result = scene.ucmd().args(&args).run();
        let expected_result = expected_result(&args);
        result
            .stdout_is(expected_result.stdout_str())
            .stderr_is(expected_result.stderr_str())
            .code_is(expected_result.code());
    }
}

#[test]
#[cfg(all(unix, not(target_os = "linux")))]
fn test_id_pretty_print() {
    // `-p` is BSD only and not supported on GNU's `id`
    let username = whoami();

    let result = new_ucmd!().arg("-p").run();
    if result.stdout_str().trim().is_empty() {
        // this fails only on: "MinRustV (ubuntu-latest, feat_os_unix)"
        // `rustc 1.40.0 (73528e339 2019-12-16)`
        // run: /home/runner/work/coreutils/coreutils/target/debug/coreutils id -p
        // thread 'test_id::test_id_pretty_print' panicked at 'Command was expected to succeed.
        // stdout =
        // stderr = ', tests/common/util.rs:157:13
        println!("test skipped:");
        return;
    } else {
        result.success().stdout_contains(username);
    }
}

#[test]
#[cfg(all(unix, not(target_os = "linux")))]
fn test_id_password_style() {
    // `-P` is BSD only and not supported on GNU's `id`
    let username = whoami();
    let result = new_ucmd!().arg("-P").arg(&username).succeeds();
    assert!(result.stdout_str().starts_with(&username));
}

#[test]
#[cfg(unix)]
fn test_id_multiple_users() {
    // Same typical users that GNU testsuite is using.
    let test_users = ["root", "man", "postfix", "sshd", &whoami()];

    let scene = TestScenario::new(util_name!());
    let mut exp_result = expected_result(&test_users);
    scene
        .ucmd()
        .args(&test_users)
        .run()
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str())
        .code_is(exp_result.code());

    // u/g/G z/n
    for &opt in &["--user", "--group", "--groups"] {
        let mut args = vec![opt];
        args.extend_from_slice(&test_users);
        exp_result = expected_result(&args);
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str())
            .code_is(exp_result.code());
        args.push("--zero");
        exp_result = expected_result(&args);
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str())
            .code_is(exp_result.code());
        args.push("--name");
        exp_result = expected_result(&args);
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str())
            .code_is(exp_result.code());
        args.pop();
        exp_result = expected_result(&args);
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str())
            .code_is(exp_result.code());
    }
}

#[test]
#[cfg(unix)]
fn test_id_multiple_users_non_existing() {
    let test_users = [
        "root",
        "hopefully_non_existing_username1",
        &whoami(),
        "man",
        "hopefully_non_existing_username2",
        "hopefully_non_existing_username3",
        "postfix",
        "sshd",
        "hopefully_non_existing_username4",
        &whoami(),
    ];

    let scene = TestScenario::new(util_name!());
    let mut exp_result = expected_result(&test_users);
    scene
        .ucmd()
        .args(&test_users)
        .run()
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str())
        .code_is(exp_result.code());

    // u/g/G z/n
    for &opt in &["--user", "--group", "--groups"] {
        let mut args = vec![opt];
        args.extend_from_slice(&test_users);
        exp_result = expected_result(&args);
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str())
            .code_is(exp_result.code());
        args.push("--zero");
        exp_result = expected_result(&args);
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str())
            .code_is(exp_result.code());
        args.push("--name");
        exp_result = expected_result(&args);
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str())
            .code_is(exp_result.code());
        args.pop();
        exp_result = expected_result(&args);
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str())
            .code_is(exp_result.code());
    }
}

#[test]
#[cfg(unix)]
fn test_id_default_format() {
    let scene = TestScenario::new(util_name!());
    for &opt1 in &["--name", "--real"] {
        // id: cannot print only names or real IDs in default format
        let args = [opt1];
        scene
            .ucmd()
            .args(&args)
            .fails()
            .stderr_only(expected_result(&args).stderr_str());
        for &opt2 in &["--user", "--group", "--groups"] {
            // u/g/G n/r
            let args = [opt2, opt1];
            let result = scene.ucmd().args(&args).run();
            let exp_result = expected_result(&args);
            result
                .stdout_is(exp_result.stdout_str())
                .stderr_is(exp_result.stderr_str())
                .code_is(exp_result.code());
        }
    }
    for &opt2 in &["--user", "--group", "--groups"] {
        // u/g/G
        let args = [opt2];
        scene
            .ucmd()
            .args(&args)
            .succeeds()
            .stdout_only(expected_result(&args).stdout_str());
    }
}

#[test]
#[cfg(unix)]
fn test_id_zero() {
    let scene = TestScenario::new(util_name!());
    for z_flag in &["-z", "--zero"] {
        // id: option --zero not permitted in default format
        scene
            .ucmd()
            .args(&[z_flag])
            .fails()
            .stderr_only(expected_result(&[z_flag]).stderr_str());
        for &opt1 in &["--name", "--real"] {
            // id: cannot print only names or real IDs in default format
            let args = [opt1, z_flag];
            scene
                .ucmd()
                .args(&args)
                .fails()
                .stderr_only(expected_result(&args).stderr_str());
            for &opt2 in &["--user", "--group", "--groups"] {
                // u/g/G n/r z
                let args = [opt2, z_flag, opt1];
                let result = scene.ucmd().args(&args).run();
                let exp_result = expected_result(&args);
                result
                    .stdout_is(exp_result.stdout_str())
                    .stderr_is(exp_result.stderr_str())
                    .code_is(exp_result.code());
            }
        }
        for &opt2 in &["--user", "--group", "--groups"] {
            // u/g/G z
            let args = [opt2, z_flag];
            scene
                .ucmd()
                .args(&args)
                .succeeds()
                .stdout_only(expected_result(&args).stdout_str());
        }
    }
}

#[allow(clippy::needless_borrow)]
#[cfg(unix)]
fn expected_result(args: &[&str]) -> CmdResult {
    #[cfg(target_os = "linux")]
    let util_name = util_name!();
    #[cfg(all(unix, not(target_os = "linux")))]
    let util_name = format!("g{}", util_name!());

    let result = TestScenario::new(&util_name)
        .cmd_keepenv(&util_name)
        .env("LANGUAGE", "C")
        .args(args)
        .run();

    let mut _o = 0;
    let mut _e = 0;
    #[cfg(all(unix, not(target_os = "linux")))]
    {
        _o = if result.stdout_str().starts_with(&util_name) {
            1
        } else {
            0
        };
        _e = if result.stderr_str().starts_with(&util_name) {
            1
        } else {
            0
        };
    }

    CmdResult::new(
        Some(result.tmpd()),
        Some(result.code()),
        result.succeeded(),
        &result.stdout()[_o..],
        &result.stderr()[_e..],
    )
}
