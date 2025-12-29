// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker:ignore mydir hardlinked tmpfs notty unwriteable

use filetime::FileTime;
use rstest::rstest;
use std::io::Write;
#[cfg(not(windows))]
use std::path::Path;
#[cfg(feature = "feat_selinux")]
use uucore::selinux::get_getfattr_output;
use uutests::new_ucmd;
#[cfg(unix)]
use uutests::util::TerminalSimulation;
use uutests::util::TestScenario;
use uutests::{at_and_ucmd, util_name};

#[test]
fn test_mv_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_mv_missing_dest() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "dir";

    at.mkdir(dir);

    ucmd.arg(dir).fails();
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
fn test_mv_rename_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "test_mv_rename_file";
    let file2 = "test_mv_rename_file2";

    at.touch(file1);

    ucmd.arg(file1).arg(file2).succeeds().no_stderr();
    assert!(at.file_exists(file2));
}

#[test]
fn test_mv_with_source_file_opened_and_target_file_exists() {
    let (at, mut ucmd) = at_and_ucmd!();

    let src = "source_file_opened";
    let dst = "target_file_exists";

    let f = at.make_file(src);

    at.touch(dst);

    ucmd.arg(src).arg(dst).succeeds().no_stderr().no_stdout();

    drop(f);
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

    ucmd.arg(format!("{dir1}/{file}"))
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

    assert!(at.dir_exists(format!("{target_dir}/{dir_a}")));
    assert!(at.dir_exists(format!("{target_dir}/{dir_b}")));
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
        .args(&[file_a, file_b, "-f", "-i", "-n", "--debug"])
        .succeeds()
        .stdout_contains("skipped 'b.txt'");

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
        .arg("--debug")
        .succeeds()
        .stdout_contains("skipped 'test_mv_no_clobber_file_b");

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
#[cfg(all(unix, not(target_os = "android")))]
fn test_mv_replace_symlink_with_symlink() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("a");
    at.mkdir("b");
    at.touch("a/empty_file_a");
    at.touch("b/empty_file_b");

    at.symlink_dir("a", "symlink_a");
    at.symlink_dir("b", "symlink_b");

    assert_eq!(at.read("symlink_a/empty_file_a"), "");

    ucmd.arg("-T")
        .arg("symlink_b")
        .arg("symlink_a")
        .succeeds()
        .no_stderr();

    assert!(at.file_exists("symlink_a/empty_file_b"));
    assert!(!at.file_exists("symlink_a/empty_file_a"));
    assert!(!at.symlink_exists("symlink_b"));
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_mv_replace_symlink_with_directory() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("a");
    at.mkdir("b");
    at.touch("b/empty_file_b");

    at.symlink_file("a", "symlink");

    ucmd.arg("-T")
        .arg("b")
        .arg("symlink")
        .fails()
        .stderr_contains("cannot overwrite non-directory")
        .stderr_contains("with directory");
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_mv_replace_symlink_with_file() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("a");
    at.touch("b");

    at.symlink_file("a", "symlink");

    ucmd.arg("-T")
        .arg("b")
        .arg("symlink")
        .succeeds()
        .no_stderr();

    assert!(at.file_exists("symlink"));
    assert!(!at.is_symlink("symlink"));
    assert!(!at.file_exists("b"));
    assert!(at.file_exists("a"));
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_mv_file_to_symlink_directory() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("a");
    at.touch("a/empty_file_a");
    at.touch("b");

    at.symlink_dir("a", "symlink");

    assert!(at.file_exists("symlink/empty_file_a"));

    ucmd.arg("b").arg("symlink").succeeds().no_stderr();

    assert!(at.dir_exists("symlink"));
    assert!(at.is_symlink("symlink"));
    assert!(at.file_exists("symlink/b"));
    assert!(!at.file_exists("b"));
    assert!(at.dir_exists("a"));
    assert!(at.file_exists("a/b"));
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
        .stderr_is(format!("mv: '{file_a}' and '{file_a}' are the same file\n"));
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
        .stderr_is(format!("mv: '{file_a}' and '{file_b}' are the same file\n"));
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_mv_dangling_symlink_to_folder() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.symlink_file("404", "abc");
    at.mkdir("x");

    ucmd.arg("abc").arg("x").succeeds();

    assert!(at.symlink_exists("x/abc"));
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
        .stderr_is(format!("mv: '{file_b}' and '{file_a}' are the same file\n"));

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
        .stderr_is(format!("mv: '{file_c}' and '{file_a}' are the same file\n"));
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_mv_same_broken_symlink() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.symlink_file("missing-target", "broken");

    ucmd.arg("broken")
        .arg("broken")
        .fails()
        .stderr_is("mv: 'broken' and 'broken' are the same file\n");
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_mv_symlink_into_target() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("dir");
    at.symlink_file("dir", "dir-link");

    ucmd.arg("dir-link").arg("dir").succeeds();
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
    assert!(at2.symlink_exists(format!("{hardlink_to_symlink_file}~")));
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
fn test_mv_simple_backup_for_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir_a = "test_mv_simple_backup_dir_a";
    let dir_b = "test_mv_simple_backup_dir_b";

    at.mkdir(dir_a);
    at.mkdir(dir_b);
    at.touch(format!("{dir_a}/file_a"));
    at.touch(format!("{dir_b}/file_b"));
    ucmd.arg("-T")
        .arg("-b")
        .arg(dir_a)
        .arg(dir_b)
        .succeeds()
        .no_stderr();

    assert!(!at.dir_exists(dir_a));
    assert!(at.dir_exists(dir_b));
    assert!(at.dir_exists(format!("{dir_b}~")));
    assert!(at.file_exists(format!("{dir_b}/file_a")));
    assert!(at.file_exists(format!("{dir_b}~/file_b")));
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
fn test_suffix_without_backup_option() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_mv_custom_backup_suffix_file_a";
    let file_b = "test_mv_custom_backup_suffix_file_b";
    let suffix = "super-suffix-of-the-century";

    at.touch(file_a);
    at.touch(file_b);
    ucmd.arg(format!("--suffix={suffix}"))
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
fn test_mv_backup_conflicting_options() {
    for conflicting_opt in ["--no-clobber", "--update=none-fail", "--update=none"] {
        new_ucmd!()
            .arg("--backup")
            .arg(conflicting_opt)
            .arg("file1")
            .arg("file2")
            .fails()
            .usage_error("cannot combine --backup with -n/--no-clobber or --update=none-fail");
    }
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

    scene
        .ucmd()
        .arg("--update")
        .arg(file_a)
        .arg(file_b)
        .succeeds();

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
fn test_mv_update_with_dest_ending_with_slash() {
    let (at, mut ucmd) = at_and_ucmd!();
    let source = "source";
    let dest = "destination/";

    at.mkdir("source");

    ucmd.arg("--update").arg(source).arg(dest).succeeds();

    assert!(!at.dir_exists(source));
    assert!(at.dir_exists(dest));
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

    let mut f = at.make_file(old);
    f.write_all(old_content.as_bytes()).unwrap();
    f.set_modified(std::time::UNIX_EPOCH).unwrap();

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

    let mut f = at.make_file(old);
    f.write_all(old_content.as_bytes()).unwrap();
    f.set_modified(std::time::UNIX_EPOCH).unwrap();

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

    let mut f = at.make_file(old);
    f.write_all(old_content.as_bytes()).unwrap();
    f.set_modified(std::time::UNIX_EPOCH).unwrap();

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

    let mut f = at.make_file(old);
    f.write_all(old_content.as_bytes()).unwrap();
    f.set_modified(std::time::UNIX_EPOCH).unwrap();

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
fn test_mv_arg_update_older_dest_older_interactive() {
    let (at, mut ucmd) = at_and_ucmd!();

    let old = "old";
    let new = "new";
    let old_content = "file1 content\n";
    let new_content = "file2 content\n";

    let mut f = at.make_file(old);
    f.write_all(old_content.as_bytes()).unwrap();
    f.set_modified(std::time::UNIX_EPOCH).unwrap();

    at.write(new, new_content);

    ucmd.arg(new)
        .arg(old)
        .arg("--interactive")
        .arg("--update=older")
        .fails()
        .stderr_contains("overwrite 'old'?")
        .no_stdout();
}

#[test]
fn test_mv_arg_update_short_overwrite() {
    // same as --update=older
    let (at, mut ucmd) = at_and_ucmd!();

    let old = "test_mv_arg_update_none_file1";
    let new = "test_mv_arg_update_none_file2";
    let old_content = "file1 content\n";
    let new_content = "file2 content\n";

    let mut f = at.make_file(old);
    f.write_all(old_content.as_bytes()).unwrap();
    f.set_modified(std::time::UNIX_EPOCH).unwrap();

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

    let mut f = at.make_file(old);
    f.write_all(old_content.as_bytes()).unwrap();
    f.set_modified(std::time::UNIX_EPOCH).unwrap();

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
fn test_mv_no_target_dir_with_dest_not_existing() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir_a = "a";
    let dir_b = "b";

    at.mkdir(dir_a);
    ucmd.arg("-T").arg(dir_a).arg(dir_b).succeeds().no_output();

    assert!(!at.dir_exists(dir_a));
    assert!(at.dir_exists(dir_b));
}

#[test]
fn test_mv_no_target_dir_with_dest_not_existing_and_ending_with_slash() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir_a = "a";
    let dir_b = "b/";

    at.mkdir(dir_a);
    ucmd.arg("-T").arg(dir_a).arg(dir_b).succeeds().no_output();

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
    assert!(at.dir_exists(format!("{dir_b}~")));
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
    assert!(
        !scene
            .ucmd()
            .arg(dir)
            .arg(file_a)
            .fails()
            .stderr_str()
            .is_empty()
    );
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
    assert!(
        !scene
            .ucmd()
            .arg("-i")
            .arg(dir)
            .arg(file_a)
            .pipe_in("y")
            .fails()
            .stderr_str()
            .is_empty()
    );
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
    ucmd.args(&["-vin", "a", "b", "--debug"])
        .succeeds()
        .stdout_contains("skipped 'b'");
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

    scene
        .ucmd()
        .arg(file1)
        .arg(sub_dir)
        .arg(sub_dir)
        .fails_with_code(1);

    // sub_dir exists, file1 has been moved, file2 still exists.
    assert!(at.dir_exists(sub_dir));
    assert!(at.file_exists(file1_result_location));
    assert!(at.file_exists(file2));
    assert!(!at.file_exists(file1));
}

#[rstest]
#[case(vec!["mydir"], vec!["mydir", "mydir"], "mv: cannot move 'mydir' to a subdirectory of itself, 'mydir/mydir'")]
#[case(vec!["mydir"], vec!["mydir/", "mydir/"], "mv: cannot move 'mydir/' to a subdirectory of itself, 'mydir/mydir'")]
#[case(vec!["mydir"], vec!["./mydir", "mydir", "mydir/"], "mv: cannot move './mydir' to a subdirectory of itself, 'mydir/mydir'")]
#[case(vec!["mydir"], vec!["mydir/", "mydir/mydir_2/"], "mv: cannot move 'mydir/' to a subdirectory of itself, 'mydir/mydir_2/'")]
#[case(vec!["mydir/mydir_2"], vec!["mydir", "mydir/mydir_2"], "mv: cannot move 'mydir' to a subdirectory of itself, 'mydir/mydir_2/mydir'\n")]
#[case(vec!["mydir/mydir_2"], vec!["mydir/", "mydir/mydir_2/"], "mv: cannot move 'mydir/' to a subdirectory of itself, 'mydir/mydir_2/mydir'\n")]
#[case(vec!["mydir", "mydir_2"], vec!["mydir/", "mydir_2/", "mydir_2/"], "mv: cannot move 'mydir_2/' to a subdirectory of itself, 'mydir_2/mydir_2'")]
#[case(vec!["mydir"], vec!["mydir/", "mydir"], "mv: cannot move 'mydir/' to a subdirectory of itself, 'mydir/mydir'")]
#[case(vec!["mydir"], vec!["-T", "mydir", "mydir"], "mv: 'mydir' and 'mydir' are the same file")]
#[case(vec!["mydir"], vec!["mydir/", "mydir/../"], "mv: 'mydir/' and 'mydir/../mydir' are the same file")]
fn test_mv_directory_self(
    #[case] dirs: Vec<&str>,
    #[case] args: Vec<&str>,
    #[case] expected_error: &str,
) {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    for dir in dirs {
        at.mkdir_all(dir);
    }
    scene
        .ucmd()
        .args(&args)
        .fails()
        .stderr_contains(expected_error);
}

#[test]
fn test_mv_dir_into_dir_with_source_name_a_prefix_of_target_name() {
    let (at, mut ucmd) = at_and_ucmd!();
    let source = "test";
    let target = "test2";

    at.mkdir(source);
    at.mkdir(target);

    ucmd.arg(source).arg(target).succeeds().no_output();

    assert!(at.dir_exists(format!("{target}/{source}")));
}

#[test]
fn test_mv_file_into_dir_where_both_are_files() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("a");
    at.touch("b");
    scene
        .ucmd()
        .arg("a")
        .arg("b/")
        .fails()
        .stderr_contains("mv: failed to access 'b/': Not a directory");
}

#[test]
fn test_mv_seen_file() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.mkdir("a");
    at.mkdir("b");
    at.mkdir("c");
    at.write("a/f", "a");
    at.write("b/f", "b");

    let result = ts.ucmd().arg("a/f").arg("b/f").arg("c").fails();

    #[cfg(not(unix))]
    assert!(
        result
            .stderr_str()
            .contains("will not overwrite just-created 'c\\f' with 'b/f'")
    );
    #[cfg(unix)]
    assert!(
        result
            .stderr_str()
            .contains("will not overwrite just-created 'c/f' with 'b/f'")
    );

    // a/f has been moved into c/f
    assert!(at.plus("c").join("f").exists());
    // b/f still exists
    assert!(at.plus("b").join("f").exists());
    // a/f no longer exists
    assert!(!at.plus("a").join("f").exists());
}

