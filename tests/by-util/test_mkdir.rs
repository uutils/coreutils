// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore bindgen testtest casetest CASETEST

#![allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]

#[cfg(not(windows))]
use libc::mode_t;
#[cfg(not(windows))]
use std::os::unix::fs::PermissionsExt;
#[cfg(feature = "feat_selinux")]
use uucore::selinux::get_getfattr_output;
#[cfg(not(windows))]
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_version_no_path() {
    use std::process::Command;
    use uutests::get_tests_binary;

    // This test verifies that when an individual utility binary is invoked with its full path,
    // the version output shows just "mkdir", not the full path like "/path/to/mkdir".
    //
    // Note: The multicall binary (coreutils) doesn't have this issue because it reads
    // the utility name from ARGV[1], not ARGV[0]. This bug only affects individual binaries.

    let tests_binary = get_tests_binary!();
    let mkdir_binary_path = std::path::Path::new(tests_binary)
        .parent()
        .unwrap()
        .join("mkdir");

    // If the individual mkdir binary exists, test it
    let output = if mkdir_binary_path.exists() {
        // Invoke the individual mkdir binary with its full path
        Command::new(&mkdir_binary_path)
            .arg("--version")
            .output()
            .expect("Failed to execute mkdir binary")
    } else {
        // If only multicall binary exists, test that (it should already pass)
        Command::new(tests_binary)
            .args(["mkdir", "--version"])
            .output()
            .expect("Failed to execute mkdir via multicall binary")
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("mkdir (uutils coreutils)"));
}

#[test]
fn test_no_arg() {
    new_ucmd!()
        .fails_with_code(1)
        .stderr_contains("error: the following required arguments were not provided:");
}

#[test]
fn test_mkdir_mkdir() {
    new_ucmd!().arg("test_dir").succeeds();
}

#[cfg(feature = "test_risky_names")]
#[test]
fn test_mkdir_non_unicode() {
    let (at, mut ucmd) = at_and_ucmd!();

    let target = uucore::os_str_from_bytes(b"some-\xc0-dir-\xf3")
        .expect("Only unix platforms can test non-unicode names");
    ucmd.arg(&target).succeeds();

    assert!(at.dir_exists(target));
}

#[test]
fn test_mkdir_verbose() {
    let expected = "mkdir: created directory 'test_dir'\n";
    new_ucmd!()
        .arg("test_dir")
        .arg("-v")
        .succeeds()
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
fn test_mkdir_trailing_dot_and_slash() {
    new_ucmd!().arg("-p").arg("-v").arg("test_dir").succeeds();

    new_ucmd!()
        .arg("-p")
        .arg("-v")
        .arg("test_dir_a/./")
        .succeeds()
        .stdout_contains("created directory 'test_dir_a'");

    let scene = TestScenario::new("ls");
    let result = scene.ucmd().arg("-al").run();
    println!("ls dest {}", result.stdout_str());
}

#[test]
fn test_mkdir_trailing_spaces_and_dots() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Test trailing space
    scene.ucmd().arg("-p").arg("test ").succeeds();
    assert!(at.dir_exists("test "));

    // Test multiple trailing spaces
    scene.ucmd().arg("-p").arg("test   ").succeeds();
    assert!(at.dir_exists("test   "));

    // Test leading dot (hidden on Unix)
    scene.ucmd().arg("-p").arg(".hidden").succeeds();
    assert!(at.dir_exists(".hidden"));

    // Test trailing dot (should work)
    scene.ucmd().arg("-p").arg("test.").succeeds();
    assert!(at.dir_exists("test."));

    // Test multiple leading dots
    scene.ucmd().arg("-p").arg("...test").succeeds();
    assert!(at.dir_exists("...test"));
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

#[test]
#[cfg(feature = "feat_selinux")]
fn test_selinux() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let dest = "test_dir_a";
    let args = ["-Z", "--context=unconfined_u:object_r:user_tmp_t:s0"];
    for arg in args {
        new_ucmd!()
            .arg(arg)
            .arg("-v")
            .arg(at.plus_as_string(dest))
            .succeeds()
            .stdout_contains("created directory");

        let context_value = get_getfattr_output(&at.plus_as_string(dest));
        assert!(
            context_value.contains("unconfined_u"),
            "Expected '{}' not found in getfattr output:\n{}",
            "unconfined_u",
            context_value
        );
        at.rmdir(dest);
    }
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_selinux_invalid() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let dest = "test_dir_a";
    new_ucmd!()
        .arg("--context=testtest")
        .arg(at.plus_as_string(dest))
        .fails()
        .no_stdout()
        .stderr_contains("failed to set default file creation context to 'testtest':");
    // invalid context, so, no directory
    assert!(!at.dir_exists(dest));
}

#[test]
fn test_mkdir_deep_nesting() {
    // Regression test for stack overflow with deeply nested directories.
    // The iterative implementation should handle arbitrary depth without stack overflow.
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Create a path with 350 levels of nesting
    let depth = 350;
    let dir_name = "d";
    let mut path = std::path::PathBuf::new();
    for _ in 0..depth {
        path.push(dir_name);
    }

    scene.ucmd().arg("-p").arg(&path).succeeds();

    assert!(at.dir_exists(&path));
}

