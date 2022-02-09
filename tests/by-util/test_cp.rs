// spell-checker:ignore (flags) reflink (fs) tmpfs (linux) rlimit Rlim NOFILE clob

use crate::common::util::*;
#[cfg(not(windows))]
use std::fs::set_permissions;

#[cfg(not(windows))]
use std::os::unix::fs;

#[cfg(unix)]
use std::os::unix::fs::symlink as symlink_file;
#[cfg(all(unix, not(target_os = "freebsd")))]
use std::os::unix::fs::PermissionsExt;
#[cfg(windows)]
use std::os::windows::fs::symlink_file;

#[cfg(any(target_os = "linux", target_os = "android"))]
use filetime::FileTime;
#[cfg(any(target_os = "linux", target_os = "android"))]
use rlimit::Resource;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::fs as std_fs;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::thread::sleep;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::time::Duration;

static TEST_EXISTING_FILE: &str = "existing_file.txt";
static TEST_HELLO_WORLD_SOURCE: &str = "hello_world.txt";
static TEST_HELLO_WORLD_SOURCE_SYMLINK: &str = "hello_world.txt.link";
static TEST_HELLO_WORLD_DEST: &str = "copy_of_hello_world.txt";
static TEST_HELLO_WORLD_DEST_SYMLINK: &str = "copy_of_hello_world.txt.link";
static TEST_HOW_ARE_YOU_SOURCE: &str = "how_are_you.txt";
static TEST_HOW_ARE_YOU_DEST: &str = "hello_dir/how_are_you.txt";
static TEST_COPY_TO_FOLDER: &str = "hello_dir/";
static TEST_COPY_TO_FOLDER_FILE: &str = "hello_dir/hello_world.txt";
static TEST_COPY_FROM_FOLDER: &str = "hello_dir_with_file/";
static TEST_COPY_FROM_FOLDER_FILE: &str = "hello_dir_with_file/hello_world.txt";
static TEST_COPY_TO_FOLDER_NEW: &str = "hello_dir_new";
static TEST_COPY_TO_FOLDER_NEW_FILE: &str = "hello_dir_new/hello_world.txt";
#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
static TEST_MOUNT_COPY_FROM_FOLDER: &str = "dir_with_mount";
#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
static TEST_MOUNT_MOUNTPOINT: &str = "mount";
#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
static TEST_MOUNT_OTHER_FILESYSTEM_FILE: &str = "mount/DO_NOT_copy_me.txt";
#[cfg(unix)]
static TEST_NONEXISTENT_FILE: &str = "nonexistent_file.txt";

#[test]
fn test_cp_cp() {
    let (at, mut ucmd) = at_and_ucmd!();
    // Invoke our binary to make the copy.
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HELLO_WORLD_DEST)
        .succeeds();

    // Check the content of the destination file that was copied.
    assert_eq!(at.read(TEST_HELLO_WORLD_DEST), "Hello, World!\n");
}

#[test]
fn test_cp_existing_target() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_EXISTING_FILE)
        .succeeds();

    // Check the content of the destination file
    assert_eq!(at.read(TEST_EXISTING_FILE), "Hello, World!\n");

    // No backup should have been created
    assert!(!at.file_exists(&*format!("{}~", TEST_EXISTING_FILE)));
}

#[test]
fn test_cp_duplicate_files() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_COPY_TO_FOLDER)
        .succeeds()
        .stderr_contains("specified more than once");
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");
}

#[test]
fn test_cp_multiple_files_target_is_file() {
    new_ucmd!()
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_EXISTING_FILE)
        .fails()
        .stderr_contains("not a directory");
}

#[test]
fn test_cp_directory_not_recursive() {
    new_ucmd!()
        .arg(TEST_COPY_TO_FOLDER)
        .arg(TEST_HELLO_WORLD_DEST)
        .fails()
        .stderr_contains("omitting directory");
}

#[test]
fn test_cp_multiple_files() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg(TEST_COPY_TO_FOLDER)
        .succeeds();

    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");
    assert_eq!(at.read(TEST_HOW_ARE_YOU_DEST), "How are you?\n");
}

#[test]
// FixME: for MacOS, this has intermittent failures; track repair progress at GH:uutils/coreutils/issues/1590
#[cfg(not(target_os = "macos"))]
fn test_cp_recurse() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg("-r")
        .arg(TEST_COPY_FROM_FOLDER)
        .arg(TEST_COPY_TO_FOLDER_NEW)
        .succeeds();

    // Check the content of the destination file that was copied.
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_NEW_FILE), "Hello, World!\n");
}

#[test]
fn test_cp_with_dirs_t() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg("-t")
        .arg(TEST_COPY_TO_FOLDER)
        .arg(TEST_HELLO_WORLD_SOURCE)
        .succeeds();
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");
}

#[test]
// FixME: for MacOS, this has intermittent failures; track repair progress at GH:uutils/coreutils/issues/1590
#[cfg(not(target_os = "macos"))]
fn test_cp_with_dirs() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    scene
        .ucmd()
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_COPY_TO_FOLDER)
        .succeeds();
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");

    scene
        .ucmd()
        .arg(TEST_COPY_FROM_FOLDER_FILE)
        .arg(TEST_HELLO_WORLD_DEST)
        .succeeds();
    assert_eq!(at.read(TEST_HELLO_WORLD_DEST), "Hello, World!\n");
}

#[test]
fn test_cp_arg_target_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("-t")
        .arg(TEST_COPY_TO_FOLDER)
        .succeeds();

    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");
}

#[test]
fn test_cp_arg_no_target_directory() {
    new_ucmd!()
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("-v")
        .arg("-T")
        .arg(TEST_COPY_TO_FOLDER)
        .fails()
        .stderr_contains("cannot overwrite directory");
}