#[test]
fn test_mv_seen_multiple_files_to_directory() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.mkdir("a");
    at.mkdir("b");
    at.mkdir("c");
    at.write("a/f", "a");
    at.write("b/f", "b");
    at.write("b/g", "g");

    let result = ts.ucmd().arg("a/f").arg("b/f").arg("b/g").arg("c").fails();
    #[cfg(not(unix))]
    assert!(
        result
            .stderr_str()
            .contains("will not overwrite just-created 'c\\f' with 'b/f'")
    );
    #[cfg(unix)]
    assert!(
        result
            .stderr_str()
            .contains("will not overwrite just-created 'c/f' with 'b/f'")
    );

    assert!(!at.plus("a").join("f").exists());
    assert!(at.plus("b").join("f").exists());
    assert!(!at.plus("b").join("g").exists());
    assert!(at.plus("c").join("f").exists());
    assert!(at.plus("c").join("g").exists());
}

#[test]
fn test_mv_dir_into_file_where_both_are_files() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("a");
    at.touch("b");
    scene
        .ucmd()
        .arg("a/")
        .arg("b")
        .fails()
        .stderr_contains("mv: cannot stat 'a/': Not a directory");
}

#[test]
fn test_mv_dir_into_path_slash() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("a");
    scene.ucmd().arg("a").arg("e/").succeeds();
    assert!(at.dir_exists("e"));
    at.mkdir("b");
    at.mkdir("f");
    scene.ucmd().arg("b").arg("f/").succeeds();
    assert!(at.dir_exists("f/b"));
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "openbsd"))))]
#[test]
fn test_acl() {
    use std::process::Command;

    use uutests::util::compare_xattrs;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let path1 = "a";
    let path2 = "b";
    let file = "a/file";
    let file_target = "b/file";
    at.mkdir(path1);
    at.mkdir(path2);
    at.touch(file);

    let path = at.plus_as_string(file);
    // calling the command directly. xattr requires some dev packages to be installed
    // and it adds a complex dependency just for a test
    match Command::new("setfacl")
        .args(["-m", "group::rwx", path1])
        .status()
        .map(|status| status.code())
    {
        Ok(Some(0)) => {}
        Ok(_) => {
            println!("test skipped: setfacl failed");
            return;
        }
        Err(e) => {
            println!("test skipped: setfacl failed with {e}");
            return;
        }
    }

    scene.ucmd().arg(&path).arg(path2).succeeds();

    assert!(compare_xattrs(&file, &file_target));
}

