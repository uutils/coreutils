// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) helloworld nodir objdump n'source

#[cfg(not(target_os = "openbsd"))]
use filetime::FileTime;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
#[cfg(not(windows))]
use std::process::Command;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::thread::sleep;
use uucore::process::{getegid, geteuid};
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::{is_ci, run_ucmd_as_root, TestScenario};
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

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
    assert!(at.file_exists(format!("{dir}/{file1}")));
    assert!(at.file_exists(format!("{dir}/{file2}")));
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

    assert!(!at.file_exists(format!("{dir}/{file}")));
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
fn test_install_ancestors_mode_directories_with_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let ancestor1 = "ancestor1";
    let ancestor2 = "ancestor1/ancestor2";
    let target_file = "ancestor1/ancestor2/target_file";
    let directories_arg = "-D";
    let mode_arg = "--mode=200";
    let file = "file";
    let probe = "probe";

    at.mkdir(probe);
    let default_perms = at.metadata(probe).permissions().mode();

    at.touch(file);

    ucmd.args(&[mode_arg, directories_arg, file, target_file])
        .succeeds()
        .no_stderr();

    assert!(at.dir_exists(ancestor1));
    assert!(at.dir_exists(ancestor2));
    assert!(at.file_exists(target_file));

    assert_eq!(default_perms, at.metadata(ancestor1).permissions().mode());
    assert_eq!(default_perms, at.metadata(ancestor2).permissions().mode());

    // Expected mode only on the target_file.
    assert_eq!(0o100_200_u32, at.metadata(target_file).permissions().mode());
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

    let dest_file = &format!("{dir}/{file}");
    assert!(at.file_exists(file));
    assert!(at.file_exists(dest_file));
    let permissions = at.metadata(dest_file).permissions();
    assert_eq!(0o100_333_u32, PermissionsExt::mode(&permissions));

    let mode_arg = "-m 0333";
    at.mkdir(dir2);

    scene.ucmd().arg(mode_arg).arg(file).arg(dir2).succeeds();

    let dest_file = &format!("{dir2}/{file}");
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

    let dest_file = &format!("{dir}/{file}");
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

    let dest_file = &format!("{dir}/{file}");
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
        .arg(format!("{dir}/{file}"))
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file));
    assert!(at.file_exists(format!("{dir}/{file}")));
}

#[test]
fn test_install_target_new_file_with_group() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "file";
    let dir = "target_dir";
    let gid = getegid();

    at.touch(file);
    at.mkdir(dir);
    let result = ucmd
        .arg(file)
        .arg("--group")
        .arg(gid.to_string())
        .arg(format!("{dir}/{file}"))
        .run();

    if is_ci() && result.stderr_str().contains("no such group:") {
        // In the CI, some server are failing to return the group.
        // As seems to be a configuration issue, ignoring it
        return;
    }

    result.success();
    assert!(at.file_exists(file));
    assert!(at.file_exists(format!("{dir}/{file}")));
}

#[test]
fn test_install_target_new_file_with_owner() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "file";
    let dir = "target_dir";
    let uid = geteuid();

    at.touch(file);
    at.mkdir(dir);
    let result = ucmd
        .arg(file)
        .arg("--owner")
        .arg(uid.to_string())
        .arg(format!("{dir}/{file}"))
        .run();

    if is_ci() && result.stderr_str().contains("no such user:") {
        // In the CI, some server are failing to return the user id.
        // As seems to be a configuration issue, ignoring it
        return;
    }

    result.success();
    assert!(at.file_exists(file));
    assert!(at.file_exists(format!("{dir}/{file}")));
}

#[test]
fn test_install_target_new_file_failing_nonexistent_parent() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "source_file";
    let file2 = "target_file";
    let dir = "target_dir";

    at.touch(file1);

    ucmd.arg(file1)
        .arg(format!("{dir}/{file2}"))
        .fails()
        .stderr_contains("No such file or directory");
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
    at.touch(format!("{dir1}/{file1}"));

    ucmd.arg(format!("{dir1}/{file1}"))
        .arg(dir2)
        .succeeds()
        .no_stderr();
    assert!(at.file_exists(format!("{dir2}/{file1}")));
}

