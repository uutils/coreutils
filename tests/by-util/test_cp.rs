// spell-checker:ignore (flags) reflink (fs) tmpfs (linux) rlimit Rlim NOFILE clob btrfs ROOTDIR USERDIR procfs outfile

use crate::common::util::TestScenario;
#[cfg(not(windows))]
use std::fs::set_permissions;

#[cfg(not(windows))]
use std::os::unix::fs;

#[cfg(all(unix, not(target_os = "freebsd")))]
use std::os::unix::fs::MetadataExt;
#[cfg(all(unix, not(target_os = "freebsd")))]
use std::os::unix::fs::PermissionsExt;
#[cfg(windows)]
use std::os::windows::fs::symlink_file;
#[cfg(not(windows))]
use std::path::Path;
#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[cfg(any(target_os = "linux", target_os = "android"))]
use filetime::FileTime;
#[cfg(any(target_os = "linux", target_os = "android"))]
use rlimit::Resource;
#[cfg(target_os = "linux")]
use std::ffi::OsString;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::fs as std_fs;
use std::thread::sleep;
use std::time::Duration;

#[cfg(any(target_os = "linux", target_os = "android"))]
#[cfg(feature = "truncate")]
use crate::common::util::PATH;

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

/// Assert that mode, ownership, and permissions of two metadata objects match.
#[cfg(all(not(windows), not(target_os = "freebsd")))]
macro_rules! assert_metadata_eq {
    ($m1:expr, $m2:expr) => {{
        assert_eq!($m1.mode(), $m2.mode(), "mode is different");
        assert_eq!($m1.uid(), $m2.uid(), "uid is different");
        assert_eq!($m1.atime(), $m2.atime(), "atime is different");
        assert_eq!(
            $m1.atime_nsec(),
            $m2.atime_nsec(),
            "atime_nsec is different"
        );
        assert_eq!($m1.mtime(), $m2.mtime(), "mtime is different");
        assert_eq!(
            $m1.mtime_nsec(),
            $m2.mtime_nsec(),
            "mtime_nsec is different"
        );
    }};
}

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
    assert!(!at.file_exists(format!("{TEST_EXISTING_FILE}~")));
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
        .stderr_contains(format!("'{TEST_HOW_ARE_YOU_SOURCE}' is not a directory"));
}

#[test]
fn test_cp_arg_update_interactive() {
    new_ucmd!()
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg("-i")
        .arg("--update")
        .succeeds()
        .no_stdout()
        .no_stderr();
}

#[test]
fn test_cp_arg_update_interactive_error() {
    new_ucmd!()
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg("-i")
        .fails()
        .no_stdout();
}

#[test]
fn test_cp_arg_interactive() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("a");
    at.touch("b");
    ucmd.args(&["-i", "a", "b"])
        .pipe_in("N\n")
        .fails()
        .no_stdout()
        .stderr_is("cp: overwrite 'b'? ");
}

#[test]
fn test_cp_arg_interactive_update() {
    // -u -i won't show the prompt to validate the override or not
    // Therefore, the error code will be 0
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("a");
    at.touch("b");
    ucmd.args(&["-i", "-u", "a", "b"])
        .pipe_in("N\n")
        .succeeds()
        .no_stdout();
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
        .succeeds();

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
        at.read(&format!("{TEST_HOW_ARE_YOU_SOURCE}~")),
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
        at.read(&format!("{TEST_HOW_ARE_YOU_SOURCE}~")),
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
        at.read(&format!("{TEST_HOW_ARE_YOU_SOURCE}~")),
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
        at.read(&format!("{TEST_HOW_ARE_YOU_SOURCE}.bak")),
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
        at.read(&format!("{TEST_HOW_ARE_YOU_SOURCE}-v")),
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
        at.read(&format!("{TEST_HOW_ARE_YOU_SOURCE}{suffix}")),
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
        at.read(&format!("{TEST_HOW_ARE_YOU_SOURCE}.~1~")),
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
        at.read(&format!("{TEST_HOW_ARE_YOU_SOURCE}.~1~")),
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
        at.read(&format!("{TEST_HOW_ARE_YOU_SOURCE}~")),
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
        at.read(&format!("{TEST_HOW_ARE_YOU_SOURCE}~")),
        "How are you?\n"
    );
}

#[test]
fn test_cp_numbered_if_existing_backup_existing() {
    let (at, mut ucmd) = at_and_ucmd!();
    let existing_backup = &format!("{TEST_HOW_ARE_YOU_SOURCE}.~1~");
    at.touch(existing_backup);

    ucmd.arg("--backup=existing")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(TEST_HOW_ARE_YOU_SOURCE));
    assert!(at.file_exists(existing_backup));
    assert_eq!(
        at.read(&format!("{TEST_HOW_ARE_YOU_SOURCE}.~2~")),
        "How are you?\n"
    );
}

