// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (words) symdir somefakedir

use std::path::PathBuf;

use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::UCommand;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_default() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.succeeds().stdout_is(at.root_dir_resolved() + "\n");
}

#[test]
fn test_ignores_non_option_arguments() {
    // GNU pwd ignores non-option operands, warning on stderr but exiting 0.
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg("will-fail")
        .succeeds()
        .stdout_is(at.root_dir_resolved() + "\n")
        .stderr_is("pwd: ignoring non-option arguments\n");
}

#[cfg(unix)]
#[uutests::wasi_ignore(NoSubprocessSpawning)]
#[test]
fn test_deleted_dir() {
    use std::process::Command;
    use uutests::util::TestScenario;
    use uutests::util_name;

    for replacement in ["deleted", "file"] {
        for flag in ["-L", "-P"] {
            let scene = TestScenario::new(util_name!());
            let output = Command::new("sh")
                .args([
                    "-c",
                    "mkdir \"$2\" && cd \"$2\" && rmdir \"$2\" && { [ \"$3\" = deleted ] || : > \"$2\"; } && exec \"$1\" pwd \"$4\"",
                    "sh",
                ])
                .arg(&scene.bin_path)
                .arg(scene.fixtures.plus("work"))
                .arg(replacement)
                .arg(flag)
                .env("LC_ALL", "C")
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert_eq!(
                output.stderr,
                b"pwd: couldn't find directory entry in '..' with matching i-node\n"
            );
        }
    }
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

#[uutests::wasi_ignore(GuestRootPathResolution)]
#[test]
fn test_symlinked_logical() {
    let mut env = symlinked_env();
    env.ucmd.arg("-L").succeeds().stdout_is(env.symdir + "\n");
}

#[uutests::wasi_ignore(GuestRootPathResolution)]
#[test]
fn test_symlinked_physical() {
    let mut env = symlinked_env();
    env.ucmd.arg("-P").succeeds().stdout_is(env.subdir + "\n");
}

#[uutests::wasi_ignore(GuestRootPathResolution)]
#[test]
fn test_symlinked_default() {
    let mut env = symlinked_env();
    env.ucmd.succeeds().stdout_is(env.subdir + "\n");
}

#[uutests::wasi_ignore(GuestRootPathResolution)]
#[test]
fn test_symlinked_default_posix() {
    let mut env = symlinked_env();
    env.ucmd
        .env("POSIXLY_CORRECT", "1")
        .succeeds()
        .stdout_is(env.symdir.clone() + "\n");
}

#[uutests::wasi_ignore(GuestRootPathResolution)]
#[test]
fn test_symlinked_default_posix_l() {
    let mut env = symlinked_env();
    env.ucmd
        .env("POSIXLY_CORRECT", "1")
        .arg("-L")
        .succeeds()
        .stdout_is(env.symdir + "\n");
}

#[uutests::wasi_ignore(GuestRootPathResolution)]
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

    #[uutests::wasi_ignore(GuestRootPathResolution)]
    #[test]
    fn test_nonexistent_logical() {
        let (at, mut ucmd) = at_and_ucmd!();
        ucmd.arg("-L")
            .env("PWD", "/somefakedir")
            .succeeds()
            .stdout_is(at.root_dir_resolved() + "\n");
    }

    #[uutests::wasi_ignore(GuestRootPathResolution)]
    #[test]
    fn test_wrong_logical() {
        let mut env = symlinked_env();
        env.ucmd
            .arg("-L")
            .env("PWD", env.root)
            .succeeds()
            .stdout_is(env.subdir + "\n");
    }

    #[uutests::wasi_ignore(GuestRootPathResolution)]
    #[test]
    fn test_redundant_logical() {
        let mut env = symlinked_env();
        env.ucmd
            .arg("-L")
            .env("PWD", Path::new(&env.symdir).join("."))
            .succeeds()
            .stdout_is(env.subdir + "\n");
    }

    #[uutests::wasi_ignore(GuestRootPathResolution)]
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

/// `pwd -P` must not leak Windows' `\\?\` verbatim prefix that
/// `std::fs::canonicalize` produces. The fixture root is already resolved, so
/// running `pwd -P` inside a canonicalized subdirectory must print a plain
/// drive path.
#[cfg(windows)]
#[test]
fn test_physical_no_verbatim_prefix() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("subdir");

    // canonicalize() yields a verbatim path (\\?\D:\...) on Windows; run the
    // utility from inside it so `current_dir` reports the verbatim form.
    let dir = at.plus("subdir").canonicalize().unwrap();

    ucmd.arg("-P")
        .current_dir(&dir)
        .succeeds()
        .stdout_does_not_contain(r"\\?\")
        .stdout_is(format!("{}\n", dunce::simplified(&dir).display()));
}