#[test]
fn test_cp_target_directory_is_file() {
    new_ucmd!()
        .arg("-t")
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg(TEST_HELLO_WORLD_SOURCE)
        .fails()
        .stderr_contains(format!("'{}' is not a directory", TEST_HOW_ARE_YOU_SOURCE));
}

#[test]
fn test_cp_arg_interactive() {
    new_ucmd!()
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg("-i")
        .pipe_in("N\n")
        .succeeds()
        .stderr_contains("Not overwriting");
}

#[test]
#[cfg(target_os = "linux")]
fn test_cp_arg_link() {
    use std::os::linux::fs::MetadataExt;

    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--link")
        .arg(TEST_HELLO_WORLD_DEST)
        .succeeds();

    assert_eq!(at.metadata(TEST_HELLO_WORLD_SOURCE).st_nlink(), 2);
}

#[test]
fn test_cp_arg_symlink() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--symbolic-link")
        .arg(TEST_HELLO_WORLD_DEST)
        .succeeds();

    assert!(at.is_symlink(TEST_HELLO_WORLD_DEST));
}

#[test]
fn test_cp_arg_no_clobber() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg("--no-clobber")
        .succeeds();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "How are you?\n");
}

#[test]
fn test_cp_arg_no_clobber_inferred_arg() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg("--no-clob")
        .succeeds();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "How are you?\n");
}

#[test]
fn test_cp_arg_no_clobber_twice() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("source.txt");
    scene
        .ucmd()
        .arg("--no-clobber")
        .arg("source.txt")
        .arg("dest.txt")
        .succeeds()
        .no_stderr();

    assert_eq!(at.read("source.txt"), "");

    at.append("source.txt", "some-content");
    scene
        .ucmd()
        .arg("--no-clobber")
        .arg("source.txt")
        .arg("dest.txt")
        .succeeds()
        .stdout_does_not_contain("Not overwriting");

    assert_eq!(at.read("source.txt"), "some-content");
    // Should be empty as the "no-clobber" should keep
    // the previous version
    assert_eq!(at.read("dest.txt"), "");
}

#[test]
#[cfg(not(windows))]
fn test_cp_arg_force() {
    let (at, mut ucmd) = at_and_ucmd!();

    // create dest without write permissions
    let mut permissions = at
        .make_file(TEST_HELLO_WORLD_DEST)
        .metadata()
        .unwrap()
        .permissions();
    permissions.set_readonly(true);
    set_permissions(at.plus(TEST_HELLO_WORLD_DEST), permissions).unwrap();

    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--force")
        .arg(TEST_HELLO_WORLD_DEST)
        .succeeds();

    assert_eq!(at.read(TEST_HELLO_WORLD_DEST), "Hello, World!\n");
}

/// TODO: write a better test that differentiates --remove-destination
/// from --force. Also this test currently doesn't work on
/// Windows. This test originally checked file timestamps, which
/// proved to be unreliable per target / CI platform
#[test]
#[cfg(not(windows))]
fn test_cp_arg_remove_destination() {
    let (at, mut ucmd) = at_and_ucmd!();

    // create dest without write permissions
    let mut permissions = at
        .make_file(TEST_HELLO_WORLD_DEST)
        .metadata()
        .unwrap()
        .permissions();
    permissions.set_readonly(true);
    set_permissions(at.plus(TEST_HELLO_WORLD_DEST), permissions).unwrap();

    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--remove-destination")
        .arg(TEST_HELLO_WORLD_DEST)
        .succeeds();

    assert_eq!(at.read(TEST_HELLO_WORLD_DEST), "Hello, World!\n");
}

#[test]
fn test_cp_arg_backup() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg("-b")
        .succeeds();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert_eq!(
        at.read(&*format!("{}~", TEST_HOW_ARE_YOU_SOURCE)),
        "How are you?\n"
    );
}

#[test]
fn test_cp_arg_backup_with_other_args() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg("-vbL")
        .succeeds();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert_eq!(
        at.read(&*format!("{}~", TEST_HOW_ARE_YOU_SOURCE)),
        "How are you?\n"
    );
}

#[test]
fn test_cp_arg_backup_arg_first() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg("--backup")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert_eq!(
        at.read(&*format!("{}~", TEST_HOW_ARE_YOU_SOURCE)),
        "How are you?\n"
    );
}

#[test]
fn test_cp_arg_suffix() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("-b")
        .arg("--suffix")
        .arg(".bak")
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert_eq!(
        at.read(&*format!("{}.bak", TEST_HOW_ARE_YOU_SOURCE)),
        "How are you?\n"
    );
}

#[test]
fn test_cp_arg_suffix_hyphen_value() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("-b")
        .arg("--suffix")
        .arg("-v")
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert_eq!(
        at.read(&*format!("{}-v", TEST_HOW_ARE_YOU_SOURCE)),
        "How are you?\n"
    );
}

#[test]
fn test_cp_custom_backup_suffix_via_env() {
    let (at, mut ucmd) = at_and_ucmd!();
    let suffix = "super-suffix-of-the-century";

    ucmd.arg("-b")
        .env("SIMPLE_BACKUP_SUFFIX", suffix)
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds()
        .no_stderr();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert_eq!(
        at.read(&*format!("{}{}", TEST_HOW_ARE_YOU_SOURCE, suffix)),
        "How are you?\n"
    );
}

#[test]
fn test_cp_backup_numbered_with_t() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg("--backup=t")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds()
        .no_stderr();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert_eq!(
        at.read(&*format!("{}.~1~", TEST_HOW_ARE_YOU_SOURCE)),
        "How are you?\n"
    );
}

