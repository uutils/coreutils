// spell-checker:ignore (flags) reflink (fs) tmpfs

use crate::common::util::*;
#[cfg(not(windows))]
use std::fs::set_permissions;

#[cfg(not(windows))]
use std::os::unix::fs;

#[cfg(windows)]
use std::os::windows::fs::symlink_file;

#[cfg(target_os = "linux")]
use filetime::FileTime;
#[cfg(not(windows))]
use std::env;
#[cfg(target_os = "linux")]
use std::fs as std_fs;
#[cfg(target_os = "linux")]
use std::thread::sleep;
#[cfg(target_os = "linux")]
use std::time::Duration;

static TEST_EXISTING_FILE: &str = "existing_file.txt";
static TEST_HELLO_WORLD_SOURCE: &str = "hello_world.txt";
static TEST_HELLO_WORLD_SOURCE_SYMLINK: &str = "hello_world.txt.link";
static TEST_HELLO_WORLD_DEST: &str = "copy_of_hello_world.txt";
static TEST_HOW_ARE_YOU_SOURCE: &str = "how_are_you.txt";
static TEST_HOW_ARE_YOU_DEST: &str = "hello_dir/how_are_you.txt";
static TEST_COPY_TO_FOLDER: &str = "hello_dir/";
static TEST_COPY_TO_FOLDER_FILE: &str = "hello_dir/hello_world.txt";
static TEST_COPY_FROM_FOLDER: &str = "hello_dir_with_file/";
static TEST_COPY_FROM_FOLDER_FILE: &str = "hello_dir_with_file/hello_world.txt";
static TEST_COPY_TO_FOLDER_NEW: &str = "hello_dir_new";
static TEST_COPY_TO_FOLDER_NEW_FILE: &str = "hello_dir_new/hello_world.txt";
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
static TEST_MOUNT_COPY_FROM_FOLDER: &str = "dir_with_mount";
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
static TEST_MOUNT_MOUNTPOINT: &str = "mount";
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
static TEST_MOUNT_OTHER_FILESYSTEM_FILE: &str = "mount/DO_NOT_copy_me.txt";

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
    let (_, mut ucmd) = at_and_ucmd!();

    ucmd.arg("--backup")
        .arg("--no-clobber")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .fails()
        .stderr_is("cp: options --backup and --no-clobber are mutually exclusive\nTry 'cp --help' for more information.");
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
    assert_eq!(at.read(&path_to_check), "Hello, World!\n");
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
    assert_eq!(at.read(&path_to_check), "Hello, World!\n");
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
// For now, disable the test on Windows. Symlinks aren't well support on Windows.
// It works on Unix for now and it works locally when run from a powershell
#[cfg(not(windows))]
fn test_cp_deref_folder_to_folder() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let cwd = env::current_dir().unwrap();

    let path_to_new_symlink = at.subdir.join(TEST_COPY_FROM_FOLDER);

    // Change the cwd to have a correct symlink
    assert!(env::set_current_dir(&path_to_new_symlink).is_ok());

    #[cfg(not(windows))]
    let _r = fs::symlink(TEST_HELLO_WORLD_SOURCE, TEST_HELLO_WORLD_SOURCE_SYMLINK);
    #[cfg(windows)]
    let _r = symlink_file(TEST_HELLO_WORLD_SOURCE, TEST_HELLO_WORLD_SOURCE_SYMLINK);

    // Back to the initial cwd (breaks the other tests)
    assert!(env::set_current_dir(&cwd).is_ok());

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
    assert_eq!(at.read(&path_to_check), "Hello, World!\n");
}

#[test]
// For now, disable the test on Windows. Symlinks aren't well support on Windows.
// It works on Unix for now and it works locally when run from a powershell
#[cfg(not(windows))]
fn test_cp_no_deref_folder_to_folder() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let cwd = env::current_dir().unwrap();

    let path_to_new_symlink = at.subdir.join(TEST_COPY_FROM_FOLDER);

    // Change the cwd to have a correct symlink
    assert!(env::set_current_dir(&path_to_new_symlink).is_ok());

    #[cfg(not(windows))]
    let _r = fs::symlink(TEST_HELLO_WORLD_SOURCE, TEST_HELLO_WORLD_SOURCE_SYMLINK);
    #[cfg(windows)]
    let _r = symlink_file(TEST_HELLO_WORLD_SOURCE, TEST_HELLO_WORLD_SOURCE_SYMLINK);

    // Back to the initial cwd (breaks the other tests)
    assert!(env::set_current_dir(&cwd).is_ok());

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
    assert_eq!(at.read(&path_to_check), "Hello, World!\n");
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
#[cfg(target_os = "unix")]
fn test_cp_archive_recursive() {
    let (at, mut ucmd) = at_and_ucmd!();
    let cwd = env::current_dir().unwrap();

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

    // Change the cwd to have a correct symlink
    assert!(env::set_current_dir(&at.subdir.join(TEST_COPY_TO_FOLDER)).is_ok());

    #[cfg(not(windows))]
    {
        let _r = fs::symlink("1", &file_1_link);
        let _r = fs::symlink("2", &file_2_link);
    }
    #[cfg(windows)]
    {
        let _r = symlink_file("1", &file_1_link);
        let _r = symlink_file("2", &file_2_link);
    }
    // Back to the initial cwd (breaks the other tests)
    assert!(env::set_current_dir(&cwd).is_ok());

    ucmd.arg("--archive")
        .arg(TEST_COPY_TO_FOLDER)
        .arg(TEST_COPY_TO_FOLDER_NEW)
        .fails(); // fails for now

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
            .join("1.link")
            .to_string_lossy()
    ));
    assert!(at.file_exists(
        &at.subdir
            .join(TEST_COPY_TO_FOLDER_NEW)
            .join("2.link")
            .to_string_lossy()
    ));
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
#[cfg(target_os = "linux")]
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
#[cfg(target_os = "linux")]
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
#[cfg(target_os = "linux")]
fn test_cp_target_file_dev_null() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "/dev/null";
    let file2 = "test_cp_target_file_file_i2";

    at.touch(file2);
    ucmd.arg(file1).arg(file2).succeeds().no_stderr();

    assert!(at.file_exists(file2));
}

#[test]
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
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
#[cfg(any(target_os = "linux", target_os = "macos"))]
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
#[cfg(any(target_os = "linux", target_os = "macos"))]
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
#[cfg(any(target_os = "linux", target_os = "macos"))]
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
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn test_cp_reflink_bad() {
    let (_, mut ucmd) = at_and_ucmd!();
    let _result = ucmd
        .arg("--reflink=bad")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_EXISTING_FILE)
        .fails()
        .stderr_contains("invalid argument");
}
