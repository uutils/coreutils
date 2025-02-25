// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) nosuchgroup groupname

use uucore::process::getegid;
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_option() {
    new_ucmd!().arg("-w").arg("/").fails();
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

static DIR: &str = "/dev";

// we should always get both arguments, regardless of whether --reference was used
#[test]
fn test_help() {
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .stdout_contains("Arguments:");
}

#[test]
fn test_help_ref() {
    new_ucmd!()
        .arg("--help")
        .arg("--reference=ref_file")
        .succeeds()
        .stdout_contains("Arguments:");
}

#[test]
fn test_ref_help() {
    new_ucmd!()
        .arg("--reference=ref_file")
        .arg("--help")
        .succeeds()
        .stdout_contains("Arguments:");
}

#[test]
fn test_invalid_group() {
    new_ucmd!()
        .arg("__nosuchgroup__")
        .arg("/")
        .fails()
        .stderr_is("chgrp: invalid group: '__nosuchgroup__'\n");
}

#[test]
fn test_error_1() {
    if getegid() != 0 {
        new_ucmd!().arg("bin").arg(DIR).fails().stderr_contains(
            // linux fails with "Operation not permitted (os error 1)"
            // because of insufficient permissions,
            // android fails with "Permission denied (os error 13)"
            // because it can't resolve /proc (even though it can resolve /proc/self/)
            "chgrp: changing group of '/dev': ",
        );
    }
}

#[test]
fn test_fail_silently() {
    if getegid() != 0 {
        for opt in ["-f", "--silent", "--quiet", "--sil", "--qui"] {
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
    new_ucmd!()
        .arg("--preserve-root")
        .arg("-R")
        .arg("bin")
        .arg("/")
        .fails()
        .stderr_is("chgrp: it is dangerous to operate recursively on '/'\nchgrp: use --no-preserve-root to override this failsafe\n");
    for d in [
        "/////dev///../../../../",
        "../../../../../../../../../../../../../../",
        "./../../../../../../../../../../../../../../",
    ] {
        let expected_error = format!(
            "chgrp: it is dangerous to operate recursively on '{}' (same as '/')\nchgrp: use --no-preserve-root to override this failsafe\n",
            d,
        );
        new_ucmd!()
            .arg("--preserve-root")
            .arg("-R")
            .arg("bin")
            .arg(d)
            .fails()
            .stderr_is(expected_error);
    }
}

#[test]
fn test_preserve_root_symlink() {
    let file = "test_chgrp_symlink2root";
    for d in [
        "/",
        "//",
        "///",
        "////dev//../../../../",
        "..//../../..//../..//../../../../../../../../",
        ".//../../../../../../..//../../../../../../../",
    ] {
        let (at, mut ucmd) = at_and_ucmd!();
        at.symlink_file(d, file);
        let expected_error =
            "chgrp: it is dangerous to operate recursively on 'test_chgrp_symlink2root' (same as '/')\nchgrp: use --no-preserve-root to override this failsafe\n";
        ucmd.arg("--preserve-root")
            .arg("-HR")
            .arg("bin")
            .arg(file)
            .fails()
            .stderr_is(expected_error);
    }

    let (at, mut ucmd) = at_and_ucmd!();
    at.symlink_file("///dev", file);
    ucmd.arg("--preserve-root")
        .arg("-HR")
        .arg("bin").arg(format!(".//{file}/..//..//../../"))
        .fails()
        .stderr_is("chgrp: it is dangerous to operate recursively on './/test_chgrp_symlink2root/..//..//../../' (same as '/')\nchgrp: use --no-preserve-root to override this failsafe\n");

    let (at, mut ucmd) = at_and_ucmd!();
    at.symlink_file("/", "__root__");
    ucmd.arg("--preserve-root")
        .arg("-R")
        .arg("bin").arg("__root__/.")
        .fails()
        .stderr_is("chgrp: it is dangerous to operate recursively on '__root__/.' (same as '/')\nchgrp: use --no-preserve-root to override this failsafe\n");
}

#[test]
fn test_preserve_root_symlink_cwd_root() {
    new_ucmd!()
        .current_dir("/")
        .arg("--preserve-root")
        .arg("-R")
        .arg("bin").arg(".")
        .fails()
        .stderr_is("chgrp: it is dangerous to operate recursively on '.' (same as '/')\nchgrp: use --no-preserve-root to override this failsafe\n");
    new_ucmd!()
        .current_dir("/")
        .arg("--preserve-root")
        .arg("-R")
        .arg("bin").arg("/.")
        .fails()
        .stderr_is("chgrp: it is dangerous to operate recursively on '/.' (same as '/')\nchgrp: use --no-preserve-root to override this failsafe\n");
    new_ucmd!()
        .current_dir("/")
        .arg("--preserve-root")
        .arg("-R")
        .arg("bin").arg("..")
        .fails()
        .stderr_is("chgrp: it is dangerous to operate recursively on '..' (same as '/')\nchgrp: use --no-preserve-root to override this failsafe\n");
    new_ucmd!()
        .current_dir("/")
        .arg("--preserve-root")
        .arg("-R")
        .arg("bin").arg("/..")
        .fails()
        .stderr_is("chgrp: it is dangerous to operate recursively on '/..' (same as '/')\nchgrp: use --no-preserve-root to override this failsafe\n");
    new_ucmd!()
        .current_dir("/")
        .arg("--preserve-root")
        .arg("-R")
        .arg("bin")
        .arg("...")
        .fails()
        .stderr_is("chgrp: cannot access '...': No such file or directory\n");
}

#[test]
#[cfg(target_os = "linux")]
fn test_reference() {
    // skip for root or MS-WSL
    // * MS-WSL is bugged (as of 2019-12-25), allowing non-root accounts su-level privileges for `chgrp`
    // * for MS-WSL, succeeds and stdout == 'group of /etc retained as root'
    if !(getegid() == 0 || uucore::os::is_wsl_1()) {
        new_ucmd!()
            .arg("-v")
            .arg("--reference=/etc/passwd")
            .arg("/etc")
            .fails()
            .stderr_is("chgrp: changing group of '/etc': Operation not permitted (os error 1)\nfailed to change group of '/etc' from root to root\n");
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
#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
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
#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
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
            "error: the following required arguments were not provided:\n  <FILE>...\n",
        );
}

#[test]
#[cfg(target_os = "linux")]
fn test_big_p() {
    if getegid() != 0 {
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
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_big_h() {
    if getegid() != 0 {
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
            .stderr_only("chgrp: cannot access 'dir': Permission denied\n");
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
            .stderr_only("chgrp: cannot access 'dir/subdir': Permission denied\n");
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

    for (args, traverse_first, traverse_second) in [
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

#[test]
#[cfg(not(target_vendor = "apple"))]
fn test_from_option() {
    use std::os::unix::fs::MetadataExt;
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let groups = nix::unistd::getgroups().unwrap();
    // Skip test if we don't have at least two different groups to work with
    if groups.len() < 2 {
        return;
    }
    let (first_group, second_group) = (groups[0], groups[1]);

    at.touch("test_file");
    scene
        .ucmd()
        .arg(first_group.to_string())
        .arg("test_file")
        .succeeds();

    // Test successful group change with --from
    scene
        .ucmd()
        .arg("--from")
        .arg(first_group.to_string())
        .arg(second_group.to_string())
        .arg("test_file")
        .succeeds()
        .no_stderr();

    // Verify the group was changed
    let new_gid = at.plus("test_file").metadata().unwrap().gid();
    assert_eq!(new_gid, second_group.as_raw());

    scene
        .ucmd()
        .arg("--from")
        .arg(first_group.to_string())
        .arg(first_group.to_string())
        .arg("test_file")
        .succeeds()
        .no_stderr();

    let unchanged_gid = at.plus("test_file").metadata().unwrap().gid();
    assert_eq!(unchanged_gid, second_group.as_raw());
}

#[test]
#[cfg(not(any(target_os = "android", target_os = "macos")))]
fn test_from_with_invalid_group() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("test_file");
    #[cfg(not(target_os = "android"))]
    let err_msg = "chgrp: invalid user: 'nonexistent_group'\n";
    #[cfg(target_os = "android")]
    let err_msg = "chgrp: invalid user: 'staff'\n";

    ucmd.arg("--from")
        .arg("nonexistent_group")
        .arg("staff")
        .arg("test_file")
        .fails()
        .stderr_is(err_msg);
}

#[test]
#[cfg(not(target_vendor = "apple"))]
fn test_verbosity_messages() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let groups = nix::unistd::getgroups().unwrap();
    // Skip test if we don't have at least one group to work with
    if groups.is_empty() {
        return;
    }

    at.touch("ref_file");
    at.touch("target_file");

    scene
        .ucmd()
        .arg("-v")
        .arg("--reference=ref_file")
        .arg("target_file")
        .succeeds()
        .stderr_contains("group of 'target_file' retained as ");
}

#[test]
#[cfg(not(target_vendor = "apple"))]
fn test_from_with_reference() {
    use std::os::unix::fs::MetadataExt;
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let groups = nix::unistd::getgroups().unwrap();
    if groups.len() < 2 {
        return;
    }
    let (first_group, second_group) = (groups[0], groups[1]);

    at.touch("ref_file");
    at.touch("test_file");

    scene
        .ucmd()
        .arg(first_group.to_string())
        .arg("test_file")
        .succeeds();

    scene
        .ucmd()
        .arg(second_group.to_string())
        .arg("ref_file")
        .succeeds();

    // Test --from with --reference
    scene
        .ucmd()
        .arg("--from")
        .arg(first_group.to_string())
        .arg("--reference=ref_file")
        .arg("test_file")
        .succeeds()
        .no_stderr();

    let new_gid = at.plus("test_file").metadata().unwrap().gid();
    let ref_gid = at.plus("ref_file").metadata().unwrap().gid();
    assert_eq!(new_gid, ref_gid);
}

#[test]
#[cfg(not(target_vendor = "apple"))]
fn test_numeric_group_formats() {
    use std::os::unix::fs::MetadataExt;
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let groups = nix::unistd::getgroups().unwrap();
    if groups.len() < 2 {
        return;
    }
    let (first_group, second_group) = (groups[0], groups[1]);

    at.touch("test_file");

    scene
        .ucmd()
        .arg(first_group.to_string())
        .arg("test_file")
        .succeeds();

    // Test :gid format in --from
    scene
        .ucmd()
        .arg(format!("--from=:{}", first_group.as_raw()))
        .arg(second_group.to_string())
        .arg("test_file")
        .succeeds()
        .no_stderr();

    let new_gid = at.plus("test_file").metadata().unwrap().gid();
    assert_eq!(new_gid, second_group.as_raw());

    // Test :gid format in target group
    scene
        .ucmd()
        .arg(format!("--from={}", second_group.as_raw()))
        .arg(format!(":{}", first_group.as_raw()))
        .arg("test_file")
        .succeeds()
        .no_stderr();

    let final_gid = at.plus("test_file").metadata().unwrap().gid();
    assert_eq!(final_gid, first_group.as_raw());
}
