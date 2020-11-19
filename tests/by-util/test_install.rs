use crate::common::util::*;
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
    let dir = "test_install_target_dir_dir_a";
    let file1 = "test_install_target_dir_file_a1";
    let file2 = "test_install_target_dir_file_a2";

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
fn test_install_failing_not_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "test_install_target_dir_file_a1";
    let file2 = "test_install_target_dir_file_a2";
    let file3 = "test_install_target_dir_file_a3";

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
    let dir = "test_install_target_dir_dir_b";
    let file = "test_install_target_dir_file_b";
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
    let component1 = "test_install_target_dir_component_c1";
    let component2 = "test_install_target_dir_component_c2";
    let component3 = "test_install_target_dir_component_c3";
    let directories_arg = "-d";

    ucmd.args(&[directories_arg, component1, component2, component3])
        .succeeds()
        .no_stderr();

    assert!(at.dir_exists(component1));
    assert!(at.dir_exists(component2));
    assert!(at.dir_exists(component3));
}

#[test]
fn test_install_component_directories_failing() {
    let (at, mut ucmd) = at_and_ucmd!();
    let component = "test_install_target_dir_component_d1";
    let directories_arg = "-d";

    at.mkdir(component);
    assert!(ucmd
        .arg(directories_arg)
        .arg(component)
        .fails()
        .stderr
        .contains("File exists"));
}

#[test]
fn test_install_mode_numeric() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_install_target_dir_dir_e";
    let file = "test_install_target_dir_file_e";
    let mode_arg = "--mode=333";

    at.touch(file);
    at.mkdir(dir);
    ucmd.arg(file).arg(dir).arg(mode_arg).succeeds().no_stderr();

    let dest_file = &format!("{}/{}", dir, file);
    assert!(at.file_exists(file));
    assert!(at.file_exists(dest_file));
    let permissions = at.metadata(dest_file).permissions();
    assert_eq!(0o100333 as u32, PermissionsExt::mode(&permissions));
}

#[test]
fn test_install_mode_symbolic() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_install_target_dir_dir_f";
    let file = "test_install_target_dir_file_f";
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
    let dir = "test_install_target_dir_dir_g";
    let file = "test_install_target_dir_file_g";
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
    let component = "test_install_target_dir_component_h";
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
    let file1 = "test_install_target_file_file_i1";
    let file2 = "test_install_target_file_file_i2";

    at.touch(file1);
    at.touch(file2);
    ucmd.arg(file1).arg(file2).succeeds().no_stderr();

    assert!(at.file_exists(file1));
    assert!(at.file_exists(file2));
}

#[test]
fn test_install_target_new_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_install_target_new_filer_file_j";
    let dir = "test_install_target_new_file_dir_j";

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
fn test_install_target_new_file_failing_nonexistent_parent() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "test_install_target_new_file_failing_file_k1";
    let file2 = "test_install_target_new_file_failing_file_k2";
    let dir = "test_install_target_new_file_failing_dir_k";

    at.touch(file1);

    let err = ucmd
        .arg(file1)
        .arg(format!("{}/{}", dir, file2))
        .fails()
        .stderr;

    assert!(err.contains("not a directory"))
}
