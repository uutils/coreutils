#[macro_use]
mod common;

extern crate libc;
extern crate time;
extern crate kernel32;
extern crate winapi;
extern crate filetime;

use filetime::*;
use common::util::*;

static UTIL_NAME: &'static str = "mv";

#[test]
fn test_mv_rename_dir() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir1 = "test_mv_rename_dir";
    let dir2 = "test_mv_rename_dir2";

    at.mkdir(dir1);

    let result = ucmd.arg(dir1).arg(dir2).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(at.dir_exists(dir2));
}

#[test]
fn test_mv_rename_file() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let file1 = "test_mv_rename_file";
    let file2 = "test_mv_rename_file2";

    at.touch(file1);

    let result = ucmd.arg(file1).arg(file2).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(at.file_exists(file2));
}

#[test]
fn test_mv_move_file_into_dir() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir = "test_mv_move_file_into_dir_dir";
    let file = "test_mv_move_file_into_dir_file";

    at.mkdir(dir);
    at.touch(file);

    let result = ucmd.arg(file).arg(dir).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(at.file_exists(&format!("{}/{}", dir, file)));
}

#[test]
fn test_mv_strip_slashes() {
    let ts = TestSet::new(UTIL_NAME);
    let at = &ts.fixtures;
    let dir = "test_mv_strip_slashes_dir";
    let file = "test_mv_strip_slashes_file";
    let mut source = file.to_owned();
    source.push_str("/");

    at.mkdir(dir);
    at.touch(file);

    let result = ts.util_cmd().arg(&source).arg(dir).run();
    assert!(!result.success);

    assert!(!at.file_exists(&format!("{}/{}", dir, file)));

    let result = ts.util_cmd().arg("--strip-trailing-slashes").arg(source).arg(dir).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(at.file_exists(&format!("{}/{}", dir, file)));
}

#[test]
fn test_mv_multiple_files() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let target_dir = "test_mv_multiple_files_dir";
    let file_a = "test_mv_multiple_file_a";
    let file_b = "test_mv_multiple_file_b";

    at.mkdir(target_dir);
    at.touch(file_a);
    at.touch(file_b);

    let result = ucmd.arg(file_a).arg(file_b).arg(target_dir).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(at.file_exists(&format!("{}/{}", target_dir, file_a)));
    assert!(at.file_exists(&format!("{}/{}", target_dir, file_b)));
}

#[test]
fn test_mv_multiple_folders() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let target_dir = "test_mv_multiple_dirs_dir";
    let dir_a = "test_mv_multiple_dir_a";
    let dir_b = "test_mv_multiple_dir_b";

    at.mkdir(target_dir);
    at.mkdir(dir_a);
    at.mkdir(dir_b);

    let result = ucmd.arg(dir_a).arg(dir_b).arg(target_dir).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(at.dir_exists(&format!("{}/{}", target_dir, dir_a)));
    assert!(at.dir_exists(&format!("{}/{}", target_dir, dir_b)));
}

#[test]
fn test_mv_interactive() {
    let ts = TestSet::new(UTIL_NAME);
    let at = &ts.fixtures;
    let file_a = "test_mv_interactive_file_a";
    let file_b = "test_mv_interactive_file_b";

    at.touch(file_a);
    at.touch(file_b);


    let result1 = ts.util_cmd().arg("-i").arg(file_a).arg(file_b).run_piped_stdin("n");

    assert_empty_stderr!(result1);
    assert!(result1.success);

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));


    let result2 = ts.util_cmd().arg("-i").arg(file_a).arg(file_b).run_piped_stdin("Yesh");

    assert_empty_stderr!(result2);
    assert!(result2.success);

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
}

#[test]
fn test_mv_no_clobber() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let file_a = "test_mv_no_clobber_file_a";
    let file_b = "test_mv_no_clobber_file_b";

    at.touch(file_a);
    at.touch(file_b);

    let result = ucmd.arg("-n").arg(file_a).arg(file_b).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));
}

#[test]
fn test_mv_replace_file() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let file_a = "test_mv_replace_file_a";
    let file_b = "test_mv_replace_file_b";

    at.touch(file_a);
    at.touch(file_b);

    let result = ucmd.arg(file_a).arg(file_b).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
}