#[test]
fn test_install_failing_omitting_directory() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let file1 = "file1";
    let dir1 = "dir1";
    let no_dir2 = "no-dir2";
    let dir3 = "dir3";

    at.mkdir(dir1);
    at.mkdir(dir3);
    at.touch(file1);

    // GNU install checks for existing target dir first before checking on source params
    scene
        .ucmd()
        .arg(file1)
        .arg(dir1)
        .arg(no_dir2)
        .fails()
        .stderr_contains("is not a directory");

    // file1 will be copied before install fails on dir1
    scene
        .ucmd()
        .arg(file1)
        .arg(dir1)
        .arg(dir3)
        .fails()
        .code_is(1)
        .stderr_contains("omitting directory");
    assert!(at.file_exists(format!("{dir3}/{file1}")));

    // install also fails, when only one source param is given
    scene
        .ucmd()
        .arg(dir1)
        .arg(dir3)
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
#[cfg(not(target_os = "openbsd"))]
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
    sleep(std::time::Duration::from_millis(100));

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

    sleep(std::time::Duration::from_millis(100));

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
#[cfg(all(not(windows), not(target_os = "freebsd")))]
const SYMBOL_DUMP_PROGRAM: &str = "objdump";
#[cfg(target_os = "freebsd")]
const SYMBOL_DUMP_PROGRAM: &str = "llvm-objdump";
#[cfg(not(windows))]
const STRIP_SOURCE_FILE_SYMBOL: &str = "main";

fn strip_source_file() -> &'static str {
    if cfg!(target_os = "freebsd") {
        "helloworld_freebsd"
    } else if cfg!(target_os = "macos") {
        "helloworld_macos"
    } else if cfg!(target_arch = "arm") || cfg!(target_arch = "aarch64") {
        "helloworld_android"
    } else {
        "helloworld_linux"
    }
}

#[test]
#[cfg(not(windows))]
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
#[cfg(not(windows))]
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

#[cfg(all(unix, feature = "chmod"))]
#[test]
fn test_install_and_strip_with_program_hyphen() {
    let scene = TestScenario::new(util_name!());

    let at = &scene.fixtures;
    let content = r#"#!/bin/sh
    printf -- '%s\n' "$1" | grep '^[^-]'
    "#;
    at.write("no-hyphen", content);
    scene.ccmd("chmod").arg("+x").arg("no-hyphen").succeeds();

    at.touch("src");
    scene
        .ucmd()
        .arg("-s")
        .arg("--strip-program")
        .arg("./no-hyphen")
        .arg("--")
        .arg("src")
        .arg("-dest")
        .succeeds()
        .no_stderr()
        .stdout_is("./-dest\n");

    scene
        .ucmd()
        .arg("-s")
        .arg("--strip-program")
        .arg("./no-hyphen")
        .arg("--")
        .arg("src")
        .arg("./-dest")
        .succeeds()
        .no_stderr()
        .stdout_is("./-dest\n");
}

#[cfg(all(unix, feature = "chmod"))]
#[test]
fn test_install_on_invalid_link_at_destination() {
    let scene = TestScenario::new(util_name!());

    let at = &scene.fixtures;
    at.mkdir("src");
    at.mkdir("dest");
    let src_dir = at.plus("src");
    let dst_dir = at.plus("dest");

    at.touch("test.sh");
    at.symlink_file(
        "/opt/FakeDestination",
        &dst_dir.join("test.sh").to_string_lossy(),
    );
    scene.ccmd("chmod").arg("+x").arg("test.sh").succeeds();
    at.symlink_file("test.sh", &src_dir.join("test.sh").to_string_lossy());

    scene
        .ucmd()
        .current_dir(&src_dir)
        .arg(src_dir.join("test.sh"))
        .arg(dst_dir.join("test.sh"))
        .succeeds()
        .no_stderr()
        .no_stdout();
}

#[cfg(all(unix, feature = "chmod"))]
#[test]
fn test_install_on_invalid_link_at_destination_and_dev_null_at_source() {
    let scene = TestScenario::new(util_name!());

    let at = &scene.fixtures;
    at.mkdir("src");
    at.mkdir("dest");
    let src_dir = at.plus("src");
    let dst_dir = at.plus("dest");

    at.touch("test.sh");
    at.symlink_file(
        "/opt/FakeDestination",
        &dst_dir.join("test.sh").to_string_lossy(),
    );
    scene.ccmd("chmod").arg("+x").arg("test.sh").succeeds();
    at.symlink_file("test.sh", &src_dir.join("test.sh").to_string_lossy());

    scene
        .ucmd()
        .current_dir(&src_dir)
        .arg("/dev/null")
        .arg(dst_dir.join("test.sh"))
        .succeeds()
        .no_stderr()
        .no_stdout();
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

    assert!(at.file_exists(target));
}

