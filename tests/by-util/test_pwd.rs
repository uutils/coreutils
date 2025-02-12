// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) symdir somefakedir

use std::path::PathBuf;

use uutests::new_ucmd;
use uutests::util::{TestScenario, UCommand};
//use uutests::at_and_ucmd;
use uutests::{at_and_ucmd, util_name};

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

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
    let at = &ts.fixtures;
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cd '{}'; mkdir foo; cd foo; rmdir ../foo; exec '{}' {}",
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

struct Env {
    ucmd: UCommand,
    #[cfg(not(windows))]
    root: String,
    subdir: String,
    symdir: String,
}

fn symlinked_env() -> Env {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("subdir");
    // Note: on Windows this requires admin permissions
    at.symlink_dir("subdir", "symdir");
    let root = PathBuf::from(at.root_dir_resolved());
    ucmd.current_dir(root.join("symdir"));
    #[cfg(not(windows))]
    ucmd.env("PWD", root.join("symdir"));
    Env {
        ucmd,
        #[cfg(not(windows))]
        root: root.to_string_lossy().into_owned(),
        subdir: root.join("subdir").to_string_lossy().into_owned(),
        symdir: root.join("symdir").to_string_lossy().into_owned(),
    }
}

#[test]
fn test_symlinked_logical() {
    let mut env = symlinked_env();
    env.ucmd.arg("-L").succeeds().stdout_is(env.symdir + "\n");
}

#[test]
fn test_symlinked_physical() {
    let mut env = symlinked_env();
    env.ucmd.arg("-P").succeeds().stdout_is(env.subdir + "\n");
}

#[test]
fn test_symlinked_default() {
    let mut env = symlinked_env();
    env.ucmd.succeeds().stdout_is(env.subdir + "\n");
}

#[test]
fn test_symlinked_default_posix() {
    let mut env = symlinked_env();
    env.ucmd
        .env("POSIXLY_CORRECT", "1")
        .succeeds()
        .stdout_is(env.symdir.clone() + "\n");
}

#[test]
fn test_symlinked_default_posix_l() {
    let mut env = symlinked_env();
    env.ucmd
        .env("POSIXLY_CORRECT", "1")
        .arg("-L")
        .succeeds()
        .stdout_is(env.symdir + "\n");
}

#[test]
fn test_symlinked_default_posix_p() {
    let mut env = symlinked_env();
    env.ucmd
        .env("POSIXLY_CORRECT", "1")
        .arg("-P")
        .succeeds()
        .stdout_is(env.subdir + "\n");
}

#[cfg(not(windows))]
pub mod untrustworthy_pwd_var {
    use std::path::Path;

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
        let mut env = symlinked_env();
        env.ucmd
            .arg("-L")
            .env("PWD", env.root)
            .succeeds()
            .stdout_is(env.subdir + "\n");
    }

    #[test]
    fn test_redundant_logical() {
        let mut env = symlinked_env();
        env.ucmd
            .arg("-L")
            .env("PWD", Path::new(&env.symdir).join("."))
            .succeeds()
            .stdout_is(env.subdir + "\n");
    }

    #[test]
    fn test_relative_logical() {
        let mut env = symlinked_env();
        env.ucmd
            .arg("-L")
            .env("PWD", ".")
            .succeeds()
            .stdout_is(env.subdir + "\n");
    }
}