#[test]
fn test_cp_backup_numbered() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg("--backup=numbered")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds()
        .no_stderr();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert_eq!(
        at.read(&*format!("{}.~1~", TEST_HOW_ARE_YOU_SOURCE)),
        "How are you?\n"
    );
}

#[test]
fn test_cp_backup_existing() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg("--backup=existing")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds()
        .no_stderr();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert_eq!(
        at.read(&*format!("{}~", TEST_HOW_ARE_YOU_SOURCE)),
        "How are you?\n"
    );
}

#[test]
fn test_cp_backup_nil() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg("--backup=nil")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds()
        .no_stderr();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert_eq!(
        at.read(&*format!("{}~", TEST_HOW_ARE_YOU_SOURCE)),
        "How are you?\n"
    );
}

#[test]
fn test_cp_numbered_if_existing_backup_existing() {
    let (at, mut ucmd) = at_and_ucmd!();
    let existing_backup = &*format!("{}.~1~", TEST_HOW_ARE_YOU_SOURCE);
    at.touch(existing_backup);

    ucmd.arg("--backup=existing")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(TEST_HOW_ARE_YOU_SOURCE));
    assert!(at.file_exists(existing_backup));
    assert_eq!(
        at.read(&*format!("{}.~2~", TEST_HOW_ARE_YOU_SOURCE)),
        "How are you?\n"
    );
}

#[test]
fn test_cp_numbered_if_existing_backup_nil() {
    let (at, mut ucmd) = at_and_ucmd!();
    let existing_backup = &*format!("{}.~1~", TEST_HOW_ARE_YOU_SOURCE);

    at.touch(existing_backup);
    ucmd.arg("--backup=nil")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(TEST_HOW_ARE_YOU_SOURCE));
    assert!(at.file_exists(existing_backup));
    assert_eq!(
        at.read(&*format!("{}.~2~", TEST_HOW_ARE_YOU_SOURCE)),
        "How are you?\n"
    );
}

#[test]
fn test_cp_backup_simple() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg("--backup=simple")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds()
        .no_stderr();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert_eq!(
        at.read(&*format!("{}~", TEST_HOW_ARE_YOU_SOURCE)),
        "How are you?\n"
    );
}

#[test]
fn test_cp_backup_never() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg("--backup=never")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds()
        .no_stderr();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert_eq!(
        at.read(&*format!("{}~", TEST_HOW_ARE_YOU_SOURCE)),
        "How are you?\n"
    );
}

#[test]
fn test_cp_backup_none() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg("--backup=none")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds()
        .no_stderr();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert!(!at.file_exists(&format!("{}~", TEST_HOW_ARE_YOU_SOURCE)));
}

#[test]
fn test_cp_backup_off() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg("--backup=off")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds()
        .no_stderr();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert!(!at.file_exists(&format!("{}~", TEST_HOW_ARE_YOU_SOURCE)));
}

#[test]
fn test_cp_backup_no_clobber_conflicting_options() {
    new_ucmd!()
        .arg("--backup")
        .arg("--no-clobber")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .fails()
        .usage_error("options --backup and --no-clobber are mutually exclusive");
}

#[test]
fn test_cp_deref_conflicting_options() {
    new_ucmd!()
        .arg("-LP")
        .arg(TEST_COPY_TO_FOLDER)
        .arg(TEST_HELLO_WORLD_SOURCE)
        .fails();
}

#[test]
fn test_cp_deref() {
    let (at, mut ucmd) = at_and_ucmd!();

    #[cfg(not(windows))]
    let _r = fs::symlink(
        TEST_HELLO_WORLD_SOURCE,
        at.subdir.join(TEST_HELLO_WORLD_SOURCE_SYMLINK),
    );
    #[cfg(windows)]
    let _r = symlink_file(
        TEST_HELLO_WORLD_SOURCE,
        at.subdir.join(TEST_HELLO_WORLD_SOURCE_SYMLINK),
    );
    //using -L option
    ucmd.arg("-L")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HELLO_WORLD_SOURCE_SYMLINK)
        .arg(TEST_COPY_TO_FOLDER)
        .succeeds();

    let path_to_new_symlink = at
        .subdir
        .join(TEST_COPY_TO_FOLDER)
        .join(TEST_HELLO_WORLD_SOURCE_SYMLINK);
    // unlike -P/--no-deref, we expect a file, not a link
    assert!(at.file_exists(
        &path_to_new_symlink
            .clone()
            .into_os_string()
            .into_string()
            .unwrap()
    ));
    // Check the content of the destination file that was copied.
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");
    let path_to_check = path_to_new_symlink.to_str().unwrap();
    assert_eq!(at.read(path_to_check), "Hello, World!\n");
}
#[test]
fn test_cp_no_deref() {
    let (at, mut ucmd) = at_and_ucmd!();

    #[cfg(not(windows))]
    let _r = fs::symlink(
        TEST_HELLO_WORLD_SOURCE,
        at.subdir.join(TEST_HELLO_WORLD_SOURCE_SYMLINK),
    );
    #[cfg(windows)]
    let _r = symlink_file(
        TEST_HELLO_WORLD_SOURCE,
        at.subdir.join(TEST_HELLO_WORLD_SOURCE_SYMLINK),
    );
    //using -P option
    ucmd.arg("-P")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HELLO_WORLD_SOURCE_SYMLINK)
        .arg(TEST_COPY_TO_FOLDER)
        .succeeds();

    let path_to_new_symlink = at
        .subdir
        .join(TEST_COPY_TO_FOLDER)
        .join(TEST_HELLO_WORLD_SOURCE_SYMLINK);
    assert!(at.is_symlink(
        &path_to_new_symlink
            .clone()
            .into_os_string()
            .into_string()
            .unwrap()
    ));
    // Check the content of the destination file that was copied.
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");
    let path_to_check = path_to_new_symlink.to_str().unwrap();
    assert_eq!(at.read(path_to_check), "Hello, World!\n");
}