#[test]
#[cfg(windows)]
fn test_move_should_not_fallback_to_copy() {
    use std::os::windows::fs::OpenOptionsExt;

    let (at, mut ucmd) = at_and_ucmd!();

    let locked_file = "a_file_is_locked";
    let locked_file_path = at.plus(locked_file);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .share_mode(
            uucore::windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ
                | uucore::windows_sys::Win32::Storage::FileSystem::FILE_SHARE_WRITE,
        )
        .open(locked_file_path);

    let target_file = "target_file";
    ucmd.arg(locked_file).arg(target_file).fails();

    assert!(at.file_exists(locked_file));
    assert!(!at.file_exists(target_file));

    drop(file);
}

#[test]
#[cfg(unix)]
fn test_move_should_not_fallback_to_copy() {
    let (at, mut ucmd) = at_and_ucmd!();

    let readonly_dir = "readonly_dir";
    let locked_file = "readonly_dir/a_file_is_locked";
    at.mkdir(readonly_dir);
    at.touch(locked_file);
    at.set_mode(readonly_dir, 0o555);

    let target_file = "target_file";
    ucmd.arg(locked_file).arg(target_file).fails();

    assert!(at.file_exists(locked_file));
    assert!(!at.file_exists(target_file));
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

#[cfg(target_os = "linux")]
mod inter_partition_copying {
    use std::fs::{read_to_string, set_permissions, write};
    use std::os::unix::fs::{PermissionsExt, symlink};
    use tempfile::TempDir;
    use uutests::util::TestScenario;
    use uutests::util_name;

    // Ensure that the copying code used in an inter-partition move unlinks the destination symlink.
    #[test]
    pub(crate) fn test_mv_unlinks_dest_symlink() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        // create a file in the current partition.
        at.write("src", "src contents");

        // create a folder in another partition.
        let other_fs_tempdir =
            TempDir::new_in("/dev/shm/").expect("Unable to create temp directory");

        // create a file inside that folder.
        let other_fs_file_path = other_fs_tempdir.path().join("other_fs_file");
        write(&other_fs_file_path, "other fs file contents")
            .expect("Unable to write to other_fs_file");

        // create a symlink to the file inside the same directory.
        let symlink_path = other_fs_tempdir.path().join("symlink_to_file");
        symlink(&other_fs_file_path, &symlink_path).expect("Unable to create symlink_to_file");

        // mv src to symlink in another partition
        scene
            .ucmd()
            .arg("src")
            .arg(symlink_path.to_str().unwrap())
            .succeeds();

        // make sure that src got removed.
        assert!(!at.file_exists("src"));

        // make sure symlink got unlinked
        assert!(!symlink_path.is_symlink());

        // make sure that file contents in other_fs_file didn't change.
        assert_eq!(
            read_to_string(&other_fs_file_path).expect("Unable to read other_fs_file"),
            "other fs file contents"
        );

        // make sure that src file contents got copied into new file created in symlink_path
        assert_eq!(
            read_to_string(&symlink_path).expect("Unable to read other_fs_file"),
            "src contents"
        );
    }

    // In an inter-partition move if unlinking the destination symlink fails, ensure
    // that it would output the proper error message.
    #[test]
    pub(crate) fn test_mv_unlinks_dest_symlink_error_message() {
        use uutests::util::TestScenario;
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        at.write("src", "src contents");

        let other_fs_tempdir =
            TempDir::new_in("/dev/shm/").expect("Unable to create temp directory");
        let other_fs_file_path = other_fs_tempdir.path().join("other_fs_file");
        write(&other_fs_file_path, "other fs file contents")
            .expect("Unable to write to other_fs_file");

        let symlink_path = other_fs_tempdir.path().join("symlink_to_file");
        symlink(&other_fs_file_path, &symlink_path).expect("Unable to create symlink_to_file");

        set_permissions(other_fs_tempdir.path(), PermissionsExt::from_mode(0o555))
            .expect("Unable to set permissions for temp directory");

        // mv src to symlink in another partition
        scene
            .ucmd()
            .arg("src")
            .arg(symlink_path.to_str().unwrap())
            .fails()
            .stderr_contains("inter-device move failed:")
            .stderr_contains("Permission denied");
    }

    // Test that hardlinks are preserved when moving files across partitions
    #[test]
    #[cfg(unix)]
    pub(crate) fn test_mv_preserves_hardlinks_across_partitions() {
        use std::fs::metadata;
        use std::os::unix::fs::MetadataExt;
        use tempfile::TempDir;
        use uutests::util::TestScenario;

        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        at.write("file1", "test content");
        at.hard_link("file1", "file2");

        let metadata1 = metadata(at.plus("file1")).expect("Failed to get metadata for file1");
        let metadata2 = metadata(at.plus("file2")).expect("Failed to get metadata for file2");
        assert_eq!(
            metadata1.ino(),
            metadata2.ino(),
            "Files should have same inode before move"
        );
        assert_eq!(
            metadata1.nlink(),
            2,
            "Files should have nlink=2 before move"
        );

        // Create a target directory in another partition (using /dev/shm which is typically tmpfs)
        let other_fs_tempdir = TempDir::new_in("/dev/shm/")
            .expect("Unable to create temp directory in /dev/shm - test requires tmpfs");

        scene
            .ucmd()
            .arg("file1")
            .arg("file2")
            .arg(other_fs_tempdir.path().to_str().unwrap())
            .succeeds();

        assert!(!at.file_exists("file1"), "file1 should not exist in source");
        assert!(!at.file_exists("file2"), "file2 should not exist in source");

        let moved_file1 = other_fs_tempdir.path().join("file1");
        let moved_file2 = other_fs_tempdir.path().join("file2");
        assert!(moved_file1.exists(), "file1 should exist in destination");
        assert!(moved_file2.exists(), "file2 should exist in destination");

        let moved_metadata1 =
            metadata(&moved_file1).expect("Failed to get metadata for moved file1");
        let moved_metadata2 =
            metadata(&moved_file2).expect("Failed to get metadata for moved file2");

        assert_eq!(
            moved_metadata1.ino(),
            moved_metadata2.ino(),
            "Files should have same inode after cross-partition move (hardlinks preserved)"
        );
        assert_eq!(
            moved_metadata1.nlink(),
            2,
            "Files should have nlink=2 after cross-partition move"
        );

        // Verify content is preserved
        assert_eq!(
            std::fs::read_to_string(&moved_file1).expect("Failed to read moved file1"),
            "test content"
        );
        assert_eq!(
            std::fs::read_to_string(&moved_file2).expect("Failed to read moved file2"),
            "test content"
        );
    }

    // Test that hardlinks are preserved even with multiple sets of hardlinked files
    #[test]
    #[cfg(unix)]
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::similar_names)]
    pub(crate) fn test_mv_preserves_multiple_hardlink_groups_across_partitions() {
        use std::fs::metadata;
        use std::os::unix::fs::MetadataExt;
        use tempfile::TempDir;
        use uutests::util::TestScenario;

        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        at.write("group1_file1", "content group 1");
        at.hard_link("group1_file1", "group1_file2");

        at.write("group2_file1", "content group 2");
        at.hard_link("group2_file1", "group2_file2");

        at.write("single_file", "single file content");

        let g1f1_meta = metadata(at.plus("group1_file1")).unwrap();
        let g1f2_meta = metadata(at.plus("group1_file2")).unwrap();
        let g2f1_meta = metadata(at.plus("group2_file1")).unwrap();
        let g2f2_meta = metadata(at.plus("group2_file2")).unwrap();
        let single_meta = metadata(at.plus("single_file")).unwrap();

        assert_eq!(
            g1f1_meta.ino(),
            g1f2_meta.ino(),
            "Group 1 files should have same inode"
        );
        assert_eq!(
            g2f1_meta.ino(),
            g2f2_meta.ino(),
            "Group 2 files should have same inode"
        );
        assert_ne!(
            g1f1_meta.ino(),
            g2f1_meta.ino(),
            "Different groups should have different inodes"
        );
        assert_eq!(single_meta.nlink(), 1, "Single file should have nlink=1");

        let other_fs_tempdir =
            TempDir::new_in("/dev/shm/").expect("Unable to create temp directory in /dev/shm");

        scene
            .ucmd()
            .arg("group1_file1")
            .arg("group1_file2")
            .arg("group2_file1")
            .arg("group2_file2")
            .arg("single_file")
            .arg(other_fs_tempdir.path().to_str().unwrap())
            .succeeds();

        // Verify hardlinks are preserved for both groups
        let moved_g1f1 = other_fs_tempdir.path().join("group1_file1");
        let moved_g1f2 = other_fs_tempdir.path().join("group1_file2");
        let moved_g2f1 = other_fs_tempdir.path().join("group2_file1");
        let moved_g2f2 = other_fs_tempdir.path().join("group2_file2");
        let moved_single = other_fs_tempdir.path().join("single_file");

        let moved_g1f1_meta = metadata(&moved_g1f1).unwrap();
        let moved_g1f2_meta = metadata(&moved_g1f2).unwrap();
        let moved_g2f1_meta = metadata(&moved_g2f1).unwrap();
        let moved_g2f2_meta = metadata(&moved_g2f2).unwrap();
        let moved_single_meta = metadata(&moved_single).unwrap();

        assert_eq!(
            moved_g1f1_meta.ino(),
            moved_g1f2_meta.ino(),
            "Group 1 files should still be hardlinked after move"
        );
        assert_eq!(
            moved_g1f1_meta.nlink(),
            2,
            "Group 1 files should have nlink=2"
        );

        assert_eq!(
            moved_g2f1_meta.ino(),
            moved_g2f2_meta.ino(),
            "Group 2 files should still be hardlinked after move"
        );
        assert_eq!(
            moved_g2f1_meta.nlink(),
            2,
            "Group 2 files should have nlink=2"
        );

        assert_ne!(
            moved_g1f1_meta.ino(),
            moved_g2f1_meta.ino(),
            "Different groups should still have different inodes"
        );

        assert_eq!(
            moved_single_meta.nlink(),
            1,
            "Single file should still have nlink=1"
        );

        assert_eq!(
            std::fs::read_to_string(&moved_g1f1).unwrap(),
            "content group 1"
        );
        assert_eq!(
            std::fs::read_to_string(&moved_g1f2).unwrap(),
            "content group 1"
        );
        assert_eq!(
            std::fs::read_to_string(&moved_g2f1).unwrap(),
            "content group 2"
        );
        assert_eq!(
            std::fs::read_to_string(&moved_g2f2).unwrap(),
            "content group 2"
        );
        assert_eq!(
            std::fs::read_to_string(&moved_single).unwrap(),
            "single file content"
        );
    }

    // Test the exact GNU test scenario: hardlinks within directories being moved
    #[test]
    #[cfg(unix)]
    pub(crate) fn test_mv_preserves_hardlinks_in_directories_across_partitions() {
        use std::fs::metadata;
        use std::os::unix::fs::MetadataExt;
        use tempfile::TempDir;
        use uutests::util::TestScenario;

        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        at.write("f", "file content");
        at.hard_link("f", "g");

        at.mkdir("a");
        at.mkdir("b");
        at.write("a/1", "directory file content");
        at.hard_link("a/1", "b/1");

        let f_meta = metadata(at.plus("f")).unwrap();
        let g_meta = metadata(at.plus("g")).unwrap();
        let a1_meta = metadata(at.plus("a/1")).unwrap();
        let b1_meta = metadata(at.plus("b/1")).unwrap();

        assert_eq!(
            f_meta.ino(),
            g_meta.ino(),
            "f and g should have same inode before move"
        );
        assert_eq!(f_meta.nlink(), 2, "f should have nlink=2 before move");
        assert_eq!(
            a1_meta.ino(),
            b1_meta.ino(),
            "a/1 and b/1 should have same inode before move"
        );
        assert_eq!(a1_meta.nlink(), 2, "a/1 should have nlink=2 before move");

        let other_fs_tempdir =
            TempDir::new_in("/dev/shm/").expect("Unable to create temp directory in /dev/shm");

        scene
            .ucmd()
            .arg("f")
            .arg("g")
            .arg(other_fs_tempdir.path().to_str().unwrap())
            .succeeds();

        scene
            .ucmd()
            .arg("a")
            .arg("b")
            .arg(other_fs_tempdir.path().to_str().unwrap())
            .succeeds();

        let moved_f = other_fs_tempdir.path().join("f");
        let moved_g = other_fs_tempdir.path().join("g");
        let moved_f_metadata = metadata(&moved_f).unwrap();
        let moved_second_file_metadata = metadata(&moved_g).unwrap();

        assert_eq!(
            moved_f_metadata.ino(),
            moved_second_file_metadata.ino(),
            "f and g should have same inode after cross-partition move"
        );
        assert_eq!(
            moved_f_metadata.nlink(),
            2,
            "f should have nlink=2 after move"
        );

        // Verify directory files' hardlinks are preserved (the main test)
        let moved_dir_a_file = other_fs_tempdir.path().join("a/1");
        let moved_dir_second_file = other_fs_tempdir.path().join("b/1");
        let moved_dir_a_file_metadata = metadata(&moved_dir_a_file).unwrap();
        let moved_dir_second_file_metadata = metadata(&moved_dir_second_file).unwrap();

        assert_eq!(
            moved_dir_a_file_metadata.ino(),
            moved_dir_second_file_metadata.ino(),
            "a/1 and b/1 should have same inode after cross-partition directory move (hardlinks preserved)"
        );
        assert_eq!(
            moved_dir_a_file_metadata.nlink(),
            2,
            "a/1 should have nlink=2 after move"
        );

        assert_eq!(std::fs::read_to_string(&moved_f).unwrap(), "file content");
        assert_eq!(std::fs::read_to_string(&moved_g).unwrap(), "file content");
        assert_eq!(
            std::fs::read_to_string(&moved_dir_a_file).unwrap(),
            "directory file content"
        );
        assert_eq!(
            std::fs::read_to_string(&moved_dir_second_file).unwrap(),
            "directory file content"
        );
    }

    // Test complex scenario with multiple hardlink groups across nested directories
    #[test]
    #[cfg(unix)]
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::similar_names)]
    pub(crate) fn test_mv_preserves_complex_hardlinks_across_nested_directories() {
        use std::fs::metadata;
        use std::os::unix::fs::MetadataExt;
        use tempfile::TempDir;
        use uutests::util::TestScenario;

        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        at.mkdir("dir1");
        at.mkdir("dir1/subdir1");
        at.mkdir("dir1/subdir2");
        at.mkdir("dir2");
        at.mkdir("dir2/subdir1");

        at.write("dir1/subdir1/file_a", "content A");
        at.hard_link("dir1/subdir1/file_a", "dir1/subdir2/file_a_link1");
        at.hard_link("dir1/subdir1/file_a", "dir2/subdir1/file_a_link2");

        at.write("dir1/file_b", "content B");
        at.hard_link("dir1/file_b", "dir2/file_b_link");

        at.write("dir1/subdir1/nested_file", "nested content");
        at.hard_link("dir1/subdir1/nested_file", "dir1/subdir2/nested_file_link");

        let orig_file_a_metadata = metadata(at.plus("dir1/subdir1/file_a")).unwrap();
        let orig_file_a_link1_metadata = metadata(at.plus("dir1/subdir2/file_a_link1")).unwrap();
        let orig_file_a_link2_metadata = metadata(at.plus("dir2/subdir1/file_a_link2")).unwrap();

        assert_eq!(orig_file_a_metadata.ino(), orig_file_a_link1_metadata.ino());
        assert_eq!(orig_file_a_metadata.ino(), orig_file_a_link2_metadata.ino());
        assert_eq!(
            orig_file_a_metadata.nlink(),
            3,
            "file_a group should have nlink=3"
        );

        let orig_file_b_metadata = metadata(at.plus("dir1/file_b")).unwrap();
        let orig_file_b_link_metadata = metadata(at.plus("dir2/file_b_link")).unwrap();
        assert_eq!(orig_file_b_metadata.ino(), orig_file_b_link_metadata.ino());
        assert_eq!(
            orig_file_b_metadata.nlink(),
            2,
            "file_b group should have nlink=2"
        );

        let nested_meta = metadata(at.plus("dir1/subdir1/nested_file")).unwrap();
        let nested_link_meta = metadata(at.plus("dir1/subdir2/nested_file_link")).unwrap();
        assert_eq!(nested_meta.ino(), nested_link_meta.ino());
        assert_eq!(
            nested_meta.nlink(),
            2,
            "nested file group should have nlink=2"
        );

        let other_fs_tempdir =
            TempDir::new_in("/dev/shm/").expect("Unable to create temp directory in /dev/shm");

        scene
            .ucmd()
            .arg("dir1")
            .arg("dir2")
            .arg(other_fs_tempdir.path().to_str().unwrap())
            .succeeds();

        let moved_file_a = other_fs_tempdir.path().join("dir1/subdir1/file_a");
        let moved_file_a_link1 = other_fs_tempdir.path().join("dir1/subdir2/file_a_link1");
        let moved_file_a_link2 = other_fs_tempdir.path().join("dir2/subdir1/file_a_link2");

        let final_file_a_metadata = metadata(&moved_file_a).unwrap();
        let final_file_a_link1_metadata = metadata(&moved_file_a_link1).unwrap();
        let final_file_a_link2_metadata = metadata(&moved_file_a_link2).unwrap();

        assert_eq!(
            final_file_a_metadata.ino(),
            final_file_a_link1_metadata.ino(),
            "file_a hardlinks should be preserved"
        );
        assert_eq!(
            final_file_a_metadata.ino(),
            final_file_a_link2_metadata.ino(),
            "file_a hardlinks should be preserved across directories"
        );
        assert_eq!(
            final_file_a_metadata.nlink(),
            3,
            "file_a group should still have nlink=3"
        );

        let moved_file_b = other_fs_tempdir.path().join("dir1/file_b");
        let moved_file_b_hardlink = other_fs_tempdir.path().join("dir2/file_b_link");
        let final_file_b_metadata = metadata(&moved_file_b).unwrap();
        let final_file_b_hardlink_metadata = metadata(&moved_file_b_hardlink).unwrap();

        assert_eq!(
            final_file_b_metadata.ino(),
            final_file_b_hardlink_metadata.ino(),
            "file_b hardlinks should be preserved"
        );
        assert_eq!(
            final_file_b_metadata.nlink(),
            2,
            "file_b group should still have nlink=2"
        );

        let moved_nested = other_fs_tempdir.path().join("dir1/subdir1/nested_file");
        let moved_nested_link = other_fs_tempdir
            .path()
            .join("dir1/subdir2/nested_file_link");
        let moved_nested_meta = metadata(&moved_nested).unwrap();
        let moved_nested_link_meta = metadata(&moved_nested_link).unwrap();

        assert_eq!(
            moved_nested_meta.ino(),
            moved_nested_link_meta.ino(),
            "nested file hardlinks should be preserved"
        );
        assert_eq!(
            moved_nested_meta.nlink(),
            2,
            "nested file group should still have nlink=2"
        );

        assert_eq!(std::fs::read_to_string(&moved_file_a).unwrap(), "content A");
        assert_eq!(
            std::fs::read_to_string(&moved_file_a_link1).unwrap(),
            "content A"
        );
        assert_eq!(
            std::fs::read_to_string(&moved_file_a_link2).unwrap(),
            "content A"
        );
        assert_eq!(std::fs::read_to_string(&moved_file_b).unwrap(), "content B");
        assert_eq!(
            std::fs::read_to_string(&moved_file_b_hardlink).unwrap(),
            "content B"
        );
        assert_eq!(
            std::fs::read_to_string(&moved_nested).unwrap(),
            "nested content"
        );
        assert_eq!(
            std::fs::read_to_string(&moved_nested_link).unwrap(),
            "nested content"
        );
    }
}

