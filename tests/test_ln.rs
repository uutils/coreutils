use common::util::*;
use std::path::PathBuf;

#[test]
fn test_symlink_existing_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_existing_file";
    let link = "test_symlink_existing_file_link";

    at.touch(file);

    ucmd.args(&["-s", file, link]).succeeds().no_stderr();
    
    assert!(at.file_exists(file));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);
}

#[test]
fn test_symlink_dangling_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_dangling_file";
    let link = "test_symlink_dangling_file_link";

    ucmd.args(&["-s", file, link]).succeeds().no_stderr();
    assert!(!at.file_exists(file));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);
}

#[test]
fn test_symlink_existing_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_symlink_existing_dir";
    let link = "test_symlink_existing_dir_link";

    at.mkdir(dir);

    ucmd.args(&["-s", dir, link]).succeeds().no_stderr();
    assert!(at.dir_exists(dir));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), dir);
}

#[test]
fn test_symlink_dangling_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_symlink_dangling_dir";
    let link = "test_symlink_dangling_dir_link";

    ucmd.args(&["-s", dir, link]).succeeds().no_stderr();
    assert!(!at.dir_exists(dir));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), dir);
}

#[test]
fn test_symlink_circular() {
    let (at, mut ucmd) = at_and_ucmd!();
    let link = "test_symlink_circular";

    ucmd.args(&["-s", link]).succeeds().no_stderr();
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), link);
}

#[test]
fn test_symlink_dont_overwrite() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_dont_overwrite";
    let link = "test_symlink_dont_overwrite_link";

    at.touch(file);
    at.touch(link);

    ucmd.args(&["-s", file, link]).fails();
    assert!(at.file_exists(file));
    assert!(at.file_exists(link));
    assert!(!at.is_symlink(link));
}

#[test]
fn test_symlink_overwrite_force() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_symlink_overwrite_force_a";
    let file_b = "test_symlink_overwrite_force_b";
    let link = "test_symlink_overwrite_force_link";

    // Create symlink
    at.symlink(file_a, link);
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file_a);

    // Force overwrite of existing symlink
    ucmd.args(&["--force", "-s", file_b, link]).succeeds();
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file_b);
}

#[test]
fn test_symlink_interactive() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let file = "test_symlink_interactive_file";
    let link = "test_symlink_interactive_file_link";

    at.touch(file);
    at.touch(link);

    scene.ucmd()
        .args(&["-i", "-s", file, link])
        .pipe_in("n").succeeds().no_stderr();

    assert!(at.file_exists(file));
    assert!(!at.is_symlink(link));

    scene.ucmd()
        .args(&["-i", "-s", file, link])
        .pipe_in("Yesh").succeeds().no_stderr();

    assert!(at.file_exists(file));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);
}

#[test]
fn test_symlink_simple_backup() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_simple_backup";
    let link = "test_symlink_simple_backup_link";

    at.touch(file);
    at.symlink(file, link);
    assert!(at.file_exists(file));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);

    ucmd.args(&["-b", "-s", file, link]).succeeds().no_stderr();

    assert!(at.file_exists(file));

    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);

    let backup = &format!("{}~", link);
    assert!(at.is_symlink(backup));
    assert_eq!(at.resolve_link(backup), file);
}

#[test]
fn test_symlink_custom_backup_suffix() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_custom_backup_suffix";
    let link = "test_symlink_custom_backup_suffix_link";
    let suffix = "super-suffix-of-the-century";

    at.touch(file);
    at.symlink(file, link);
    assert!(at.file_exists(file));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);

    let arg = &format!("--suffix={}", suffix);
    ucmd.args(&["-b", arg, "-s", file, link]).succeeds().no_stderr();
    assert!(at.file_exists(file));

    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);

    let backup = &format!("{}{}", link, suffix);
    assert!(at.is_symlink(backup));
    assert_eq!(at.resolve_link(backup), file);
}

#[test]
fn test_symlink_backup_numbering() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_backup_numbering";
    let link = "test_symlink_backup_numbering_link";

    at.touch(file);
    at.symlink(file, link);
    assert!(at.file_exists(file));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);

    ucmd.args(&["-s", "--backup=t", file, link]).succeeds().no_stderr();
    assert!(at.file_exists(file));

    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);

    let backup = &format!("{}.~1~", link);
    assert!(at.is_symlink(backup));
    assert_eq!(at.resolve_link(backup), file);
}

#[test]
fn test_symlink_existing_backup() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_existing_backup";
    let link = "test_symlink_existing_backup_link";
    let link_backup = "test_symlink_existing_backup_link.~1~";
    let resulting_backup = "test_symlink_existing_backup_link.~2~";

    // Create symlink and verify
    at.touch(file);
    at.symlink(file, link);
    assert!(at.file_exists(file));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);

    // Create backup symlink and verify
    at.symlink(file, link_backup);
    assert!(at.file_exists(file));
    assert!(at.is_symlink(link_backup));
    assert_eq!(at.resolve_link(link_backup), file);

    ucmd.args(&["-s", "--backup=nil", file, link]).succeeds().no_stderr();
    assert!(at.file_exists(file));

    assert!(at.is_symlink(link_backup));
    assert_eq!(at.resolve_link(link_backup), file);

    assert!(at.is_symlink(resulting_backup));
    assert_eq!(at.resolve_link(resulting_backup), file);
}

