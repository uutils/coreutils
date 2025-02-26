// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore bindgen

#![allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]

#[cfg(not(windows))]
use libc::mode_t;
#[cfg(not(windows))]
use std::os::unix::fs::PermissionsExt;
#[cfg(not(windows))]
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_no_arg() {
    new_ucmd!()
        .fails()
        .code_is(1)
        .stderr_contains("error: the following required arguments were not provided:");
}

#[test]
fn test_mkdir_mkdir() {
    new_ucmd!().arg("test_dir").succeeds();
}

#[test]
fn test_mkdir_verbose() {
    let expected = "mkdir: created directory 'test_dir'\n";
    new_ucmd!()
        .arg("test_dir")
        .arg("-v")
        .run()
        .stdout_is(expected);
}

#[test]
fn test_mkdir_dup_dir() {
    let scene = TestScenario::new(util_name!());
    let test_dir = "test_dir";

    scene.ucmd().arg(test_dir).succeeds();
    scene.ucmd().arg(test_dir).fails();
}

#[test]
fn test_mkdir_mode() {
    new_ucmd!().arg("-m").arg("755").arg("test_dir").succeeds();
}

#[test]
fn test_mkdir_parent() {
    let scene = TestScenario::new(util_name!());
    let test_dir = "parent_dir/child_dir";

    scene.ucmd().arg("-p").arg(test_dir).succeeds();
    scene.ucmd().arg("-p").arg("-p").arg(test_dir).succeeds();
    scene.ucmd().arg("--parent").arg(test_dir).succeeds();
    scene
        .ucmd()
        .arg("--parent")
        .arg("--parent")
        .arg(test_dir)
        .succeeds();
    scene.ucmd().arg("--parents").arg(test_dir).succeeds();
    scene
        .ucmd()
        .arg("--parents")
        .arg("--parents")
        .arg(test_dir)
        .succeeds();
}

#[test]
fn test_mkdir_no_parent() {
    new_ucmd!().arg("parent_dir/child_dir").fails();
}

#[test]
fn test_mkdir_dup_dir_parent() {
    let scene = TestScenario::new(util_name!());
    let test_dir = "test_dir";

    scene.ucmd().arg(test_dir).succeeds();
    scene.ucmd().arg("-p").arg(test_dir).succeeds();
}

#[cfg(not(windows))]
#[test]
fn test_mkdir_parent_mode() {
    let (at, mut ucmd) = at_and_ucmd!();

    let default_umask: mode_t = 0o160;

    ucmd.arg("-p")
        .arg("a/b")
        .umask(default_umask)
        .succeeds()
        .no_stderr()
        .no_stdout();

    assert!(at.dir_exists("a"));
    // parents created by -p have permissions set to "=rwx,u+wx"
    assert_eq!(
        at.metadata("a").permissions().mode() as mode_t,
        ((!default_umask & 0o777) | 0o300) + 0o40000
    );
    assert!(at.dir_exists("a/b"));
    // sub directory's permission is determined only by the umask
    assert_eq!(
        at.metadata("a/b").permissions().mode() as mode_t,
        (!default_umask & 0o777) + 0o40000
    );
}

#[cfg(not(windows))]
#[test]
fn test_mkdir_parent_mode_check_existing_parent() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("a");
    let parent_a_perms = at.metadata("a").permissions().mode();

    let default_umask: mode_t = 0o160;

    ucmd.arg("-p")
        .arg("a/b/c")
        .umask(default_umask)
        .succeeds()
        .no_stderr()
        .no_stdout();

    assert!(at.dir_exists("a"));
    // parent dirs that already exist do not get their permissions modified
    assert_eq!(at.metadata("a").permissions().mode(), parent_a_perms);
    assert!(at.dir_exists("a/b"));
    assert_eq!(
        at.metadata("a/b").permissions().mode() as mode_t,
        ((!default_umask & 0o777) | 0o300) + 0o40000
    );
    assert!(at.dir_exists("a/b/c"));
    assert_eq!(
        at.metadata("a/b/c").permissions().mode() as mode_t,
        (!default_umask & 0o777) + 0o40000
    );
}

#[cfg(not(windows))]
#[test]
fn test_mkdir_parent_mode_skip_existing_last_component_chmod() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("a");
    at.mkdir("a/b");
    at.set_mode("a/b", 0);

    let default_umask: mode_t = 0o160;

    ucmd.arg("-p")
        .arg("a/b")
        .umask(default_umask)
        .succeeds()
        .no_stderr()
        .no_stdout();

    assert_eq!(at.metadata("a/b").permissions().mode() as mode_t, 0o40000);
}

#[test]
fn test_mkdir_dup_file() {
    let scene = TestScenario::new(util_name!());
    let test_file = "test_file.txt";

    scene.fixtures.touch(test_file);

    scene.ucmd().arg(test_file).fails();

    // mkdir should fail for a file even if -p is specified.
    scene.ucmd().arg("-p").arg(test_file).fails();
}

#[test]
#[cfg(not(windows))]
fn test_symbolic_mode() {
    let (at, mut ucmd) = at_and_ucmd!();
    let test_dir = "test_dir";

    ucmd.arg("-m").arg("a=rwx").arg(test_dir).succeeds();
    let perms = at.metadata(test_dir).permissions().mode();
    assert_eq!(perms, 0o40777);
}