#[test]
fn test_mv_error_msg_with_multiple_sources_that_does_not_exist() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("d");
    scene
        .ucmd()
        .arg("a")
        .arg("b/")
        .arg("d")
        .fails()
        .stderr_contains("mv: cannot stat 'a': No such file or directory")
        .stderr_contains("mv: cannot stat 'b/': No such file or directory");
}

// Tests for hardlink preservation (now always enabled)
#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_mv_hardlink_preservation() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.write("file1", "test content");
    at.hard_link("file1", "file2");
    at.mkdir("target");

    ucmd.arg("file1")
        .arg("file2")
        .arg("target")
        .succeeds()
        .no_stderr();

    assert!(at.file_exists("target/file1"));
    assert!(at.file_exists("target/file2"));
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_mv_hardlink_progress_indication() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.write("file1", "content1");
    at.write("file2", "content2");
    at.hard_link("file1", "file1_link");

    at.mkdir("target");

    // Test with progress bar and verbose mode
    ucmd.arg("--progress")
        .arg("--verbose")
        .arg("file1")
        .arg("file1_link")
        .arg("file2")
        .arg("target")
        .succeeds();

    // Verify all files were moved
    assert!(at.file_exists("target/file1"));
    assert!(at.file_exists("target/file1_link"));
    assert!(at.file_exists("target/file2"));
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_mv_mixed_hardlinks_and_regular_files() {
    use std::fs::metadata;
    use std::os::unix::fs::MetadataExt;

    let (at, mut ucmd) = at_and_ucmd!();

    // Create a mix of hardlinked and regular files
    at.write("hardlink1", "hardlink content");
    at.hard_link("hardlink1", "hardlink2");
    at.write("regular1", "regular content");
    at.write("regular2", "regular content 2");

    at.mkdir("target");

    // Move all files (hardlinks automatically preserved)
    ucmd.arg("hardlink1")
        .arg("hardlink2")
        .arg("regular1")
        .arg("regular2")
        .arg("target")
        .succeeds();

    // Verify all files moved
    assert!(at.file_exists("target/hardlink1"));
    assert!(at.file_exists("target/hardlink2"));
    assert!(at.file_exists("target/regular1"));
    assert!(at.file_exists("target/regular2"));

    // Verify hardlinks are preserved (on same filesystem)
    let h1_meta = metadata(at.plus("target/hardlink1")).unwrap();
    let h2_meta = metadata(at.plus("target/hardlink2")).unwrap();
    let r1_meta = metadata(at.plus("target/regular1")).unwrap();
    let r2_meta = metadata(at.plus("target/regular2")).unwrap();

    // Hardlinked files should have same inode if on same filesystem
    if h1_meta.dev() == h2_meta.dev() {
        assert_eq!(h1_meta.ino(), h2_meta.ino());
    }

    // Regular files should have different inodes
    assert_ne!(r1_meta.ino(), r2_meta.ino());
}

