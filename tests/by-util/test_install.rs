use crate::common::util::*;
use rust_users::*;
use std::os::unix::fs::PermissionsExt;

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
fn test_install_component_directories() {
    let (at, mut ucmd) = at_and_ucmd!();
    let component1 = "component1";
    let component2 = "component2";
    let component3 = "component3";
    let directories_arg = "-d";

    ucmd.args(&[directories_arg, component1, component2, component3])
        .succeeds()
        .no_stderr();

    assert!(at.dir_exists(component1));
    assert!(at.dir_exists(component2));
    assert!(at.dir_exists(component3));
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
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "/dev/null";
    let file2 = "target_file";

    ucmd.arg(file1).arg(file2).succeeds().no_stderr();
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
