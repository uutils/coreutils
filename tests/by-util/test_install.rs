// spell-checker:ignore (words) helloworld objdump n'source

use crate::common::util::*;
use filetime::FileTime;
use rust_users::*;
use std::os::unix::fs::PermissionsExt;
#[cfg(not(any(windows, target_os = "freebsd")))]
use std::process::Command;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::thread::sleep;

#[test]
fn test_install_basic() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "target_dir";
    let file1 = "source_file1";
    let file2 = "source_file2";

    at.touch(file1);
    at.touch(file2);
    at.mkdir(dir);
    ucmd.arg(file1).arg(file2).arg(dir).succeeds().no_stderr();

    assert!(at.file_exists(file1));
    assert!(at.file_exists(file2));
    assert!(at.file_exists(&format!("{}/{}", dir, file1)));
    assert!(at.file_exists(&format!("{}/{}", dir, file2)));
}

#[test]
fn test_install_twice_dir() {
    let dir = "dir";
    let scene = TestScenario::new(util_name!());

    scene.ucmd().arg("-d").arg(dir).succeeds();
    scene.ucmd().arg("-d").arg(dir).succeeds();
    let at = &scene.fixtures;

    assert!(at.dir_exists(dir));
}

#[test]
fn test_install_failing_not_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "file1";
    let file2 = "file2";
    let file3 = "file3";

    at.touch(file1);
    at.touch(file2);
    at.touch(file3);
    ucmd.arg(file1)
        .arg(file2)
        .arg(file3)
        .fails()
        .stderr_contains("not a directory");
}

#[test]
fn test_install_unimplemented_arg() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "target_dir";
    let file = "source_file";
    let context_arg = "--context";

    at.touch(file);
    at.mkdir(dir);
    ucmd.arg(context_arg)
        .arg(file)
        .arg(dir)
        .fails()
        .stderr_contains("Unimplemented");

    assert!(!at.file_exists(&format!("{}/{}", dir, file)));
}

#[test]
fn test_install_ancestors_directories() {
    let (at, mut ucmd) = at_and_ucmd!();
    let ancestor1 = "ancestor1";
    let ancestor2 = "ancestor1/ancestor2";
    let target_dir = "ancestor1/ancestor2/target_dir";
    let directories_arg = "-d";

    ucmd.args(&[directories_arg, target_dir])
        .succeeds()
        .no_stderr();

    assert!(at.dir_exists(ancestor1));
    assert!(at.dir_exists(ancestor2));
    assert!(at.dir_exists(target_dir));
}

#[test]
fn test_install_ancestors_mode_directories() {
    let (at, mut ucmd) = at_and_ucmd!();
    let ancestor1 = "ancestor1";
    let ancestor2 = "ancestor1/ancestor2";
    let target_dir = "ancestor1/ancestor2/target_dir";
    let directories_arg = "-d";
    let mode_arg = "--mode=200";
    let probe = "probe";

    at.mkdir(probe);
    let default_perms = at.metadata(probe).permissions().mode();

    ucmd.args(&[mode_arg, directories_arg, target_dir])
        .succeeds()
        .no_stderr();

    assert!(at.dir_exists(ancestor1));
    assert!(at.dir_exists(ancestor2));
    assert!(at.dir_exists(target_dir));

    assert_eq!(default_perms, at.metadata(ancestor1).permissions().mode());
    assert_eq!(default_perms, at.metadata(ancestor2).permissions().mode());

    // Expected mode only on the target_dir.
    assert_eq!(0o40_200_u32, at.metadata(target_dir).permissions().mode());
}

#[test]
fn test_install_parent_directories() {
    let (at, mut ucmd) = at_and_ucmd!();
    let ancestor1 = "ancestor1";
    let ancestor2 = "ancestor1/ancestor2";
    let target_dir = "ancestor1/ancestor2/target_dir";
    let directories_arg = "-d";

    // Here one of the ancestors already exist and only the target_dir and
    // its parent must be created.
    at.mkdir(ancestor1);

    ucmd.args(&[directories_arg, target_dir])
        .succeeds()
        .no_stderr();

    assert!(at.dir_exists(ancestor2));
    assert!(at.dir_exists(target_dir));
}

#[test]
fn test_install_several_directories() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir1 = "dir1";
    let dir2 = "dir2";
    let dir3 = "dir3";
    let directories_arg = "-d";

    ucmd.args(&[directories_arg, dir1, dir2, dir3])
        .succeeds()
        .no_stderr();

    assert!(at.dir_exists(dir1));
    assert!(at.dir_exists(dir2));
    assert!(at.dir_exists(dir3));
}

