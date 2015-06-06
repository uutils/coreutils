#![feature(fs_time, path_ext)]

extern crate libc;
extern crate time;

use std::fs::{self, PathExt};
use std::path::Path;
use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./mv";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_mv_rename_dir() {
    let dir1 = "test_mv_rename_dir";
    let dir2 = "test_mv_rename_dir2";

    mkdir(dir1);

    let result = run(Command::new(PROGNAME).arg(dir1).arg(dir2));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(Path::new(dir2).is_dir());
}

#[test]
fn test_mv_rename_file() {
    let file1 = "test_mv_rename_file";
    let file2 = "test_mv_rename_file2";

    touch(file1);

    let result = run(Command::new(PROGNAME).arg(file1).arg(file2));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(Path::new(file2).is_file());
}

#[test]
fn test_mv_move_file_into_dir() {
    let dir = "test_mv_move_file_into_dir_dir";
    let file = "test_mv_move_file_into_dir_file";

    mkdir(dir);
    touch(file);

    let result = run(Command::new(PROGNAME).arg(file).arg(dir));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(Path::new(&format!("{}/{}", dir, file)).is_file());
}

#[test]
fn test_mv_multiple_files() {
    let target_dir = "test_mv_multiple_files_dir";
    let file_a = "test_mv_multiple_file_a";
    let file_b = "test_mv_multiple_file_b";

    mkdir(target_dir);
    touch(file_a);
    touch(file_b);

    let result = run(Command::new(PROGNAME).arg(file_a).arg(file_b).arg(target_dir));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(Path::new(&format!("{}/{}", target_dir, file_a)).is_file());
    assert!(Path::new(&format!("{}/{}", target_dir, file_b)).is_file());
}

#[test]
fn test_mv_multiple_folders() {
    let target_dir = "test_mv_multiple_dirs_dir";
    let dir_a = "test_mv_multiple_dir_a";
    let dir_b = "test_mv_multiple_dir_b";

    mkdir(target_dir);
    mkdir(dir_a);
    mkdir(dir_b);

    let result = run(Command::new(PROGNAME).arg(dir_a).arg(dir_b).arg(target_dir));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(Path::new(&format!("{}/{}", target_dir, dir_a)).is_dir());
    assert!(Path::new(&format!("{}/{}", target_dir, dir_b)).is_dir());
}

#[test]
fn test_mv_interactive() {
    let file_a = "test_mv_interactive_file_a";
    let file_b = "test_mv_interactive_file_b";

    touch(file_a);
    touch(file_b);


    let result1 = run_piped_stdin(Command::new(PROGNAME).arg("-i").arg(file_a).arg(file_b), b"n");

    assert_empty_stderr!(result1);
    assert!(result1.success);

    assert!(Path::new(file_a).is_file());
    assert!(Path::new(file_b).is_file());


    let result2 = run_piped_stdin(Command::new(PROGNAME).arg("-i").arg(file_a).arg(file_b), b"Yesh");

    assert_empty_stderr!(result2);
    assert!(result2.success);

    assert!(!Path::new(file_a).is_file());
    assert!(Path::new(file_b).is_file());
}

#[test]
fn test_mv_no_clobber() {
    let file_a = "test_mv_no_clobber_file_a";
    let file_b = "test_mv_no_clobber_file_b";

    touch(file_a);
    touch(file_b);

    let result = run(Command::new(PROGNAME).arg("-n").arg(file_a).arg(file_b));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(Path::new(file_a).is_file());
    assert!(Path::new(file_b).is_file());
}

#[test]
fn test_mv_replace_file() {
    let file_a = "test_mv_replace_file_a";
    let file_b = "test_mv_replace_file_b";

    touch(file_a);
    touch(file_b);

    let result = run(Command::new(PROGNAME).arg(file_a).arg(file_b));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(file_a).is_file());
    assert!(Path::new(file_b).is_file());
}

#[test]
fn test_mv_force_replace_file() {
    let file_a = "test_mv_force_replace_file_a";
    let file_b = "test_mv_force_replace_file_b";

    touch(file_a);
    touch(file_b);

    let result = run(Command::new(PROGNAME).arg("--force").arg(file_a).arg(file_b));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(file_a).is_file());
    assert!(Path::new(file_b).is_file());
}