#[test]
fn test_cp_numbered_if_existing_backup_nil() {
    let (at, mut ucmd) = at_and_ucmd!();
    let existing_backup = &format!("{TEST_HOW_ARE_YOU_SOURCE}.~1~");

    at.touch(existing_backup);
    ucmd.arg("--backup=nil")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .succeeds()
        .no_stderr();

    assert!(at.file_exists(TEST_HOW_ARE_YOU_SOURCE));
    assert!(at.file_exists(existing_backup));
    assert_eq!(
        at.read(&format!("{TEST_HOW_ARE_YOU_SOURCE}.~2~")),
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
        at.read(&format!("{TEST_HOW_ARE_YOU_SOURCE}~")),
        "How are you?\n"
    );
}

#[test]
fn test_cp_backup_simple_protect_source() {
    let (at, mut ucmd) = at_and_ucmd!();
    let source = format!("{TEST_HELLO_WORLD_SOURCE}~");
    at.touch(&source);
    ucmd.arg("--backup=simple")
        .arg(&source)
        .arg(TEST_HELLO_WORLD_SOURCE)
        .fails()
        .stderr_only(format!(
            "cp: backing up '{TEST_HELLO_WORLD_SOURCE}' might destroy source;  '{source}' not copied\n",
        ));

    assert_eq!(at.read(TEST_HELLO_WORLD_SOURCE), "Hello, World!\n");
    assert_eq!(at.read(&source), "");
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
        at.read(&format!("{TEST_HOW_ARE_YOU_SOURCE}~")),
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
    assert!(!at.file_exists(format!("{TEST_HOW_ARE_YOU_SOURCE}~")));
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
    assert!(!at.file_exists(format!("{TEST_HOW_ARE_YOU_SOURCE}~")));
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
        path_to_new_symlink
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
        .arg(format!("{TEST_HELLO_WORLD_SOURCE}/"))
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
            "{TEST_COPY_TO_FOLDER}/{TEST_COPY_FROM_FOLDER_FILE}"
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
            "{TEST_COPY_TO_FOLDER}/{TEST_COPY_FROM_FOLDER_FILE}"
        )),
        "Hello, World!\n"
    );
    assert_eq!(
        at.read(&format!("{TEST_COPY_TO_FOLDER}/{TEST_HOW_ARE_YOU_SOURCE}")),
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
    let (at, mut ucmd) = at_and_ucmd!();
    let src_file = "a";
    let dst_file = "b";

    // Prepare the source file
    at.touch(src_file);
    #[cfg(unix)]
    at.set_mode(src_file, 0o0500);

    // Copy
    ucmd.arg(src_file)
        .arg(dst_file)
        .arg("--preserve")
        .succeeds();

    #[cfg(all(unix, not(target_os = "freebsd")))]
    {
        // Assert that the mode, ownership, and timestamps are preserved
        // NOTICE: the ownership is not modified on the src file, because that requires root permissions
        let metadata_src = at.metadata(src_file);
        let metadata_dst = at.metadata(dst_file);
        assert_metadata_eq!(metadata_src, metadata_dst);
    }
}

