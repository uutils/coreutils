#[cfg(not(windows))]
use std::fs;

use crate::common::util::*;
use std::env;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_env_help() {
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .no_stderr()
        .stdout_contains("OPTIONS:");
}

#[test]
fn test_env_version() {
    new_ucmd!()
        .arg("--version")
        .succeeds()
        .no_stderr()
        .stdout_contains(util_name!());
}

#[test]
fn test_echo() {
    let result = new_ucmd!().arg("echo").arg("FOO-bar").succeeds();

    assert_eq!(result.stdout_str().trim(), "FOO-bar");
}

#[test]
fn test_file_option() {
    let out = new_ucmd!()
        .arg("-f")
        .arg("vars.conf.txt")
        .run()
        .stdout_move_str();

    assert_eq!(
        out.lines()
            .filter(|&line| line == "FOO=bar" || line == "BAR=bamf this")
            .count(),
        2
    );
}

#[test]
fn test_combined_file_set() {
    let out = new_ucmd!()
        .arg("-f")
        .arg("vars.conf.txt")
        .arg("FOO=bar.alt")
        .run()
        .stdout_move_str();

    assert_eq!(out.lines().filter(|&line| line == "FOO=bar.alt").count(), 1);
}

#[test]
fn test_combined_file_set_unset() {
    let out = new_ucmd!()
        .arg("-u")
        .arg("BAR")
        .arg("-f")
        .arg("vars.conf.txt")
        .arg("FOO=bar.alt")
        .succeeds()
        .stdout_move_str();

    assert_eq!(
        out.lines()
            .filter(|&line| line == "FOO=bar.alt" || line.starts_with("BAR="))
            .count(),
        1
    );
}

#[test]
fn test_single_name_value_pair() {
    let out = new_ucmd!().arg("FOO=bar").run();

    assert!(out.stdout_str().lines().any(|line| line == "FOO=bar"));
}

#[test]
fn test_multiple_name_value_pairs() {
    let out = new_ucmd!().arg("FOO=bar").arg("ABC=xyz").run();

    assert_eq!(
        out.stdout_str()
            .lines()
            .filter(|&line| line == "FOO=bar" || line == "ABC=xyz")
            .count(),
        2
    );
}

#[test]
fn test_ignore_environment() {
    let scene = TestScenario::new(util_name!());

    scene.ucmd().arg("-i").run().no_stdout();
    scene.ucmd().arg("-").run().no_stdout();
}

#[test]
fn test_null_delimiter() {
    let out = new_ucmd!()
        .arg("-i")
        .arg("--null")
        .arg("FOO=bar")
        .arg("ABC=xyz")
        .succeeds()
        .stdout_move_str();

    let mut vars: Vec<_> = out.split('\0').collect();
    assert_eq!(vars.len(), 3);
    vars.sort();
    assert_eq!(vars[0], "");
    assert_eq!(vars[1], "ABC=xyz");
    assert_eq!(vars[2], "FOO=bar");
}

#[test]
fn test_unset_variable() {
    // This test depends on the HOME variable being pre-defined by the
    // default shell
    let out = TestScenario::new(util_name!())
        .ucmd_keepenv()
        .arg("-u")
        .arg("HOME")
        .succeeds()
        .stdout_move_str();

    assert_eq!(out.lines().any(|line| line.starts_with("HOME=")), false);
}

#[test]
fn test_fail_null_with_program() {
    new_ucmd!()
        .arg("--null")
        .arg("cd")
        .fails()
        .stderr_contains("cannot specify --null (-0) with command");
}

#[cfg(not(windows))]
#[test]
fn test_change_directory() {
    let scene = TestScenario::new(util_name!());
    let temporary_directory = tempdir().unwrap();
    let temporary_path = fs::canonicalize(temporary_directory.path()).unwrap();
    assert_ne!(env::current_dir().unwrap(), temporary_path);

    // command to print out current working directory
    let pwd = "pwd";

    let out = scene
        .ucmd()
        .arg("--chdir")
        .arg(&temporary_path)
        .arg(pwd)
        .succeeds()
        .stdout_move_str();
    assert_eq!(out.trim(), temporary_path.as_os_str())
}

// no way to consistently get "current working directory", `cd` doesn't work @ CI
// instead, we test that the unique temporary directory appears somewhere in the printed variables
#[cfg(windows)]
#[test]
fn test_change_directory() {
    let scene = TestScenario::new(util_name!());
    let temporary_directory = tempdir().unwrap();
    let temporary_path = temporary_directory.path();

    assert_ne!(env::current_dir().unwrap(), temporary_path);

    let out = scene
        .ucmd()
        .arg("--chdir")
        .arg(&temporary_path)
        .succeeds()
        .stdout_move_str();
    assert_eq!(
        out.lines()
            .any(|line| line.ends_with(temporary_path.file_name().unwrap().to_str().unwrap())),
        false
    );
}

#[test]
fn test_fail_change_directory() {
    let scene = TestScenario::new(util_name!());
    let some_non_existing_path = "some_nonexistent_path";
    assert_eq!(Path::new(some_non_existing_path).is_dir(), false);

    let out = scene
        .ucmd()
        .arg("--chdir")
        .arg(some_non_existing_path)
        .arg("pwd")
        .fails()
        .stderr_move_str();
    assert!(out.contains("env: cannot change directory to "));
}