#[test]
fn test_mv_simple_backup() {
    let file_a = "test_mv_simple_backup_file_a";
    let file_b = "test_mv_simple_backup_file_b";

    touch(file_a);
    touch(file_b);
    let result = run(Command::new(PROGNAME).arg("-b").arg(file_a).arg(file_b));

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(file_a).is_file());
    assert!(Path::new(file_b).is_file());
    assert!(Path::new(&format!("{}~", file_b)).is_file());
}

#[test]
fn test_mv_custom_backup_suffix() {
    let file_a = "test_mv_custom_backup_suffix_file_a";
    let file_b = "test_mv_custom_backup_suffix_file_b";
    let suffix = "super-suffix-of-the-century";

    touch(file_a);
    touch(file_b);
    let result = run(Command::new(PROGNAME)
            .arg("-b").arg(format!("--suffix={}", suffix))
            .arg(file_a).arg(file_b));

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(file_a).is_file());
    assert!(Path::new(file_b).is_file());
    assert!(Path::new(&format!("{}{}", file_b, suffix)).is_file());
}

#[test]
fn test_mv_backup_numbering() {
    let file_a = "test_mv_backup_numbering_file_a";
    let file_b = "test_mv_backup_numbering_file_b";

    touch(file_a);
    touch(file_b);
    let result = run(Command::new(PROGNAME).arg("--backup=t").arg(file_a).arg(file_b));

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(file_a).is_file());
    assert!(Path::new(file_b).is_file());
    assert!(Path::new(&format!("{}.~1~", file_b)).is_file());
}

#[test]
fn test_mv_existing_backup() {
    let file_a = "test_mv_existing_backup_file_a";
    let file_b = "test_mv_existing_backup_file_b";
    let file_b_backup = "test_mv_existing_backup_file_b.~1~";
    let resulting_backup = "test_mv_existing_backup_file_b.~2~";

    touch(file_a);
    touch(file_b);
    touch(file_b_backup);
    let result = run(Command::new(PROGNAME).arg("--backup=nil").arg(file_a).arg(file_b));

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(file_a).is_file());
    assert!(Path::new(file_b).is_file());
    assert!(Path::new(file_b_backup).is_file());
    assert!(Path::new(resulting_backup).is_file());
}

#[test]
fn test_mv_update_option() {
    let file_a = "test_mv_update_option_file_a";
    let file_b = "test_mv_update_option_file_b";

    touch(file_a);
    touch(file_b);
    let now = (time::get_time().sec * 1000) as u64;
    fs::set_file_times(Path::new(file_a), now, now).unwrap();
    fs::set_file_times(Path::new(file_b), now, now+3600).unwrap();

    let result1 = run(Command::new(PROGNAME).arg("--update").arg(file_a).arg(file_b));

    assert_empty_stderr!(result1);
    assert!(result1.success);

    assert!(Path::new(file_a).is_file());
    assert!(Path::new(file_b).is_file());

    let result2 = run(Command::new(PROGNAME).arg("--update").arg(file_b).arg(file_a));

    assert_empty_stderr!(result2);
    assert!(result2.success);

    assert!(Path::new(file_a).is_file());
    assert!(!Path::new(file_b).is_file());
}

#[test]
fn test_mv_target_dir() {
    let dir = "test_mv_target_dir_dir";
    let file_a = "test_mv_target_dir_file_a";
    let file_b = "test_mv_target_dir_file_b";

    touch(file_a);
    touch(file_b);
    mkdir(dir);
    let result = run(Command::new(PROGNAME).arg("-t").arg(dir).arg(file_a).arg(file_b));

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(file_a).is_file());
    assert!(!Path::new(file_b).is_file());
    assert!(Path::new(&format!("{}/{}", dir, file_a)).is_file());
    assert!(Path::new(&format!("{}/{}", dir, file_b)).is_file());
}

#[test]
fn test_mv_overwrite_dir() {
    let dir_a = "test_mv_overwrite_dir_a";
    let dir_b = "test_mv_overwrite_dir_b";

    mkdir(dir_a);
    mkdir(dir_b);
    let result = run(Command::new(PROGNAME).arg("-T").arg(dir_a).arg(dir_b));

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(dir_a).is_dir());
    assert!(Path::new(dir_b).is_dir());
}