#[test]
fn test_cp_no_deref_link_onto_link() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.copy(TEST_HELLO_WORLD_SOURCE, TEST_HELLO_WORLD_DEST);

    #[cfg(not(windows))]
    let _r = fs::symlink(
        TEST_HELLO_WORLD_SOURCE,
        at.subdir.join(TEST_HELLO_WORLD_SOURCE_SYMLINK),
    );
    #[cfg(windows)]
    let _r = symlink_file(
        TEST_HELLO_WORLD_SOURCE,
        at.subdir.join(TEST_HELLO_WORLD_SOURCE_SYMLINK),
    );

    #[cfg(not(windows))]
    let _r = fs::symlink(
        TEST_HELLO_WORLD_DEST,
        at.subdir.join(TEST_HELLO_WORLD_DEST_SYMLINK),
    );
    #[cfg(windows)]
    let _r = symlink_file(
        TEST_HELLO_WORLD_DEST,
        at.subdir.join(TEST_HELLO_WORLD_DEST_SYMLINK),
    );

    ucmd.arg("-P")
        .arg(TEST_HELLO_WORLD_SOURCE_SYMLINK)
        .arg(TEST_HELLO_WORLD_DEST_SYMLINK)
        .succeeds();

    // Ensure that the target of the destination was not modified.
    assert!(!at
        .symlink_metadata(TEST_HELLO_WORLD_DEST)
        .file_type()
        .is_symlink());
    assert!(at
        .symlink_metadata(TEST_HELLO_WORLD_DEST_SYMLINK)
        .file_type()
        .is_symlink());
    assert_eq!(at.read(TEST_HELLO_WORLD_DEST_SYMLINK), "Hello, World!\n");
}

#[test]
fn test_cp_strip_trailing_slashes() {
    let (at, mut ucmd) = at_and_ucmd!();

    //using --strip-trailing-slashes option
    ucmd.arg("--strip-trailing-slashes")
        .arg(format!("{}/", TEST_HELLO_WORLD_SOURCE))
        .arg(TEST_HELLO_WORLD_DEST)
        .succeeds();

    // Check the content of the destination file that was copied.
    assert_eq!(at.read(TEST_HELLO_WORLD_DEST), "Hello, World!\n");
}

#[test]
fn test_cp_parents() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg("--parents")
        .arg(TEST_COPY_FROM_FOLDER_FILE)
        .arg(TEST_COPY_TO_FOLDER)
        .succeeds();

    assert_eq!(
        at.read(&format!(
            "{}/{}",
            TEST_COPY_TO_FOLDER, TEST_COPY_FROM_FOLDER_FILE
        )),
        "Hello, World!\n"
    );
}

#[test]
fn test_cp_parents_multiple_files() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg("--parents")
        .arg(TEST_COPY_FROM_FOLDER_FILE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg(TEST_COPY_TO_FOLDER)
        .succeeds();

    assert_eq!(
        at.read(&format!(
            "{}/{}",
            TEST_COPY_TO_FOLDER, TEST_COPY_FROM_FOLDER_FILE
        )),
        "Hello, World!\n"
    );
    assert_eq!(
        at.read(&format!(
            "{}/{}",
            TEST_COPY_TO_FOLDER, TEST_HOW_ARE_YOU_SOURCE
        )),
        "How are you?\n"
    );
}

#[test]
fn test_cp_parents_dest_not_directory() {
    new_ucmd!()
        .arg("--parents")
        .arg(TEST_COPY_FROM_FOLDER_FILE)
        .arg(TEST_HELLO_WORLD_DEST)
        .fails()
        .stderr_contains("with --parents, the destination must be a directory");
}

#[test]
#[cfg(unix)]
fn test_cp_writable_special_file_permissions() {
    new_ucmd!().arg("/dev/null").arg("/dev/zero").succeeds();
}

#[test]
#[cfg(unix)]
fn test_cp_issue_1665() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg("/dev/null").arg("foo").succeeds();
    assert!(at.file_exists("foo"));
    assert_eq!(at.read("foo"), "");
}

#[test]
fn test_cp_preserve_no_args() {
    new_ucmd!()
        .arg(TEST_COPY_FROM_FOLDER_FILE)
        .arg(TEST_HELLO_WORLD_DEST)
        .arg("--preserve")
        .succeeds();
}

