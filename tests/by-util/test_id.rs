use crate::common::util::*;

// spell-checker:ignore (ToDO) coreutil

// These tests run the GNU coreutils `(g)id` binary in `$PATH` in order to gather reference values.
// If the `(g)id` in `$PATH` doesn't include a coreutils version string,
// or the version is too low, the test is skipped.

// The reference version is 8.32. Here 8.30 was chosen because right now there's no
// ubuntu image for github action available with a higher version than 8.30.
const VERSION_MIN: &str = "8.30"; // minimum Version for the reference `id` in $PATH
const VERSION_MIN_MULTIPLE_USERS: &str = "8.31"; // this feature was introduced in GNU's coreutils 8.31
const UUTILS_WARNING: &str = "uutils-tests-warning";
const UUTILS_INFO: &str = "uutils-tests-info";

#[allow(clippy::needless_return)]
macro_rules! unwrap_or_return {
    ( $e:expr ) => {
        match $e {
            Ok(x) => x,
            Err(e) => {
                println!("{}: test skipped: {}", UUTILS_INFO, e);
                return;
            }
        }
    };
}

fn whoami() -> String {
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

    // Use environment variable to get current user instead of
    // invoking `whoami` and fall back to user "nobody" on error.
    std::env::var("USER").unwrap_or_else(|e| {
        println!("{}: {}, using \"nobody\" instead", UUTILS_WARNING, e);
        "nobody".to_string()
    })
}

#[test]
#[cfg(unix)]
fn test_id_no_specified_user() {
    let result = new_ucmd!().run();
    let exp_result = unwrap_or_return!(expected_result(&[]));
    let mut _exp_stdout = exp_result.stdout_str().to_string();

    result
        .stdout_is(_exp_stdout)
        .stderr_is(exp_result.stderr_str())
        .code_is(exp_result.code());
}

#[test]
#[cfg(unix)]
fn test_id_single_user() {
    let test_users = [&whoami()[..]];

    let scene = TestScenario::new(util_name!());
    let mut exp_result = unwrap_or_return!(expected_result(&test_users));
    scene
        .ucmd()
        .args(&test_users)
        .run()
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
        .code_is(exp_result.code());

    // u/g/G z/n
    for &opt in &["--user", "--group", "--groups"] {
        let mut args = vec![opt];
        args.extend_from_slice(&test_users);
        exp_result = unwrap_or_return!(expected_result(&args));
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.push("--zero");
        exp_result = unwrap_or_return!(expected_result(&args));
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.push("--name");
        exp_result = unwrap_or_return!(expected_result(&args));
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.pop();
        exp_result = unwrap_or_return!(expected_result(&args));
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
    }
}

