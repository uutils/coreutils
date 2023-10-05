// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::common::util::TestScenario;
use filetime::FileTime;
use std::thread::sleep;
use std::time::Duration;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_mv_rename_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir1 = "test_mv_rename_dir";
    let dir2 = "test_mv_rename_dir2";

    at.mkdir(dir1);

    ucmd.arg(dir1).arg(dir2).succeeds().no_stderr();

    assert!(at.dir_exists(dir2));
}

#[test]
fn test_mv_fail() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir1 = "test_mv_rename_dir";

    at.mkdir(dir1);

    ucmd.arg(dir1).fails();
}

#[test]
fn test_mv_rename_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "test_mv_rename_file";
    let file2 = "test_mv_rename_file2";

    at.touch(file1);

    ucmd.arg(file1).arg(file2).succeeds().no_stderr();
    assert!(at.file_exists(file2));
}

#[test]
fn test_mv_move_file_into_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_mv_move_file_into_dir_dir";
    let file = "test_mv_move_file_into_dir_file";

    at.mkdir(dir);
    at.touch(file);

    ucmd.arg(file).arg(dir).succeeds().no_stderr();

    assert!(at.file_exists(format!("{dir}/{file}")));
}

#[test]
fn test_mv_move_file_into_dir_with_target_arg() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_mv_move_file_into_dir_with_target_arg_dir";
    let file = "test_mv_move_file_into_dir_with_target_arg_file";

    at.mkdir(dir);
    at.touch(file);

    ucmd.arg("--target")
        .arg(dir)
        .arg(file)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(format!("{dir}/{file}")));
}

#[test]
fn test_mv_move_file_into_file_with_target_arg() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "test_mv_move_file_into_file_with_target_arg_file1";
    let file2 = "test_mv_move_file_into_file_with_target_arg_file2";

    at.touch(file1);
    at.touch(file2);

    ucmd.arg("--target")
        .arg(file1)
        .arg(file2)
        .fails()
        .stderr_is(format!("mv: target directory '{file1}': Not a directory\n"));

    assert!(at.file_exists(file1));
}

#[test]
fn test_mv_move_multiple_files_into_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "test_mv_move_multiple_files_into_file1";
    let file2 = "test_mv_move_multiple_files_into_file2";
    let file3 = "test_mv_move_multiple_files_into_file3";

    at.touch(file1);
    at.touch(file2);
    at.touch(file3);

    ucmd.arg(file1)
        .arg(file2)
        .arg(file3)
        .fails()
        .stderr_is(format!("mv: target '{file3}': Not a directory\n"));

    assert!(at.file_exists(file1));
    assert!(at.file_exists(file2));
}

#[test]
fn test_mv_move_file_between_dirs() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir1 = "test_mv_move_file_between_dirs_dir1";
    let dir2 = "test_mv_move_file_between_dirs_dir2";
    let file = "test_mv_move_file_between_dirs_file";

    at.mkdir(dir1);
    at.mkdir(dir2);
    at.touch(format!("{dir1}/{file}"));

    assert!(at.file_exists(format!("{dir1}/{file}")));

    ucmd.arg(&format!("{dir1}/{file}"))
        .arg(dir2)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(format!("{dir1}/{file}")));
    assert!(at.file_exists(format!("{dir2}/{file}")));
}

#[test]
fn test_mv_strip_slashes() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let dir = "test_mv_strip_slashes_dir";
    let file = "test_mv_strip_slashes_file";
    let mut source = file.to_owned();
    source.push('/');

    at.mkdir(dir);
    at.touch(file);

    scene.ucmd().arg(&source).arg(dir).fails();

    assert!(!at.file_exists(format!("{dir}/{file}")));

    scene
        .ucmd()
        .arg("--strip-trailing-slashes")
        .arg(source)
        .arg(dir)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(format!("{dir}/{file}")));
}

