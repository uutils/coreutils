// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore getpriority setpriority

#[cfg(unix)]
use uutests::new_ucmd;

#[test]
#[cfg(unix)]
#[cfg(not(target_os = "android"))]
fn test_get_current_niceness() {
    // Test that the nice command with no arguments returns the default nice value
    let nice = rustix::process::getpriority_process(None).unwrap();
    new_ucmd!().succeeds().stdout_is(format!("{nice}\n"));
}

#[test]
#[cfg(unix)]
#[cfg(not(target_os = "android"))]
fn test_nice_adj_negative() {
    // This assumes the test suite is run as a normal (non-root) user, and as
    // such attempting to set a negative niceness value will be rejected by
    // the OS.  If it gets denied, then we know a negative value was parsed
    // correctly.

    new_ucmd!()
        .args(&["--adj", "-20", "true"])
        .succeeds()
        .stderr_is("nice: warning: setpriority: Permission denied\n");
}

#[test]
#[cfg(unix)]
fn test_adjustment_with_no_command_should_error() {
    new_ucmd!()
        .args(&["-n", "19"])
        .fails()
        .usage_error("A command must be given with an adjustment.");
}

#[test]
#[cfg(unix)]
fn test_command_with_no_adjustment() {
    new_ucmd!().args(&["echo", "a"]).succeeds().stdout_is("a\n");
}

#[test]
#[cfg(unix)]
fn test_command_with_no_args() {
    new_ucmd!()
        .args(&["-n", "19", "echo"])
        .succeeds()
        .stdout_is("\n");
}

#[test]
#[cfg(unix)]
fn test_command_with_args() {
    new_ucmd!()
        .args(&["-n", "19", "echo", "a", "b", "c"])
        .succeeds()
        .stdout_is("a b c\n");
}

#[test]
#[cfg(unix)]
fn test_command_where_command_takes_n_flag() {
    new_ucmd!()
        .args(&["-n", "19", "echo", "-n", "a"])
        .succeeds()
        .stdout_is("a");
}

#[test]
#[cfg(unix)]
fn test_command_with_non_utf8_args() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    new_ucmd!()
        .args(&["-n", "19", "echo"])
        .arg(OsStr::from_bytes(b"a\xffb"))
        .succeeds()
        .stdout_only_bytes(b"a\xffb\n");
}

#[test]
#[cfg(unix)]
fn test_non_utf8_command_not_found() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    new_ucmd!()
        .arg(OsStr::from_bytes(b"no-such-command-\xff"))
        .fails_with_code(127)
        .no_stdout()
        .stderr_contains("no-such-command-\\xFF");
}

#[test]
#[cfg(all(unix, not(target_vendor = "apple")))]
fn test_non_utf8_command_path() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use uutests::util::TestScenario;
    use uutests::util_name;

    // The path of the command is not valid UTF-8 and must reach exec unchanged.
    let ts = TestScenario::new(util_name!());
    let dir = ts.fixtures.plus(OsStr::from_bytes(b"dir-\xff"));
    std::fs::create_dir(&dir).unwrap();
    let cmd = dir.join("coreutils");
    std::os::unix::fs::symlink(&ts.bin_path, &cmd).unwrap();
    ts.ucmd()
        .arg(&cmd)
        .args(&["nice", "--version"])
        .succeeds()
        .stdout_only(format!("nice {}\n", uucore::crate_version!()));
}

#[test]
#[cfg(unix)]
fn test_invalid_argument() {
    new_ucmd!().arg("--invalid").fails_with_code(125);
}

#[test]
#[cfg(unix)]
fn test_bare_adjustment() {
    new_ucmd!()
        .args(&["-1", "echo", "-n", "a"])
        .succeeds()
        .stdout_is("a");
}

#[test]
#[cfg(unix)]
fn test_trailing_empty_adjustment() {
    new_ucmd!()
        .args(&["-n", "1", "-n"])
        .fails()
        .stderr_str()
        .starts_with(
        "error: The argument '--adjustment <adjustment>' requires a value but none was supplied",
    );
}

#[test]
#[cfg(unix)]
fn test_nice_huge() {
    new_ucmd!()
        .args(&[
            "-n",
            "99999999999999999999999999999999999999999999999999999999999999999999999999999999999999999",
            "true",
        ])
        .succeeds()
        .no_stdout();
}

#[test]
#[cfg(unix)]
fn test_nice_huge_negative() {
    new_ucmd!().args(&["-n", "-9999999999", "true"]).succeeds();
    //.stderr_contains("Permission denied"); Depending on platform?
}

#[test]
#[cfg(unix)]
fn test_sign_middle() {
    new_ucmd!()
        .args(&["-n", "-2+4", "true"])
        .fails_with_code(125)
        .no_stdout()
        .stderr_contains("invalid");
}
//uu: "-2+4" is not a valid number: invalid digit found in string
//gnu: invalid adjustment `-2+4'
//Both message is fine