#[test]
// For now, disable the test on Windows. Symlinks aren't well support on Windows.
// It works on Unix for now and it works locally when run from a powershell
#[cfg(not(windows))]
fn test_cp_deref_folder_to_folder() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let path_to_new_symlink = at.plus(TEST_COPY_FROM_FOLDER);

    at.symlink_file(
        &path_to_new_symlink
            .join(TEST_HELLO_WORLD_SOURCE)
            .to_string_lossy(),
        &path_to_new_symlink
            .join(TEST_HELLO_WORLD_SOURCE_SYMLINK)
            .to_string_lossy(),
    );

    //using -P -R option
    scene
        .ucmd()
        .arg("-L")
        .arg("-R")
        .arg("-v")
        .arg(TEST_COPY_FROM_FOLDER)
        .arg(TEST_COPY_TO_FOLDER_NEW)
        .succeeds();

    #[cfg(not(windows))]
    {
        let scene2 = TestScenario::new("ls");
        let result = scene2.cmd("ls").arg("-al").arg(path_to_new_symlink).run();
        println!("ls source {}", result.stdout_str());

        let path_to_new_symlink = at.subdir.join(TEST_COPY_TO_FOLDER_NEW);

        let result = scene2.cmd("ls").arg("-al").arg(path_to_new_symlink).run();
        println!("ls dest {}", result.stdout_str());
    }

    #[cfg(windows)]
    {
        // No action as this test is disabled but kept in case we want to
        // try to make it work in the future.
        let a = Command::new("cmd").args(&["/C", "dir"]).output();
        println!("output {:#?}", a);

        let a = Command::new("cmd")
            .args(&["/C", "dir", &at.as_string()])
            .output();
        println!("output {:#?}", a);

        let a = Command::new("cmd")
            .args(&["/C", "dir", path_to_new_symlink.to_str().unwrap()])
            .output();
        println!("output {:#?}", a);

        let path_to_new_symlink = at.subdir.join(TEST_COPY_FROM_FOLDER);

        let a = Command::new("cmd")
            .args(&["/C", "dir", path_to_new_symlink.to_str().unwrap()])
            .output();
        println!("output {:#?}", a);

        let path_to_new_symlink = at.subdir.join(TEST_COPY_TO_FOLDER_NEW);

        let a = Command::new("cmd")
            .args(&["/C", "dir", path_to_new_symlink.to_str().unwrap()])
            .output();
        println!("output {:#?}", a);
    }

    let path_to_new_symlink = at
        .subdir
        .join(TEST_COPY_TO_FOLDER_NEW)
        .join(TEST_HELLO_WORLD_SOURCE_SYMLINK);
    assert!(at.file_exists(
        &path_to_new_symlink
            .clone()
            .into_os_string()
            .into_string()
            .unwrap()
    ));

    let path_to_new = at.subdir.join(TEST_COPY_TO_FOLDER_NEW_FILE);

    // Check the content of the destination file that was copied.
    let path_to_check = path_to_new.to_str().unwrap();
    assert_eq!(at.read(path_to_check), "Hello, World!\n");

    // Check the content of the symlink
    let path_to_check = path_to_new_symlink.to_str().unwrap();
    assert_eq!(at.read(path_to_check), "Hello, World!\n");
}

#[test]
// For now, disable the test on Windows. Symlinks aren't well support on Windows.
// It works on Unix for now and it works locally when run from a powershell
#[cfg(not(windows))]
fn test_cp_no_deref_folder_to_folder() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let path_to_new_symlink = at.plus(TEST_COPY_FROM_FOLDER);

    at.symlink_file(
        &path_to_new_symlink
            .join(TEST_HELLO_WORLD_SOURCE)
            .to_string_lossy(),
        &path_to_new_symlink
            .join(TEST_HELLO_WORLD_SOURCE_SYMLINK)
            .to_string_lossy(),
    );

    //using -P -R option
    scene
        .ucmd()
        .arg("-P")
        .arg("-R")
        .arg("-v")
        .arg(TEST_COPY_FROM_FOLDER)
        .arg(TEST_COPY_TO_FOLDER_NEW)
        .succeeds();

    #[cfg(not(windows))]
    {
        let scene2 = TestScenario::new("ls");
        let result = scene2.cmd("ls").arg("-al").arg(path_to_new_symlink).run();
        println!("ls source {}", result.stdout_str());

        let path_to_new_symlink = at.subdir.join(TEST_COPY_TO_FOLDER_NEW);

        let result = scene2.cmd("ls").arg("-al").arg(path_to_new_symlink).run();
        println!("ls dest {}", result.stdout_str());
    }

    #[cfg(windows)]
    {
        // No action as this test is disabled but kept in case we want to
        // try to make it work in the future.
        let a = Command::new("cmd").args(&["/C", "dir"]).output();
        println!("output {:#?}", a);

        let a = Command::new("cmd")
            .args(&["/C", "dir", &at.as_string()])
            .output();
        println!("output {:#?}", a);

        let a = Command::new("cmd")
            .args(&["/C", "dir", path_to_new_symlink.to_str().unwrap()])
            .output();
        println!("output {:#?}", a);

        let path_to_new_symlink = at.subdir.join(TEST_COPY_FROM_FOLDER);

        let a = Command::new("cmd")
            .args(&["/C", "dir", path_to_new_symlink.to_str().unwrap()])
            .output();
        println!("output {:#?}", a);

        let path_to_new_symlink = at.subdir.join(TEST_COPY_TO_FOLDER_NEW);

        let a = Command::new("cmd")
            .args(&["/C", "dir", path_to_new_symlink.to_str().unwrap()])
            .output();
        println!("output {:#?}", a);
    }

    let path_to_new_symlink = at
        .subdir
        .join(TEST_COPY_TO_FOLDER_NEW)
        .join(TEST_HELLO_WORLD_SOURCE_SYMLINK);
    assert!(at.is_symlink(
        &path_to_new_symlink
            .clone()
            .into_os_string()
            .into_string()
            .unwrap()
    ));

    let path_to_new = at.subdir.join(TEST_COPY_TO_FOLDER_NEW_FILE);

    // Check the content of the destination file that was copied.
    let path_to_check = path_to_new.to_str().unwrap();
    assert_eq!(at.read(path_to_check), "Hello, World!\n");

    // Check the content of the symlink
    let path_to_check = path_to_new_symlink.to_str().unwrap();
    assert_eq!(at.read(path_to_check), "Hello, World!\n");
}