#[test]
fn test_mv_multiple_files() {
    let (at, mut ucmd) = at_and_ucmd!();
    let target_dir = "test_mv_multiple_files_dir";
    let file_a = "test_mv_multiple_file_a";
    let file_b = "test_mv_multiple_file_b";

    at.mkdir(target_dir);
    at.touch(file_a);
    at.touch(file_b);

    ucmd.arg(file_a)
        .arg(file_b)
        .arg(target_dir)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(format!("{target_dir}/{file_a}")));
    assert!(at.file_exists(format!("{target_dir}/{file_b}")));
}

#[test]
fn test_mv_multiple_folders() {
    let (at, mut ucmd) = at_and_ucmd!();
    let target_dir = "test_mv_multiple_dirs_dir";
    let dir_a = "test_mv_multiple_dir_a";
    let dir_b = "test_mv_multiple_dir_b";

    at.mkdir(target_dir);
    at.mkdir(dir_a);
    at.mkdir(dir_b);

    ucmd.arg(dir_a)
        .arg(dir_b)
        .arg(target_dir)
        .succeeds()
        .no_stderr();

    assert!(at.dir_exists(&format!("{target_dir}/{dir_a}")));
    assert!(at.dir_exists(&format!("{target_dir}/{dir_b}")));
}

#[test]
fn test_mv_interactive() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let file_a = "test_mv_interactive_file_a";
    let file_b = "test_mv_interactive_file_b";

    at.touch(file_a);
    at.touch(file_b);

    scene
        .ucmd()
        .arg("-i")
        .arg(file_a)
        .arg(file_b)
        .pipe_in("n")
        .fails()
        .no_stdout();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));

    scene
        .ucmd()
        .arg("-i")
        .arg(file_a)
        .arg(file_b)
        .pipe_in("Yesh") // spell-checker:disable-line
        .succeeds()
        .no_stdout();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
}

#[test]
fn test_mv_interactive_with_dir_as_target() {
    let (at, mut ucmd) = at_and_ucmd!();

    let file = "test_mv_interactive_file";
    let target_dir = "target";

    at.mkdir(target_dir);
    at.touch(file);
    at.touch(format!("{target_dir}/{file}"));

    ucmd.arg(file)
        .arg(target_dir)
        .arg("-i")
        .pipe_in("n")
        .fails()
        .stderr_does_not_contain("cannot move")
        .no_stdout();
}

#[test]
fn test_mv_interactive_dir_to_file_not_affirmative() {
    let (at, mut ucmd) = at_and_ucmd!();

    let dir = "test_mv_interactive_dir_to_file_not_affirmative_dir";
    let file = "test_mv_interactive_dir_to_file_not_affirmative_file";

    at.mkdir(dir);
    at.touch(file);

    ucmd.arg(dir)
        .arg(file)
        .arg("-i")
        .pipe_in("n")
        .fails()
        .no_stdout();

    assert!(at.dir_exists(dir));
}

#[test]
fn test_mv_interactive_no_clobber_force_last_arg_wins() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "a.txt";
    let file_b = "b.txt";

    at.touch(file_a);
    at.touch(file_b);

    scene
        .ucmd()
        .args(&[file_a, file_b, "-f", "-i", "-n"])
        .fails()
        .stderr_is(format!("mv: not replacing '{file_b}'\n"));

    scene
        .ucmd()
        .args(&[file_a, file_b, "-n", "-f", "-i"])
        .fails()
        .stderr_is(format!("mv: overwrite '{file_b}'? "));

    at.write(file_a, "aa");

    scene
        .ucmd()
        .args(&[file_a, file_b, "-i", "-n", "-f"])
        .succeeds()
        .no_output();

    assert!(!at.file_exists(file_a));
    assert_eq!("aa", at.read(file_b));
}

#[test]
fn test_mv_arg_update_interactive() {
    let (at, mut ucmd) = at_and_ucmd!();

    let file_a = "test_mv_replace_file_a";
    let file_b = "test_mv_replace_file_b";

    at.touch(file_a);
    at.touch(file_b);

    ucmd.arg(file_a)
        .arg(file_b)
        .arg("-i")
        .arg("--update")
        .succeeds()
        .no_stdout()
        .no_stderr();
}

