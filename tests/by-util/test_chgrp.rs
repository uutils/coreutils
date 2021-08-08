// spell-checker:ignore (words) nosuchgroup groupname

use crate::common::util::*;
use rust_users::*;

#[test]
fn test_invalid_option() {
    new_ucmd!().arg("-w").arg("/").fails();
}

static DIR: &str = "/tmp";

// we should always get both arguments, regardless of whether --reference was used
#[test]
fn test_help() {
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .stdout_contains("ARGS:\n    <GROUP>      \n    <FILE>...    ");
}

#[test]
fn test_help_ref() {
    new_ucmd!()
        .arg("--help")
        .arg("--reference=ref_file")
        .succeeds()
        .stdout_contains("ARGS:\n    <GROUP>      \n    <FILE>...    ");
}

#[test]
fn test_ref_help() {
    new_ucmd!()
        .arg("--reference=ref_file")
        .arg("--help")
        .succeeds()
        .stdout_contains("ARGS:\n    <GROUP>      \n    <FILE>...    ");
}

#[test]
fn test_invalid_group() {
    new_ucmd!()
        .arg("__nosuchgroup__")
        .arg("/")
        .fails()
        .stderr_is("chgrp: invalid group: __nosuchgroup__");
}

#[test]
fn test_1() {
    if get_effective_gid() != 0 {
        new_ucmd!()
            .arg("bin")
            .arg(DIR)
            .fails()
            .stderr_is("chgrp: changing group of '/tmp': Operation not permitted (os error 1)");
    }
}

#[test]
fn test_fail_silently() {
    if get_effective_gid() != 0 {
        for opt in &["-f", "--silent", "--quiet"] {
            new_ucmd!()
                .arg(opt)
                .arg("bin")
                .arg(DIR)
                .run()
                .fails_silently();
        }
    }
}

#[test]
fn test_preserve_root() {
    // It's weird that on OS X, `realpath /etc/..` returns '/private'
    for d in &[
        "/",
        "/////tmp///../../../../",
        "../../../../../../../../../../../../../../",
        "./../../../../../../../../../../../../../../",
    ] {
        new_ucmd!()
            .arg("--preserve-root")
            .arg("-R")
            .arg("bin").arg(d)
            .fails()
            .stderr_is("chgrp: it is dangerous to operate recursively on '/'\nchgrp: use --no-preserve-root to override this failsafe");
    }
}

#[test]
fn test_preserve_root_symlink() {
    let file = "test_chgrp_symlink2root";
    for d in &[
        "/",
        "////tmp//../../../../",
        "..//../../..//../..//../../../../../../../../",
        ".//../../../../../../..//../../../../../../../",
    ] {
        let (at, mut ucmd) = at_and_ucmd!();
        at.symlink_file(d, file);
        ucmd.arg("--preserve-root")
            .arg("-HR")
            .arg("bin").arg(file)
            .fails()
            .stderr_is("chgrp: it is dangerous to operate recursively on '/'\nchgrp: use --no-preserve-root to override this failsafe");
    }

    let (at, mut ucmd) = at_and_ucmd!();
    at.symlink_file("///usr", file);
    ucmd.arg("--preserve-root")
        .arg("-HR")
        .arg("bin").arg(format!(".//{}/..//..//../../", file))
        .fails()
        .stderr_is("chgrp: it is dangerous to operate recursively on '/'\nchgrp: use --no-preserve-root to override this failsafe");

    let (at, mut ucmd) = at_and_ucmd!();
    at.symlink_file("/", "/tmp/__root__");
    ucmd.arg("--preserve-root")
        .arg("-R")
        .arg("bin").arg("/tmp/__root__/.")
        .fails()
        .stderr_is("chgrp: it is dangerous to operate recursively on '/'\nchgrp: use --no-preserve-root to override this failsafe");

    use std::fs;
    fs::remove_file("/tmp/__root__").unwrap();
}

#[test]
#[cfg(target_os = "linux")]
fn test_reference() {
    // skip for root or MS-WSL
    // * MS-WSL is bugged (as of 2019-12-25), allowing non-root accounts su-level privileges for `chgrp`
    // * for MS-WSL, succeeds and stdout == 'group of /etc retained as root'
    if !(get_effective_gid() == 0 || uucore::os::is_wsl_1()) {
        new_ucmd!()
            .arg("-v")
            .arg("--reference=/etc/passwd")
            .arg("/etc")
            .fails()
            .stderr_is("chgrp: changing group of '/etc': Operation not permitted (os error 1)\nfailed to change group of '/etc' from root to root");
    }
}

#[test]
#[cfg(target_vendor = "apple")]
fn test_reference() {
    new_ucmd!()
        .arg("-v")
        .arg("--reference=ref_file")
        .arg("/etc")
        .fails()
        // group name can differ, so just check the first part of the message
        .stderr_contains("chgrp: changing group of '/etc': Operation not permitted (os error 1)\nfailed to change group of '/etc' from ");
}

#[test]
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
fn test_reference_multi_no_equal() {
    new_ucmd!()
        .arg("-v")
        .arg("--reference")
        .arg("ref_file")
        .arg("file1")
        .arg("file2")
        .succeeds()
        .stderr_contains("chgrp: group of 'file1' retained as ")
        .stderr_contains("\nchgrp: group of 'file2' retained as ");
}

#[test]
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
fn test_reference_last() {
    new_ucmd!()
        .arg("-v")
        .arg("file1")
        .arg("file2")
        .arg("file3")
        .arg("--reference")
        .arg("ref_file")
        .succeeds()
        .stderr_contains("chgrp: group of 'file1' retained as ")
        .stderr_contains("\nchgrp: group of 'file2' retained as ")
        .stderr_contains("\nchgrp: group of 'file3' retained as ");
}

#[test]
fn test_missing_files() {
    new_ucmd!()
        .arg("-v")
        .arg("groupname")
        .fails()
        .stderr_contains(
            "error: The following required arguments were not provided:\n    <FILE>...\n",
        );
}

#[test]
#[cfg(target_os = "linux")]
fn test_big_p() {
    if get_effective_gid() != 0 {
        new_ucmd!()
            .arg("-RP")
            .arg("bin")
            .arg("/proc/self/cwd")
            .fails()
            .stderr_contains(
                "chgrp: changing group of '/proc/self/cwd': Operation not permitted (os error 1)\n",
            );
    }
}

#[test]
#[cfg(target_os = "linux")]
fn test_big_h() {
    if get_effective_gid() != 0 {
        assert!(
            new_ucmd!()
                .arg("-RH")
                .arg("bin")
                .arg("/proc/self/fd")
                .fails()
                .stderr_str()
                .lines()
                .fold(0, |acc, _| acc + 1)
                > 1
        );
    }
}

#[test]
#[cfg(target_os = "linux")]
fn basic_succeeds() {
    let (at, mut ucmd) = at_and_ucmd!();
    let one_group = nix::unistd::getgroups().unwrap();
    // if there are no groups we can't run this test.
    if let Some(group) = one_group.first() {
        at.touch("f1");
        ucmd.arg(group.as_raw().to_string())
            .arg("f1")
            .succeeds()
            .no_stdout()
            .no_stderr();
    }
}
