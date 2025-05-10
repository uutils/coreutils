// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) coreutil

use uutests::new_ucmd;
use uutests::unwrap_or_return;
use uutests::util::{TestScenario, check_coreutil_version, expected_result, is_ci, whoami};
use uutests::util_name;

const VERSION_MIN_MULTIPLE_USERS: &str = "8.31"; // this feature was introduced in GNU's coreutils 8.31

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
#[allow(unused_mut)]
fn test_id_no_specified_user() {
    let ts = TestScenario::new(util_name!());
    let result = ts.ucmd().run();
    let exp_result = unwrap_or_return!(expected_result(&ts, &[]));
    let mut exp_stdout = exp_result.stdout_str().to_string();

    #[cfg(not(feature = "feat_selinux"))]
    {
        // NOTE: strip 'context' part from exp_stdout if selinux not enabled:
        // example:
        // uid=1001(runner) gid=121(docker) groups=121(docker),4(adm),101(systemd-journal) \
        // context=unconfined_u:unconfined_r:unconfined_t:s0-s0:c0.c1023
        if let Some(context_offset) = exp_result.stdout_str().find(" context=") {
            exp_stdout.replace_range(context_offset..exp_stdout.len() - 1, "");
        }
    }

    result
        .stdout_is(exp_stdout)
        .stderr_is(exp_result.stderr_str())
        .code_is(exp_result.code());
}

#[test]
fn test_id_single_user() {
    let test_users = [&whoami()[..]];

    let ts = TestScenario::new(util_name!());
    let mut exp_result = unwrap_or_return!(expected_result(&ts, &test_users));
    ts.ucmd()
        .args(&test_users)
        .run()
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
        .code_is(exp_result.code());

    // u/g/G z/n
    for opt in ["--user", "--group", "--groups"] {
        let mut args = vec![opt];
        args.extend_from_slice(&test_users);
        exp_result = unwrap_or_return!(expected_result(&ts, &args));
        ts.ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.push("--zero");
        exp_result = unwrap_or_return!(expected_result(&ts, &args));
        ts.ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.push("--name");
        exp_result = unwrap_or_return!(expected_result(&ts, &args));
        ts.ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.pop();
        exp_result = unwrap_or_return!(expected_result(&ts, &args));
        ts.ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
    }
}

#[test]
fn test_id_single_user_non_existing() {
    let args = &["hopefully_non_existing_username"];
    let ts = TestScenario::new(util_name!());
    let result = ts.ucmd().args(args).run();
    let exp_result = unwrap_or_return!(expected_result(&ts, args));

    // It is unknown why on macOS (and possibly others?) `id` adds "Invalid argument".
    // coreutils 8.32: $ LC_ALL=C id foobar
    // macOS: stderr: "id: 'foobar': no such user: Invalid argument"
    // linux: stderr: "id: 'foobar': no such user"
    result
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
        .code_is(exp_result.code());
}

#[test]
fn test_id_name() {
    let ts = TestScenario::new(util_name!());
    for opt in ["--user", "--group", "--groups"] {
        let args = [opt, "--name"];
        let result = ts.ucmd().args(&args).run();
        let exp_result = unwrap_or_return!(expected_result(&ts, &args));
        result
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str())
            .code_is(exp_result.code());

        if opt == "--user" {
            assert_eq!(result.stdout_str().trim_end(), whoami());
        }
    }
}

#[test]
fn test_id_real() {
    let ts = TestScenario::new(util_name!());
    for opt in ["--user", "--group", "--groups"] {
        let args = [opt, "--real"];
        let result = ts.ucmd().args(&args).run();
        let exp_result = unwrap_or_return!(expected_result(&ts, &args));
        result
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str())
            .code_is(exp_result.code());
    }
}

#[test]
#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn test_id_pretty_print() {
    // `-p` is BSD only and not supported on GNU's `id`
    let username = whoami();
    let result = new_ucmd!().arg("-p").run();
    result.success().stdout_contains(username);
}

#[test]
#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn test_id_password_style() {
    // `-P` is BSD only and not supported on GNU's `id`
    let username = whoami();
    let result = new_ucmd!().arg("-P").arg(&username).succeeds();
    assert!(result.stdout_str().starts_with(&username));
}