#[test]
fn test_mv_no_clobber() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_no_clobber_file_a";
    let file_b = "test_mv_no_clobber_file_b";

    at.touch(file_a);
    at.touch(file_b);

    ucmd.arg("-n")
        .arg(file_a)
        .arg(file_b)
        .fails()
        .code_is(1)
        .stderr_only(format!("mv: not replacing '{file_b}'\n"));

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
}

#[test]
fn test_mv_replace_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_replace_file_a";
    let file_b = "test_mv_replace_file_b";

    at.touch(file_a);
    at.touch(file_b);

    ucmd.arg(file_a).arg(file_b).succeeds().no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
}

#[test]
fn test_mv_force_replace_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_force_replace_file_a";
    let file_b = "test_mv_force_replace_file_b";

    at.touch(file_a);
    at.touch(file_b);

    ucmd.arg("--force")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
}

#[test]
fn test_mv_same_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_same_file_a";

    at.touch(file_a);
    ucmd.arg(file_a)
        .arg(file_a)
        .fails()
        .stderr_is(format!("mv: '{file_a}' and '{file_a}' are the same file\n",));
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_mv_same_hardlink() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_same_file_a";
    let file_b = "test_mv_same_file_b";
    at.touch(file_a);

    at.hard_link(file_a, file_b);

    at.touch(file_a);
    ucmd.arg(file_a)
        .arg(file_b)
        .fails()
        .stderr_is(format!("mv: '{file_a}' and '{file_b}' are the same file\n",));
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_mv_same_symlink() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_same_file_a";
    let file_b = "test_mv_same_file_b";
    let file_c = "test_mv_same_file_c";

    at.touch(file_a);

    at.symlink_file(file_a, file_b);

    ucmd.arg(file_b)
        .arg(file_a)
        .fails()
        .stderr_is(format!("mv: '{file_b}' and '{file_a}' are the same file\n",));

    let (at2, mut ucmd2) = at_and_ucmd!();
    at2.touch(file_a);

    at2.symlink_file(file_a, file_b);
    ucmd2.arg(file_a).arg(file_b).succeeds();
    assert!(at2.file_exists(file_b));
    assert!(!at2.file_exists(file_a));

    let (at3, mut ucmd3) = at_and_ucmd!();
    at3.touch(file_a);

    at3.symlink_file(file_a, file_b);
    at3.symlink_file(file_b, file_c);

    ucmd3.arg(file_c).arg(file_b).succeeds();
    assert!(!at3.symlink_exists(file_c));
    assert!(at3.symlink_exists(file_b));

    let (at4, mut ucmd4) = at_and_ucmd!();
    at4.touch(file_a);

    at4.symlink_file(file_a, file_b);
    at4.symlink_file(file_b, file_c);

    ucmd4
        .arg(file_c)
        .arg(file_a)
        .fails()
        .stderr_is(format!("mv: '{file_c}' and '{file_a}' are the same file\n",));
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_mv_hardlink_to_symlink() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "file";
    let symlink_file = "symlink";
    let hardlink_to_symlink_file = "hardlink_to_symlink";

    at.touch(file);
    at.symlink_file(file, symlink_file);
    at.hard_link(symlink_file, hardlink_to_symlink_file);

    ucmd.arg(symlink_file).arg(hardlink_to_symlink_file).fails();

    let (at2, mut ucmd2) = at_and_ucmd!();

    at2.touch(file);
    at2.symlink_file(file, symlink_file);
    at2.hard_link(symlink_file, hardlink_to_symlink_file);

    ucmd2
        .arg("--backup")
        .arg(symlink_file)
        .arg(hardlink_to_symlink_file)
        .succeeds();
    assert!(!at2.symlink_exists(symlink_file));
    assert!(at2.symlink_exists(&format!("{hardlink_to_symlink_file}~")));
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_mv_same_hardlink_backup_simple() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_same_file_a";
    let file_b = "test_mv_same_file_b";
    at.touch(file_a);

    at.hard_link(file_a, file_b);

    ucmd.arg(file_a)
        .arg(file_b)
        .arg("--backup=simple")
        .succeeds();
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_mv_same_hardlink_backup_simple_destroy() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_same_file_a~";
    let file_b = "test_mv_same_file_a";
    at.touch(file_a);
    at.touch(file_b);

    ucmd.arg(file_a)
        .arg(file_b)
        .arg("--b=simple")
        .fails()
        .stderr_contains("backing up 'test_mv_same_file_a' might destroy source");
}