#[test]
fn test_cp_preserve_all() {
    let (at, mut ucmd) = at_and_ucmd!();
    let src_file = "a";
    let dst_file = "b";

    // Prepare the source file
    at.touch(src_file);
    #[cfg(unix)]
    at.set_mode(src_file, 0o0500);

    // TODO: create a destination that does not allow copying of xattr and context
    // Copy
    ucmd.arg(src_file)
        .arg(dst_file)
        .arg("--preserve=all")
        .succeeds();

    #[cfg(all(unix, not(target_os = "freebsd")))]
    {
        // Assert that the mode, ownership, and timestamps are preserved
        // NOTICE: the ownership is not modified on the src file, because that requires root permissions
        let metadata_src = at.metadata(src_file);
        let metadata_dst = at.metadata(dst_file);
        assert_metadata_eq!(metadata_src, metadata_dst);
    }
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_cp_preserve_xattr() {
    let (at, mut ucmd) = at_and_ucmd!();
    let src_file = "a";
    let dst_file = "b";

    // Prepare the source file
    at.touch(src_file);
    #[cfg(unix)]
    at.set_mode(src_file, 0o0500);

    // Sleep so that the time stats are different
    sleep(Duration::from_secs(1));

    // TODO: create a destination that does not allow copying of xattr and context
    // Copy
    ucmd.arg(src_file)
        .arg(dst_file)
        .arg("--preserve=xattr")
        .succeeds();

    // FIXME: macos copy keeps the original mtime
    #[cfg(not(any(target_os = "freebsd", target_os = "macos")))]
    {
        // Assert that the mode, ownership, and timestamps are *NOT* preserved
        // NOTICE: the ownership is not modified on the src file, because that requires root permissions
        let metadata_src = at.metadata(src_file);
        let metadata_dst = at.metadata(dst_file);
        assert_ne!(metadata_src.mtime(), metadata_dst.mtime());
        // TODO: verify access time as well. It shouldn't change, however, it does change in this test.
    }
}

#[test]
#[cfg(all(target_os = "linux", not(feature = "feat_selinux")))]
fn test_cp_preserve_all_context_fails_on_non_selinux() {
    new_ucmd!()
        .arg(TEST_COPY_FROM_FOLDER_FILE)
        .arg(TEST_HELLO_WORLD_DEST)
        .arg("--preserve=all,context")
        .fails();
}

#[test]
#[cfg(any(target_os = "android"))]
fn test_cp_preserve_xattr_fails_on_android() {
    // Because of the SELinux extended attributes used on Android, trying to copy extended
    // attributes has to fail in this case, since we specify `--preserve=xattr` and this puts it
    // into the required attributes
    new_ucmd!()
        .arg(TEST_COPY_FROM_FOLDER_FILE)
        .arg(TEST_HELLO_WORLD_DEST)
        .arg("--preserve=xattr")
        .fails();
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
        path_to_new_symlink
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
    let ts = time::OffsetDateTime::now_utc();
    let previous = FileTime::from_unix_time(ts.unix_timestamp() - 3600, ts.nanosecond());
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

    at.touch(file_1);
    at.touch(file_2);

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
    assert!(at.file_exists(at.subdir.join(TEST_COPY_TO_FOLDER_NEW).join("1")));
    assert!(at.file_exists(at.subdir.join(TEST_COPY_TO_FOLDER_NEW).join("2")));

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
    let ts = time::OffsetDateTime::now_utc();
    let previous = FileTime::from_unix_time(ts.unix_timestamp() - 3600, ts.nanosecond());
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
    let ts = time::OffsetDateTime::now_utc();
    let previous = FileTime::from_unix_time(ts.unix_timestamp() - 3600, ts.nanosecond());
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
    println!("creation {creation:?} / {creation2:?}");

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
fn test_cp_reflink_none() {
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .arg("--reflink")
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
        .stderr_contains("error: invalid value 'bad' for '--reflink[=<WHEN>]'");
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
        .stderr_only("cp: 'unreadable' -> 'existing_file.txt': Permission denied (os error 13)\n");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_closes_file_descriptors() {
    use procfs::process::Process;
    let me = Process::myself().unwrap();

    // The test suite runs in parallel, we have pipe, sockets
    // opened by other tests.
    // So, we take in account the various fd to increase the limit
    let number_file_already_opened: u64 = me.fd_count().unwrap().try_into().unwrap();
    let limit_fd: u64 = number_file_already_opened + 9;

    // For debugging purposes:
    #[cfg(not(target_os = "android"))]
    for f in me.fd().unwrap() {
        let fd = f.unwrap();
        println!("{:?} {:?}", fd, fd.mode());
    }

    new_ucmd!()
        .arg("-r")
        .arg("--reflink=auto")
        .arg("dir_with_10_files/")
        .arg("dir_with_10_files_new/")
        .limit(Resource::NOFILE, limit_fd, limit_fd)
        .succeeds();
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_cp_sparse_never_empty() {
    let (at, mut ucmd) = at_and_ucmd!();

    const BUFFER_SIZE: usize = 4096 * 4;
    let buf: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

    at.make_file("src_file1");
    at.write_bytes("src_file1", &buf);

    ucmd.args(&["--sparse=never", "src_file1", "dst_file_non_sparse"])
        .succeeds();
    assert_eq!(at.read_bytes("dst_file_non_sparse"), buf);
    assert_eq!(
        at.metadata("dst_file_non_sparse").blocks() * 512,
        buf.len() as u64
    );
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_cp_sparse_always_empty() {
    let (at, mut ucmd) = at_and_ucmd!();

    const BUFFER_SIZE: usize = 4096 * 4;
    let buf: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

    at.make_file("src_file1");
    at.write_bytes("src_file1", &buf);

    ucmd.args(&["--sparse=always", "src_file1", "dst_file_sparse"])
        .succeeds();

    assert_eq!(at.read_bytes("dst_file_sparse"), buf);
    assert_eq!(at.metadata("dst_file_sparse").blocks(), 0);
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_cp_sparse_always_non_empty() {
    let (at, mut ucmd) = at_and_ucmd!();

    const BUFFER_SIZE: usize = 4096 * 16 + 3;
    let mut buf: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
    let blocks_to_touch = [buf.len() / 3, 2 * (buf.len() / 3)];

    for i in blocks_to_touch {
        buf[i] = b'x';
    }

    at.make_file("src_file1");
    at.write_bytes("src_file1", &buf);

    ucmd.args(&["--sparse=always", "src_file1", "dst_file_sparse"])
        .succeeds();

    let touched_block_count =
        blocks_to_touch.len() as u64 * at.metadata("dst_file_sparse").blksize() / 512;

    assert_eq!(at.read_bytes("dst_file_sparse"), buf);
    assert_eq!(at.metadata("dst_file_sparse").blocks(), touched_block_count);
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_cp_sparse_invalid_option() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.make_file("src_file1");

    ucmd.args(&["--sparse=invalid", "src_file1", "dst_file"])
        .fails();
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_cp_sparse_always_reflink_always() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.make_file("src_file1");

    ucmd.args(&[
        "--sparse=always",
        "--reflink=always",
        "src_file1",
        "dst_file",
    ])
    .fails();
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_cp_sparse_never_reflink_always() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.make_file("src_file1");

    ucmd.args(&[
        "--sparse=never",
        "--reflink=always",
        "src_file1",
        "dst_file",
    ])
    .fails();
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[cfg(feature = "truncate")]
#[test]
fn test_cp_reflink_always_override() {
    let scene = TestScenario::new(util_name!());

    const DISK: &str = "disk.img";
    const ROOTDIR: &str = "disk_root/";
    const USERDIR: &str = "dir/";
    const MOUNTPOINT: &str = "mountpoint/";

    let src1_path: &str = &vec![MOUNTPOINT, USERDIR, "src1"].concat();
    let src2_path: &str = &vec![MOUNTPOINT, USERDIR, "src2"].concat();
    let dst_path: &str = &vec![MOUNTPOINT, USERDIR, "dst"].concat();

    scene.fixtures.mkdir(ROOTDIR);
    scene.fixtures.mkdir(vec![ROOTDIR, USERDIR].concat());

    // Setup:
    // Because neither `mkfs.btrfs` not btrfs `mount` options allow us to have a mountpoint owned
    // by a non-root user, we want the following directory structure:
    //
    // uid  | path
    // ---------------------------
    // user | .
    // root | └── mountpoint
    // user |     └── dir
    // user |         ├── src1
    // user |         └── src2

    scene
        .ccmd("truncate")
        .args(&["-s", "128M", DISK])
        .succeeds();

    if !scene
        .cmd("env")
        .env("PATH", PATH)
        .args(&["mkfs.btrfs", "--rootdir", ROOTDIR, DISK])
        .run()
        .succeeded()
    {
        print!("Test skipped; couldn't make btrfs disk image");
        return;
    }

    scene.fixtures.mkdir(MOUNTPOINT);

    let mount = scene
        .cmd("sudo")
        .env("PATH", PATH)
        .args(&["-E", "--non-interactive", "mount", DISK, MOUNTPOINT])
        .run();

    if !mount.succeeded() {
        print!("Test skipped; requires root user");
        return;
    }

    scene.fixtures.make_file(src1_path);
    scene.fixtures.write_bytes(src1_path, &[0x64; 8192]);

    scene.fixtures.make_file(src2_path);
    scene.fixtures.write(src2_path, "other data");

    scene
        .ucmd()
        .args(&["--reflink=always", src1_path, dst_path])
        .succeeds();

    scene
        .ucmd()
        .args(&["--reflink=always", src2_path, dst_path])
        .succeeds();

    scene
        .cmd("sudo")
        .env("PATH", PATH)
        .args(&["-E", "--non-interactive", "umount", MOUNTPOINT])
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
#[cfg(not(target_os = "freebsd"))] // FIXME: fix this test for FreeBSD
#[cfg(feature = "ln")]
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
    at.relative_symlink_file("../dir/file", "dir/file-ln");
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
        at.relative_symlink_file("../t", "a/1");
        at.touch("b/1");
        at.write("b/1", "hello");
        if create_t {
            at.touch("t");
            at.write("t", "world");
        }
        ucmd.arg("--no-dereference")
            .arg("a/1")
            .arg("b/1")
            .arg("c")
            .fails()
            .stderr_only(if cfg!(not(target_os = "windows")) {
                "cp: will not copy 'b/1' through just-created symlink 'c/1'\n"
            } else {
                "cp: will not copy 'b/1' through just-created symlink 'c\\1'\n"
            });
        if create_t {
            assert_eq!(at.read("a/1"), "world");
        }
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
        .stderr_only("cp: not writing through dangling symlink 'target'\n");
}

#[test]
fn test_copy_through_dangling_symlink_no_dereference() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.symlink_file("no-such-file", "dangle");
    ucmd.arg("-P")
        .arg("dangle")
        .arg("d2")
        .succeeds()
        .no_stderr()
        .no_stdout();
}

/// Test for copying a dangling symbolic link and its permissions.
#[cfg(not(target_os = "freebsd"))] // FIXME: fix this test for FreeBSD
#[test]
fn test_copy_through_dangling_symlink_no_dereference_permissions() {
    let (at, mut ucmd) = at_and_ucmd!();
    //               target name    link name
    at.symlink_file("no-such-file", "dangle");
    // to check if access time and modification time didn't change
    sleep(Duration::from_millis(5000));
    //          don't dereference the link
    //           |    copy permissions, too
    //           |      |    from the link
    //           |      |      |     to new file d2
    //           |      |      |        |
    //           V      V      V        V
    ucmd.args(&["-P", "-p", "dangle", "d2"])
        .succeeds()
        .no_stderr()
        .no_stdout();
    assert!(at.symlink_exists("d2"), "symlink wasn't created");

    // `-p` means `--preserve=mode,ownership,timestamps`
    #[cfg(all(unix, not(target_os = "freebsd")))]
    {
        let metadata1 = at.symlink_metadata("dangle");
        let metadata2 = at.symlink_metadata("d2");
        assert_metadata_eq!(metadata1, metadata2);
    }
}

#[test]
fn test_copy_through_dangling_symlink_no_dereference_2() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file");
    at.symlink_file("nonexistent", "target");
    ucmd.args(&["-P", "file", "target"])
        .fails()
        .stderr_only("cp: not writing through dangling symlink 'target'\n");
}

/// Test that copy through a dangling symbolic link fails, even with --force.
#[test]
#[cfg(not(windows))]
fn test_copy_through_dangling_symlink_force() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("src");
    at.symlink_file("no-such-file", "dest");
    ucmd.args(&["--force", "src", "dest"])
        .fails()
        .stderr_only("cp: not writing through dangling symlink 'dest'\n");
    assert!(!at.file_exists("dest"));
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
            "cp: cannot stat 'nonexistent_file.txt': No such file or directory (os error 2)\n",
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
        .stderr_only("cp: cannot overwrite non-directory with directory\n");
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

#[test]
fn test_copy_no_dereference_1() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("a");
    at.mkdir("b");
    at.touch("a/foo");
    at.write("a/foo", "bar");
    at.relative_symlink_file("../a/foo", "b/foo");
    ucmd.args(&["-P", "a/foo", "b"]).fails();
}

#[test]
fn test_abuse_existing() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("a");
    at.mkdir("b");
    at.mkdir("c");
    at.relative_symlink_file("../t", "a/1");
    at.touch("b/1");
    at.write("b/1", "hello");
    at.relative_symlink_file("../t", "c/1");
    at.touch("t");
    at.write("t", "i");
    ucmd.args(&["-dR", "a/1", "b/1", "c"])
        .fails()
        .stderr_contains(format!(
            "will not copy 'b/1' through just-created symlink 'c{}1'",
            if cfg!(windows) { "\\" } else { "/" }
        ));
    assert_eq!(at.read("t"), "i");
}

#[test]
fn test_copy_same_symlink_no_dereference() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.relative_symlink_file("t", "a");
    at.relative_symlink_file("t", "b");
    at.touch("t");
    ucmd.args(&["-d", "a", "b"]).succeeds();
}

#[test]
fn test_copy_same_symlink_no_dereference_dangling() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.relative_symlink_file("t", "a");
    at.relative_symlink_file("t", "b");
    ucmd.args(&["-d", "a", "b"]).succeeds();
}

// TODO: enable for Android, when #3477 solved
#[cfg(not(any(windows, target_os = "android")))]
#[test]
fn test_cp_parents_2_dirs() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir_all("a/b/c");
    at.mkdir("d");
    ucmd.args(&["-a", "--parents", "a/b/c", "d"])
        .succeeds()
        .no_stderr()
        .no_stdout();
    assert!(at.dir_exists("d/a/b/c"));
}