#[test]
fn test_install_mode_numeric() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let dir = "dir1";
    let dir2 = "dir2";

    let file = "file";
    let mode_arg = "--mode=333";

    at.touch(file);
    at.mkdir(dir);
    scene
        .ucmd()
        .arg(file)
        .arg(dir)
        .arg(mode_arg)
        .succeeds()
        .no_stderr();

    let dest_file = &format!("{}/{}", dir, file);
    assert!(at.file_exists(file));
    assert!(at.file_exists(dest_file));
    let permissions = at.metadata(dest_file).permissions();
    assert_eq!(0o100_333_u32, PermissionsExt::mode(&permissions));

    let mode_arg = "-m 0333";
    at.mkdir(dir2);

    scene.ucmd().arg(mode_arg).arg(file).arg(dir2).succeeds();

    let dest_file = &format!("{}/{}", dir2, file);
    assert!(at.file_exists(file));
    assert!(at.file_exists(dest_file));
    let permissions = at.metadata(dest_file).permissions();
    assert_eq!(0o100_333_u32, PermissionsExt::mode(&permissions));
}

#[test]
fn test_install_mode_symbolic() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "target_dir";
    let file = "source_file";
    let mode_arg = "--mode=o+wx";

    at.touch(file);
    at.mkdir(dir);
    ucmd.arg(file).arg(dir).arg(mode_arg).succeeds().no_stderr();

    let dest_file = &format!("{}/{}", dir, file);
    assert!(at.file_exists(file));
    assert!(at.file_exists(dest_file));
    let permissions = at.metadata(dest_file).permissions();
    assert_eq!(0o100_003_u32, PermissionsExt::mode(&permissions));
}

#[test]
fn test_install_mode_failing() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "target_dir";
    let file = "source_file";
    let mode_arg = "--mode=999";

    at.touch(file);
    at.mkdir(dir);
    ucmd.arg(file)
        .arg(dir)
        .arg(mode_arg)
        .fails()
        .stderr_contains("Invalid mode string: invalid digit found in string");

    let dest_file = &format!("{}/{}", dir, file);
    assert!(at.file_exists(file));
    assert!(!at.file_exists(dest_file));
}

#[test]
fn test_install_mode_directories() {
    let (at, mut ucmd) = at_and_ucmd!();
    let component = "component";
    let directories_arg = "-d";
    let mode_arg = "--mode=333";

    ucmd.arg(directories_arg)
        .arg(component)
        .arg(mode_arg)
        .succeeds()
        .no_stderr();

    assert!(at.dir_exists(component));
    let permissions = at.metadata(component).permissions();
    assert_eq!(0o040_333_u32, PermissionsExt::mode(&permissions));
}

#[test]
fn test_install_target_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "source_file";
    let file2 = "target_file";

    at.touch(file1);
    at.touch(file2);
    ucmd.arg(file1).arg(file2).succeeds().no_stderr();

    assert!(at.file_exists(file1));
    assert!(at.file_exists(file2));
}

#[test]
fn test_install_target_new_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "file";
    let dir = "target_dir";

    at.touch(file);
    at.mkdir(dir);
    ucmd.arg(file)
        .arg(format!("{}/{}", dir, file))
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));
    assert!(at.file_exists(&format!("{}/{}", dir, file)));
}

#[test]
fn test_install_target_new_file_with_group() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "file";
    let dir = "target_dir";
    let gid = get_effective_gid();

    at.touch(file);
    at.mkdir(dir);
    let result = ucmd
        .arg(file)
        .arg("--group")
        .arg(gid.to_string())
        .arg(format!("{}/{}", dir, file))
        .run();

    if is_ci() && result.stderr_str().contains("no such group:") {
        // In the CI, some server are failing to return the group.
        // As seems to be a configuration issue, ignoring it
        return;
    }

    result.success();
    assert!(at.file_exists(file));
    assert!(at.file_exists(&format!("{}/{}", dir, file)));
}

#[test]
fn test_install_target_new_file_with_owner() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "file";
    let dir = "target_dir";
    let uid = get_effective_uid();

    at.touch(file);
    at.mkdir(dir);
    let result = ucmd
        .arg(file)
        .arg("--owner")
        .arg(uid.to_string())
        .arg(format!("{}/{}", dir, file))
        .run();

    if is_ci() && result.stderr_str().contains("no such user:") {
        // In the CI, some server are failing to return the user id.
        // As seems to be a configuration issue, ignoring it
        return;
    }

    result.success();
    assert!(at.file_exists(file));
    assert!(at.file_exists(&format!("{}/{}", dir, file)));
}