#[test]
fn test_mkdir_dot_components() {
    // Test handling of "." (current directory) components
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Test case from review comment: test/././test2/././test3/././test4
    // GNU mkdir normalizes this path and creates test/test2/test3/test4
    let path = "test/././test2/././test3/././test4";
    scene.ucmd().arg("-p").arg(path).succeeds();

    // Verify expected structure exists (GNU compatibility)
    assert!(at.dir_exists("test/test2/test3/test4"));

    // Test leading "." - should create test_dot/test_dot2
    scene
        .ucmd()
        .arg("-p")
        .arg("./test_dot/test_dot2")
        .succeeds();
    assert!(at.dir_exists("test_dot/test_dot2"));

    // Test mixed "." and normal components
    scene
        .ucmd()
        .arg("-p")
        .arg("mixed/./normal/./path")
        .succeeds();
    assert!(at.dir_exists("mixed/normal/path"));

    // Test that the command works without creating redundant directories
    // The key test is that it doesn't fail or create incorrect structure
    // The actual filesystem behavior (whether literal "." dirs exist)
    // may vary, but the logical result should be correct
}

#[test]
fn test_mkdir_parent_dir_components() {
    // Test handling of ".." (parent directory) components
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("base");
    at.mkdir("base/child");

    scene
        .ucmd()
        .arg("-p")
        .arg("base/child/../sibling")
        .succeeds();
    assert!(at.dir_exists("base/sibling"));

    scene
        .ucmd()
        .arg("-p")
        .arg("base/child/../../other")
        .succeeds();
    assert!(at.dir_exists("other"));

    scene
        .ucmd()
        .arg("-p")
        .arg("base/child/../sibling")
        .succeeds();
    assert!(at.dir_exists("base/sibling"));
}

#[test]
fn test_mkdir_mixed_special_components() {
    // Test complex paths with both "." and ".." components
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    scene
        .ucmd()
        .arg("-p")
        .arg("./start/./middle/../end/./final")
        .succeeds();
    assert!(at.dir_exists("start/end/final"));
}

#[test]
fn test_mkdir_control_characters() {
    // Test handling of control characters in filenames
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Test NUL byte (should fail on most systems)
    // Note: This test is skipped because NUL bytes cause panics in the test framework
    // let result = scene.ucmd().arg("test\x00name").run();
    // assert!(!result.succeeded());

    // Test newline character (may fail on Windows)
    let result = scene.ucmd().arg("-p").arg("test\nname").run();
    if result.succeeded() {
        assert!(at.dir_exists("test\nname"));
    }

    // Test tab character (may fail on Windows)
    let result = scene.ucmd().arg("-p").arg("test\tname").run();
    if result.succeeded() {
        assert!(at.dir_exists("test\tname"));
    }

    // Test space character in directory name (should work on all systems)
    scene.ucmd().arg("-p").arg("test name").succeeds();
    assert!(at.dir_exists("test name"));

    // Test double quotes in path - platform-specific behavior expected
    // On Linux/Unix: Should succeed and create directory with quotes
    // On Windows: Should fail with "Invalid argument" error
    #[cfg(unix)]
    {
        scene.ucmd().arg("-pv").arg("a/\"\"/b/c").succeeds();
        assert!(at.dir_exists("a/\"\"/b/c"));
    }
    #[cfg(windows)]
    {
        let result = scene.ucmd().arg("-pv").arg("a/\"\"/b/c").run();
        // Should fail after creating 'a' directory
        assert!(at.dir_exists("a"));
        assert!(!at.dir_exists("a/\"\""));
        assert!(!result.succeeded());
        // Windows should reject directory names with quotes
        // We don't assert on specific error message as it varies by Windows version
    }

    // Test single quotes in path - should work on both Unix and Windows
    scene.ucmd().arg("-p").arg("a/''/b/c").succeeds();
    assert!(at.dir_exists("a/''/b/c"));
}

#[test]
fn test_mkdir_maximum_path_length() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Test moderate length path (should work on all systems)
    let long_path = "a".repeat(50) + "/" + &"b".repeat(50) + "/" + &"c".repeat(50);
    scene.ucmd().arg("-p").arg(&long_path).succeeds();
    assert!(at.dir_exists(&long_path));

    // Test longer but reasonable path
    let longer_path = "x".repeat(100) + "/" + &"y".repeat(50) + "/" + &"z".repeat(30);
    scene.ucmd().arg("-p").arg(&longer_path).succeeds();
    assert!(at.dir_exists(&longer_path));

    // Test extremely long path (may fail on some systems)
    let very_long_path = "very_long_directory_name_".repeat(20) + "/final";
    let result = scene.ucmd().arg("-p").arg(&very_long_path).run();
    if result.succeeded() {
        assert!(at.dir_exists(&very_long_path));
    }
    // If it fails, that's acceptable on some systems with path limits
}