#[test]
fn test_mv_same_file_not_dot_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_mv_errors_dir";

    at.mkdir(dir);
    ucmd.arg(dir).arg(dir).fails().stderr_is(format!(
        "mv: cannot move '{dir}' to a subdirectory of itself, '{dir}/{dir}'\n",
    ));
}

#[test]
fn test_mv_same_file_dot_dir() {
    let (_at, mut ucmd) = at_and_ucmd!();

    ucmd.arg(".")
        .arg(".")
        .fails()
        .stderr_is("mv: '.' and '.' are the same file\n");
}

#[test]
fn test_mv_simple_backup() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_simple_backup_file_a";
    let file_b = "test_mv_simple_backup_file_b";

    at.touch(file_a);
    at.touch(file_b);
    ucmd.arg("-b")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(format!("{file_b}~")));
}

#[test]
fn test_mv_simple_backup_with_file_extension() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_simple_backup_file_a.txt";
    let file_b = "test_mv_simple_backup_file_b.txt";

    at.touch(file_a);
    at.touch(file_b);
    ucmd.arg("-b")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(format!("{file_b}~")));
}

#[test]
fn test_mv_arg_backup_arg_first() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_simple_backup_file_a";
    let file_b = "test_mv_simple_backup_file_b";

    at.touch(file_a);
    at.touch(file_b);
    ucmd.arg("--backup").arg(file_a).arg(file_b).succeeds();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(format!("{file_b}~")));
}

#[test]
fn test_mv_custom_backup_suffix() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_custom_backup_suffix_file_a";
    let file_b = "test_mv_custom_backup_suffix_file_b";
    let suffix = "super-suffix-of-the-century";

    at.touch(file_a);
    at.touch(file_b);
    ucmd.arg("-b")
        .arg(format!("--suffix={suffix}"))
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(format!("{file_b}{suffix}")));
}

#[test]
fn test_mv_custom_backup_suffix_hyphen_value() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_custom_backup_suffix_file_a";
    let file_b = "test_mv_custom_backup_suffix_file_b";
    let suffix = "-v";

    at.touch(file_a);
    at.touch(file_b);
    ucmd.arg("-b")
        .arg(format!("--suffix={suffix}"))
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(format!("{file_b}{suffix}")));
}

#[test]
fn test_mv_custom_backup_suffix_via_env() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_custom_backup_suffix_file_a";
    let file_b = "test_mv_custom_backup_suffix_file_b";
    let suffix = "super-suffix-of-the-century";
    at.touch(file_a);
    at.touch(file_b);
    ucmd.arg("-b")
        .env("SIMPLE_BACKUP_SUFFIX", suffix)
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(format!("{file_b}{suffix}")));
}

#[test]
fn test_mv_backup_numbered_with_t() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_backup_numbering_file_a";
    let file_b = "test_mv_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    ucmd.arg("--backup=t")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(format!("{file_b}.~1~")));
}

#[test]
fn test_mv_backup_numbered() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_backup_numbering_file_a";
    let file_b = "test_mv_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    ucmd.arg("--backup=numbered")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(format!("{file_b}.~1~")));
}

#[test]
fn test_mv_backup_existing() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_backup_numbering_file_a";
    let file_b = "test_mv_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    ucmd.arg("--backup=existing")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(format!("{file_b}~")));
}

#[test]
fn test_mv_backup_nil() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_backup_numbering_file_a";
    let file_b = "test_mv_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    ucmd.arg("--backup=nil")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(format!("{file_b}~")));
}