#[test]
fn test_cp_parents_2() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir_all("a/b");
    at.touch("a/b/c");
    at.mkdir("d");
    #[cfg(not(windows))]
    let expected_stdout = "a -> d/a\na/b -> d/a/b\n'a/b/c' -> 'd/a/b/c'\n";
    #[cfg(windows)]
    let expected_stdout = "a -> d\\a\na/b -> d\\a/b\n'a/b/c' -> 'd\\a/b/c'\n";
    ucmd.args(&["--verbose", "--parents", "a/b/c", "d"])
        .succeeds()
        .stdout_only(expected_stdout);
    assert!(at.file_exists("d/a/b/c"));
}

#[test]
fn test_cp_parents_2_link() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir_all("a/b");
    at.touch("a/b/c");
    at.mkdir("d");
    at.relative_symlink_file("b", "a/link");
    #[cfg(not(windows))]
    let expected_stdout = "a -> d/a\na/link -> d/a/link\n'a/link/c' -> 'd/a/link/c'\n";
    #[cfg(windows)]
    let expected_stdout = "a -> d\\a\na/link -> d\\a/link\n'a/link/c' -> 'd\\a/link/c'\n";
    ucmd.args(&["--verbose", "--parents", "a/link/c", "d"])
        .succeeds()
        .stdout_only(expected_stdout);
    assert!(at.dir_exists("d/a/link"));
    assert!(!at.symlink_exists("d/a/link"));
    assert!(at.file_exists("d/a/link/c"));
}