#[test]
fn test_mkdir_reserved_device_names() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // These should work on Unix but may fail on Windows
    let result = scene.ucmd().arg("-p").arg("CON").run();
    if result.succeeded() {
        assert!(at.dir_exists("CON"));
    }

    let result = scene.ucmd().arg("-p").arg("PRN").run();
    if result.succeeded() {
        assert!(at.dir_exists("PRN"));
    }

    let result = scene.ucmd().arg("-p").arg("AUX").run();
    if result.succeeded() {
        assert!(at.dir_exists("AUX"));
    }

    // Test device names with extensions
    let result = scene.ucmd().arg("-p").arg("COM1").run();
    if result.succeeded() {
        assert!(at.dir_exists("COM1"));
    }

    let result = scene.ucmd().arg("-p").arg("LPT1").run();
    if result.succeeded() {
        assert!(at.dir_exists("LPT1"));
    }
}

#[test]
fn test_mkdir_case_sensitivity() {
    // Test case sensitivity behavior (varies by filesystem)
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Create directory with lowercase name
    scene.ucmd().arg("-p").arg("CaseTest").succeeds();
    assert!(at.dir_exists("CaseTest"));

    // Try to create directory with different case
    // On case-sensitive filesystems: creates separate directory
    // On case-insensitive filesystems: fails (directory already exists)
    let result = scene.ucmd().arg("-p").arg("casetest").run();

    // The test passes regardless of filesystem behavior
    // We just verify the command doesn't crash
    if result.succeeded() {
        // Case-sensitive filesystem - both exist
        assert!(at.dir_exists("CaseTest"));
        assert!(at.dir_exists("casetest"));
    } else {
        // Case-insensitive filesystem - only one exists
        assert!(at.dir_exists("CaseTest"));
    }

    // Test mixed case variations
    scene.ucmd().arg("-p").arg("CASETEST").succeeds();
    scene.ucmd().arg("-p").arg("caseTEST").succeeds();
}

#[test]
fn test_mkdir_network_paths() {
    // Test network path formats (UNC paths on Windows)
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Test UNC-style path prefix (may fail on some systems)
    let result = scene.ucmd().arg("-p").arg("//server/share/test").run();
    if result.succeeded() {
        assert!(at.dir_exists("//server/share/test"));
    }
    // If it fails, that's acceptable on some systems with read-only restrictions

    // Test path that looks like network but is actually local
    scene.ucmd().arg("-p").arg("server_share_test").succeeds();
    assert!(at.dir_exists("server_share_test"));

    // Test path with double slash in middle (should work)
    scene.ucmd().arg("-p").arg("test//double//slash").succeeds();
    assert!(at.dir_exists("test//double//slash"));
}

#[test]
fn test_mkdir_environment_expansion() {
    // Test that mkdir doesn't expand environment variables (unlike some shells)
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    unsafe {
        std::env::set_var("TEST_VAR", "expanded_value");
    }

    // Create directory with literal $VAR (should not expand)
    scene.ucmd().arg("-p").arg("$TEST_VAR/dir").succeeds();
    assert!(at.dir_exists("$TEST_VAR/dir"));

    // Verify the literal name exists, not the expanded value
    assert!(!at.dir_exists("expanded_value/dir"));

    // Test with braces
    scene
        .ucmd()
        .arg("-p")
        .arg("${TEST_VAR}_braced/dir")
        .succeeds();
    assert!(at.dir_exists("${TEST_VAR}_braced/dir"));

    // Test with tilde (should not expand to home directory)
    scene.ucmd().arg("-p").arg("~/test_dir").succeeds();
    assert!(at.dir_exists("~/test_dir"));

    unsafe {
        std::env::remove_var("TEST_VAR");
    }
}

#[test]
fn test_mkdir_concurrent_creation() {
    // Test concurrent mkdir -p operations: 10 iterations, 8 threads, 40 levels nesting
    use std::thread;

    for _ in 0..10 {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        let mut dir = at.plus("concurrent_test");
        dir.push("a");

        for _ in 0..40 {
            dir.push("a");
        }

        let path_str = dir.to_string_lossy().to_string();
        let bin_path = scene.bin_path.clone();

        let mut handles = vec![];

        for _ in 0..8 {
            let path_clone = path_str.clone();
            let bin_path_clone = bin_path.clone();

            let handle = thread::spawn(move || {
                // Use the actual uutils mkdir binary to test the real implementation
                let result = std::process::Command::new(&bin_path_clone)
                    .arg("mkdir")
                    .arg("-p")
                    .arg(&path_clone)
                    .current_dir(std::env::current_dir().unwrap())
                    .output();

                match result {
                    Ok(output) => {
                        assert!(
                            output.status.success(),
                            "mkdir failed: {}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                    }
                    Err(e) => panic!("Failed to execute mkdir: {e}"),
                }
            });
            handles.push(handle);
        }

        handles
            .drain(..)
            .map(|handle| handle.join().unwrap())
            .count();

        assert!(at.dir_exists(&path_str));
    }
}
