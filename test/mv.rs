#![allow(unstable)]

extern crate time;

use std::old_io::{process, fs, FilePermission};
use std::old_io::process::Command;
use std::old_io::fs::PathExtensions;
use std::str::from_utf8;
use std::borrow::ToOwned;

static EXE: &'static str = "./mv";


macro_rules! assert_empty_stderr(
    ($cond:expr) => (
        if $cond.stderr.len() > 0 {
            panic!(format!("stderr: {}", $cond.stderr))
        }
    );
);
struct CmdResult {
    success: bool,
    stderr: String,
    stdout: String,
}
fn run(cmd: &mut Command) -> CmdResult {
    let prog = cmd.spawn().unwrap().wait_with_output().unwrap();
    CmdResult {
        success: prog.status.success(),
        stderr: from_utf8(prog.error.as_slice()).unwrap().to_owned(),
        stdout: from_utf8(prog.output.as_slice()).unwrap().to_owned(),
    }
}
fn run_interactive(cmd: &mut Command, input: &[u8])-> CmdResult {
    let stdin_cfg = process::CreatePipe(true, false);
    let mut command = cmd.stdin(stdin_cfg).spawn().unwrap();

    command.stdin.as_mut().unwrap().write(input);

    let prog = command.wait_with_output().unwrap();
    CmdResult {
        success: prog.status.success(),
        stderr: from_utf8(prog.error.as_slice()).unwrap().to_owned(),
        stdout: from_utf8(prog.output.as_slice()).unwrap().to_owned(),
    }
}

fn mkdir(dir: &str) {
    fs::mkdir(&Path::new(dir), FilePermission::from_bits_truncate(0o755 as u32)).unwrap();
}
fn touch(file: &str) {
    fs::File::create(&Path::new(file)).unwrap();
}


#[test]
fn test_mv_rename_dir() {
    let dir1 = "test_mv_rename_dir";
    let dir2 = "test_mv_rename_dir2";

    mkdir(dir1);

    let result = run(Command::new(EXE).arg(dir1).arg(dir2));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(Path::new(dir2).is_dir());
}

#[test]
fn test_mv_rename_file() {
    let file1 = "test_mv_rename_file";
    let file2 = "test_mv_rename_file2";

    touch(file1);

    let result = run(Command::new(EXE).arg(file1).arg(file2));
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

    let result = run(Command::new(EXE).arg(file).arg(dir));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(Path::new(format!("{}/{}", dir, file)).is_file());
}

#[test]
fn test_mv_multiple_files() {
    let target_dir = "test_mv_multiple_files_dir";
    let file_a = "test_mv_multiple_file_a";
    let file_b = "test_mv_multiple_file_b";

    mkdir(target_dir);
    touch(file_a);
    touch(file_b);

    let result = run(Command::new(EXE).arg(file_a).arg(file_b).arg(target_dir));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(Path::new(format!("{}/{}", target_dir, file_a)).is_file());
    assert!(Path::new(format!("{}/{}", target_dir, file_b)).is_file());
}

#[test]
fn test_mv_multiple_folders() {
    let target_dir = "test_mv_multiple_dirs_dir";
    let dir_a = "test_mv_multiple_dir_a";
    let dir_b = "test_mv_multiple_dir_b";

    mkdir(target_dir);
    mkdir(dir_a);
    mkdir(dir_b);

    let result = run(Command::new(EXE).arg(dir_a).arg(dir_b).arg(target_dir));
    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(Path::new(format!("{}/{}", target_dir, dir_a)).is_dir());
    assert!(Path::new(format!("{}/{}", target_dir, dir_b)).is_dir());
}

#[test]
fn test_mv_interactive() {
    let file_a = "test_mv_interactive_file_a";
    let file_b = "test_mv_interactive_file_b";

    touch(file_a);
    touch(file_b);


    let result1 = run_interactive(Command::new(EXE).arg("-i").arg(file_a).arg(file_b), b"n");

    assert_empty_stderr!(result1);
    assert!(result1.success);

    assert!(Path::new(file_a).is_file());
    assert!(Path::new(file_b).is_file());


    let result2 = run_interactive(Command::new(EXE).arg("-i").arg(file_a).arg(file_b), b"Yesh");

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

    let result = run(Command::new(EXE).arg("-n").arg(file_a).arg(file_b));
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

    let result = run(Command::new(EXE).arg(file_a).arg(file_b));
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

    let result = run(Command::new(EXE).arg("--force").arg(file_a).arg(file_b));
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
    let result = run(Command::new(EXE).arg("-b").arg(file_a).arg(file_b));

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(file_a).is_file());
    assert!(Path::new(file_b).is_file());
    assert!(Path::new(format!("{}~", file_b)).is_file());
}