#[test]
fn test_install_creating_leading_dirs_verbose() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let source = "create_leading_test_file";
    let target = "dir1/no-dir2/no-dir3/test_file";

    at.touch(source);
    at.mkdir("dir1");

    let creating_dir1 = regex::Regex::new("(?m)^install: creating directory.*dir1'$").unwrap();
    let creating_nodir23 =
        regex::Regex::new(r"(?m)^install: creating directory.*no-dir[23]'$").unwrap();

    scene
        .ucmd()
        .arg("-Dv")
        .arg(source)
        .arg(at.plus(target))
        .succeeds()
        .stdout_matches(&creating_nodir23)
        .stdout_does_not_match(&creating_dir1)
        .no_stderr();

    assert!(at.file_exists(target));
}

#[test]
fn test_install_creating_leading_dirs_with_single_source_and_target_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let source1 = "source_file_1";
    let target_dir = "missing_target_dir/";

    at.touch(source1);

    // installing a single file into a missing directory will fail, when -D is used w/o -t parameter
    scene
        .ucmd()
        .arg("-D")
        .arg(source1)
        .arg(at.plus(target_dir))
        .fails()
        .stderr_contains("missing_target_dir/' is not a directory");

    assert!(!at.dir_exists(target_dir));

    scene
        .ucmd()
        .arg("-D")
        .arg(source1)
        .arg("-t")
        .arg(at.plus(target_dir))
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(format!("{target_dir}/{source1}")));
}

#[test]
fn test_install_creating_leading_dirs_with_multiple_sources_and_target_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let source1 = "source_file_1";
    let source2 = "source_file_2";
    let target_dir = "missing_target_dir";

    at.touch(source1);
    at.touch(source2);

    // installing multiple files into a missing directory will fail, when -D is used w/o -t parameter
    scene
        .ucmd()
        .arg("-D")
        .arg(source1)
        .arg(source2)
        .arg(at.plus(target_dir))
        .fails()
        .stderr_contains("missing_target_dir' is not a directory");

    assert!(!at.dir_exists(target_dir));

    scene
        .ucmd()
        .arg("-D")
        .arg(source1)
        .arg(source2)
        .arg("-t")
        .arg(at.plus(target_dir))
        .succeeds()
        .no_stderr();

    assert!(at.dir_exists(target_dir));
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
        .arg(format!("--target-directory={dir}"))
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file1));
    assert!(at.file_exists(file2));
    assert!(at.file_exists(format!("{dir}/{file1}")));
    assert!(at.file_exists(format!("{dir}/{file2}")));
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
    assert!(at.file_exists(format!("{file_b}~")));
}

#[test]
fn test_install_backup_short_no_args_file_to_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file = "test_install_simple_backup_file_a";
    let dest_dir = "test_install_dest/";
    let expect = format!("{dest_dir}{file}");

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
    assert!(at.file_exists(format!("{expect}~")));
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
    assert!(at.file_exists(format!("{file_b}~")));
}

#[test]
fn test_install_backup_long_no_args_file_to_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file = "test_install_simple_backup_file_a";
    let dest_dir = "test_install_dest/";
    let expect = format!("{dest_dir}{file}");

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
    assert!(at.file_exists(format!("{expect}~")));
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
        .arg(format!("--suffix={suffix}"))
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(format!("{file_b}{suffix}")));
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
        .arg(format!("--suffix={suffix}"))
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(format!("{file_b}{suffix}")));
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
    assert!(at.file_exists(format!("{file_b}{suffix}")));
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
    assert!(at.file_exists(format!("{file_b}.~1~")));
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
    assert!(at.file_exists(format!("{file_b}.~1~")));
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
    assert!(at.file_exists(format!("{file_b}~")));
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
    assert!(at.file_exists(format!("{file_b}~")));
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
    assert!(at.file_exists(format!("{file_b}.~2~")));
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
    assert!(at.file_exists(format!("{file_b}.~2~")));
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
    assert!(at.file_exists(format!("{file_b}~")));
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
    assert!(at.file_exists(format!("{file_b}~")));
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
    assert!(!at.file_exists(format!("{file_b}~")));
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
    assert!(!at.file_exists(format!("{file_b}~")));
}