#[test]
fn test_install_target_new_file_failing_nonexistent_parent() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "source_file";
    let file2 = "target_file";
    let dir = "target_dir";

    at.touch(file1);

    ucmd.arg(file1)
        .arg(format!("{}/{}", dir, file2))
        .fails()
        .stderr_contains(&"No such file or directory");
}

#[test]
fn test_install_preserve_timestamps() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "source_file";
    let file2 = "target_file";
    at.touch(file1);

    ucmd.arg(file1).arg(file2).arg("-p").succeeds().no_stderr();

    assert!(at.file_exists(file1));
    assert!(at.file_exists(file2));

    let file1_metadata = at.metadata(file1);
    let file2_metadata = at.metadata(file2);

    assert_eq!(
        file1_metadata.accessed().ok(),
        file2_metadata.accessed().ok()
    );
    assert_eq!(
        file1_metadata.modified().ok(),
        file2_metadata.modified().ok()
    );
}

// These two tests are failing but should work
#[test]
fn test_install_copy_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "source_file";
    let file2 = "target_file";

    at.touch(file1);
    ucmd.arg(file1).arg(file2).succeeds().no_stderr();

    assert!(at.file_exists(file1));
    assert!(at.file_exists(file2));
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_install_target_file_dev_null() {
    let (at, mut ucmd) = at_and_ucmd!();

    let file1 = "/dev/null";
    let file2 = "target_file";

    ucmd.arg(file1).arg(file2).succeeds();

    assert!(at.file_exists(file2));
}

#[test]
fn test_install_nested_paths_copy_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "source_file";
    let dir1 = "source_dir";
    let dir2 = "target_dir";

    at.mkdir(dir1);
    at.mkdir(dir2);
    at.touch(&format!("{}/{}", dir1, file1));

    ucmd.arg(format!("{}/{}", dir1, file1))
        .arg(dir2)
        .succeeds()
        .no_stderr();
    assert!(at.file_exists(&format!("{}/{}", dir2, file1)));
}

#[test]
fn test_install_failing_omitting_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "source_file";
    let dir1 = "source_dir";
    let dir2 = "target_dir";

    at.mkdir(dir1);
    at.mkdir(dir2);
    at.touch(file1);

    ucmd.arg(dir1)
        .arg(file1)
        .arg(dir2)
        .fails()
        .code_is(1)
        .stderr_contains("omitting directory");
}

#[test]
fn test_install_failing_no_such_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "source_file";
    let file2 = "inexistent_file";
    let dir1 = "target_dir";

    at.mkdir(dir1);
    at.touch(file1);

    ucmd.arg(file1)
        .arg(file2)
        .arg(dir1)
        .fails()
        .code_is(1)
        .stderr_contains("No such file or directory");
}

#[test]
fn test_install_copy_then_compare_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let file1 = "test_install_copy_then_compare_file_a1";
    let file2 = "test_install_copy_then_compare_file_a2";

    at.touch(file1);
    scene
        .ucmd()
        .arg("-C")
        .arg(file1)
        .arg(file2)
        .succeeds()
        .no_stderr();

    let mut file2_meta = at.metadata(file2);
    let before = FileTime::from_last_modification_time(&file2_meta);

    scene
        .ucmd()
        .arg("-C")
        .arg(file1)
        .arg(file2)
        .succeeds()
        .no_stderr();

    file2_meta = at.metadata(file2);
    let after = FileTime::from_last_modification_time(&file2_meta);

    assert_eq!(before, after);
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_install_copy_then_compare_file_with_extra_mode() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    // XXX: can't tests introspect on their own names?
    let file1 = "test_install_copy_then_compare_file_with_extra_mode_a1";
    let file2 = "test_install_copy_then_compare_file_with_extra_mode_a2";

    at.touch(file1);
    scene
        .ucmd()
        .arg("-C")
        .arg(file1)
        .arg(file2)
        .succeeds()
        .no_stderr();

    let mut file2_meta = at.metadata(file2);
    let before = FileTime::from_last_modification_time(&file2_meta);
    sleep(std::time::Duration::from_millis(1000));

    scene
        .ucmd()
        .arg("-C")
        .arg(file1)
        .arg(file2)
        .arg("-m")
        .arg("1644")
        .succeeds()
        .no_stderr();

    file2_meta = at.metadata(file2);
    let after_install_sticky = FileTime::from_last_modification_time(&file2_meta);

    assert!(before != after_install_sticky);

    sleep(std::time::Duration::from_millis(1000));

    // dest file still 1644, so need_copy ought to return `true`
    scene
        .ucmd()
        .arg("-C")
        .arg(file1)
        .arg(file2)
        .succeeds()
        .no_stderr();

    file2_meta = at.metadata(file2);
    let after_install_sticky_again = FileTime::from_last_modification_time(&file2_meta);

    assert!(after_install_sticky != after_install_sticky_again);
}

