extern crate libc;

use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./ln";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_symlink_existing_file() {
    let file = "test_symlink_existing_file";
    let link = "test_symlink_existing_file_link";

    touch(file);

    let result = run(Command::new(PROGNAME).args(&["-s", file, link]));
    assert_empty_stderr!(result);
    assert!(result.success);
    assert!(file_exists(file));
    assert!(is_symlink(link));
    assert_eq!(resolve_link(link), file);
}

#[test]
fn test_symlink_dangling_file() {
    let file = "test_symlink_dangling_file";
    let link = "test_symlink_dangling_file_link";

    let result = run(Command::new(PROGNAME).args(&["-s", file, link]));
    assert_empty_stderr!(result);
    assert!(result.success);
    assert!(!file_exists(file));
    assert!(is_symlink(link));
    assert_eq!(resolve_link(link), file);
}

#[test]
fn test_symlink_existing_directory() {
    let dir = "test_symlink_existing_dir";
    let link = "test_symlink_existing_dir_link";

    mkdir(dir);

    let result = run(Command::new(PROGNAME).args(&["-s", dir, link]));
    assert_empty_stderr!(result);
    assert!(result.success);
    assert!(dir_exists(dir));
    assert!(is_symlink(link));
    assert_eq!(resolve_link(link), dir);
}

#[test]
fn test_symlink_dangling_directory() {
    let dir = "test_symlink_dangling_dir";
    let link = "test_symlink_dangling_dir_link";

    let result = run(Command::new(PROGNAME).args(&["-s", dir, link]));
    assert_empty_stderr!(result);
    assert!(result.success);
    assert!(!dir_exists(dir));
    assert!(is_symlink(link));
    assert_eq!(resolve_link(link), dir);
}

#[test]
fn test_symlink_circular() {
    let link = "test_symlink_circular";

    let result = run(Command::new(PROGNAME).args(&["-s", link]));
    assert_empty_stderr!(result);
    assert!(result.success);
    assert!(is_symlink(link));
    assert_eq!(resolve_link(link), link);
}

#[test]
fn test_symlink_dont_overwrite() {
    let file = "test_symlink_dont_overwrite";
    let link = "test_symlink_dont_overwrite_link";

    touch(file);
    touch(link);

    let result = run(Command::new(PROGNAME).args(&["-s", file, link]));
    assert!(!result.success);
    assert!(file_exists(file));
    assert!(file_exists(link));
    assert!(!is_symlink(link));
}

#[test]
fn test_symlink_overwrite_force() {
    let file_a = "test_symlink_overwrite_force_a";
    let file_b = "test_symlink_overwrite_force_b";
    let link = "test_symlink_overwrite_force_link";

    // Create symlink
    symlink(file_a, link);
    assert!(is_symlink(link));
    assert_eq!(resolve_link(link), file_a);

    // Force overwrite of existing symlink
    let result = run(Command::new(PROGNAME).args(&["--force", "-s", file_b, link]));
    assert!(result.success);
    assert!(is_symlink(link));
    assert_eq!(resolve_link(link), file_b);
}

#[test]
fn test_symlink_interactive() {
    let file = "test_symlink_interactive_file";
    let link = "test_symlink_interactive_file_link";

    touch(file);
    touch(link);

    let result1 = run_piped_stdin(Command::new(PROGNAME).args(&["-i", "-s", file, link]), b"n");

    assert_empty_stderr!(result1);
    assert!(result1.success);

    assert!(file_exists(file));
    assert!(!is_symlink(link));

    let result2 = run_piped_stdin(Command::new(PROGNAME).args(&["-i", "-s", file, link]), b"Yesh");

    assert_empty_stderr!(result2);
    assert!(result2.success);

    assert!(file_exists(file));
    assert!(is_symlink(link));
    assert_eq!(resolve_link(link), file);
}

#[test]
fn test_symlink_simple_backup() {
    let file = "test_symlink_simple_backup";
    let link = "test_symlink_simple_backup_link";

    touch(file);
    symlink(file, link);
    assert!(file_exists(file));
    assert!(is_symlink(link));
    assert_eq!(resolve_link(link), file);

    let result = run(Command::new(PROGNAME).args(&["-b", "-s", file, link]));

    assert_empty_stderr!(result);
    assert!(result.success);
    assert!(file_exists(file));

    assert!(is_symlink(link));
    assert_eq!(resolve_link(link), file);

    let backup = &format!("{}~", link);
    assert!(is_symlink(backup));
    assert_eq!(resolve_link(backup), file);
}

#[test]
fn test_symlink_custom_backup_suffix() {
    let file = "test_symlink_custom_backup_suffix";
    let link = "test_symlink_custom_backup_suffix_link";
    let suffix = "super-suffix-of-the-century";

    touch(file);
    symlink(file, link);
    assert!(file_exists(file));
    assert!(is_symlink(link));
    assert_eq!(resolve_link(link), file);

    let arg = &format!("--suffix={}", suffix);
    let result = run(Command::new(PROGNAME).args(&["-b", arg, "-s", file, link]));

    assert_empty_stderr!(result);
    assert!(result.success);
    assert!(file_exists(file));

    assert!(is_symlink(link));
    assert_eq!(resolve_link(link), file);

    let backup = &format!("{}{}", link, suffix);
    assert!(is_symlink(backup));
    assert_eq!(resolve_link(backup), file);
}