#[test]
fn test_mv_numbered_if_existing_backup_existing() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_backup_numbering_file_a";
    let file_b = "test_mv_backup_numbering_file_b";
    let file_b_backup = "test_mv_backup_numbering_file_b.~1~";

    at.touch(file_a);
    at.touch(file_b);
    at.touch(file_b_backup);
    ucmd.arg("--backup=existing")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_b));
    assert!(at.file_exists(file_b_backup));
    assert!(at.file_exists(format!("{file_b}.~2~")));
}

#[test]
fn test_mv_numbered_if_existing_backup_nil() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_backup_numbering_file_a";
    let file_b = "test_mv_backup_numbering_file_b";
    let file_b_backup = "test_mv_backup_numbering_file_b.~1~";

    at.touch(file_a);
    at.touch(file_b);
    at.touch(file_b_backup);
    ucmd.arg("--backup=nil")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_b));
    assert!(at.file_exists(file_b_backup));
    assert!(at.file_exists(format!("{file_b}.~2~")));
}

#[test]
fn test_mv_backup_simple() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_backup_numbering_file_a";
    let file_b = "test_mv_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    ucmd.arg("--backup=simple")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(format!("{file_b}~")));
}

#[test]
fn test_mv_backup_never() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_backup_numbering_file_a";
    let file_b = "test_mv_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    ucmd.arg("--backup=never")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(format!("{file_b}~")));
}

#[test]
fn test_mv_backup_none() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_backup_numbering_file_a";
    let file_b = "test_mv_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    ucmd.arg("--backup=none")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(!at.file_exists(format!("{file_b}~")));
}

#[test]
fn test_mv_backup_off() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_backup_numbering_file_a";
    let file_b = "test_mv_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    ucmd.arg("--backup=off")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(!at.file_exists(format!("{file_b}~")));
}

#[test]
fn test_mv_backup_no_clobber_conflicting_options() {
    new_ucmd!()
        .arg("--backup")
        .arg("--no-clobber")
        .arg("file1")
        .arg("file2")
        .fails()
        .usage_error("options --backup and --no-clobber are mutually exclusive");
}

#[test]
fn test_mv_update_option() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let file_a = "test_mv_update_option_file_a";
    let file_b = "test_mv_update_option_file_b";

    at.touch(file_a);
    at.touch(file_b);
    let ts = time::OffsetDateTime::now_utc();
    let now = FileTime::from_unix_time(ts.unix_timestamp(), ts.nanosecond());
    let later = FileTime::from_unix_time(ts.unix_timestamp() + 3600, ts.nanosecond());
    filetime::set_file_times(at.plus_as_string(file_a), now, now).unwrap();
    filetime::set_file_times(at.plus_as_string(file_b), now, later).unwrap();

    scene.ucmd().arg("--update").arg(file_a).arg(file_b).run();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));

    scene
        .ucmd()
        .arg("--update")
        .arg(file_b)
        .arg(file_a)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(file_a));
    assert!(!at.file_exists(file_b));
}

#[test]
fn test_mv_arg_update_none() {
    let (at, mut ucmd) = at_and_ucmd!();

    let file1 = "test_mv_arg_update_none_file1";
    let file2 = "test_mv_arg_update_none_file2";
    let file1_content = "file1 content\n";
    let file2_content = "file2 content\n";

    at.write(file1, file1_content);
    at.write(file2, file2_content);

    ucmd.arg(file1)
        .arg(file2)
        .arg("--update=none")
        .succeeds()
        .no_stderr()
        .no_stdout();

    assert_eq!(at.read(file2), file2_content);
}

#[test]
fn test_mv_arg_update_all() {
    let (at, mut ucmd) = at_and_ucmd!();

    let file1 = "test_mv_arg_update_none_file1";
    let file2 = "test_mv_arg_update_none_file2";
    let file1_content = "file1 content\n";
    let file2_content = "file2 content\n";

    at.write(file1, file1_content);
    at.write(file2, file2_content);

    ucmd.arg(file1)
        .arg(file2)
        .arg("--update=all")
        .succeeds()
        .no_stderr()
        .no_stdout();

    assert_eq!(at.read(file2), file1_content);
}