#[test]
fn test_cp_parents_2_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir_all("a/b/c");
    at.mkdir("d");
    #[cfg(not(windows))]
    let expected_stdout = "a -> d/a\na/b -> d/a/b\n'a/b/c' -> 'd/a/b/c'\n";
    #[cfg(windows)]
    let expected_stdout = "a -> d\\a\na/b -> d\\a/b\n'a/b/c' -> 'd\\a/b\\c'\n";
    ucmd.args(&["--verbose", "-r", "--parents", "a/b/c", "d"])
        .succeeds()
        .stdout_only(expected_stdout);
    assert!(at.dir_exists("d/a/b/c"));
}

#[test]
fn test_cp_parents_2_deep_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir_all("a/b/c");
    at.mkdir_all("d/e");
    #[cfg(not(windows))]
    let expected_stdout = "a -> d/e/a\na/b -> d/e/a/b\n'a/b/c' -> 'd/e/a/b/c'\n";
    #[cfg(windows)]
    let expected_stdout = "a -> d/e\\a\na/b -> d/e\\a/b\n'a/b/c' -> 'd/e\\a/b\\c'\n";
    ucmd.args(&["--verbose", "-r", "--parents", "a/b/c", "d/e"])
        .succeeds()
        .stdout_only(expected_stdout);
    assert!(at.dir_exists("d/e/a/b/c"));
}

