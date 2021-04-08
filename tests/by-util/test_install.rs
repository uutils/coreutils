use crate::common::util::*;
use filetime::FileTime;
use rust_users::*;
use std::os::unix::fs::PermissionsExt;
#[cfg(target_os = "linux")]
use std::process::Command;
#[cfg(target_os = "linux")]
use std::thread::sleep;

#[test]
fn test_install_help() {
    let (_, mut ucmd) = at_and_ucmd!();

    assert!(ucmd
        .arg("--help")
        .succeeds()
        .no_stderr()
        .stdout
        .contains("FLAGS:"));
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
    assert!(ucmd
        .arg(file1)
        .arg(file2)
        .arg(file3)
        .fails()
        .stderr
        .contains("not a directory"));
}

#[test]
fn test_install_unimplemented_arg() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "target_dir";
    let file = "source_file";
    let context_arg = "--context";

    at.touch(file);
    at.mkdir(dir);
    assert!(ucmd
        .arg(context_arg)
        .arg(file)
        .arg(dir)
        .fails()
        .stderr
        .contains("Unimplemented"));

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
    let mode_arg = "--mode=700";

    ucmd.args(&[mode_arg, directories_arg, target_dir])
        .succeeds()
        .no_stderr();

    assert!(at.dir_exists(ancestor1));
    assert!(at.dir_exists(ancestor2));
    assert!(at.dir_exists(target_dir));

    assert_ne!(0o40700 as u32, at.metadata(ancestor1).permissions().mode());
    assert_ne!(0o40700 as u32, at.metadata(ancestor2).permissions().mode());

    // Expected mode only on the target_dir.
    assert_eq!(0o40700 as u32, at.metadata(target_dir).permissions().mode());
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
    assert_eq!(0o100333 as u32, PermissionsExt::mode(&permissions));

    let mode_arg = "-m 0333";
    at.mkdir(dir2);

    let result = scene.ucmd().arg(mode_arg).arg(file).arg(dir2).run();

    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);

    assert!(result.success);
    let dest_file = &format!("{}/{}", dir2, file);
    assert!(at.file_exists(file));
    assert!(at.file_exists(dest_file));
    let permissions = at.metadata(dest_file).permissions();
    assert_eq!(0o100333 as u32, PermissionsExt::mode(&permissions));
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
    assert_eq!(0o100003 as u32, PermissionsExt::mode(&permissions));
}

#[test]
fn test_install_mode_failing() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "target_dir";
    let file = "source_file";
    let mode_arg = "--mode=999";

    at.touch(file);
    at.mkdir(dir);
    assert!(ucmd
        .arg(file)
        .arg(dir)
        .arg(mode_arg)
        .fails()
        .stderr
        .contains("Invalid mode string: invalid digit found in string"));

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
    assert_eq!(0o040333 as u32, PermissionsExt::mode(&permissions));
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

    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);

    if is_ci() && result.stderr.contains("error: no such group:") {
        // In the CI, some server are failing to return the group.
        // As seems to be a configuration issue, ignoring it
        return;
    }

    assert!(result.success);
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

    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);

    if is_ci() && result.stderr.contains("error: no such user:") {
        // In the CI, some server are failing to return the user id.
        // As seems to be a configuration issue, ignoring it
        return;
    }

    assert!(result.success);
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

    let err = ucmd
        .arg(file1)
        .arg(format!("{}/{}", dir, file2))
        .fails()
        .stderr;

    assert!(err.contains("not a directory"))
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
#[cfg(target_os = "linux")]
fn test_install_target_file_dev_null() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file1 = "/dev/null";
    let file2 = "target_file";

    let result = scene.ucmd().arg(file1).arg(file2).run();

    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);

    assert!(result.success);

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

    let r = ucmd.arg(dir1).arg(file1).arg(dir2).run();
    assert!(r.code == Some(1));
    assert!(r.stderr.contains("omitting directory"));
}

#[test]
fn test_install_failing_no_such_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "source_file";
    let file2 = "inexistent_file";
    let dir1 = "target_dir";

    at.mkdir(dir1);
    at.touch(file1);

    let r = ucmd.arg(file1).arg(file2).arg(dir1).run();
    assert!(r.code == Some(1));
    assert!(r.stderr.contains("No such file or directory"));
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

    assert!(before == after);
}

#[test]
#[cfg(target_os = "linux")]
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

#[test]
#[cfg(target_os = "linux")]
fn test_install_and_strip() {
    const SYMBOL_DUMP_PROGRAM: &str = "objdump";

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let source_file = "helloworld.o";
    let target_file = "helloworld_installed.o";
    scene
        .ucmd()
        .arg("-s")
        .arg(source_file)
        .arg(target_file)
        .succeeds()
        .no_stderr();

    let output = Command::new(SYMBOL_DUMP_PROGRAM)
        .arg("-t")
        .arg(at.plus(target_file))
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("no symbols"));

    scene
        .ucmd()
        .arg("-s")
        .arg("--strip-program")
        .arg("/usr/bin/strip")
        .arg(source_file)
        .arg(target_file)
        .succeeds()
        .no_stderr();

    let output = Command::new(SYMBOL_DUMP_PROGRAM)
        .arg("-t")
        .arg(at.plus(target_file))
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("no symbols"));

    scene
        .ucmd()
        .arg("-s")
        .arg("--strip-program")
        .arg("/usr/bin/non_existent_program")
        .arg(source_file)
        .arg(target_file)
        .fails()
        .stderr
        .contains("No such file or directory");
}