#[test]
fn test_id_multiple_users() {
    unwrap_or_return!(check_coreutil_version(
        util_name!(),
        VERSION_MIN_MULTIPLE_USERS
    ));

    // Same typical users that GNU test suite is using.
    let test_users = ["root", "man", "postfix", "sshd", &whoami()];

    let ts = TestScenario::new(util_name!());
    let mut exp_result = unwrap_or_return!(expected_result(&ts, &test_users));
    ts.ucmd()
        .args(&test_users)
        .run()
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
        .code_is(exp_result.code());

    // u/g/G z/n
    for opt in ["--user", "--group", "--groups"] {
        let mut args = vec![opt];
        args.extend_from_slice(&test_users);
        exp_result = unwrap_or_return!(expected_result(&ts, &args));
        ts.ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.push("--zero");
        exp_result = unwrap_or_return!(expected_result(&ts, &args));
        ts.ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.push("--name");
        exp_result = unwrap_or_return!(expected_result(&ts, &args));
        ts.ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.pop();
        exp_result = unwrap_or_return!(expected_result(&ts, &args));
        ts.ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
    }
}

#[test]
fn test_id_multiple_users_non_existing() {
    unwrap_or_return!(check_coreutil_version(
        util_name!(),
        VERSION_MIN_MULTIPLE_USERS
    ));

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

    let ts = TestScenario::new(util_name!());
    let mut exp_result = unwrap_or_return!(expected_result(&ts, &test_users));
    ts.ucmd()
        .args(&test_users)
        .run()
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
        .code_is(exp_result.code());

    // u/g/G z/n
    for opt in ["--user", "--group", "--groups"] {
        let mut args = vec![opt];
        args.extend_from_slice(&test_users);
        exp_result = unwrap_or_return!(expected_result(&ts, &args));
        ts.ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.push("--zero");
        exp_result = unwrap_or_return!(expected_result(&ts, &args));
        ts.ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.push("--name");
        exp_result = unwrap_or_return!(expected_result(&ts, &args));
        ts.ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.pop();
        exp_result = unwrap_or_return!(expected_result(&ts, &args));
        ts.ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
    }
}

#[test]
fn test_id_name_or_real_with_default_format() {
    for flag in ["-n", "--name", "-r", "--real"] {
        new_ucmd!()
            .arg(flag)
            .fails()
            .stderr_only("id: printing only names or real IDs requires -u, -g, or -G\n");
    }
}

#[test]
fn test_id_default_format() {
    let ts = TestScenario::new(util_name!());
    for opt1 in ["--name", "--real"] {
        for opt2 in ["--user", "--group", "--groups"] {
            // u/g/G n/r
            let args = [opt2, opt1];
            let result = ts.ucmd().args(&args).run();
            let exp_result = unwrap_or_return!(expected_result(&ts, &args));
            result
                .stdout_is(exp_result.stdout_str())
                .stderr_is(exp_result.stderr_str())
                .code_is(exp_result.code());
        }
    }
    for opt2 in ["--user", "--group", "--groups"] {
        // u/g/G
        let args = [opt2];
        ts.ucmd()
            .args(&args)
            .succeeds()
            .stdout_only(unwrap_or_return!(expected_result(&ts, &args)).stdout_str());
        let args = [opt2, opt2];
        ts.ucmd()
            .args(&args)
            .succeeds()
            .stdout_only(unwrap_or_return!(expected_result(&ts, &args)).stdout_str());
    }
}

#[test]
fn test_id_zero_with_default_format() {
    for z_flag in ["-z", "--zero"] {
        new_ucmd!()
            .arg(z_flag)
            .fails()
            .stderr_only("id: option --zero not permitted in default format\n");
    }
}

#[test]
fn test_id_zero_with_name_or_real() {
    for z_flag in ["-z", "--zero"] {
        for flag in ["-n", "--name", "-r", "--real"] {
            new_ucmd!()
                .args(&[z_flag, flag])
                .fails()
                .stderr_only("id: printing only names or real IDs requires -u, -g, or -G\n");
        }
    }
}