#[cfg(not(windows))]
#[ignore = "requires access to a different filesystem"]
#[test]
fn test_special_file_different_filesystem() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkfifo("f");
    // TODO Use `TestScenario::mount_temp_fs()` for this purpose and
    // un-ignore this test.
    std::fs::create_dir("/dev/shm/tmp").unwrap();
    ucmd.args(&["f", "/dev/shm/tmp"]).succeeds().no_output();
    assert!(!at.file_exists("f"));
    assert!(Path::new("/dev/shm/tmp/f").exists());
    std::fs::remove_dir_all("/dev/shm/tmp").unwrap();
}

/// Test cross-device move with permission denied error
/// This test mimics the scenario from the GNU part-fail test where
/// a cross-device move fails due to permission errors when removing the target file
#[test]
#[cfg(target_os = "linux")]
fn test_mv_cross_device_permission_denied() {
    use std::fs::{set_permissions, write};
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;
    use uutests::util::TestScenario;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("k", "source content");

    let other_fs_tempdir =
        TempDir::new_in("/dev/shm/").expect("Unable to create temp directory in /dev/shm");

    let target_file_path = other_fs_tempdir.path().join("k");
    write(&target_file_path, "target content").expect("Unable to write target file");

    // Remove write permissions from the directory to cause permission denied
    set_permissions(other_fs_tempdir.path(), PermissionsExt::from_mode(0o555))
        .expect("Unable to set directory permissions");

    // Attempt to move file to the other filesystem
    // This should fail with a permission denied error
    let result = scene
        .ucmd()
        .arg("-f")
        .arg("k")
        .arg(target_file_path.to_str().unwrap())
        .fails();

    // Check that it contains permission denied and references the file
    // The exact format may vary but should contain these key elements
    let stderr = result.stderr_str();
    assert!(stderr.contains("Permission denied") || stderr.contains("permission denied"));

    set_permissions(other_fs_tempdir.path(), PermissionsExt::from_mode(0o755))
        .expect("Unable to restore directory permissions");
}