#[test]
fn test_cp_copy_symlink_contents_recursive() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("src-dir");
    at.mkdir("dest-dir");
    at.touch("f");
    at.write("f", "f");
    at.relative_symlink_file("f", "slink");
    at.relative_symlink_file("no-file", &path_concat!("src-dir", "slink"));
    ucmd.args(&["-H", "-R", "slink", "src-dir", "dest-dir"])
        .succeeds();
    assert!(at.dir_exists("src-dir"));
    assert!(at.dir_exists("dest-dir"));
    assert!(at.dir_exists(&path_concat!("dest-dir", "src-dir")));
    let regular_file = path_concat!("dest-dir", "slink");
    assert!(!at.symlink_exists(&regular_file) && at.file_exists(&regular_file));
    assert_eq!(at.read(&regular_file), "f");
}

#[test]
fn test_cp_mode_symlink() {
    for from in ["file", "slink", "slink2"] {
        let (at, mut ucmd) = at_and_ucmd!();
        at.touch("file");
        at.write("file", "f");
        at.relative_symlink_file("file", "slink");
        at.relative_symlink_file("slink", "slink2");
        ucmd.args(&["-s", "-L", from, "z"]).succeeds();
        assert!(at.symlink_exists("z"));
        assert_eq!(at.read_symlink("z"), from);
    }
}

// Android doesn't allow creating hard links
#[cfg(not(target_os = "android"))]
#[test]
fn test_cp_mode_hardlink() {
    for from in ["file", "slink", "slink2"] {
        let (at, mut ucmd) = at_and_ucmd!();
        at.touch("file");
        at.write("file", "f");
        at.relative_symlink_file("file", "slink");
        at.relative_symlink_file("slink", "slink2");
        ucmd.args(&["--link", "-L", from, "z"]).succeeds();
        assert!(at.file_exists("z") && !at.symlink_exists("z"));
        assert_eq!(at.read("z"), "f");
        // checking that it's the same hard link
        at.append("z", "g");
        assert_eq!(at.read("file"), "fg");
    }
}

// Android doesn't allow creating hard links
#[cfg(not(target_os = "android"))]
#[test]
fn test_cp_mode_hardlink_no_dereference() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file");
    at.write("file", "f");
    at.relative_symlink_file("file", "slink");
    at.relative_symlink_file("slink", "slink2");
    ucmd.args(&["--link", "-P", "slink2", "z"]).succeeds();
    assert!(at.symlink_exists("z"));
    assert_eq!(at.read_symlink("z"), "slink");
}

#[test]
fn test_remove_destination_symbolic_link_loop() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.symlink_file("loop", "loop");
    at.plus("loop");
    at.touch("f");
    ucmd.args(&["--remove-destination", "f", "loop"])
        .succeeds()
        .no_stdout()
        .no_stderr();
    assert!(at.file_exists("loop"));
}

/// Test that copying a directory to itself is disallowed.
#[test]
fn test_copy_directory_to_itself_disallowed() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("d");
    #[cfg(not(windows))]
    let expected = "cp: cannot copy a directory, 'd', into itself, 'd/d'\n";
    #[cfg(windows)]
    let expected = "cp: cannot copy a directory, 'd', into itself, 'd\\d'\n";
    ucmd.args(&["-R", "d", "d"]).fails().stderr_only(expected);
}