#[test]
fn test_mv_arg_update_older_dest_not_older() {
    let (at, mut ucmd) = at_and_ucmd!();

    let old = "test_mv_arg_update_none_file1";
    let new = "test_mv_arg_update_none_file2";
    let old_content = "file1 content\n";
    let new_content = "file2 content\n";

    at.write(old, old_content);

    sleep(Duration::from_secs(1));

    at.write(new, new_content);

    ucmd.arg(old)
        .arg(new)
        .arg("--update=older")
        .succeeds()
        .no_stderr()
        .no_stdout();

    assert_eq!(at.read(new), new_content);
}

#[test]
fn test_mv_arg_update_none_then_all() {
    // take last if multiple update args are supplied,
    // update=all wins in this case
    let (at, mut ucmd) = at_and_ucmd!();

    let old = "test_mv_arg_update_none_then_all_file1";
    let new = "test_mv_arg_update_none_then_all_file2";
    let old_content = "old content\n";
    let new_content = "new content\n";

    at.write(old, old_content);

    sleep(Duration::from_secs(1));

    at.write(new, new_content);

    ucmd.arg(old)
        .arg(new)
        .arg("--update=none")
        .arg("--update=all")
        .succeeds()
        .no_stderr()
        .no_stdout();

    assert_eq!(at.read(new), "old content\n");
}

#[test]
fn test_mv_arg_update_all_then_none() {
    // take last if multiple update args are supplied,
    // update=none wins in this case
    let (at, mut ucmd) = at_and_ucmd!();

    let old = "test_mv_arg_update_all_then_none_file1";
    let new = "test_mv_arg_update_all_then_none_file2";
    let old_content = "old content\n";
    let new_content = "new content\n";

    at.write(old, old_content);

    sleep(Duration::from_secs(1));

    at.write(new, new_content);

    ucmd.arg(old)
        .arg(new)
        .arg("--update=all")
        .arg("--update=none")
        .succeeds()
        .no_stderr()
        .no_stdout();

    assert_eq!(at.read(new), "new content\n");
}

#[test]
fn test_mv_arg_update_older_dest_older() {
    let (at, mut ucmd) = at_and_ucmd!();

    let old = "test_mv_arg_update_none_file1";
    let new = "test_mv_arg_update_none_file2";
    let old_content = "file1 content\n";
    let new_content = "file2 content\n";

    at.write(old, old_content);

    sleep(Duration::from_secs(1));

    at.write(new, new_content);

    ucmd.arg(new)
        .arg(old)
        .arg("--update=all")
        .succeeds()
        .no_stderr()
        .no_stdout();

    assert_eq!(at.read(old), new_content);
}

#[test]
fn test_mv_arg_update_short_overwrite() {
    // same as --update=older
    let (at, mut ucmd) = at_and_ucmd!();

    let old = "test_mv_arg_update_none_file1";
    let new = "test_mv_arg_update_none_file2";
    let old_content = "file1 content\n";
    let new_content = "file2 content\n";

    at.write(old, old_content);

    sleep(Duration::from_secs(1));

    at.write(new, new_content);

    ucmd.arg(new)
        .arg(old)
        .arg("-u")
        .succeeds()
        .no_stderr()
        .no_stdout();

    assert_eq!(at.read(old), new_content);
}

#[test]
fn test_mv_arg_update_short_no_overwrite() {
    // same as --update=older
    let (at, mut ucmd) = at_and_ucmd!();

    let old = "test_mv_arg_update_none_file1";
    let new = "test_mv_arg_update_none_file2";
    let old_content = "file1 content\n";
    let new_content = "file2 content\n";

    at.write(old, old_content);

    sleep(Duration::from_secs(1));

    at.write(new, new_content);

    ucmd.arg(old)
        .arg(new)
        .arg("-u")
        .succeeds()
        .no_stderr()
        .no_stdout();

    assert_eq!(at.read(new), new_content);
}

