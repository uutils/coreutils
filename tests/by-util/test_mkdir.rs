// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::common::util::TestScenario;
#[cfg(not(windows))]
use libc::{mode_t, umask};
use once_cell::sync::Lazy;
#[cfg(not(windows))]
use std::os::unix::fs::PermissionsExt;
use std::sync::Mutex;

// tests in `test_mkdir.rs` cannot run in parallel since some tests alter the umask. This may cause
// other tests to run under a wrong set of permissions
//
// when writing a test case, acquire this mutex before proceeding with the main logic of the test
static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[test]
fn test_invalid_arg() {
    let _guard = TEST_MUTEX.lock();
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_mkdir_mkdir() {
    let _guard = TEST_MUTEX.lock();
    new_ucmd!().arg("test_dir").succeeds();
}

#[test]
fn test_mkdir_verbose() {
    let _guard = TEST_MUTEX.lock();
    let expected = "mkdir: created directory 'test_dir'\n";
    new_ucmd!()
        .arg("test_dir")
        .arg("-v")
        .run()
        .stdout_is(expected);
}

#[test]
fn test_mkdir_dup_dir() {
    let _guard = TEST_MUTEX.lock();

    let scene = TestScenario::new(util_name!());
    let test_dir = "test_dir";

    scene.ucmd().arg(test_dir).succeeds();
    scene.ucmd().arg(test_dir).fails();
}

#[test]
fn test_mkdir_mode() {
    let _guard = TEST_MUTEX.lock();
    new_ucmd!().arg("-m").arg("755").arg("test_dir").succeeds();
}

#[test]
fn test_mkdir_parent() {
    let _guard = TEST_MUTEX.lock();
    let scene = TestScenario::new(util_name!());
    let test_dir = "parent_dir/child_dir";

    scene.ucmd().arg("-p").arg(test_dir).succeeds();
    scene.ucmd().arg("-p").arg(test_dir).succeeds();
    scene.ucmd().arg("--parent").arg(test_dir).succeeds();
    scene.ucmd().arg("--parents").arg(test_dir).succeeds();
}

#[test]
fn test_mkdir_no_parent() {
    let _guard = TEST_MUTEX.lock();
    new_ucmd!().arg("parent_dir/child_dir").fails();
}

#[test]
fn test_mkdir_dup_dir_parent() {
    let _guard = TEST_MUTEX.lock();

    let scene = TestScenario::new(util_name!());
    let test_dir = "test_dir";

    scene.ucmd().arg(test_dir).succeeds();
    scene.ucmd().arg("-p").arg(test_dir).succeeds();
}

#[cfg(not(windows))]
#[test]
fn test_mkdir_parent_mode() {
    let _guard = TEST_MUTEX.lock();
    let (at, mut ucmd) = at_and_ucmd!();

    let default_umask: mode_t = 0o160;
    let original_umask = unsafe { umask(default_umask) };

    ucmd.arg("-p").arg("a/b").succeeds().no_stderr().no_stdout();

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

    unsafe {
        umask(original_umask);
    }
}

#[cfg(not(windows))]
#[test]
fn test_mkdir_parent_mode_check_existing_parent() {
    let _guard = TEST_MUTEX.lock();
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("a");

    let default_umask: mode_t = 0o160;
    let original_umask = unsafe { umask(default_umask) };

    ucmd.arg("-p")
        .arg("a/b/c")
        .succeeds()
        .no_stderr()
        .no_stdout();

    assert!(at.dir_exists("a"));
    // parent dirs that already exist do not get their permissions modified
    assert_eq!(
        at.metadata("a").permissions().mode() as mode_t,
        (!original_umask & 0o777) + 0o40000
    );
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

    unsafe {
        umask(original_umask);
    }
}

#[test]
fn test_mkdir_dup_file() {
    let _guard = TEST_MUTEX.lock();

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
    let _guard = TEST_MUTEX.lock();
    let (at, mut ucmd) = at_and_ucmd!();
    let test_dir = "test_dir";

    ucmd.arg("-m").arg("a=rwx").arg(test_dir).succeeds();
    let perms = at.metadata(test_dir).permissions().mode();
    assert_eq!(perms, 0o40777);
}

#[test]
#[cfg(not(windows))]
fn test_symbolic_alteration() {
    let _guard = TEST_MUTEX.lock();
    let (at, mut ucmd) = at_and_ucmd!();
    let test_dir = "test_dir";

    let default_umask = 0o022;
    let original_umask = unsafe { umask(default_umask) };

    ucmd.arg("-m").arg("-w").arg(test_dir).succeeds();
    let perms = at.metadata(test_dir).permissions().mode();
    assert_eq!(perms, 0o40577);

    unsafe { umask(original_umask) };
}

#[test]
#[cfg(not(windows))]
fn test_multi_symbolic() {
    let _guard = TEST_MUTEX.lock();
    let (at, mut ucmd) = at_and_ucmd!();
    let test_dir = "test_dir";

    ucmd.arg("-m").arg("u=rwx,g=rx,o=").arg(test_dir).succeeds();
    let perms = at.metadata(test_dir).permissions().mode();
    assert_eq!(perms, 0o40750);
}

#[test]
fn test_recursive_reporting() {
    let _guard = TEST_MUTEX.lock();
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
fn test_mkdir_trailing_dot() {
    let _guard = TEST_MUTEX.lock();

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
        let _guard = TEST_MUTEX.lock();

        let test_dir = "test_dir";
        let (at, mut ucmd) = at_and_ucmd!();

        let original_umask = unsafe { umask(umask_set) };

        ucmd.arg(test_dir).succeeds();
        let perms = at.metadata(test_dir).permissions().mode() as mode_t;

        assert_eq!(perms, (!umask_set & 0o0777) + 0o40000); // before compare, add the set GUID, UID bits
        unsafe {
            umask(original_umask);
        } // set umask back to original
    }

    for i in 0o0..0o027 {
        // tests all permission combinations
        test_single_case(i as mode_t);
    }
}