#[test]
fn test_symlink_target_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_ln_target_dir_dir";
    let file_a = "test_ln_target_dir_file_a";
    let file_b = "test_ln_target_dir_file_b";

    at.touch(file_a);
    at.touch(file_b);
    at.mkdir(dir);

    ucmd.args(&["-s", "-t", dir, file_a, file_b]).succeeds().no_stderr();

    let file_a_link = &format!("{}/{}", dir, file_a);
    assert!(at.is_symlink(file_a_link));
    assert_eq!(at.resolve_link(file_a_link), file_a);

    let file_b_link = &format!("{}/{}", dir, file_b);
    assert!(at.is_symlink(file_b_link));
    assert_eq!(at.resolve_link(file_b_link), file_b);
}

#[test]
fn test_symlink_target_dir_from_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_ln_target_dir_dir";
    let from_dir = "test_ln_target_dir_from_dir";
    let filename_a = "test_ln_target_dir_file_a";
    let filename_b = "test_ln_target_dir_file_b";
    let file_a = &format!("{}/{}", from_dir, filename_a);
    let file_b = &format!("{}/{}", from_dir, filename_b);

    at.mkdir(from_dir);
    at.touch(file_a);
    at.touch(file_b);
    at.mkdir(dir);

    ucmd.args(&["-s", "-t", dir, file_a, file_b]).succeeds().no_stderr();

    let file_a_link = &format!("{}/{}", dir, filename_a);
    assert!(at.is_symlink(file_a_link));
    assert_eq!(&at.resolve_link(file_a_link), file_a);

    let file_b_link = &format!("{}/{}", dir, filename_b);
    assert!(at.is_symlink(file_b_link));
    assert_eq!(&at.resolve_link(file_b_link), file_b);
}

#[test]
fn test_symlink_overwrite_dir_fail() {
    let (at, mut ucmd) = at_and_ucmd!();
    let path_a = "test_symlink_overwrite_dir_a";
    let path_b = "test_symlink_overwrite_dir_b";

    at.touch(path_a);
    at.mkdir(path_b);

    assert!(ucmd.args(&["-s", "-T", path_a, path_b]).fails().stderr.len() > 0);
}

#[test]
fn test_symlink_errors() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_symlink_errors_dir";
    let file_a = "test_symlink_errors_file_a";
    let file_b = "test_symlink_errors_file_b";

    at.mkdir(dir);
    at.touch(file_a);
    at.touch(file_b);

    // $ ln -T -t a b
    // ln: cannot combine --target-directory (-t) and --no-target-directory (-T)
    ucmd.args(&["-T", "-t", dir, file_a, file_b]).fails()
        .stderr_is("ln: error: cannot combine --target-directory (-t) and --no-target-directory \
                (-T)\n");
}

#[test]
fn test_symlink_verbose() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let file_a = "test_symlink_verbose_file_a";
    let file_b = "test_symlink_verbose_file_b";

    at.touch(file_a);

    scene.ucmd().args(&["-v", file_a, file_b])
        .succeeds().stdout_only(format!("'{}' -> '{}'\n", file_b, file_a));

    at.touch(file_b);

    scene.ucmd().args(&["-v", "-b", file_a, file_b])
        .succeeds().stdout_only(format!("'{}' -> '{}' (backup: '{}~')\n", file_b, file_a, file_b));
}

#[test]
fn test_symlink_target_only() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_symlink_target_only";

    at.mkdir(dir);

    assert!(ucmd.args(&["-s", "-t", dir]).fails().stderr.len() > 0);
}

#[test]
fn test_symlink_implicit_target_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_symlink_implicit_target_dir";
    // On windows, slashes aren't allowed in symlink targets, so use
    // PathBuf to construct `file` instead of simple "dir/file".
    let filename = "test_symlink_implicit_target_file";
    let path = PathBuf::from(dir).join(filename);
    let file = &path.to_string_lossy();

    at.mkdir(dir);
    at.touch(file);

    ucmd.args(&["-s", file]).succeeds().no_stderr();

    assert!(at.file_exists(filename));
    assert!(at.is_symlink(filename));
    assert_eq!(at.resolve_link(filename), *file);
}

#[test]
fn test_symlink_to_dir_2args() {
    let (at, mut ucmd) = at_and_ucmd!();
    let filename = "test_symlink_to_dir_2args_file";
    let from_file = &format!("{}/{}", at.as_string(), filename);
    let to_dir = "test_symlink_to_dir_2args_to_dir";
    let to_file = &format!("{}/{}", to_dir, filename);

    at.mkdir(to_dir);
    at.touch(from_file);

    ucmd.args(&["-s", from_file, to_dir]).succeeds().no_stderr();

    assert!(at.file_exists(to_file));
    assert!(at.is_symlink(to_file));
    assert_eq!(at.resolve_link(to_file), filename);
}

#[test]
fn test_symlink_missing_destination() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_missing_destination";

    at.touch(file);

    ucmd.args(&["-s", "-T", file]).fails()
        .stderr_is(format!("ln: error: missing destination file operand after '{}'", file));
}