#[test]
#[cfg(unix)]
fn test_id_single_user_non_existing() {
    let args = &["hopefully_non_existing_username"];
    let result = new_ucmd!().args(args).run();
    let exp_result = unwrap_or_return!(expected_result(args));

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
#[cfg(unix)]
fn test_id_name() {
    let scene = TestScenario::new(util_name!());
    for &opt in &["--user", "--group", "--groups"] {
        let args = [opt, "--name"];
        let result = scene.ucmd().args(&args).run();
        let exp_result = unwrap_or_return!(expected_result(&args));
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
#[cfg(unix)]
fn test_id_real() {
    let scene = TestScenario::new(util_name!());
    for &opt in &["--user", "--group", "--groups"] {
        let args = [opt, "--real"];
        let result = scene.ucmd().args(&args).run();
        let exp_result = unwrap_or_return!(expected_result(&args));
        result
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str())
            .code_is(exp_result.code());
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
    #[cfg(target_os = "linux")]
    let util_name = util_name!();
    #[cfg(all(unix, not(target_os = "linux")))]
    let util_name = &format!("g{}", util_name!());
    let version_check_string = check_coreutil_version(util_name, VERSION_MIN_MULTIPLE_USERS);
    if version_check_string.starts_with(UUTILS_WARNING) {
        println!("{}\ntest skipped", version_check_string);
        return;
    }

    // Same typical users that GNU test suite is using.
    let test_users = ["root", "man", "postfix", "sshd", &whoami()];

    let scene = TestScenario::new(util_name!());
    let mut exp_result = unwrap_or_return!(expected_result(&test_users));
    scene
        .ucmd()
        .args(&test_users)
        .run()
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
        .code_is(exp_result.code());

    // u/g/G z/n
    for &opt in &["--user", "--group", "--groups"] {
        let mut args = vec![opt];
        args.extend_from_slice(&test_users);
        exp_result = unwrap_or_return!(expected_result(&args));
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.push("--zero");
        exp_result = unwrap_or_return!(expected_result(&args));
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.push("--name");
        exp_result = unwrap_or_return!(expected_result(&args));
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.pop();
        exp_result = unwrap_or_return!(expected_result(&args));
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
    }
}

#[test]
#[cfg(unix)]
fn test_id_multiple_users_non_existing() {
    #[cfg(target_os = "linux")]
    let util_name = util_name!();
    #[cfg(all(unix, not(target_os = "linux")))]
    let util_name = &format!("g{}", util_name!());
    let version_check_string = check_coreutil_version(util_name, VERSION_MIN_MULTIPLE_USERS);
    if version_check_string.starts_with(UUTILS_WARNING) {
        println!("{}\ntest skipped", version_check_string);
        return;
    }

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
    let mut exp_result = unwrap_or_return!(expected_result(&test_users));
    scene
        .ucmd()
        .args(&test_users)
        .run()
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
        .code_is(exp_result.code());

    // u/g/G z/n
    for &opt in &["--user", "--group", "--groups"] {
        let mut args = vec![opt];
        args.extend_from_slice(&test_users);
        exp_result = unwrap_or_return!(expected_result(&args));
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.push("--zero");
        exp_result = unwrap_or_return!(expected_result(&args));
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.push("--name");
        exp_result = unwrap_or_return!(expected_result(&args));
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
            .code_is(exp_result.code());
        args.pop();
        exp_result = unwrap_or_return!(expected_result(&args));
        scene
            .ucmd()
            .args(&args)
            .run()
            .stdout_is(exp_result.stdout_str())
            .stderr_is(exp_result.stderr_str().replace(": Invalid argument", ""))
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
            .stderr_only(unwrap_or_return!(expected_result(&args)).stderr_str());
        for &opt2 in &["--user", "--group", "--groups"] {
            // u/g/G n/r
            let args = [opt2, opt1];
            let result = scene.ucmd().args(&args).run();
            let exp_result = unwrap_or_return!(expected_result(&args));
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
            .stdout_only(unwrap_or_return!(expected_result(&args)).stdout_str());
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
            .stderr_only(unwrap_or_return!(expected_result(&[z_flag])).stderr_str());
        for &opt1 in &["--name", "--real"] {
            // id: cannot print only names or real IDs in default format
            let args = [opt1, z_flag];
            scene
                .ucmd()
                .args(&args)
                .fails()
                .stderr_only(unwrap_or_return!(expected_result(&args)).stderr_str());
            for &opt2 in &["--user", "--group", "--groups"] {
                // u/g/G n/r z
                let args = [opt2, z_flag, opt1];
                let result = scene.ucmd().args(&args).run();
                let exp_result = unwrap_or_return!(expected_result(&args));
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
                .stdout_only(unwrap_or_return!(expected_result(&args)).stdout_str());
        }
    }
}

#[test]
#[cfg(target_os = "linux")]
fn test_id_context() {
    use selinux::{self, KernelSupport};
    if selinux::kernel_support() == KernelSupport::Unsupported {
        println!(
            "{}: test skipped: Kernel has no support for SElinux context",
            UUTILS_INFO
        );
        return;
    }
    let scene = TestScenario::new(util_name!());
    for c_flag in &["-Z", "--context"] {
        scene
            .ucmd()
            .args(&[c_flag])
            .succeeds()
            .stdout_only(unwrap_or_return!(expected_result(&[c_flag])).stdout_str());
        for &z_flag in &["-z", "--zero"] {
            let args = [c_flag, z_flag];
            scene
                .ucmd()
                .args(&args)
                .succeeds()
                .stdout_only(unwrap_or_return!(expected_result(&args)).stdout_str());
            for &opt1 in &["--name", "--real"] {
                // id: cannot print only names or real IDs in default format
                let args = [opt1, c_flag];
                scene
                    .ucmd()
                    .args(&args)
                    .succeeds()
                    .stdout_only(unwrap_or_return!(expected_result(&args)).stdout_str());
                let args = [opt1, c_flag, z_flag];
                scene
                    .ucmd()
                    .args(&args)
                    .succeeds()
                    .stdout_only(unwrap_or_return!(expected_result(&args)).stdout_str());
                for &opt2 in &["--user", "--group", "--groups"] {
                    // u/g/G n/r z Z
                    // for now, we print clap's standard response for "conflicts_with" instead of:
                    // id: cannot print "only" of more than one choice
                    let args = [opt2, c_flag, opt1];
                    let _result = scene.ucmd().args(&args).fails();
                    // let exp_result = unwrap_or_return!(expected_result(&args));
                    // result
                    //     .stdout_is(exp_result.stdout_str())
                    //     .stderr_is(exp_result.stderr_str())
                    //     .code_is(exp_result.code());
                }
            }
            for &opt2 in &["--user", "--group", "--groups"] {
                // u/g/G z Z
                // for now, we print clap's standard response for "conflicts_with" instead of:
                // id: cannot print "only" of more than one choice
                let args = [opt2, c_flag];
                let _result = scene.ucmd().args(&args).fails();
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
#[cfg(unix)]
fn test_id_no_specified_user_posixly() {
    // gnu/tests/id/no-context.sh

    let scene = TestScenario::new(util_name!());
    let result = scene.ucmd().env("POSIXLY_CORRECT", "1").succeeds();
    assert!(!result.stdout_str().contains("context="));

    #[cfg(target_os = "linux")]
    {
        use selinux::{self, KernelSupport};
        if selinux::kernel_support() == KernelSupport::Unsupported {
            println!(
                "{}: test skipped: Kernel has no support for SElinux context",
                UUTILS_INFO
            );
            return;
        } else {
            let result = scene.ucmd().succeeds();
            assert!(result.stdout_str().contains("context="));
        }
    }
}

fn check_coreutil_version(util_name: &str, version_expected: &str) -> String {
    // example:
    // $ id --version | head -n 1
    // id (GNU coreutils) 8.32.162-4eda
    let scene = TestScenario::new(util_name);
    let version_check = scene
        .cmd_keepenv(&util_name)
        .env("LC_ALL", "C")
        .arg("--version")
        .run();
    version_check
        .stdout_str()
        .split('\n')
        .collect::<Vec<_>>()
        .get(0)
        .map_or_else(
            || format!("{}: unexpected output format for reference coreutil: '{} --version'", UUTILS_WARNING, util_name),
            |s| {
                if s.contains(&format!("(GNU coreutils) {}", version_expected)) {
                    s.to_string()
                } else if s.contains("(GNU coreutils)") {
                    let version_found = s.split_whitespace().last().unwrap()[..4].parse::<f32>().unwrap_or_default();
                    let version_expected = version_expected.parse::<f32>().unwrap_or_default();
                    if version_found > version_expected {
                    format!("{}: version for the reference coreutil '{}' is higher than expected; expected: {}, found: {}", UUTILS_INFO, util_name, version_expected, version_found)
                    } else {
                    format!("{}: version for the reference coreutil '{}' does not match; expected: {}, found: {}", UUTILS_WARNING, util_name, version_expected, version_found) }
                } else {
                    format!("{}: no coreutils version string found for reference coreutils '{} --version'", UUTILS_WARNING, util_name)
                }
            },
        )
}

#[allow(clippy::needless_borrow)]
#[cfg(unix)]
fn expected_result(args: &[&str]) -> Result<CmdResult, String> {
    #[cfg(target_os = "linux")]
    let util_name = util_name!();
    #[cfg(all(unix, not(target_os = "linux")))]
    let util_name = &format!("g{}", util_name!());

    let version_check_string = check_coreutil_version(util_name, VERSION_MIN);
    if version_check_string.starts_with(UUTILS_WARNING) {
        return Err(version_check_string);
    }
    println!("{}", version_check_string);

    let scene = TestScenario::new(util_name);
    let result = scene
        .cmd_keepenv(util_name)
        .env("LC_ALL", "C")
        .args(args)
        .run();

    let (stdout, stderr): (String, String) = if cfg!(target_os = "linux") {
        (
            result.stdout_str().to_string(),
            result.stderr_str().to_string(),
        )
    } else {
        // strip 'g' prefix from results:
        let from = util_name.to_string() + ":";
        let to = &from[1..];
        (
            result.stdout_str().replace(&from, to),
            result.stderr_str().replace(&from, to),
        )
    };

    Ok(CmdResult::new(
        Some(result.tmpd()),
        Some(result.code()),
        result.succeeded(),
        stdout.as_bytes(),
        stderr.as_bytes(),
    ))
}