#[test]
fn test_mv_overwrite_nonempty_dir() {
    let dir_a = "test_mv_overwrite_nonempty_dir_a";
    let dir_b = "test_mv_overwrite_nonempty_dir_b";
    let dummy = "test_mv_overwrite_nonempty_dir_b/file";

    mkdir(dir_a);
    mkdir(dir_b);
    touch(dummy);
    let result = run(Command::new(PROGNAME).arg("-vT").arg(dir_a).arg(dir_b));

    // Not same error as GNU; the error message is a rust builtin
    // TODO: test (and implement) correct error message (or at least decide whether to do so)
    // Current: "mv: error: couldn't rename path (Directory not empty; from=a; to=b)"
    // GNU:     "mv: cannot move ‘a’ to ‘b’: Directory not empty"
    assert!(result.stderr.len() > 0);

    // Verbose output for the move should not be shown on failure
    assert!(result.stdout.len() == 0);

    assert!(!result.success);
    assert!(Path::new(dir_a).is_dir());
    assert!(Path::new(dir_b).is_dir());
}

#[test]
fn test_mv_backup_dir() {
    let dir_a = "test_mv_backup_dir_dir_a";
    let dir_b = "test_mv_backup_dir_dir_b";

    mkdir(dir_a);
    mkdir(dir_b);
    let result = run(Command::new(PROGNAME).arg("-vbT").arg(dir_a).arg(dir_b));

    assert_empty_stderr!(result);
    assert_eq!(result.stdout,
        format!("‘{}’ -> ‘{}’ (backup: ‘{}~’)\n", dir_a, dir_b, dir_b));
    assert!(result.success);

    assert!(!Path::new(dir_a).is_dir());
    assert!(Path::new(dir_b).is_dir());
    assert!(Path::new(&format!("{}~", dir_b)).is_dir());
}

#[test]
fn test_mv_errors() {
    let dir = "test_mv_errors_dir";
    let file_a = "test_mv_errors_file_a";
    let file_b = "test_mv_errors_file_b";
    mkdir(dir);
    touch(file_a);
    touch(file_b);

    // $ mv -T -t a b
    // mv: cannot combine --target-directory (-t) and --no-target-directory (-T)
    let result = run(Command::new(PROGNAME).arg("-T").arg("-t").arg(dir).arg(file_a).arg(file_b));
    assert_eq!(result.stderr,
        "mv: error: cannot combine --target-directory (-t) and --no-target-directory (-T)\n");
    assert!(!result.success);


    // $ touch file && mkdir dir
    // $ mv -T file dir
    // err == mv: cannot overwrite directory ‘dir’ with non-directory
    let result = run(Command::new(PROGNAME).arg("-T").arg(file_a).arg(dir));
    assert_eq!(result.stderr,
        format!("mv: error: cannot overwrite directory ‘{}’ with non-directory\n", dir));
    assert!(!result.success);

    // $ mkdir dir && touch file
    // $ mv dir file
    // err == mv: cannot overwrite non-directory ‘file’ with directory ‘dir’
    let result = run(Command::new(PROGNAME).arg(dir).arg(file_a));
    assert!(result.stderr.len() > 0);
    assert!(!result.success);
}

#[test]
fn test_mv_verbose() {
    let dir = "test_mv_verbose_dir";
    let file_a = "test_mv_verbose_file_a";
    let file_b = "test_mv_verbose_file_b";
    mkdir(dir);
    touch(file_a);
    touch(file_b);

    let result = run(Command::new(PROGNAME).arg("-v").arg(file_a).arg(file_b));
    assert_empty_stderr!(result);
    assert_eq!(result.stdout,
        format!("‘{}’ -> ‘{}’\n", file_a, file_b));
    assert!(result.success);


    touch(file_a);
    let result = run(Command::new(PROGNAME).arg("-vb").arg(file_a).arg(file_b));
    assert_empty_stderr!(result);
    assert_eq!(result.stdout,
        format!("‘{}’ -> ‘{}’ (backup: ‘{}~’)\n", file_a, file_b, file_b));
    assert!(result.success);
}

// Todo:

// $ touch a b
// $ chmod -w b
// $ ll
// total 0
// -rw-rw-r-- 1 user user 0 okt 25 11:21 a
// -r--r--r-- 1 user user 0 okt 25 11:21 b
// $
// $ mv -v a b
// mv: try to overwrite ‘b’, overriding mode 0444 (r--r--r--)? y
// ‘a’ -> ‘b’