/// Test that copying a nested directory to itself is disallowed.
#[test]
fn test_copy_nested_directory_to_itself_disallowed() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("a");
    at.mkdir("a/b");
    at.mkdir("a/b/c");
    #[cfg(not(windows))]
    let expected = "cp: cannot copy a directory, 'a/b', into itself, 'a/b/c/b'\n";
    #[cfg(windows)]
    let expected = "cp: cannot copy a directory, 'a/b', into itself, 'a/b/c\\b'\n";
    ucmd.args(&["-R", "a/b", "a/b/c"])
        .fails()
        .stderr_only(expected);
}

/// Test for preserving permissions when copying a directory.
#[cfg(all(not(windows), not(target_os = "freebsd")))]
#[test]
fn test_copy_dir_preserve_permissions() {
    // Create a directory that has some non-default permissions.
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("d1");
    at.set_mode("d1", 0o0500);

    // Copy the directory, preserving those permissions.
    //
    //         preserve permissions (mode, ownership, timestamps)
    //            |    copy directories recursively
    //            |      |   from this source directory
    //            |      |    |   to this destination
    //            |      |    |     |
    //            V      V    V     V
    ucmd.args(&["-p", "-R", "d1", "d2"])
        .succeeds()
        .no_stderr()
        .no_stdout();
    assert!(at.dir_exists("d2"));

    // Assert that the permissions are preserved.
    let metadata1 = at.metadata("d1");
    let metadata2 = at.metadata("d2");
    assert_metadata_eq!(metadata1, metadata2);
}

/// Test for preserving permissions when copying a directory, even in
/// the face of an inaccessible file in that directory.
#[cfg(all(not(windows), not(target_os = "freebsd")))]
#[test]
fn test_copy_dir_preserve_permissions_inaccessible_file() {
    // Create a directory that has some non-default permissions and
    // contains an inaccessible file.
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("d1");
    at.touch("d1/f");
    at.set_mode("d1/f", 0);
    at.set_mode("d1", 0o0500);

    // Copy the directory, preserving those permissions. There should
    // be an error message that the file `d1/f` is inaccessible.
    //
    //         preserve permissions (mode, ownership, timestamps)
    //            |    copy directories recursively
    //            |      |   from this source directory
    //            |      |    |   to this destination
    //            |      |    |     |
    //            V      V    V     V
    ucmd.args(&["-p", "-R", "d1", "d2"])
        .fails()
        .code_is(1)
        .stderr_only("cp: cannot open 'd1/f' for reading: Permission denied\n");
    assert!(at.dir_exists("d2"));
    assert!(!at.file_exists("d2/f"));

    // Assert that the permissions are preserved.
    let metadata1 = at.metadata("d1");
    let metadata2 = at.metadata("d2");
    assert_metadata_eq!(metadata1, metadata2);
}

/// Test that copying file to itself with backup fails.
#[test]
fn test_same_file_backup() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("f");
    ucmd.args(&["--backup", "f", "f"])
        .fails()
        .stderr_only("cp: 'f' and 'f' are the same file\n");
    assert!(!at.file_exists("f~"));
}

/// Test that copying file to itself with forced backup succeeds.
#[cfg(not(windows))]
#[test]
fn test_same_file_force() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("f");
    ucmd.args(&["--force", "f", "f"])
        .fails()
        .stderr_only("cp: 'f' and 'f' are the same file\n");
    assert!(!at.file_exists("f~"));
}

/// Test that copying file to itself with forced backup succeeds.
#[cfg(all(not(windows)))]
#[test]
fn test_same_file_force_backup() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("f");
    ucmd.args(&["--force", "--backup", "f", "f"])
        .succeeds()
        .no_stdout()
        .no_stderr();
    assert!(at.file_exists("f~"));
}

/// Test for copying the contents of a FIFO as opposed to the FIFO object itself.
#[cfg(all(unix, not(target_os = "freebsd")))]
#[test]
fn test_copy_contents_fifo() {
    // TODO this test should work on FreeBSD, but the command was
    // causing an error:
    //
    // cp: 'fifo' -> 'outfile': the source path is neither a regular file nor a symlink to a regular file
    //
    // the underlying `std::fs:copy` doesn't support copying fifo on freeBSD
    let scenario = TestScenario::new(util_name!());
    let at = &scenario.fixtures;

    // Start the `cp` process, reading the contents of `fifo` and
    // writing to regular file `outfile`.
    at.mkfifo("fifo");
    let mut ucmd = scenario.ucmd();
    let child = ucmd
        .args(&["--copy-contents", "fifo", "outfile"])
        .run_no_wait();

    // Write some bytes to the `fifo`. We expect these bytes to get
    // copied through to `outfile`.
    std::fs::write(at.plus("fifo"), "foo").unwrap();

    // At this point the child process should have terminated
    // successfully with no output. The `outfile` should have the
    // contents of `fifo` copied into it.
    child.wait().unwrap().no_stdout().no_stderr().success();
    assert_eq!(at.read("outfile"), "foo");
}