#[test]
fn test_mv_force_replace_file() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let file_a = "test_mv_force_replace_file_a";
    let file_b = "test_mv_force_replace_file_b";

    at.touch(file_a);
    at.touch(file_b);

    let result = ucmd.arg("--force").arg(file_a).arg(file_b).run();
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
}

#[test]
fn test_mv_simple_backup() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let file_a = "test_mv_simple_backup_file_a";
    let file_b = "test_mv_simple_backup_file_b";

    at.touch(file_a);
    at.touch(file_b);
    let result = ucmd.arg("-b").arg(file_a).arg(file_b).run();

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(&format!("{}~", file_b)));
}

#[test]
fn test_mv_custom_backup_suffix() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let file_a = "test_mv_custom_backup_suffix_file_a";
    let file_b = "test_mv_custom_backup_suffix_file_b";
    let suffix = "super-suffix-of-the-century";

    at.touch(file_a);
    at.touch(file_b);
    let result = ucmd.arg("-b")
                     .arg(format!("--suffix={}", suffix))
                     .arg(file_a)
                     .arg(file_b)
                     .run();

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(&format!("{}{}", file_b, suffix)));
}

#[test]
fn test_mv_backup_numbering() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let file_a = "test_mv_backup_numbering_file_a";
    let file_b = "test_mv_backup_numbering_file_b";

    at.touch(file_a);
    at.touch(file_b);
    let result = ucmd.arg("--backup=t").arg(file_a).arg(file_b).run();

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(&format!("{}.~1~", file_b)));
}

#[test]
fn test_mv_existing_backup() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let file_a = "test_mv_existing_backup_file_a";
    let file_b = "test_mv_existing_backup_file_b";
    let file_b_backup = "test_mv_existing_backup_file_b.~1~";
    let resulting_backup = "test_mv_existing_backup_file_b.~2~";

    at.touch(file_a);
    at.touch(file_b);
    at.touch(file_b_backup);
    let result = ucmd.arg("--backup=nil").arg(file_a).arg(file_b).run();

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
    assert!(at.file_exists(file_b_backup));
    assert!(at.file_exists(resulting_backup));
}

#[test]
fn test_mv_update_option() {
    let test_set = TestSet::new(UTIL_NAME);
    let at = &test_set.fixtures;
    let file_a = "test_mv_update_option_file_a";
    let file_b = "test_mv_update_option_file_b";

    at.touch(file_a);
    at.touch(file_b);
    let ts = time::now().to_timespec();
    let now = FileTime::from_seconds_since_1970(ts.sec as u64, ts.nsec as u32);
    let later = FileTime::from_seconds_since_1970(ts.sec as u64 + 3600, ts.nsec as u32);
    filetime::set_file_times(at.plus_as_string(file_a), now, now).unwrap();
    filetime::set_file_times(at.plus_as_string(file_b), now, later).unwrap();

    let result1 = test_set.util_cmd().arg("--update").arg(file_a).arg(file_b).run();

    assert_empty_stderr!(result1);
    assert!(result1.success);

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));

    let result2 = test_set.util_cmd().arg("--update").arg(file_b).arg(file_a).run();

    assert_empty_stderr!(result2);
    assert!(result2.success);

    assert!(at.file_exists(file_a));
    assert!(!at.file_exists(file_b));
}

#[test]
fn test_mv_target_dir() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir = "test_mv_target_dir_dir";
    let file_a = "test_mv_target_dir_file_a";
    let file_b = "test_mv_target_dir_file_b";

    at.touch(file_a);
    at.touch(file_b);
    at.mkdir(dir);
    let result = ucmd.arg("-t").arg(dir).arg(file_a).arg(file_b).run();

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.file_exists(file_a));
    assert!(!at.file_exists(file_b));
    assert!(at.file_exists(&format!("{}/{}", dir, file_a)));
    assert!(at.file_exists(&format!("{}/{}", dir, file_b)));
}

#[test]
fn test_mv_overwrite_dir() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir_a = "test_mv_overwrite_dir_a";
    let dir_b = "test_mv_overwrite_dir_b";

    at.mkdir(dir_a);
    at.mkdir(dir_b);
    let result = ucmd.arg("-T").arg(dir_a).arg(dir_b).run();

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!at.dir_exists(dir_a));
    assert!(at.dir_exists(dir_b));
}