#[test]
fn test_install_missing_arguments() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let no_target_dir = "no-target_dir";

    scene
        .ucmd()
        .fails()
        .code_is(1)
        .usage_error("missing file operand");

    scene
        .ucmd()
        .arg("-D")
        .arg(format!("-t {no_target_dir}"))
        .fails()
        .usage_error("missing file operand");
    assert!(!at.dir_exists(no_target_dir));
}

#[test]
fn test_install_missing_destination() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_1 = "source_file1";
    let dir_1 = "source_dir1";

    at.touch(file_1);
    at.mkdir(dir_1);

    // will fail and also print some info on correct usage
    scene
        .ucmd()
        .arg(file_1)
        .fails()
        .usage_error(format!("missing destination file operand after '{file_1}'"));

    // GNU's install will check for correct num of arguments and then fail
    // and it does not recognize, that the source is not a file but a directory.
    scene
        .ucmd()
        .arg(dir_1)
        .fails()
        .usage_error(format!("missing destination file operand after '{dir_1}'"));
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
    at.touch(file_1);
    scene
        .ucmd()
        .arg("-Dv")
        .arg(file_1)
        .arg("sub3/a/b/c/file")
        .succeeds()
        .stdout_contains("install: creating directory 'sub3'\ninstall: creating directory 'sub3/a'\ninstall: creating directory 'sub3/a/b'\ninstall: creating directory 'sub3/a/b/c'\n'source_file1' -> 'sub3/a/b/c/file'");

    scene
        .ucmd()
        .arg("-t")
        .arg("sub4/a")
        .arg("-Dv")
        .arg(file_1)
        .succeeds()
        .stdout_contains("install: creating directory 'sub4'\ninstall: creating directory 'sub4/a'\n'source_file1' -> 'sub4/a/source_file1'");

    at.mkdir("sub5");
    scene
        .ucmd()
        .arg("-Dv")
        .arg(file_1)
        .arg("sub5/a/b/c/file")
        .succeeds()
        .stdout_contains("install: creating directory 'sub5/a'\ninstall: creating directory 'sub5/a/b'\ninstall: creating directory 'sub5/a/b/c'\n'source_file1' -> 'sub5/a/b/c/file'");
}

#[test]
fn test_install_chown_file_invalid() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_1 = "source_file1";
    at.touch(file_1);

    scene
        .ucmd()
        .arg("-o")
        .arg("test_invalid_user")
        .arg(file_1)
        .arg("target_file1")
        .fails()
        .stderr_contains("install: invalid user: 'test_invalid_user'");

    scene
        .ucmd()
        .arg("-g")
        .arg("test_invalid_group")
        .arg(file_1)
        .arg("target_file1")
        .fails()
        .stderr_contains("install: invalid group: 'test_invalid_group'");

    scene
        .ucmd()
        .arg("-o")
        .arg("test_invalid_user")
        .arg("-g")
        .arg("test_invalid_group")
        .arg(file_1)
        .arg("target_file1")
        .fails()
        .stderr_contains("install: invalid user: 'test_invalid_user'");

    scene
        .ucmd()
        .arg("-g")
        .arg("test_invalid_group")
        .arg("-o")
        .arg("test_invalid_user")
        .arg(file_1)
        .arg("target_file1")
        .fails()
        .stderr_contains("install: invalid user: 'test_invalid_user'");
}

#[test]
fn test_install_chown_directory_invalid() {
    let scene = TestScenario::new(util_name!());

    scene
        .ucmd()
        .arg("-o")
        .arg("test_invalid_user")
        .arg("-d")
        .arg("dir1/dir2")
        .fails()
        .stderr_contains("install: invalid user: 'test_invalid_user'");

    scene
        .ucmd()
        .arg("-g")
        .arg("test_invalid_group")
        .arg("-d")
        .arg("dir1/dir2")
        .fails()
        .stderr_contains("install: invalid group: 'test_invalid_group'");

    scene
        .ucmd()
        .arg("-o")
        .arg("test_invalid_user")
        .arg("-g")
        .arg("test_invalid_group")
        .arg("-d")
        .arg("dir1/dir2")
        .fails()
        .stderr_contains("install: invalid user: 'test_invalid_user'");

    scene
        .ucmd()
        .arg("-g")
        .arg("test_invalid_group")
        .arg("-o")
        .arg("test_invalid_user")
        .arg("-d")
        .arg("dir1/dir2")
        .fails()
        .stderr_contains("install: invalid user: 'test_invalid_user'");
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_install_compare_option() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let first = "a";
    let second = "b";
    at.touch(first);
    scene
        .ucmd()
        .args(&["-Cv", first, second])
        .succeeds()
        .stdout_contains(format!("'{first}' -> '{second}'"));
    scene
        .ucmd()
        .args(&["-Cv", first, second])
        .succeeds()
        .no_stdout();
    scene
        .ucmd()
        .args(&["-Cv", "-m0644", first, second])
        .succeeds()
        .stdout_contains(format!("removed '{second}'\n'{first}' -> '{second}'"));
    scene
        .ucmd()
        .args(&["-Cv", first, second])
        .succeeds()
        .stdout_contains(format!("removed '{second}'\n'{first}' -> '{second}'"));
    scene
        .ucmd()
        .args(&["-C", "--preserve-timestamps", first, second])
        .fails()
        .code_is(1)
        .stderr_contains("Options --compare and --preserve-timestamps are mutually exclusive");
    scene
        .ucmd()
        .args(&["-C", "--strip", "--strip-program=echo", first, second])
        .fails()
        .code_is(1)
        .stderr_contains("Options --compare and --strip are mutually exclusive");
}