#[test]
fn test_id_zero() {
    let ts = TestScenario::new(util_name!());
    for z_flag in ["-z", "--zero"] {
        for opt1 in ["--name", "--real"] {
            for opt2 in ["--user", "--group", "--groups"] {
                // u/g/G n/r z
                let args = [opt2, z_flag, opt1];
                let result = ts.ucmd().args(&args).run();
                let exp_result = unwrap_or_return!(expected_result(&ts, &args));
                result
                    .stdout_is(exp_result.stdout_str())
                    .stderr_is(exp_result.stderr_str())
                    .code_is(exp_result.code());
            }
        }
        for opt2 in ["--user", "--group", "--groups"] {
            // u/g/G z
            let args = [opt2, z_flag];
            ts.ucmd()
                .args(&args)
                .succeeds()
                .stdout_only(unwrap_or_return!(expected_result(&ts, &args)).stdout_str());
        }
    }
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_id_context() {
    if !uucore::selinux::is_selinux_enabled() {
        println!("test skipped: Kernel has no support for SElinux context");
        return;
    }
    let ts = TestScenario::new(util_name!());
    for c_flag in ["-Z", "--context"] {
        ts.ucmd()
            .args(&[c_flag])
            .succeeds()
            .stdout_only(unwrap_or_return!(expected_result(&ts, &[c_flag])).stdout_str());
        for z_flag in ["-z", "--zero"] {
            let args = [c_flag, z_flag];
            ts.ucmd()
                .args(&args)
                .succeeds()
                .stdout_only(unwrap_or_return!(expected_result(&ts, &args)).stdout_str());
            for opt1 in ["--name", "--real"] {
                // id: cannot print only names or real IDs in default format
                let args = [opt1, c_flag];
                ts.ucmd()
                    .args(&args)
                    .succeeds()
                    .stdout_only(unwrap_or_return!(expected_result(&ts, &args)).stdout_str());
                let args = [opt1, c_flag, z_flag];
                ts.ucmd()
                    .args(&args)
                    .succeeds()
                    .stdout_only(unwrap_or_return!(expected_result(&ts, &args)).stdout_str());
                for opt2 in ["--user", "--group", "--groups"] {
                    // u/g/G n/r z Z
                    // for now, we print clap's standard response for "conflicts_with" instead of:
                    // id: cannot print "only" of more than one choice
                    let args = [opt2, c_flag, opt1];
                    let _result = ts.ucmd().args(&args).fails();
                    // let exp_result = unwrap_or_return!(expected_result(&args));
                    // result
                    //     .stdout_is(exp_result.stdout_str())
                    //     .stderr_is(exp_result.stderr_str())
                    //     .code_is(exp_result.code());
                }
            }
            for opt2 in ["--user", "--group", "--groups"] {
                // u/g/G z Z
                // for now, we print clap's standard response for "conflicts_with" instead of:
                // id: cannot print "only" of more than one choice
                let args = [opt2, c_flag];
                let _result = ts.ucmd().args(&args).fails();
                // let exp_result = unwrap_or_return!(expected_result(&args));
                // result
                //     .stdout_is(exp_result.stdout_str())
                //     .stderr_is(exp_result.stderr_str())
                //     .code_is(exp_result.code());
            }
        }
    }
}

#[test]
fn test_id_no_specified_user_posixly() {
    // gnu/tests/id/no-context.sh

    let ts = TestScenario::new(util_name!());
    let result = ts.ucmd().env("POSIXLY_CORRECT", "1").run();
    assert!(!result.stdout_str().contains("context="));
    if !is_ci() {
        result.success();
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        feature = "feat_selinux"
    ))]
    {
        if uucore::selinux::is_selinux_enabled() {
            let result = ts.ucmd().succeeds();
            assert!(result.stdout_str().contains("context="));
        } else {
            println!("test skipped: Kernel has no support for SElinux context");
        }
    }
}

#[test]
#[cfg(not(target_os = "android"))]
fn test_id_pretty_print_password_record() {
    // `-p` is BSD only and not supported on GNU's `id`.
    // `-P` is our own extension, and not supported by either GNU nor BSD.
    // These must conflict, because they both set the output format.
    new_ucmd!()
        .arg("-p")
        .arg("-P")
        .fails()
        .stderr_contains("the argument '-p' cannot be used with '-P'");
}
