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
        .stderr_is("chgrp: invalid group: '__nosuchgroup__'");
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
        for opt in &["-f", "--silent", "--quiet", "--sil", "--qui"] {
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
#[cfg(not(target_vendor = "apple"))]
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

#[test]
fn test_no_change() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file");
    ucmd.arg("").arg(at.plus("file")).succeeds();
}

#[test]
#[cfg(not(target_vendor = "apple"))]
fn test_permission_denied() {
    use std::os::unix::prelude::PermissionsExt;

    if let Some(group) = nix::unistd::getgroups().unwrap().first() {
        let (at, mut ucmd) = at_and_ucmd!();
        at.mkdir("dir");
        at.touch("dir/file");
        std::fs::set_permissions(at.plus("dir"), PermissionsExt::from_mode(0o0000)).unwrap();
        ucmd.arg("-R")
            .arg(group.as_raw().to_string())
            .arg("dir")
            .fails()
            .stderr_only("chgrp: cannot access 'dir': Permission denied");
    }
}

#[test]
#[cfg(not(target_vendor = "apple"))]
fn test_subdir_permission_denied() {
    use std::os::unix::prelude::PermissionsExt;

    if let Some(group) = nix::unistd::getgroups().unwrap().first() {
        let (at, mut ucmd) = at_and_ucmd!();
        at.mkdir("dir");
        at.mkdir("dir/subdir");
        at.touch("dir/subdir/file");
        std::fs::set_permissions(at.plus("dir/subdir"), PermissionsExt::from_mode(0o0000)).unwrap();
        ucmd.arg("-R")
            .arg(group.as_raw().to_string())
            .arg("dir")
            .fails()
            .stderr_only("chgrp: cannot access 'dir/subdir': Permission denied");
    }
}

#[test]
#[cfg(not(target_vendor = "apple"))]
fn test_traverse_symlinks() {
    use std::os::unix::prelude::MetadataExt;
    let groups = nix::unistd::getgroups().unwrap();
    if groups.len() < 2 {
        return;
    }
    let (first_group, second_group) = (groups[0], groups[1]);

    for &(args, traverse_first, traverse_second) in &[
        (&[][..] as &[&str], false, false),
        (&["-H"][..], true, false),
        (&["-P"][..], false, false),
        (&["-L"][..], true, true),
    ] {
        let scenario = TestScenario::new("chgrp");

        let (at, mut ucmd) = (scenario.fixtures.clone(), scenario.ucmd());

        at.mkdir("dir");
        at.mkdir("dir2");
        at.touch("dir2/file");
        at.mkdir("dir3");
        at.touch("dir3/file");
        at.symlink_dir("dir2", "dir/dir2_ln");
        at.symlink_dir("dir3", "dir3_ln");

        scenario
            .ccmd("chgrp")
            .arg(first_group.to_string())
            .arg("dir2/file")
            .arg("dir3/file")
            .succeeds();

        assert!(at.plus("dir2/file").metadata().unwrap().gid() == first_group.as_raw());
        assert!(at.plus("dir3/file").metadata().unwrap().gid() == first_group.as_raw());

        ucmd.arg("-R")
            .args(args)
            .arg(second_group.to_string())
            .arg("dir")
            .arg("dir3_ln")
            .succeeds()
            .no_stderr();

        assert_eq!(
            at.plus("dir2/file").metadata().unwrap().gid(),
            if traverse_second {
                second_group.as_raw()
            } else {
                first_group.as_raw()
            }
        );
        assert_eq!(
            at.plus("dir3/file").metadata().unwrap().gid(),
            if traverse_first {
                second_group.as_raw()
            } else {
                first_group.as_raw()
            }
        );
    }
}