const STRIP_TARGET_FILE: &str = "helloworld_installed";
#[cfg(not(any(windows, target_os = "freebsd")))]
const SYMBOL_DUMP_PROGRAM: &str = "objdump";
#[cfg(not(any(windows, target_os = "freebsd")))]
const STRIP_SOURCE_FILE_SYMBOL: &str = "main";

fn strip_source_file() -> &'static str {
    if cfg!(target_os = "macos") {
        "helloworld_macos"
    } else if cfg!(target_arch = "arm") || cfg!(target_arch = "aarch64") {
        "helloworld_android"
    } else {
        "helloworld_linux"
    }
}

#[test]
// FixME: Freebsd fails on 'No such file or directory'
#[cfg(not(any(windows, target_os = "freebsd")))]
fn test_install_and_strip() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    scene
        .ucmd()
        .arg("-s")
        .arg(strip_source_file())
        .arg(STRIP_TARGET_FILE)
        .succeeds()
        .no_stderr();

    let output = Command::new(SYMBOL_DUMP_PROGRAM)
        .arg("-t")
        .arg(at.plus(STRIP_TARGET_FILE))
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains(STRIP_SOURCE_FILE_SYMBOL));
}

#[test]
// FixME: Freebsd fails on 'No such file or directory'
#[cfg(not(any(windows, target_os = "freebsd")))]
fn test_install_and_strip_with_program() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    scene
        .ucmd()
        .arg("-s")
        .arg("--strip-program")
        .arg("/usr/bin/strip")
        .arg(strip_source_file())
        .arg(STRIP_TARGET_FILE)
        .succeeds()
        .no_stderr();

    let output = Command::new(SYMBOL_DUMP_PROGRAM)
        .arg("-t")
        .arg(at.plus(STRIP_TARGET_FILE))
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains(STRIP_SOURCE_FILE_SYMBOL));
}

#[test]
#[cfg(not(windows))]
fn test_install_and_strip_with_invalid_program() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    scene
        .ucmd()
        .arg("-s")
        .arg("--strip-program")
        .arg("/bin/date")
        .arg(strip_source_file())
        .arg(STRIP_TARGET_FILE)
        .fails()
        .stderr_contains("strip program failed");
    assert!(!at.file_exists(STRIP_TARGET_FILE));
}

#[test]
#[cfg(not(windows))]
fn test_install_and_strip_with_non_existent_program() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    scene
        .ucmd()
        .arg("-s")
        .arg("--strip-program")
        .arg("/usr/bin/non_existent_program")
        .arg(strip_source_file())
        .arg(STRIP_TARGET_FILE)
        .fails()
        .stderr_contains("No such file or directory");
    assert!(!at.file_exists(STRIP_TARGET_FILE));
}

#[test]
fn test_install_creating_leading_dirs() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let source = "create_leading_test_file";
    let target = "dir1/dir2/dir3/test_file";

    at.touch(source);

    scene
        .ucmd()
        .arg("-D")
        .arg(source)
        .arg(at.plus(target))
        .succeeds()
        .no_stderr();
}

#[test]
#[cfg(not(windows))]
fn test_install_creating_leading_dir_fails_on_long_name() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let source = "create_leading_test_file";
    let target = format!("{}/test_file", "d".repeat(libc::PATH_MAX as usize + 1));

    at.touch(source);

    scene
        .ucmd()
        .arg("-D")
        .arg(source)
        .arg(at.plus(target.as_str()))
        .fails()
        .stderr_contains("failed to create");
}