#[test]
fn test_mv_target_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_mv_target_dir_dir";
    let file_a = "test_mv_target_dir_file_a";
    let file_b = "test_mv_target_dir_file_b";

    at.touch(file_a);
    at.touch(file_b);
    at.mkdir(dir);
    ucmd.arg("-t")
        .arg(dir)
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(!at.file_exists(file_b));
    assert!(at.file_exists(format!("{dir}/{file_a}")));
    assert!(at.file_exists(format!("{dir}/{file_b}")));
}

#[test]
fn test_mv_target_dir_single_source() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_mv_target_dir_single_source_dir";
    let file = "test_mv_target_dir_single_source_file";

    at.touch(file);
    at.mkdir(dir);
    ucmd.arg("-t").arg(dir).arg(file).succeeds().no_stderr();

    assert!(!at.file_exists(file));
    assert!(at.file_exists(format!("{dir}/{file}")));
}

#[test]
fn test_mv_overwrite_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir_a = "test_mv_overwrite_dir_a";
    let dir_b = "test_mv_overwrite_dir_b";

    at.mkdir(dir_a);
    at.mkdir(dir_b);
    ucmd.arg("-T").arg(dir_a).arg(dir_b).succeeds().no_stderr();

    assert!(!at.dir_exists(dir_a));
    assert!(at.dir_exists(dir_b));
}

#[test]
fn test_mv_overwrite_nonempty_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir_a = "test_mv_overwrite_nonempty_dir_a";
    let dir_b = "test_mv_overwrite_nonempty_dir_b";
    let dummy = "test_mv_overwrite_nonempty_dir_b/file";

    at.mkdir(dir_a);
    at.mkdir(dir_b);
    at.touch(dummy);
    // Not same error as GNU; the error message is a rust builtin
    // TODO: test (and implement) correct error message (or at least decide whether to do so)
    // Current: "mv: couldn't rename path (Directory not empty; from=a; to=b)"
    // GNU:     "mv: cannot move 'a' to 'b': Directory not empty"

    // Verbose output for the move should not be shown on failure
    let result = ucmd.arg("-vT").arg(dir_a).arg(dir_b).fails();
    result.no_stdout();
    assert!(!result.stderr_str().is_empty());

    assert!(at.dir_exists(dir_a));
    assert!(at.dir_exists(dir_b));
}

#[test]
fn test_mv_backup_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir_a = "test_mv_backup_dir_dir_a";
    let dir_b = "test_mv_backup_dir_dir_b";

    at.mkdir(dir_a);
    at.mkdir(dir_b);
    ucmd.arg("-vbT")
        .arg(dir_a)
        .arg(dir_b)
        .succeeds()
        .stdout_only(format!(
            "renamed '{dir_a}' -> '{dir_b}' (backup: '{dir_b}~')\n"
        ));

    assert!(!at.dir_exists(dir_a));
    assert!(at.dir_exists(dir_b));
    assert!(at.dir_exists(&format!("{dir_b}~")));
}

#[test]
fn test_mv_errors() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let dir = "test_mv_errors_dir";
    let file_a = "test_mv_errors_file_a";
    let file_b = "test_mv_errors_file_b";
    at.mkdir(dir);
    at.touch(file_a);
    at.touch(file_b);

    // $ mv -T -t a b
    // mv: cannot combine --target-directory (-t) and --no-target-directory (-T)
    scene
        .ucmd()
        .arg("-T")
        .arg("-t")
        .arg(dir)
        .arg(file_a)
        .arg(file_b)
        .fails()
        .stderr_contains("cannot be used with");

    // $ at.touch file && at.mkdir dir
    // $ mv -T file dir
    // err == mv: cannot overwrite directory 'dir' with non-directory
    scene
        .ucmd()
        .arg("-T")
        .arg(file_a)
        .arg(dir)
        .fails()
        .stderr_is(format!(
            "mv: cannot overwrite directory '{dir}' with non-directory\n"
        ));

    // $ at.mkdir dir && at.touch file
    // $ mv dir file
    // err == mv: cannot overwrite non-directory 'file' with directory 'dir'
    assert!(!scene
        .ucmd()
        .arg(dir)
        .arg(file_a)
        .fails()
        .stderr_str()
        .is_empty());
}

