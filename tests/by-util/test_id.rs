// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) coreutil

use crate::common::util::{is_ci, whoami, TestScenario};
use regex::Regex;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
#[cfg(unix)]
fn test_id_no_specified_user() {
    new_ucmd!().succeeds().stdout_matches(
        &Regex::new(r#"(uid=\d+\(\w+\) gid=\d+\(\w+\) groups=\d+\(\w+\)(,\d+\(\w+\))*)+"#).unwrap(),
    );
}

#[test]
#[cfg(unix)]
fn test_id_single_user() {
    let test_user = whoami();

    new_ucmd!().arg(&test_user).succeeds().stdout_matches(
        &Regex::new(r#"uid=\d+\(\w+\) gid=\d+\(\w+\) groups=\d+\(\w+\)(,\d+\(\w+\))*"#).unwrap(),
    );

    // u/g/G z/n
    for opt in ["--user", "--group", "--groups"] {
        new_ucmd!()
            .arg(&test_user)
            .arg(opt)
            .succeeds()
            .stdout_matches(&Regex::new(r#"\d+"#).unwrap());

        new_ucmd!()
            .arg(&test_user)
            .arg(opt)
            .arg("--zero")
            .succeeds()
            .stdout_matches(&Regex::new(r#"\d+"#).unwrap());

        new_ucmd!()
            .arg(&test_user)
            .arg(opt)
            .arg("--name")
            .succeeds()
            .stdout_matches(&Regex::new(r#"\w+"#).unwrap());
    }
}

#[test]
#[cfg(unix)]
fn test_id_single_user_non_existing() {
    new_ucmd!()
        .arg("hopefully_non_existing_username")
        .fails()
        .stderr_contains("no such user");
}

#[test]
#[cfg(unix)]
fn test_id_name() {
    new_ucmd!()
        .args(&["--user", "--name"])
        .succeeds()
        .stdout_is(format!("{}\n", whoami()));

    for opt in ["--group", "--groups"] {
        let args = [opt, "--name"];
        new_ucmd!()
            .args(&args)
            .succeeds()
            .stdout_matches(&Regex::new(r#"\w+"#).unwrap());
    }
}

#[test]
#[cfg(unix)]
fn test_id_real() {
    for opt in ["--user", "--group", "--groups"] {
        new_ucmd!()
            .arg(opt)
            .arg("--real")
            .succeeds()
            .stdout_matches(&Regex::new(r#"\d+"#).unwrap());
    }
}

#[test]
#[cfg(unix)]
fn test_id_pretty_print() {
    // `-p` is BSD only and not supported on GNU's `id`
    let username = whoami();
    new_ucmd!().arg("-p").succeeds().stdout_contains(username);
}

#[test]
#[cfg(unix)]
fn test_id_password_style() {
    // `-P` is BSD only and not supported on GNU's `id`
    let username = whoami();
    new_ucmd!()
        .arg("-P")
        .arg(&username)
        .succeeds()
        .stdout_str()
        .starts_with(&username);
}

#[test]
#[cfg(unix)]
fn test_id_multiple_users() {
    // Same typical users that GNU test suite is using.
    let test_users = ["root", "man", "postfix", "sshd", &whoami()];

    let result = new_ucmd!().args(&test_users).stderr_to_stdout().run();
    let lines = test_users.iter().zip(result.stdout_str().lines());
    for (name, line) in lines {
        let line_regex = Regex::new(&format!(
            "uid=\\d+\\({name}\\) gid=\\d+\\({name}\\) groups=.*"
        ))
        .unwrap();
        assert!(line_regex.is_match(line) || line.contains("no such user"));
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

    let result = new_ucmd!().args(&test_users).stderr_to_stdout().run();
    let lines = test_users.iter().zip(result.stdout_str().lines());
    for (name, line) in lines {
        dbg!(name);
        let line_regex = Regex::new(&format!(
            "uid=\\d+\\({name}\\) gid=\\d+\\({name}\\) groups=.*"
        ))
        .unwrap();
        assert!(line_regex.is_match(line) || line.contains("no such user"));
    }
}

#[test]
#[cfg(unix)]
fn test_id_zero() {
    for z_flag in ["-z", "--zero"] {
        new_ucmd!()
            .arg(z_flag)
            .fails()
            .stderr_contains("not permitted in default format");
        for opt1 in ["--name", "--real"] {
            new_ucmd!()
                .arg(opt1)
                .arg(z_flag)
                .fails()
                .stderr_contains("cannot print only names or real IDs in default format");
        }
    }
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_id_context() {
    use selinux::{self, KernelSupport};
    if selinux::kernel_support() == KernelSupport::Unsupported {
        println!("test skipped: Kernel has no support for SElinux context",);
        return;
    }
    let ts = TestScenario::new(util_name!());
    for c_flag in ["-Z", "--context"] {
        new_ucmd!().arg(c_flag).succeeds();
        for z_flag in ["-z", "--zero"] {
            ts.ucmd().arg(&[c_flag, z_flag]).fails();
            for opt1 in ["--name", "--real"] {
                // id: cannot print only names or real IDs in default format
                ts.ucmd().arg(&[opt1, c_flag]).fails();
                ts.ucmd().arg(&[opt1, c_flag, z_flag]).fails();
                for opt2 in ["--user", "--group", "--groups"] {
                    ts.ucmd().args(&[opt2, c_flag, opt1]).succeeds();
                }
            }
            for opt2 in ["--user", "--group", "--groups"] {
                ts.ucmd().args(&[opt2, c_flag]).succeeds();
            }
        }
    }
}

#[test]
#[cfg(unix)]
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
        use selinux::{self, KernelSupport};
        if selinux::kernel_support() == KernelSupport::Unsupported {
            println!("test skipped: Kernel has no support for SElinux context",);
        } else {
            let result = ts.ucmd().succeeds();
            assert!(result.stdout_str().contains("context="));
        }
    }
}