#[test]
#[cfg(feature = "selinux")]
fn test_mv_selinux_context() {
    let test_cases = [
        ("-Z", None),
        (
            "--context=unconfined_u:object_r:user_tmp_t:s0",
            Some("unconfined_u"),
        ),
    ];

    for (arg, expected_context) in test_cases {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        let src = "source.txt";
        let dest = "dest.txt";

        at.touch(src);

        let mut cmd = scene.ucmd();
        cmd.arg(arg);

        let result = cmd
            .arg(at.plus_as_string(src))
            .arg(at.plus_as_string(dest))
            .run();

        // Skip test if SELinux is not enabled
        if result
            .stderr_str()
            .contains("SELinux is not enabled on this system")
        {
            println!("Skipping SELinux test: SELinux is not enabled");
            return;
        }

        result.success();
        assert!(at.file_exists(dest));
        assert!(!at.file_exists(src));

        // Verify SELinux context was set using getfattr
        let context_value = get_getfattr_output(&at.plus_as_string(dest));
        if !context_value.is_empty() {
            if let Some(expected) = expected_context {
                assert!(
                    context_value.contains(expected),
                    "Expected context to contain '{expected}', got: {context_value}"
                );
            }
        }

        // Clean up files
        let _ = std::fs::remove_file(at.plus_as_string(dest));
        let _ = std::fs::remove_file(at.plus_as_string(src));
    }
}