#[test]
fn test_mv_verbose() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let dir = "test_mv_verbose_dir";
    let file_a = "test_mv_verbose_file_a";
    let file_b = "test_mv_verbose_file_b";
    at.mkdir(dir);
    at.touch(file_a);
    at.touch(file_b);

    scene
        .ucmd()
        .arg("-v")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .stdout_only(format!("renamed '{file_a}' -> '{file_b}'\n"));

    at.touch(file_a);
    scene
        .ucmd()
        .arg("-vb")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .stdout_only(format!(
            "renamed '{file_a}' -> '{file_b}' (backup: '{file_b}~')\n"
        ));
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))] // mkdir does not support -m on windows. Freebsd doesn't return a permission error either.
#[cfg(feature = "mkdir")]
fn test_mv_permission_error() {
    let scene = TestScenario::new("mkdir");
    let folder1 = "bar";
    let folder2 = "foo";
    let folder_to_move = "bar/foo";
    scene.ucmd().arg("-m444").arg(folder1).succeeds();
    scene.ucmd().arg("-m777").arg(folder2).succeeds();

    scene
        .ccmd("mv")
        .arg(folder2)
        .arg(folder_to_move)
        .fails()
        .stderr_contains("Permission denied");
}

#[test]
fn test_mv_interactive_error() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let dir = "test_mv_errors_dir";
    let file_a = "test_mv_errors_file_a";
    at.mkdir(dir);
    at.touch(file_a);

    // $ at.mkdir dir && at.touch file
    // $ mv -i dir file
    // err == mv: cannot overwrite non-directory 'file' with directory 'dir'
    assert!(!scene
        .ucmd()
        .arg("-i")
        .arg(dir)
        .arg(file_a)
        .pipe_in("y")
        .fails()
        .stderr_str()
        .is_empty());
}

#[test]
fn test_mv_into_self() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let dir1 = "dir1";
    let dir2 = "dir2";
    at.mkdir(dir1);
    at.mkdir(dir2);

    scene
        .ucmd()
        .arg(dir1)
        .arg(dir2)
        .arg(dir2)
        .fails()
        .stderr_contains("mv: cannot move 'dir2' to a subdirectory of itself, 'dir2/dir2'");
}

#[test]
fn test_mv_arg_interactive_skipped() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("a");
    at.touch("b");
    ucmd.args(&["-vi", "a", "b"])
        .pipe_in("N\n")
        .ignore_stdin_write_error()
        .fails()
        .stderr_is("mv: overwrite 'b'? ")
        .no_stdout();
}

#[test]
fn test_mv_arg_interactive_skipped_vin() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("a");
    at.touch("b");
    ucmd.args(&["-vin", "a", "b"])
        .fails()
        .stderr_is("mv: not replacing 'b'\n")
        .no_stdout();
}

#[test]
fn test_mv_into_self_data() {
    let scene = TestScenario::new(util_name!());

    let at = &scene.fixtures;
    let sub_dir = "sub_folder";
    let file1 = "t1.test";
    let file2 = "sub_folder/t2.test";

    let file1_result_location = "sub_folder/t1.test";

    at.mkdir(sub_dir);
    at.touch(file1);
    at.touch(file2);

    let result = scene.ucmd().arg(file1).arg(sub_dir).arg(sub_dir).run();

    // sub_dir exists, file1 has been moved, file2 still exists.
    result.code_is(1);

    assert!(at.dir_exists(sub_dir));
    assert!(at.file_exists(file1_result_location));
    assert!(at.file_exists(file2));
    assert!(!at.file_exists(file1));
}
// Todo:

// $ at.touch a b
// $ chmod -w b
// $ ll
// total 0
// -rw-rw-r-- 1 user user 0 okt 25 11:21 a
// -r--r--r-- 1 user user 0 okt 25 11:21 b
// $
// $ mv -v a b
// mv: try to overwrite 'b', overriding mode 0444 (r--r--r--)? y
// 'a' -> 'b'