#[test]
// Matches part of tests/install/basic-1
fn test_t_exist_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let source1 = "file";
    let target_dir = "sub4/";
    let target_file = "sub4/file_exists";

    at.touch(source1);
    at.mkdir(target_dir);
    at.touch(target_file);

    scene
        .ucmd()
        .arg("-t")
        .arg(target_file)
        .arg("-Dv")
        .arg(source1)
        .fails()
        .stderr_contains("failed to access 'sub4/file_exists': Not a directory");
}

#[test]
fn test_target_file_ends_with_slash() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let source = "source_file";
    let target_dir = "dir";
    let target_file = "dir/target_file";
    let target_file_slash = format!("{}/", target_file);

    at.touch(source);
    at.mkdir(target_dir);
    at.touch(target_file);

    scene
        .ucmd()
        .arg("-t")
        .arg(target_file_slash)
        .arg("-D")
        .arg(source)
        .fails()
        .stderr_contains("failed to access 'dir/target_file/': Not a directory");
}

#[test]
fn test_install_root_combined() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");
    at.touch("c");

    let run_and_check = |args: &[&str], target: &str, expected_uid: u32, expected_gid: u32| {
        if let Ok(result) = run_ucmd_as_root(&ts, args) {
            result.success();
            assert!(at.file_exists(target));

            let metadata = fs::metadata(at.plus(target)).unwrap();
            assert_eq!(metadata.uid(), expected_uid);
            assert_eq!(metadata.gid(), expected_gid);
        } else {
            print!("Test skipped; requires root user");
        }
    };

    run_and_check(&["-Cv", "-o1", "-g1", "a", "b"], "b", 1, 1);
    run_and_check(&["-Cv", "-o2", "-g1", "a", "b"], "b", 2, 1);
    run_and_check(&["-Cv", "-o2", "-g2", "a", "b"], "b", 2, 2);

    run_and_check(&["-Cv", "-o2", "c", "d"], "d", 2, 0);
    run_and_check(&["-Cv", "c", "d"], "d", 0, 0);
    run_and_check(&["-Cv", "c", "d"], "d", 0, 0);
}

#[test]
#[cfg(unix)]
fn test_install_from_fifo() {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::thread;

    let pipe_name = "pipe";
    let target_name = "target";
    let test_string = "Hello, world!\n";

    let s = TestScenario::new(util_name!());
    s.fixtures.mkfifo(pipe_name);
    assert!(s.fixtures.is_fifo(pipe_name));

    let proc = s.ucmd().arg(pipe_name).arg(target_name).run_no_wait();

    let pipe_path = s.fixtures.plus(pipe_name);
    let thread = thread::spawn(move || {
        let mut pipe = OpenOptions::new()
            .write(true)
            .create(false)
            .open(pipe_path)
            .unwrap();
        pipe.write_all(test_string.as_bytes()).unwrap();
    });

    proc.wait().unwrap();
    thread.join().unwrap();

    assert!(s.fixtures.file_exists(target_name));
    assert_eq!(s.fixtures.read(target_name), test_string);
}

#[test]
#[cfg(unix)]
fn test_install_from_stdin() {
    let (at, mut ucmd) = at_and_ucmd!();
    let target = "target";
    let test_string = "Hello, World!\n";

    ucmd.arg("/dev/fd/0")
        .arg(target)
        .pipe_in(test_string)
        .succeeds();

    assert!(at.file_exists(target));
    assert_eq!(at.read(target), test_string);
}