#[test]
#[cfg(target_os = "linux")]
fn test_cp_archive() {
    let (at, mut ucmd) = at_and_ucmd!();
    let ts = time::now().to_timespec();
    let previous = FileTime::from_unix_time(ts.sec as i64 - 3600, ts.nsec as u32);
    // set the file creation/modification an hour ago
    filetime::set_file_times(
        at.plus_as_string(TEST_HELLO_WORLD_SOURCE),
        previous,
        previous,
    )
    .unwrap();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--archive")
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");

    let metadata = std_fs::metadata(at.subdir.join(TEST_HELLO_WORLD_SOURCE)).unwrap();
    let creation = metadata.modified().unwrap();

    let metadata2 = std_fs::metadata(at.subdir.join(TEST_HOW_ARE_YOU_SOURCE)).unwrap();
    let creation2 = metadata2.modified().unwrap();

    let scene2 = TestScenario::new("ls");
    let result = scene2.cmd("ls").arg("-al").arg(at.subdir).succeeds();

    println!("ls dest {}", result.stdout_str());
    assert_eq!(creation, creation2);
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_cp_archive_recursive() {
    let (at, mut ucmd) = at_and_ucmd!();

    // creates
    // dir/1
    // dir/1.link => dir/1
    // dir/2
    // dir/2.link => dir/2

    let file_1 = at.subdir.join(TEST_COPY_TO_FOLDER).join("1");
    let file_1_link = at.subdir.join(TEST_COPY_TO_FOLDER).join("1.link");
    let file_2 = at.subdir.join(TEST_COPY_TO_FOLDER).join("2");
    let file_2_link = at.subdir.join(TEST_COPY_TO_FOLDER).join("2.link");

    at.touch(&file_1.to_string_lossy());
    at.touch(&file_2.to_string_lossy());

    at.symlink_file("1", &file_1_link.to_string_lossy());
    at.symlink_file("2", &file_2_link.to_string_lossy());

    ucmd.arg("--archive")
        .arg(TEST_COPY_TO_FOLDER)
        .arg(TEST_COPY_TO_FOLDER_NEW)
        .succeeds();

    let scene2 = TestScenario::new("ls");
    let result = scene2
        .cmd("ls")
        .arg("-al")
        .arg(&at.subdir.join(TEST_COPY_TO_FOLDER))
        .run();

    println!("ls dest {}", result.stdout_str());

    let result = scene2
        .cmd("ls")
        .arg("-al")
        .arg(&at.subdir.join(TEST_COPY_TO_FOLDER_NEW))
        .run();

    println!("ls dest {}", result.stdout_str());
    assert!(at.file_exists(
        &at.subdir
            .join(TEST_COPY_TO_FOLDER_NEW)
            .join("1")
            .to_string_lossy()
    ));
    assert!(at.file_exists(
        &at.subdir
            .join(TEST_COPY_TO_FOLDER_NEW)
            .join("2")
            .to_string_lossy()
    ));

    assert!(at.is_symlink(
        &at.subdir
            .join(TEST_COPY_TO_FOLDER_NEW)
            .join("1.link")
            .to_string_lossy()
    ));
    assert!(at.is_symlink(
        &at.subdir
            .join(TEST_COPY_TO_FOLDER_NEW)
            .join("2.link")
            .to_string_lossy()
    ));
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_preserve_timestamps() {
    let (at, mut ucmd) = at_and_ucmd!();
    let ts = time::now().to_timespec();
    let previous = FileTime::from_unix_time(ts.sec as i64 - 3600, ts.nsec as u32);
    // set the file creation/modification an hour ago
    filetime::set_file_times(
        at.plus_as_string(TEST_HELLO_WORLD_SOURCE),
        previous,
        previous,
    )
    .unwrap();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--preserve=timestamps")
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");

    let metadata = std_fs::metadata(at.subdir.join(TEST_HELLO_WORLD_SOURCE)).unwrap();
    let creation = metadata.modified().unwrap();

    let metadata2 = std_fs::metadata(at.subdir.join(TEST_HOW_ARE_YOU_SOURCE)).unwrap();
    let creation2 = metadata2.modified().unwrap();

    let scene2 = TestScenario::new("ls");
    let result = scene2.cmd("ls").arg("-al").arg(at.subdir).run();

    println!("ls dest {}", result.stdout_str());
    assert_eq!(creation, creation2);
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_no_preserve_timestamps() {
    let (at, mut ucmd) = at_and_ucmd!();
    let ts = time::now().to_timespec();
    let previous = FileTime::from_unix_time(ts.sec as i64 - 3600, ts.nsec as u32);
    // set the file creation/modification an hour ago
    filetime::set_file_times(
        at.plus_as_string(TEST_HELLO_WORLD_SOURCE),
        previous,
        previous,
    )
    .unwrap();
    sleep(Duration::from_secs(3));

    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--no-preserve=timestamps")
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds();

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");

    let metadata = std_fs::metadata(at.subdir.join(TEST_HELLO_WORLD_SOURCE)).unwrap();
    let creation = metadata.modified().unwrap();

    let metadata2 = std_fs::metadata(at.subdir.join(TEST_HOW_ARE_YOU_SOURCE)).unwrap();
    let creation2 = metadata2.modified().unwrap();

    let scene2 = TestScenario::new("ls");
    let result = scene2.cmd("ls").arg("-al").arg(at.subdir).run();

    println!("ls dest {}", result.stdout_str());
    println!("creation {:?} / {:?}", creation, creation2);

    assert_ne!(creation, creation2);
    let res = creation.elapsed().unwrap() - creation2.elapsed().unwrap();
    // Some margins with time check
    assert!(res.as_secs() > 3595);
    assert!(res.as_secs() < 3605);
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_target_file_dev_null() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "/dev/null";
    let file2 = "test_cp_target_file_file_i2";

    at.touch(file2);
    ucmd.arg(file1).arg(file2).succeeds().no_stderr();

    assert!(at.file_exists(file2));
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
fn test_cp_one_file_system() {
    use crate::common::util::AtPath;
    use walkdir::WalkDir;

    let scene = TestScenario::new(util_name!());

    // Test must be run as root (or with `sudo -E`)
    if scene.cmd("whoami").run().stdout_str() != "root\n" {
        return;
    }

    let at = scene.fixtures.clone();
    let at_src = AtPath::new(&at.plus(TEST_MOUNT_COPY_FROM_FOLDER));
    let at_dst = AtPath::new(&at.plus(TEST_COPY_TO_FOLDER_NEW));

    // Prepare the mount
    at_src.mkdir(TEST_MOUNT_MOUNTPOINT);
    let mountpoint_path = &at_src.plus_as_string(TEST_MOUNT_MOUNTPOINT);

    scene
        .cmd("mount")
        .arg("-t")
        .arg("tmpfs")
        .arg("-o")
        .arg("size=640k") // ought to be enough
        .arg("tmpfs")
        .arg(mountpoint_path)
        .succeeds();

    at_src.touch(TEST_MOUNT_OTHER_FILESYSTEM_FILE);

    // Begin testing -x flag
    scene
        .ucmd()
        .arg("-rx")
        .arg(TEST_MOUNT_COPY_FROM_FOLDER)
        .arg(TEST_COPY_TO_FOLDER_NEW)
        .succeeds();

    // Ditch the mount before the asserts
    scene.cmd("umount").arg(mountpoint_path).succeeds();

    assert!(!at_dst.file_exists(TEST_MOUNT_OTHER_FILESYSTEM_FILE));
    // Check if the other files were copied from the source folder hierarchy
    for entry in WalkDir::new(at_src.as_string()) {
        let entry = entry.unwrap();
        let relative_src = entry
            .path()
            .strip_prefix(at_src.as_string())
            .unwrap()
            .to_str()
            .unwrap();

        let ft = entry.file_type();
        match (ft.is_dir(), ft.is_file(), ft.is_symlink()) {
            (true, _, _) => assert!(at_dst.dir_exists(relative_src)),
            (_, true, _) => assert!(at_dst.file_exists(relative_src)),
            (_, _, _) => panic!(),
        }
    }
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
fn test_cp_reflink_always() {
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .arg("--reflink=always")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_EXISTING_FILE)
        .run();

    if result.succeeded() {
        // Check the content of the destination file
        assert_eq!(at.read(TEST_EXISTING_FILE), "Hello, World!\n");
    } else {
        // Older Linux versions do not support cloning.
    }
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
fn test_cp_reflink_auto() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg("--reflink=auto")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_EXISTING_FILE)
        .succeeds();

    // Check the content of the destination file
    assert_eq!(at.read(TEST_EXISTING_FILE), "Hello, World!\n");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
fn test_cp_reflink_never() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg("--reflink=never")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_EXISTING_FILE)
        .succeeds();

    // Check the content of the destination file
    assert_eq!(at.read(TEST_EXISTING_FILE), "Hello, World!\n");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
fn test_cp_reflink_bad() {
    let (_, mut ucmd) = at_and_ucmd!();
    let _result = ucmd
        .arg("--reflink=bad")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_EXISTING_FILE)
        .fails()
        .stderr_contains("invalid argument");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_reflink_insufficient_permission() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.make_file("unreadable")
        .set_permissions(PermissionsExt::from_mode(0o000))
        .unwrap();

    ucmd.arg("-r")
        .arg("--reflink=auto")
        .arg("unreadable")
        .arg(TEST_EXISTING_FILE)
        .fails()
        .stderr_only("cp: 'unreadable' -> 'existing_file.txt': Permission denied (os error 13)");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_closes_file_descriptors() {
    new_ucmd!()
        .arg("-r")
        .arg("--reflink=auto")
        .arg("dir_with_10_files/")
        .arg("dir_with_10_files_new/")
        .with_limit(Resource::NOFILE, 9, 9)
        .succeeds();
}

#[test]
fn test_copy_dir_symlink() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir");
    at.symlink_dir("dir", "dir-link");
    ucmd.args(&["-r", "dir-link", "copy"]).succeeds();
    assert_eq!(at.resolve_link("copy"), "dir");
}

#[test]
fn test_copy_dir_with_symlinks() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir");
    at.make_file("dir/file");

    TestScenario::new("ln")
        .ucmd()
        .arg("-sr")
        .arg(at.subdir.join("dir/file"))
        .arg(at.subdir.join("dir/file-link"))
        .succeeds();

    ucmd.args(&["-r", "dir", "copy"]).succeeds();
    assert_eq!(at.resolve_link("copy/file-link"), "file");
}

#[test]
#[cfg(not(windows))]
fn test_copy_symlink_force() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file");
    at.symlink_file("file", "file-link");
    at.touch("copy");

    ucmd.args(&["file-link", "copy", "-f", "--no-dereference"])
        .succeeds();
    assert_eq!(at.resolve_link("copy"), "file");
}