#[test]
fn test_mv_custom_backup_suffix() {
    let file_a = "test_mv_custom_backup_suffix_file_a";
    let file_b = "test_mv_custom_backup_suffix_file_b";
    let suffix = "super-suffix-of-the-century";

    touch(file_a);
    touch(file_b);
    let result = run(Command::new(EXE)
            .arg("-b").arg(format!("--suffix={}", suffix))
            .arg(file_a).arg(file_b));

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(file_a).is_file());
    assert!(Path::new(file_b).is_file());
    assert!(Path::new(format!("{}{}", file_b, suffix)).is_file());
}

#[test]
fn test_mv_backup_numbering() {
    let file_a = "test_mv_backup_numbering_file_a";
    let file_b = "test_mv_backup_numbering_file_b";

    touch(file_a);
    touch(file_b);
    let result = run(Command::new(EXE).arg("--backup=t").arg(file_a).arg(file_b));

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(file_a).is_file());
    assert!(Path::new(file_b).is_file());
    assert!(Path::new(format!("{}.~1~", file_b)).is_file());
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
    let result = run(Command::new(EXE).arg("--backup=nil").arg(file_a).arg(file_b));

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
    fs::change_file_times(&Path::new(file_a), now, now).unwrap();
    fs::change_file_times(&Path::new(file_b), now, now+3600).unwrap();

    let result1 = run(Command::new(EXE).arg("--update").arg(file_a).arg(file_b));

    assert_empty_stderr!(result1);
    assert!(result1.success);

    assert!(Path::new(file_a).is_file());
    assert!(Path::new(file_b).is_file());

    let result2 = run(Command::new(EXE).arg("--update").arg(file_b).arg(file_a));

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
    let result = run(Command::new(EXE).arg("-t").arg(dir).arg(file_a).arg(file_b));

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(!Path::new(file_a).is_file());
    assert!(!Path::new(file_b).is_file());
    assert!(Path::new(format!("{}/{}", dir, file_a)).is_file());
    assert!(Path::new(format!("{}/{}", dir, file_b)).is_file());
}

#[test]
fn test_mv_overwrite_dir() {
    let dir_a = "test_mv_overwrite_dir_a";
    let dir_b = "test_mv_overwrite_dir_b";

    mkdir(dir_a);
    mkdir(dir_b);
    let result = run(Command::new(EXE).arg("-T").arg(dir_a).arg(dir_b));

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
    let result = run(Command::new(EXE).arg("-vT").arg(dir_a).arg(dir_b));

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
    let result = run(Command::new(EXE).arg("-vbT").arg(dir_a).arg(dir_b));

    assert_empty_stderr!(result);
    assert_eq!(result.stdout.as_slice(),
        format!("‘{}’ -> ‘{}’ (backup: ‘{}~’)\n", dir_a, dir_b, dir_b).as_slice());
    assert!(result.success);

    assert!(!Path::new(dir_a).is_dir());
    assert!(Path::new(dir_b).is_dir());
    assert!(Path::new(format!("{}~", dir_b)).is_dir());
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
    let result = run(Command::new(EXE).arg("-T").arg("-t").arg(dir).arg(file_a).arg(file_b));
    assert_eq!(result.stderr.as_slice(),
        "mv: error: cannot combine --target-directory (-t) and --no-target-directory (-T)\n");
    assert!(!result.success);


    // $ touch file && mkdir dir
    // $ mv -T file dir
    // err == mv: cannot overwrite directory ‘dir’ with non-directory
    let result = run(Command::new(EXE).arg("-T").arg(file_a).arg(dir));
    assert_eq!(result.stderr.as_slice(),
        format!("mv: error: cannot overwrite directory ‘{}’ with non-directory\n", dir).as_slice());
    assert!(!result.success);

    // $ mkdir dir && touch file
    // $ mv dir file
    // err == mv: cannot overwrite non-directory ‘file’ with directory ‘dir’
    let result = run(Command::new(EXE).arg(dir).arg(file_a));
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

    let result = run(Command::new(EXE).arg("-v").arg(file_a).arg(file_b));
    assert_empty_stderr!(result);
    assert_eq!(result.stdout.as_slice(),
        format!("‘{}’ -> ‘{}’\n", file_a, file_b).as_slice());
    assert!(result.success);


    touch(file_a);
    let result = run(Command::new(EXE).arg("-vb").arg(file_a).arg(file_b));
    assert_empty_stderr!(result);
    assert_eq!(result.stdout.as_slice(),
        format!("‘{}’ -> ‘{}’ (backup: ‘{}~’)\n", file_a, file_b, file_b).as_slice());
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