#[cfg(target_os = "linux")]
#[test]
fn test_reflink_never_sparse_always() {
    let (at, mut ucmd) = at_and_ucmd!();

    // Create a file and make it a large sparse file.
    //
    // On common Linux filesystems, setting the length to one megabyte
    // should cause the file to become a sparse file, but it depends
    // on the system.
    std::fs::File::create(at.plus("src"))
        .unwrap()
        .set_len(1024 * 1024)
        .unwrap();

    ucmd.args(&["--reflink=never", "--sparse=always", "src", "dest"])
        .succeeds()
        .no_stdout()
        .no_stderr();
    at.file_exists("dest");

    let src_metadata = std::fs::metadata(at.plus("src")).unwrap();
    let dest_metadata = std::fs::metadata(at.plus("dest")).unwrap();
    assert_eq!(src_metadata.blocks(), dest_metadata.blocks());
    assert_eq!(dest_metadata.len(), 1024 * 1024);
}

/// Test for preserving attributes of a hard link in a directory.
#[test]
#[cfg(not(target_os = "android"))]
fn test_preserve_hardlink_attributes_in_directory() {
    let (at, mut ucmd) = at_and_ucmd!();

    // The source directory tree.
    at.mkdir("src");
    at.touch("src/f");
    at.hard_link("src/f", "src/link");

    // The destination directory tree.
    //
    // The file `f` already exists, but the `link` does not.
    at.mkdir_all("dest/src");
    at.touch("dest/src/f");

    ucmd.args(&["-a", "src", "dest"]).succeeds().no_output();

    // The hard link should now appear in the destination directory tree.
    //
    // A hard link should have the same inode as the target file.
    at.file_exists("dest/src/link");
    #[cfg(all(unix, not(target_os = "freebsd")))]
    assert_eq!(
        at.metadata("dest/src/f").ino(),
        at.metadata("dest/src/link").ino()
    );
}

#[test]
#[cfg(not(any(windows, target_os = "android")))]
fn test_hard_link_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("src");
    at.touch("dest");
    ucmd.args(&["-f", "--link", "src", "dest"])
        .succeeds()
        .no_output();
    #[cfg(all(unix, not(target_os = "freebsd")))]
    assert_eq!(at.metadata("src").ino(), at.metadata("dest").ino());
}

#[test]
#[cfg(not(windows))]
fn test_symbolic_link_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("src");
    at.touch("dest");
    ucmd.args(&["-f", "--symbolic-link", "src", "dest"])
        .succeeds()
        .no_output();
    assert_eq!(
        std::fs::read_link(at.plus("dest")).unwrap(),
        Path::new("src")
    );
}

#[test]
fn test_src_base_dot() {
    let ts = TestScenario::new(util_name!());
    let at = ts.fixtures.clone();
    at.mkdir("x");
    at.mkdir("y");
    ts.ucmd()
        .current_dir(at.plus("y"))
        .args(&["--verbose", "-r", "../x/.", "."])
        .succeeds()
        .no_stderr()
        .no_stdout();
    assert!(!at.dir_exists("y/x"));
}

#[cfg(target_os = "linux")]
fn non_utf8_name(suffix: &str) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    let mut name = OsString::from_vec(vec![0xff, 0xff]);
    name.push(suffix);
    name
}

#[cfg(target_os = "linux")]
#[test]
fn test_non_utf8_src() {
    let (at, mut ucmd) = at_and_ucmd!();
    let src = non_utf8_name("src");
    std::fs::File::create(at.plus(&src)).unwrap();
    ucmd.args(&[src, "dest".into()])
        .succeeds()
        .no_stderr()
        .no_stdout();
    assert!(at.file_exists("dest"));
}

#[cfg(target_os = "linux")]
#[test]
fn test_non_utf8_dest() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dest = non_utf8_name("dest");
    ucmd.args(&[TEST_HELLO_WORLD_SOURCE.as_ref(), &*dest])
        .succeeds()
        .no_stderr()
        .no_stdout();
    assert!(at.file_exists(dest));
}

#[cfg(target_os = "linux")]
#[test]
fn test_non_utf8_target() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dest = non_utf8_name("dest");
    at.mkdir(&dest);
    ucmd.args(&["-t".as_ref(), &*dest, TEST_HELLO_WORLD_SOURCE.as_ref()])
        .succeeds()
        .no_stderr()
        .no_stdout();
    let mut copied_file = PathBuf::from(dest);
    copied_file.push(TEST_HELLO_WORLD_SOURCE);
    assert!(at.file_exists(copied_file));
}

#[test]
#[cfg(not(windows))]
fn test_cp_archive_on_directory_ending_dot() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir1");
    at.mkdir("dir2");
    at.touch("dir1/file");
    ucmd.args(&["-a", "dir1/.", "dir2"]).succeeds();
    assert!(at.file_exists("dir2/file"));
}
