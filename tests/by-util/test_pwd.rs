// spell-checker:ignore (words) symdir somefakedir

use crate::common::util::*;

#[test]
fn test_default() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.succeeds().stdout_is(at.root_dir_resolved() + "\n");
}

#[test]
fn test_failed() {
    let (_at, mut ucmd) = at_and_ucmd!();
    ucmd.arg("will-fail").fails();
}

#[cfg(unix)]
#[test]
fn test_deleted_dir() {
    use std::process::Command;

    let ts = TestScenario::new(util_name!());
    let at = ts.fixtures.clone();
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cd '{}'; mkdir foo; cd foo; rmdir ../foo; exec {} {}",
            at.root_dir_resolved(),
            ts.bin_path.to_str().unwrap(),
            ts.util_name,
        ))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(
        output.stderr,
        b"pwd: failed to get current directory: No such file or directory\n"
    );
}

fn symlinked_at_and_ucmd() -> (AtPath, UCommand) {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("subdir");
    // Note: on Windows this requires admin permissions
    at.symlink_dir("subdir", "symdir");
    ucmd.raw.current_dir(at.plus("symdir"));
    #[cfg(not(windows))]
    ucmd.env("PWD", at.plus("symdir"));
    (at, ucmd)
}

#[test]
fn test_symlinked_logical() {
    let (at, mut ucmd) = symlinked_at_and_ucmd();
    ucmd.arg("-L")
        .succeeds()
        .stdout_is(at.plus("symdir").to_string_lossy() + "\n");
}

#[test]
fn test_symlinked_physical() {
    let (at, mut ucmd) = symlinked_at_and_ucmd();
    ucmd.arg("-P")
        .succeeds()
        .stdout_is(at.plus("subdir").to_string_lossy() + "\n");
}

#[test]
fn test_symlinked_default() {
    let (at, mut ucmd) = symlinked_at_and_ucmd();
    ucmd.succeeds()
        .stdout_is(at.plus("subdir").to_string_lossy() + "\n");
}

#[cfg(not(windows))]
pub mod untrustworthy_pwd_var {
    use super::*;

    #[test]
    fn test_nonexistent_logical() {
        let (at, mut ucmd) = at_and_ucmd!();
        ucmd.arg("-L")
            .env("PWD", "/somefakedir")
            .succeeds()
            .stdout_is(at.root_dir_resolved() + "\n");
    }

    #[test]
    fn test_wrong_logical() {
        let (at, mut ucmd) = symlinked_at_and_ucmd();
        ucmd.arg("-L")
            .env("PWD", at.root_dir_resolved())
            .succeeds()
            .stdout_is(at.plus("subdir").to_string_lossy() + "\n");
    }

    #[test]
    fn test_redundant_logical() {
        let (at, mut ucmd) = symlinked_at_and_ucmd();
        ucmd.arg("-L")
            .env("PWD", at.plus("symdir").join("."))
            .succeeds()
            .stdout_is(at.plus("subdir").to_string_lossy() + "\n");
    }

    #[test]
    fn test_relative_logical() {
        let (at, mut ucmd) = symlinked_at_and_ucmd();
        ucmd.arg("-L")
            .env("PWD", ".")
            .succeeds()
            .stdout_is(at.plus("subdir").to_string_lossy() + "\n");
    }
}