#[test]
fn test_install_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "target_dir";
    let file1 = "source_file1";
    let file2 = "source_file2";

    at.touch(file1);
    at.touch(file2);
    at.mkdir(dir);
    ucmd.arg(file1)
        .arg(file2)
        .arg(&format!("--target-directory={}", dir))
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file1));
    assert!(at.file_exists(file2));
    assert!(at.file_exists(&format!("{}/{}", dir, file1)));
    assert!(at.file_exists(&format!("{}/{}", dir, file2)));
}
//
// test backup functionality
#[test]
fn test_install_backup_short_no_args_files() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_install_simple_backup_file_a";
    let file_b = "test_install_simple_backup_file_b";

    at.touch(file_a);
    at.touch(file_b);
    scene
        .ucmd()
        .arg("-b")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(&format!("{}~", file_b)));
}

#[test]
fn test_install_backup_short_no_args_file_to_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file = "test_install_simple_backup_file_a";
    let dest_dir = "test_install_dest/";
    let expect = format!("{}{}", dest_dir, file);

    at.touch(file);
    at.mkdir(dest_dir);
    at.touch(&expect);
    scene
        .ucmd()
        .arg("-b")
        .arg(file)
        .arg(dest_dir)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));
    assert!(at.file_exists(&expect));
    assert!(at.file_exists(&format!("{}~", expect)));
}

// Long --backup option is tested separately as it requires a slightly different
// handling than '-b' does.
#[test]
fn test_install_backup_long_no_args_files() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_install_simple_backup_file_a";
    let file_b = "test_install_simple_backup_file_b";

    at.touch(file_a);
    at.touch(file_b);
    scene
        .ucmd()
        .arg("--backup")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(&format!("{}~", file_b)));
}

#[test]
fn test_install_backup_long_no_args_file_to_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file = "test_install_simple_backup_file_a";
    let dest_dir = "test_install_dest/";
    let expect = format!("{}{}", dest_dir, file);

    at.touch(file);
    at.mkdir(dest_dir);
    at.touch(&expect);
    scene
        .ucmd()
        .arg("--backup")
        .arg(file)
        .arg(dest_dir)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));
    assert!(at.file_exists(&expect));
    assert!(at.file_exists(&format!("{}~", expect)));
}

#[test]
fn test_install_backup_short_custom_suffix() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_install_backup_custom_suffix_file_a";
    let file_b = "test_install_backup_custom_suffix_file_b";
    let suffix = "super-suffix-of-the-century";

    at.touch(file_a);
    at.touch(file_b);
    scene
        .ucmd()
        .arg("-b")
        .arg(format!("--suffix={}", suffix))
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(&format!("{}{}", file_b, suffix)));
}

#[test]
fn test_install_backup_short_custom_suffix_hyphen_value() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_install_backup_custom_suffix_file_a";
    let file_b = "test_install_backup_custom_suffix_file_b";
    let suffix = "-v";

    at.touch(file_a);
    at.touch(file_b);
    scene
        .ucmd()
        .arg("-b")
        .arg(format!("--suffix={}", suffix))
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(&format!("{}{}", file_b, suffix)));
}

#[test]
fn test_install_backup_custom_suffix_via_env() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_install_backup_custom_suffix_file_a";
    let file_b = "test_install_backup_custom_suffix_file_b";
    let suffix = "super-suffix-of-the-century";

    at.touch(file_a);
    at.touch(file_b);
    scene
        .ucmd()
        .arg("-b")
        .env("SIMPLE_BACKUP_SUFFIX", suffix)
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(&format!("{}{}", file_b, suffix)));
}

#[test]
fn test_install_backup_numbered_with_t() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_install_backup_numbering_file_a";
    let file_b = "test_install_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    scene
        .ucmd()
        .arg("--backup=t")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(&format!("{}.~1~", file_b)));
}

#[test]
fn test_install_backup_numbered_with_numbered() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_install_backup_numbering_file_a";
    let file_b = "test_install_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    scene
        .ucmd()
        .arg("--backup=numbered")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(&format!("{}.~1~", file_b)));
}

#[test]
fn test_install_backup_existing() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_install_backup_numbering_file_a";
    let file_b = "test_install_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    scene
        .ucmd()
        .arg("--backup=existing")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(&format!("{}~", file_b)));
}

#[test]
fn test_install_backup_nil() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_install_backup_numbering_file_a";
    let file_b = "test_install_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    scene
        .ucmd()
        .arg("--backup=nil")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(&format!("{}~", file_b)));
}