#[test]
#[cfg(all(unix, not(target_os = "freebsd")))]
fn test_no_preserve_mode() {
    use std::os::unix::prelude::MetadataExt;

    use uucore::mode::get_umask;

    const PERMS_ALL: u32 = 0o7777;

    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file");
    set_permissions(at.plus("file"), PermissionsExt::from_mode(PERMS_ALL)).unwrap();
    ucmd.arg("file")
        .arg("dest")
        .succeeds()
        .no_stderr()
        .no_stdout();
    let umask = get_umask();
    // remove sticky bit, setuid and setgid bit; apply umask
    let expected_perms = PERMS_ALL & !0o7000 & !umask;
    assert_eq!(
        at.plus("dest").metadata().unwrap().mode() & 0o7777,
        expected_perms
    );
}

#[test]
#[cfg(all(unix, not(target_os = "freebsd")))]
fn test_preserve_mode() {
    use std::os::unix::prelude::MetadataExt;

    const PERMS_ALL: u32 = 0o7777;

    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file");
    set_permissions(at.plus("file"), PermissionsExt::from_mode(PERMS_ALL)).unwrap();
    ucmd.arg("file")
        .arg("dest")
        .arg("-p")
        .succeeds()
        .no_stderr()
        .no_stdout();
    assert_eq!(
        at.plus("dest").metadata().unwrap().mode() & 0o7777,
        PERMS_ALL
    );
}