#[test]
#[cfg(not(windows))]
fn test_symbolic_alteration() {
    let (at, mut ucmd) = at_and_ucmd!();
    let test_dir = "test_dir";

    let default_umask = 0o022;

    ucmd.arg("-m")
        .arg("-w")
        .arg(test_dir)
        .umask(default_umask)
        .succeeds();
    let perms = at.metadata(test_dir).permissions().mode();
    assert_eq!(perms, 0o40577);
}

#[test]
#[cfg(not(windows))]
fn test_multi_symbolic() {
    let (at, mut ucmd) = at_and_ucmd!();
    let test_dir = "test_dir";

    ucmd.arg("-m").arg("u=rwx,g=rx,o=").arg(test_dir).succeeds();
    let perms = at.metadata(test_dir).permissions().mode();
    assert_eq!(perms, 0o40750);
}

#[test]
fn test_recursive_reporting() {
    let test_dir = "test_dir/test_dir_a/test_dir_b";

    new_ucmd!()
        .arg("-p")
        .arg("-v")
        .arg(test_dir)
        .succeeds()
        .stdout_contains("created directory 'test_dir'")
        .stdout_contains("created directory 'test_dir/test_dir_a'")
        .stdout_contains("created directory 'test_dir/test_dir_a/test_dir_b'");
    new_ucmd!().arg("-v").arg(test_dir).fails().no_stdout();

    let test_dir = "test_dir/../test_dir_a/../test_dir_b";

    new_ucmd!()
        .arg("-p")
        .arg("-v")
        .arg(test_dir)
        .succeeds()
        .stdout_contains("created directory 'test_dir'")
        .stdout_contains("created directory 'test_dir/../test_dir_a'")
        .stdout_contains("created directory 'test_dir/../test_dir_a/../test_dir_b'");
}

#[test]
// Windows don't have acl entries
// TODO Enable and modify this for macos when xattr processing for macos is added.
// TODO Enable and modify this for freebsd when xattr processing for freebsd is enabled.
#[cfg(target_os = "linux")]
fn test_mkdir_acl() {
    use std::{collections::HashMap, ffi::OsString};

    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("a");

    let mut map: HashMap<OsString, Vec<u8>> = HashMap::new();
    // posix_acl entries are in the form of
    // struct posix_acl_entry{
    //  tag: u16,
    //  perm: u16,
    //  id: u32,
    // }
    // the fields are serialized in little endian.
    // The entries are preceded by a header of value of 0x0002
    // Reference: `<https://github.com/torvalds/linux/blob/master/include/uapi/linux/posix_acl_xattr.h>`
    // The id is undefined i.e. -1 which in u32 is 0xFFFFFFFF and tag and perm bits as given in the
    // header file.
    // Reference: `<https://github.com/torvalds/linux/blob/master/include/uapi/linux/posix_acl.h>`
    //
    //
    // There is a bindgen bug which generates the ACL_OTHER constant whose value is 0x20 into 32.
    // which when the bug is fixed will need to be changed back to 20 from 32 in the vec 'xattr_val'.
    //
    // Reference `<https://github.com/rust-lang/rust-bindgen/issues/2926>`
    //
    // The xattr_val vector is the header 0x0002 followed by tag and permissions for user_obj , tag
    // and permissions and for group_obj and finally the tag and permissions for ACL_OTHER. Each
    // entry has undefined id as mentioned above.
    //
    //

    let xattr_val: Vec<u8> = vec![
        2, 0, 0, 0, 1, 0, 7, 0, 255, 255, 255, 255, 4, 0, 7, 0, 255, 255, 255, 255, 32, 0, 5, 0,
        255, 255, 255, 255,
    ];

    map.insert(OsString::from("system.posix_acl_default"), xattr_val);

    uucore::fsxattr::apply_xattrs(at.plus("a"), map).unwrap();

    ucmd.arg("-p").arg("a/b").umask(0x077).succeeds();

    let perms = at.metadata("a/b").permissions().mode();

    // 0x770 would be user:rwx,group:rwx permissions
    assert_eq!(perms, 16893);
}

#[test]
fn test_mkdir_trailing_dot() {
    new_ucmd!().arg("-p").arg("-v").arg("test_dir").succeeds();

    new_ucmd!()
        .arg("-p")
        .arg("-v")
        .arg("test_dir_a/.")
        .succeeds()
        .stdout_contains("created directory 'test_dir_a'");

    new_ucmd!()
        .arg("-p")
        .arg("-v")
        .arg("test_dir_b/..")
        .succeeds()
        .stdout_contains("created directory 'test_dir_b'");

    let scene = TestScenario::new("ls");
    let result = scene.ucmd().arg("-al").run();
    println!("ls dest {}", result.stdout_str());
}

#[test]
#[cfg(not(windows))]
fn test_umask_compliance() {
    fn test_single_case(umask_set: mode_t) {
        let test_dir = "test_dir";
        let (at, mut ucmd) = at_and_ucmd!();

        ucmd.arg(test_dir).umask(umask_set).succeeds();
        let perms = at.metadata(test_dir).permissions().mode() as mode_t;

        assert_eq!(perms, (!umask_set & 0o0777) + 0o40000); // before compare, add the set GUID, UID bits
    }

    for i in 0o0..0o027 {
        // tests all permission combinations
        test_single_case(i as mode_t);
    }
}

#[test]
fn test_empty_argument() {
    new_ucmd!()
        .arg("")
        .fails()
        .stderr_only("mkdir: cannot create directory '': No such file or directory\n");
}