#[test]
fn test_symlink_backup_numbering() {
    let file = "test_symlink_backup_numbering";
    let link = "test_symlink_backup_numbering_link";

    touch(file);
    symlink(file, link);
    assert!(file_exists(file));
    assert!(is_symlink(link));
    assert_eq!(resolve_link(link), file);

    let result = run(Command::new(PROGNAME).args(&["-s", "--backup=t", file, link]));

    assert_empty_stderr!(result);
    assert!(result.success);
    assert!(file_exists(file));

    assert!(is_symlink(link));
    assert_eq!(resolve_link(link), file);

    let backup = &format!("{}.~1~", link);
    assert!(is_symlink(backup));
    assert_eq!(resolve_link(backup), file);
}

#[test]
fn test_symlink_existing_backup() {
    let file = "test_symlink_existing_backup";
    let link = "test_symlink_existing_backup_link";
    let link_backup = "test_symlink_existing_backup_link.~1~";
    let resulting_backup = "test_symlink_existing_backup_link.~2~";

    // Create symlink and verify
    touch(file);
    symlink(file, link);
    assert!(file_exists(file));
    assert!(is_symlink(link));
    assert_eq!(resolve_link(link), file);

    // Create backup symlink and verify
    symlink(file, link_backup);
    assert!(file_exists(file));
    assert!(is_symlink(link_backup));
    assert_eq!(resolve_link(link_backup), file);

    let result = run(Command::new(PROGNAME).args(&["-s", "--backup=nil", file, link]));

    assert_empty_stderr!(result);
    assert!(result.success);
    assert!(file_exists(file));

    assert!(is_symlink(link_backup));
    assert_eq!(resolve_link(link_backup), file);

    assert!(is_symlink(resulting_backup));
    assert_eq!(resolve_link(resulting_backup), file);
}

#[test]
fn test_symlink_target_dir() {
    let dir = "test_ln_target_dir_dir";
    let file_a = "test_ln_target_dir_file_a";
    let file_b = "test_ln_target_dir_file_b";

    touch(file_a);
    touch(file_b);
    mkdir(dir);

    let result = run(Command::new(PROGNAME).args(&["-s", "-t", dir, file_a, file_b]));

    assert_empty_stderr!(result);
    assert!(result.success);

    let file_a_link = &format!("{}/{}", dir, file_a);
    assert!(is_symlink(file_a_link));
    assert_eq!(resolve_link(file_a_link), file_a);

    let file_b_link = &format!("{}/{}", dir, file_b);
    assert!(is_symlink(file_b_link));
    assert_eq!(resolve_link(file_b_link), file_b);
}

#[test]
fn test_symlink_overwrite_dir() {
    let path_a = "test_symlink_overwrite_dir_a";
    let path_b = "test_symlink_overwrite_dir_b";

    touch(path_a);
    mkdir(path_b);

    let result = run(Command::new(PROGNAME).args(&["-s", "-T", path_a, path_b]));

    assert_empty_stderr!(result);
    assert!(result.success);

    assert!(file_exists(path_a));
    assert!(is_symlink(path_b));
    assert_eq!(resolve_link(path_b), path_a);
}

#[test]
fn test_symlink_overwrite_nonempty_dir() {
    let path_a = "test_symlink_overwrite_nonempty_dir_a";
    let path_b = "test_symlink_overwrite_nonempty_dir_b";
    let dummy = "test_symlink_overwrite_nonempty_dir_b/file";

    touch(path_a);
    mkdir(path_b);
    touch(dummy);

    let result = run(Command::new(PROGNAME).args(&["-v", "-T", "-s", path_a, path_b]));

    // Not same error as GNU; the error message is a Rust builtin
    // TODO: test (and implement) correct error message (or at least decide whether to do so)
    // Current: "ln: error: Directory not empty (os error 66)"
    // GNU:     "ln: cannot link 'a' to 'b': Directory not empty"
    assert!(result.stderr.len() > 0);

    // Verbose output for the link should not be shown on failure
    assert!(result.stdout.len() == 0);

    assert!(!result.success);
    assert!(file_exists(path_a));
    assert!(dir_exists(path_b));
}

#[test]
fn test_symlink_errors() {
    let dir = "test_symlink_errors_dir";
    let file_a = "test_symlink_errors_file_a";
    let file_b = "test_symlink_errors_file_b";

    mkdir(dir);
    touch(file_a);
    touch(file_b);

    // $ ln -T -t a b
    // ln: cannot combine --target-directory (-t) and --no-target-directory (-T)
    let result = run(Command::new(PROGNAME).args(&["-T", "-t", dir, file_a, file_b]));
    assert_eq!(result.stderr,
        "ln: error: cannot combine --target-directory (-t) and --no-target-directory (-T)\n");
    assert!(!result.success);
}

#[test]
fn test_symlink_verbose() {
    let file_a = "test_symlink_verbose_file_a";
    let file_b = "test_symlink_verbose_file_b";

    touch(file_a);

    let result = run(Command::new(PROGNAME).args(&["-v", file_a, file_b]));
    assert_empty_stderr!(result);
    assert_eq!(result.stdout,
        format!("'{}' -> '{}'\n", file_b, file_a));
    assert!(result.success);

    touch(file_b);

    let result = run(Command::new(PROGNAME).args(&["-v", "-b", file_a, file_b]));
    assert_empty_stderr!(result);
    assert_eq!(result.stdout,
        format!("'{}' -> '{}' (backup: '{}~')\n", file_b, file_a, file_b));
    assert!(result.success);
}