#[test]
fn test_mv_error_usage_display_missing_arg() {
    new_ucmd!()
        .arg("--target-directory=.")
        .fails()
        .code_is(1)
        .stderr_contains("error: the following required arguments were not provided:")
        .stderr_contains("<files>...")
        .stderr_contains("Usage: mv [OPTION]... [-T] SOURCE DEST")
        .stderr_contains("For more information, try '--help'.");
}

#[test]
fn test_mv_error_usage_display_too_few() {
    new_ucmd!()
        .arg("file1")
        .fails()
        .code_is(1)
        .stderr_contains("requires at least 2 values, but only 1 was provided")
        .stderr_contains("Usage: mv [OPTION]... [-T] SOURCE DEST")
        .stderr_contains("For more information, try '--help'.");
}

#[test]
#[cfg(target_os = "linux")]
fn test_mv_verbose_directory_recursive() {
    use tempfile::TempDir;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("mv-dir");
    at.mkdir("mv-dir/a");
    at.mkdir("mv-dir/a/b");
    at.mkdir("mv-dir/a/b/c");
    at.mkdir("mv-dir/d");
    at.mkdir("mv-dir/d/e");
    at.mkdir("mv-dir/d/e/f");
    at.touch("mv-dir/a/b/c/file1");
    at.touch("mv-dir/d/e/f/file2");

    // Force cross-filesystem move using /dev/shm (tmpfs)
    let target_dir =
        TempDir::new_in("/dev/shm/").expect("Unable to create temp directory in /dev/shm");
    let target_path = target_dir.path().to_str().unwrap();

    let result = scene
        .ucmd()
        .arg("--verbose")
        .arg("mv-dir")
        .arg(target_path)
        .succeeds();

    // Check that the directory structure was moved
    assert!(!at.dir_exists("mv-dir"));
    assert!(target_dir.path().join("mv-dir").exists());
    assert!(target_dir.path().join("mv-dir/a").exists());
    assert!(target_dir.path().join("mv-dir/a/b").exists());
    assert!(target_dir.path().join("mv-dir/a/b/c").exists());
    assert!(target_dir.path().join("mv-dir/d").exists());
    assert!(target_dir.path().join("mv-dir/d/e").exists());
    assert!(target_dir.path().join("mv-dir/d/e/f").exists());
    assert!(target_dir.path().join("mv-dir/a/b/c/file1").exists());
    assert!(target_dir.path().join("mv-dir/d/e/f/file2").exists());

    let stdout = result.stdout_str();

    // With cross-filesystem move, we MUST see recursive verbose output
    assert!(stdout.contains("'mv-dir/a' -> "));
    assert!(stdout.contains("'mv-dir/a/b' -> "));
    assert!(stdout.contains("'mv-dir/a/b/c' -> "));
    assert!(stdout.contains("'mv-dir/a/b/c/file1' -> "));
    assert!(stdout.contains("'mv-dir/d' -> "));
    assert!(stdout.contains("'mv-dir/d/e' -> "));
    assert!(stdout.contains("'mv-dir/d/e/f' -> "));
    assert!(stdout.contains("'mv-dir/d/e/f/file2' -> "));
}