#[test]
fn test_install_backup_numbered_if_existing_backup_existing() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_install_backup_numbering_file_a";
    let file_b = "test_install_backup_numbering_file_b";
    let file_b_backup = "test_install_backup_numbering_file_b.~1~";

    at.touch(file_a);
    at.touch(file_b);
    at.touch(file_b_backup);
    scene
        .ucmd()
        .arg("--backup=existing")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(file_b_backup));
    assert!(at.file_exists(&*format!("{}.~2~", file_b)));
}

#[test]
fn test_install_backup_numbered_if_existing_backup_nil() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_install_backup_numbering_file_a";
    let file_b = "test_install_backup_numbering_file_b";
    let file_b_backup = "test_install_backup_numbering_file_b.~1~";

    at.touch(file_a);
    at.touch(file_b);
    at.touch(file_b_backup);
    scene
        .ucmd()
        .arg("--backup=nil")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(file_b_backup));
    assert!(at.file_exists(&*format!("{}.~2~", file_b)));
}

#[test]
fn test_install_backup_simple() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_install_backup_numbering_file_a";
    let file_b = "test_install_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    scene
        .ucmd()
        .arg("--backup=simple")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(&format!("{}~", file_b)));
}

#[test]
fn test_install_backup_never() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_install_backup_numbering_file_a";
    let file_b = "test_install_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    scene
        .ucmd()
        .arg("--backup=never")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(&format!("{}~", file_b)));
}

#[test]
fn test_install_backup_none() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_install_backup_numbering_file_a";
    let file_b = "test_install_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    scene
        .ucmd()
        .arg("--backup=none")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(!at.file_exists(&format!("{}~", file_b)));
}

#[test]
fn test_install_backup_off() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_install_backup_numbering_file_a";
    let file_b = "test_install_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    scene
        .ucmd()
        .arg("--backup=off")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(!at.file_exists(&format!("{}~", file_b)));
}

#[test]
fn test_install_missing_arguments() {
    let scene = TestScenario::new(util_name!());

    scene
        .ucmd()
        .fails()
        .stderr_contains("install: missing file operand");
}

#[test]
fn test_install_missing_destination() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_1 = "source_file1";

    at.touch(file_1);
    scene.ucmd().arg(file_1).fails().stderr_contains(format!(
        "install: missing destination file operand after '{}'",
        file_1
    ));
}

#[test]
fn test_install_dir_dot() {
    // To match tests/install/d-slashdot.sh
    let scene = TestScenario::new(util_name!());

    scene.ucmd().arg("-d").arg("dir1/.").succeeds();
    scene.ucmd().arg("-d").arg("dir2/..").succeeds();
    // Tests that we don't have dir3/. in the output
    // but only 'dir3'
    scene
        .ucmd()
        .arg("-d")
        .arg("dir3/.")
        .arg("-v")
        .succeeds()
        .stdout_contains("creating directory 'dir3'");
    scene
        .ucmd()
        .arg("-d")
        .arg("dir4/./cal")
        .arg("-v")
        .succeeds()
        .stdout_contains("creating directory 'dir4/./cal'");
    scene
        .ucmd()
        .arg("-d")
        .arg("dir5/./cali/.")
        .arg("-v")
        .succeeds()
        .stdout_contains("creating directory 'dir5/cali'");

    let at = &scene.fixtures;

    assert!(at.dir_exists("dir1"));
    assert!(at.dir_exists("dir2"));
    assert!(at.dir_exists("dir3"));
    assert!(at.dir_exists("dir4/cal"));
    assert!(at.dir_exists("dir5/cali"));
}

#[test]
fn test_install_dir_req_verbose() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_1 = "source_file1";
    let dest_dir = "sub4";
    at.touch(file_1);
    scene
        .ucmd()
        .arg("-Dv")
        .arg(file_1)
        .arg("sub3/a/b/c/file")
        .succeeds()
        .stdout_contains("install: creating directory 'sub3'\ninstall: creating directory 'sub3/a'\ninstall: creating directory 'sub3/a/b'\ninstall: creating directory 'sub3/a/b/c'\n'source_file1' -> 'sub3/a/b/c/file'");

    at.mkdir(dest_dir);
    scene
        .ucmd()
        .arg("-Dv")
        .arg(file_1)
        .arg("sub4/a/b/c/file")
        .succeeds()
        .stdout_contains("install: creating directory 'sub4/a'\ninstall: creating directory 'sub4/a/b'\ninstall: creating directory 'sub4/a/b/c'\n'source_file1' -> 'sub4/a/b/c/file'");
}