#[test]
fn test_canonicalize_symlink() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir");
    at.touch("dir/file");
    symlink_file("../dir/file", at.plus("dir/file-ln")).unwrap();
    ucmd.arg("dir/file-ln")
        .arg(".")
        .succeeds()
        .no_stderr()
        .no_stdout();
}

#[test]
fn test_copy_through_just_created_symlink() {
    for create_t in [true, false] {
        let (at, mut ucmd) = at_and_ucmd!();
        at.mkdir("a");
        at.mkdir("b");
        at.mkdir("c");
        #[cfg(unix)]
        fs::symlink("../t", at.plus("a/1")).unwrap();
        #[cfg(target_os = "windows")]
        symlink_file("../t", at.plus("a/1")).unwrap();
        at.touch("b/1");
        if create_t {
            at.touch("t");
        }
        ucmd.arg("--no-dereference")
            .arg("a/1")
            .arg("b/1")
            .arg("c")
            .fails()
            .stderr_only(if cfg!(not(target_os = "windows")) {
                "cp: will not copy 'b/1' through just-created symlink 'c/1'"
            } else {
                "cp: will not copy 'b/1' through just-created symlink 'c\\1'"
            });
    }
}

#[test]
fn test_copy_through_dangling_symlink() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file");
    at.symlink_file("nonexistent", "target");
    ucmd.arg("file")
        .arg("target")
        .fails()
        .stderr_only("cp: not writing through dangling symlink 'target'");
}

#[test]
#[cfg(unix)]
fn test_cp_archive_on_nonexistent_file() {
    new_ucmd!()
        .arg("-a")
        .arg(TEST_NONEXISTENT_FILE)
        .arg(TEST_EXISTING_FILE)
        .fails()
        .stderr_only(
            "cp: cannot stat 'nonexistent_file.txt': No such file or directory (os error 2)",
        );
}

#[test]
#[cfg(not(target_os = "android"))]
fn test_cp_link_backup() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file2");
    ucmd.arg("-l")
        .arg("-b")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("file2")
        .succeeds();

    assert!(at.file_exists("file2~"));
    assert_eq!(at.read("file2"), "Hello, World!\n");
}

#[test]
#[cfg(unix)]
fn test_cp_fifo() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkfifo("fifo");
    ucmd.arg("-r")
        .arg("fifo")
        .arg("fifo2")
        .succeeds()
        .no_stderr()
        .no_stdout();
    assert!(at.is_fifo("fifo2"));
}

#[test]
fn test_dir_recursive_copy() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("parent1");
    at.mkdir("parent2");
    at.mkdir("parent1/child");
    at.mkdir("parent2/child1");
    at.mkdir("parent2/child1/child2");
    at.mkdir("parent2/child1/child2/child3");

    // case-1: copy parent1 -> parent1: should fail
    scene
        .ucmd()
        .arg("-R")
        .arg("parent1")
        .arg("parent1")
        .fails()
        .stderr_contains("cannot copy a directory");
    // case-2: copy parent1 -> parent1/child should fail
    scene
        .ucmd()
        .arg("-R")
        .arg("parent1")
        .arg("parent1/child")
        .fails()
        .stderr_contains("cannot copy a directory");
    // case-3: copy parent1/child -> parent2 should pass
    scene
        .ucmd()
        .arg("-R")
        .arg("parent1/child")
        .arg("parent2")
        .succeeds();
    // case-4: copy parent2/child1/ -> parent2/child1/child2/child3
    scene
        .ucmd()
        .arg("-R")
        .arg("parent2/child1/")
        .arg("parent2/child1/child2/child3")
        .fails()
        .stderr_contains("cannot copy a directory");
}

#[test]
fn test_cp_dir_vs_file() {
    new_ucmd!()
        .arg("-R")
        .arg(TEST_COPY_FROM_FOLDER)
        .arg(TEST_EXISTING_FILE)
        .fails()
        .stderr_only("cp: cannot overwrite non-directory with directory");
}

#[test]
fn test_cp_overriding_arguments() {
    let s = TestScenario::new(util_name!());
    s.fixtures.touch("file1");
    for (arg1, arg2) in [
        #[cfg(not(windows))]
        ("--remove-destination", "--force"),
        #[cfg(not(windows))]
        ("--force", "--remove-destination"),
        ("--interactive", "--no-clobber"),
        ("--link", "--symbolic-link"),
        #[cfg(not(target_os = "android"))]
        ("--symbolic-link", "--link"),
        ("--dereference", "--no-dereference"),
        ("--no-dereference", "--dereference"),
    ] {
        s.ucmd()
            .arg(arg1)
            .arg(arg2)
            .arg("file1")
            .arg("file2")
            .succeeds();
        s.fixtures.remove("file2");
    }
}