#[cfg(unix)]
#[test]
fn test_mv_prompt_unwriteable_file_when_using_tty() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("source");
    at.touch("target");
    at.set_mode("target", 0o000);

    ucmd.arg("source")
        .arg("target")
        .terminal_sim_stdio(TerminalSimulation {
            stdin: true,
            stdout: false,
            stderr: false,
            ..Default::default()
        })
        .pipe_in("n\n")
        .fails()
        .stderr_contains("replace 'target', overriding mode 0000");

    assert!(at.file_exists("source"));
}

#[cfg(unix)]
#[test]
fn test_mv_force_no_prompt_unwriteable_file() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("source_f");
    at.touch("target_f");
    at.set_mode("target_f", 0o000);

    ucmd.arg("-f")
        .arg("source_f")
        .arg("target_f")
        .terminal_sim_stdio(TerminalSimulation {
            stdin: true,
            stdout: false,
            stderr: false,
            ..Default::default()
        })
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists("source_f"));
    assert!(at.file_exists("target_f"));
}

#[cfg(unix)]
#[test]
fn test_mv_no_prompt_unwriteable_file_with_no_tty() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("source_notty");
    at.touch("target_notty");
    at.set_mode("target_notty", 0o000);

    ucmd.arg("source_notty")
        .arg("target_notty")
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists("source_notty"));
    assert!(at.file_exists("target_notty"));
}