#[test]
fn test_mv_overwrite_nonempty_dir() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir_a = "test_mv_overwrite_nonempty_dir_a";
    let dir_b = "test_mv_overwrite_nonempty_dir_b";
    let dummy = "test_mv_overwrite_nonempty_dir_b/file";

    at.mkdir(dir_a);
    at.mkdir(dir_b);
    at.touch(dummy);
    let result = ucmd.arg("-vT").arg(dir_a).arg(dir_b).run();

    // Not same error as GNU; the error message is a rust builtin
    // TODO: test (and implement) correct error message (or at least decide whether to do so)
    // Current: "mv: error: couldn't rename path (Directory not empty; from=a; to=b)"
    // GNU:     "mv: cannot move ‘a’ to ‘b’: Directory not empty"
    assert!(result.stderr.len() > 0);

    // Verbose output for the move should not be shown on failure
    assert!(result.stdout.len() == 0);

    assert!(!result.success);
    assert!(at.dir_exists(dir_a));
    assert!(at.dir_exists(dir_b));
}

#[test]
fn test_mv_backup_dir() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    let dir_a = "test_mv_backup_dir_dir_a";
    let dir_b = "test_mv_backup_dir_dir_b";

    at.mkdir(dir_a);
    at.mkdir(dir_b);
    let result = ucmd.arg("-vbT").arg(dir_a).arg(dir_b).run();

    assert_empty_stderr!(result);
    assert_eq!(result.stdout,
               format!("‘{}’ -> ‘{}’ (backup: ‘{}~’)\n",
                       dir_a,
                       dir_b,
                       dir_b));
    assert!(result.success);

    assert!(!at.dir_exists(dir_a));
    assert!(at.dir_exists(dir_b));
    assert!(at.dir_exists(&format!("{}~", dir_b)));
}

#[test]
fn test_mv_errors() {
    let ts = TestSet::new(UTIL_NAME);
    let at = &ts.fixtures;
    let dir = "test_mv_errors_dir";
    let file_a = "test_mv_errors_file_a";
    let file_b = "test_mv_errors_file_b";
    at.mkdir(dir);
    at.touch(file_a);
    at.touch(file_b);

    // $ mv -T -t a b
    // mv: cannot combine --target-directory (-t) and --no-target-directory (-T)
    let result = ts.util_cmd().arg("-T").arg("-t").arg(dir).arg(file_a).arg(file_b).run();
    assert_eq!(result.stderr,
               "mv: error: cannot combine --target-directory (-t) and --no-target-directory \
                (-T)\n");
    assert!(!result.success);


    // $ at.touch file && at.mkdir dir
    // $ mv -T file dir
    // err == mv: cannot overwrite directory ‘dir’ with non-directory
    let result = ts.util_cmd().arg("-T").arg(file_a).arg(dir).run();
    assert_eq!(result.stderr,
               format!("mv: error: cannot overwrite directory ‘{}’ with non-directory\n",
                       dir));
    assert!(!result.success);

    // $ at.mkdir dir && at.touch file
    // $ mv dir file
    // err == mv: cannot overwrite non-directory ‘file’ with directory ‘dir’
    let result = ts.util_cmd().arg(dir).arg(file_a).run();
    assert!(result.stderr.len() > 0);
    assert!(!result.success);
}

#[test]
fn test_mv_verbose() {
    let ts = TestSet::new(UTIL_NAME);
    let at = &ts.fixtures;
    let dir = "test_mv_verbose_dir";
    let file_a = "test_mv_verbose_file_a";
    let file_b = "test_mv_verbose_file_b";
    at.mkdir(dir);
    at.touch(file_a);
    at.touch(file_b);

    let result = ts.util_cmd().arg("-v").arg(file_a).arg(file_b).run();
    assert_empty_stderr!(result);
    assert_eq!(result.stdout,
               format!("‘{}’ -> ‘{}’\n", file_a, file_b));
    assert!(result.success);


    at.touch(file_a);
    let result = ts.util_cmd().arg("-vb").arg(file_a).arg(file_b).run();
    assert_empty_stderr!(result);
    assert_eq!(result.stdout,
               format!("‘{}’ -> ‘{}’ (backup: ‘{}~’)\n",
                       file_a,
                       file_b,
                       file_b));
    assert!(result.success);
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
// mv: try to overwrite ‘b’, overriding mode 0444 (r--r--r--)? y
// ‘a’ -> ‘b’
