use crate::common::util::*;

// spell-checker:ignore (ToDO) coreutil

// These tests run the GNU coreutils `(g)groups` binary in `$PATH` in order to gather reference values.
// If the `(g)groups` in `$PATH` doesn't include a coreutils version string,
// or the version is too low, the test is skipped.

// The reference version is 8.32. Here 8.30 was chosen because right now there's no
// ubuntu image for github action available with a higher version than 8.30.
const VERSION_MIN: &str = "8.30"; // minimum Version for the reference `groups` in $PATH
const VERSION_MIN_MULTIPLE_USERS: &str = "8.31"; // this feature was introduced in GNU's coreutils 8.31
const UUTILS_WARNING: &str = "uutils-tests-warning";
const UUTILS_INFO: &str = "uutils-tests-info";

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
fn test_groups() {
    let result = new_ucmd!().run();
    let exp_result = unwrap_or_return!(expected_result(&[]));

    result
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str())
        .code_is(exp_result.code());
}

#[test]
#[cfg(unix)]
fn test_groups_username() {
    let test_users = [&whoami()[..]];

    let result = new_ucmd!().args(&test_users).run();
    let exp_result = unwrap_or_return!(expected_result(&test_users));

    result
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str())
        .code_is(exp_result.code());
}

#[test]
#[cfg(unix)]
fn test_groups_username_multiple() {
    // TODO: [2021-06; jhscheer] refactor this as `let util_name = host_name_for(util_name!())` when that function is added to 'tests/common'
    #[cfg(target_os = "linux")]
    let util_name = util_name!();
    #[cfg(all(unix, not(target_os = "linux")))]
    let util_name = &format!("g{}", util_name!());
    let version_check_string = check_coreutil_version(util_name, VERSION_MIN_MULTIPLE_USERS);
    if version_check_string.starts_with(UUTILS_WARNING) {
        println!("{}\ntest skipped", version_check_string);
        return;
    }
    let test_users = ["root", "man", "postfix", "sshd", &whoami()];

    let result = new_ucmd!().args(&test_users).run();
    let exp_result = unwrap_or_return!(expected_result(&test_users));

    result
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str())
        .code_is(exp_result.code());
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
    // TODO: [2021-06; jhscheer] refactor this as `let util_name = host_name_for(util_name!())` when that function is added to 'tests/common'
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
