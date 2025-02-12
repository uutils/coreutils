// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (flags) reflink (fs) tmpfs (linux) rlimit Rlim NOFILE clob btrfs neve ROOTDIR USERDIR procfs outfile uufs xattrs
// spell-checker:ignore bdfl hlsl IRWXO IRWXG getfattr
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::path_concat;
use uutests::util::TestScenario;
use uutests::util_name;

#[cfg(not(windows))]
use std::fs::set_permissions;

use std::io::Write;
#[cfg(not(windows))]
use std::os::unix::fs;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(windows)]
use std::os::windows::fs::symlink_file;
#[cfg(not(windows))]
use std::path::Path;
#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[cfg(any(target_os = "linux", target_os = "android"))]
use filetime::FileTime;
#[cfg(target_os = "linux")]
use std::ffi::OsString;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::fs as std_fs;
use std::thread::sleep;
use std::time::Duration;

#[cfg(any(target_os = "linux", target_os = "android"))]
#[cfg(feature = "truncate")]
use uutests::util::PATH;

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
static TEST_NONEXISTENT_FILE: &str = "nonexistent_file.txt";
#[cfg(all(
    unix,
    not(any(target_os = "android", target_os = "macos", target_os = "openbsd"))
))]
use uutests::util::compare_xattrs;

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
        .stderr_contains(format!(
            "source file '{TEST_HELLO_WORLD_SOURCE}' specified more than once"
        ));
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");
}

#[test]
fn test_cp_duplicate_folder() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg("-r")
        .arg(TEST_COPY_FROM_FOLDER)
        .arg(TEST_COPY_FROM_FOLDER)
        .arg(TEST_COPY_TO_FOLDER)
        .succeeds()
        .stderr_contains(format!(
            "source directory '{TEST_COPY_FROM_FOLDER}' specified more than once"
        ));
    assert!(at.dir_exists(format!("{TEST_COPY_TO_FOLDER}/{TEST_COPY_FROM_FOLDER}").as_str()));
}

#[test]
fn test_cp_duplicate_files_normalized_path() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(format!("./{TEST_HELLO_WORLD_SOURCE}"))
        .arg(TEST_COPY_TO_FOLDER)
        .succeeds()
        .stderr_contains(format!(
            "source file './{TEST_HELLO_WORLD_SOURCE}' specified more than once"
        ));
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");
}

#[test]
fn test_cp_duplicate_files_with_plain_backup() {
    let (_, mut ucmd) = at_and_ucmd!();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_COPY_TO_FOLDER)
        .arg("--backup")
        .fails()
        // cp would skip duplicate src check and fail when it tries to overwrite the "just-created" file.
        .stderr_contains(
            "will not overwrite just-created 'hello_dir/hello_world.txt' with 'hello_world.txt",
        );
}

#[test]
fn test_cp_duplicate_files_with_numbered_backup() {
    let (at, mut ucmd) = at_and_ucmd!();
    // cp would skip duplicate src check and succeeds
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_COPY_TO_FOLDER)
        .arg("--backup=numbered")
        .succeeds();
    at.file_exists(TEST_COPY_TO_FOLDER_FILE);
    at.file_exists(format!("{TEST_COPY_TO_FOLDER}.~1~"));
}

#[test]
fn test_cp_same_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "a";

    at.touch(file);

    ucmd.arg(file)
        .arg(file)
        .fails()
        .code_is(1)
        .stderr_contains(format!("'{file}' and '{file}' are the same file"));
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
        .stderr_is(format!(
            "cp: -r not specified; omitting directory '{TEST_COPY_TO_FOLDER}'\n"
        ));
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
fn test_cp_multiple_files_with_nonexistent_file() {
    #[cfg(windows)]
    let error_msg = "The system cannot find the file specified";
    #[cfg(not(windows))]
    let error_msg = format!("'{TEST_NONEXISTENT_FILE}': No such file or directory");
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_NONEXISTENT_FILE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg(TEST_COPY_TO_FOLDER)
        .fails()
        .stderr_contains(error_msg);

    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");
    assert_eq!(at.read(TEST_HOW_ARE_YOU_DEST), "How are you?\n");
}

#[test]
fn test_cp_multiple_files_with_empty_file_name() {
    #[cfg(windows)]
    let error_msg = "The system cannot find the path specified";
    #[cfg(not(windows))]
    let error_msg = "'': No such file or directory";
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("")
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg(TEST_COPY_TO_FOLDER)
        .fails()
        .stderr_contains(error_msg);

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
#[cfg(not(target_os = "macos"))]
fn test_cp_recurse_several() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg("-r")
        .arg("-r")
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
fn test_cp_arg_no_target_directory_with_recursive() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("dir");
    at.mkdir("dir2");
    at.touch("dir/a");
    at.touch("dir/b");

    ucmd.arg("-rT").arg("dir").arg("dir2").succeeds();

    assert!(at.plus("dir2").join("a").exists());
    assert!(at.plus("dir2").join("b").exists());
    assert!(!at.plus("dir2").join("dir").exists());
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
fn test_cp_arg_update_interactive_error() {
    new_ucmd!()
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg("-i")
        .fails()
        .no_stdout();
}

#[test]
fn test_cp_arg_update_none() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg("--update=none")
        .succeeds()
        .no_stderr()
        .no_stdout();
    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "How are you?\n");
}

#[test]
fn test_cp_arg_update_none_fail() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg("--update=none-fail")
        .fails()
        .stderr_contains(format!("not replacing '{TEST_HOW_ARE_YOU_SOURCE}'"));
    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "How are you?\n");
}

#[test]
fn test_cp_arg_update_all() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg("--update=all")
        .succeeds()
        .no_stderr()
        .no_stdout();

    assert_eq!(
        at.read(TEST_HOW_ARE_YOU_SOURCE),
        at.read(TEST_HELLO_WORLD_SOURCE)
    );
}

#[test]
fn test_cp_arg_update_older_dest_not_older_than_src() {
    let (at, mut ucmd) = at_and_ucmd!();

    let old = "test_cp_arg_update_dest_not_older_file1";
    let new = "test_cp_arg_update_dest_not_older_file2";
    let old_content = "old content\n";
    let new_content = "new content\n";

    at.write(old, old_content);
    at.write(new, new_content);

    ucmd.arg(old)
        .arg(new)
        .arg("--update=older")
        .succeeds()
        .no_stderr()
        .no_stdout();

    assert_eq!(at.read(new), "new content\n");
}

#[test]
fn test_cp_arg_update_older_dest_older_than_src() {
    let (at, mut ucmd) = at_and_ucmd!();

    let old = "test_cp_arg_update_dest_older_file1";
    let new = "test_cp_arg_update_dest_older_file2";
    let old_content = "old content\n";
    let new_content = "new content\n";

    let mut f = at.make_file(old);
    f.write_all(old_content.as_bytes()).unwrap();
    f.set_modified(std::time::UNIX_EPOCH).unwrap();

    at.write(new, new_content);

    ucmd.arg(new)
        .arg(old)
        .arg("--update=older")
        .succeeds()
        .no_stderr()
        .no_stdout();

    assert_eq!(at.read(old), "new content\n");
}

#[test]
fn test_cp_arg_update_short_no_overwrite() {
    // same as --update=older
    let (at, mut ucmd) = at_and_ucmd!();

    let old = "test_cp_arg_update_short_no_overwrite_file1";
    let new = "test_cp_arg_update_short_no_overwrite_file2";
    let old_content = "old content\n";
    let new_content = "new content\n";

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

    assert_eq!(at.read(new), "new content\n");
}

#[test]
fn test_cp_arg_update_short_overwrite() {
    // same as --update=older
    let (at, mut ucmd) = at_and_ucmd!();

    let old = "test_cp_arg_update_short_overwrite_file1";
    let new = "test_cp_arg_update_short_overwrite_file2";
    let old_content = "old content\n";
    let new_content = "new content\n";

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

    assert_eq!(at.read(old), "new content\n");
}

#[test]
fn test_cp_arg_update_none_then_all() {
    // take last if multiple update args are supplied,
    // update=all wins in this case
    let (at, mut ucmd) = at_and_ucmd!();

    let old = "test_cp_arg_update_none_then_all_file1";
    let new = "test_cp_arg_update_none_then_all_file2";
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
fn test_cp_arg_update_all_then_none() {
    // take last if multiple update args are supplied,
    // update=none wins in this case
    let (at, mut ucmd) = at_and_ucmd!();

    let old = "test_cp_arg_update_all_then_none_file1";
    let new = "test_cp_arg_update_all_then_none_file2";
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
#[cfg(not(any(target_os = "android", target_os = "freebsd", target_os = "openbsd")))]
fn test_cp_arg_interactive_update_overwrite_newer() {
    // -u -i won't show the prompt to validate the override or not
    // Therefore, the error code will be 0
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("a");
    at.touch("b");
    ucmd.args(&["-i", "-u", "a", "b"])
        .pipe_in("")
        .succeeds()
        .no_stdout();
    // Make extra sure that closing stdin behaves identically to piping-in nothing.
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("a");
    at.touch("b");
    ucmd.args(&["-i", "-u", "a", "b"]).succeeds().no_stdout();
}

#[test]
#[cfg(not(any(target_os = "android", target_os = "freebsd")))]
fn test_cp_arg_interactive_update_overwrite_older() {
    // -u -i *WILL* show the prompt to validate the override.
    // Therefore, the error code depends on the prompt response.
    // Option N
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("b");
    sleep(Duration::from_millis(100));
    at.touch("a");
    ucmd.args(&["-i", "-u", "a", "b"])
        .pipe_in("N\n")
        .fails()
        .code_is(1)
        .no_stdout()
        .stderr_is("cp: overwrite 'b'? ");

    // Option Y
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("b");
    sleep(Duration::from_millis(100));
    at.touch("a");
    ucmd.args(&["-i", "-u", "a", "b"])
        .pipe_in("Y\n")
        .succeeds()
        .no_stdout();
}

#[test]
#[cfg(not(target_os = "android"))]
fn test_cp_arg_interactive_verbose() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("a");
    at.touch("b");
    ucmd.args(&["-vi", "a", "b"])
        .pipe_in("N\n")
        .fails()
        .stderr_is("cp: overwrite 'b'? ")
        .no_stdout();
}

#[test]
#[cfg(not(target_os = "android"))]
fn test_cp_arg_interactive_verbose_clobber() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("a");
    at.touch("b");
    ucmd.args(&["-vin", "--debug", "a", "b"])
        .succeeds()
        .stdout_contains("skipped 'b'");
}

#[test]
#[cfg(unix)]
fn test_cp_f_i_verbose_non_writeable_destination_y() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("a");
    at.touch("b");

    // Non-writeable file
    at.set_mode("b", 0o0000);

    ucmd.args(&["-f", "-i", "--verbose", "a", "b"])
        .pipe_in("y")
        .succeeds()
        .stderr_is("cp: replace 'b', overriding mode 0000 (---------)? ")
        .stdout_is("'a' -> 'b'\n");
}

#[test]
#[cfg(unix)]
fn test_cp_f_i_verbose_non_writeable_destination_empty() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("a");
    at.touch("b");

    // Non-writeable file
    at.set_mode("b", 0o0000);

    ucmd.args(&["-f", "-i", "--verbose", "a", "b"])
        .pipe_in("")
        .fails()
        .stderr_only("cp: replace 'b', overriding mode 0000 (---------)? ");
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
#[cfg(target_os = "linux")]
fn test_cp_arg_link_with_dest_hardlink_to_source() {
    use std::os::linux::fs::MetadataExt;

    let (at, mut ucmd) = at_and_ucmd!();
    let file = "file";
    let hardlink = "hardlink";

    at.touch(file);
    at.hard_link(file, hardlink);

    ucmd.args(&["--link", file, hardlink]).succeeds();

    assert_eq!(at.metadata(file).st_nlink(), 2);
    assert!(at.file_exists(file));
    assert!(at.file_exists(hardlink));
}

#[test]
#[cfg(target_os = "linux")]
fn test_cp_arg_link_with_same_file() {
    use std::os::linux::fs::MetadataExt;

    let (at, mut ucmd) = at_and_ucmd!();
    let file = "file";

    at.touch(file);

    ucmd.args(&["--link", file, file]).succeeds();

    assert_eq!(at.metadata(file).st_nlink(), 1);
    assert!(at.file_exists(file));
}

#[test]
#[cfg(target_os = "linux")]
fn test_cp_verbose_preserved_link_to_dir() {
    use std::os::linux::fs::MetadataExt;

    let (at, mut ucmd) = at_and_ucmd!();
    let file = "file";
    let hardlink = "hardlink";
    let dir = "dir";
    let dst_file = "dir/file";
    let dst_hardlink = "dir/hardlink";

    at.touch(file);
    at.hard_link(file, hardlink);
    at.mkdir(dir);

    ucmd.args(&["-d", "--verbose", file, hardlink, dir])
        .succeeds()
        .stdout_is("'file' -> 'dir/file'\n'hardlink' -> 'dir/hardlink'\n");

    assert!(at.file_exists(dst_file));
    assert!(at.file_exists(dst_hardlink));
    assert_eq!(at.metadata(dst_file).st_nlink(), 2);
    assert_eq!(at.metadata(dst_hardlink).st_nlink(), 2);
    assert_eq!(
        at.metadata(dst_file).st_ino(),
        at.metadata(dst_hardlink).st_ino()
    );
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
        .arg("--debug")
        .succeeds()
        .stdout_contains("skipped 'how_are_you.txt'");

    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "How are you?\n");
}

#[test]
fn test_cp_arg_no_clobber_inferred_arg() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg("--no-clob")
        .arg("--debug")
        .succeeds()
        .stdout_contains("skipped 'how_are_you.txt'");

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
        .arg("--debug")
        .succeeds()
        .no_stderr();

    assert_eq!(at.read("source.txt"), "");

    at.append("source.txt", "some-content");
    scene
        .ucmd()
        .arg("--no-clobber")
        .arg("source.txt")
        .arg("dest.txt")
        .arg("--debug")
        .succeeds()
        .stdout_contains("skipped 'dest.txt'");

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
fn test_cp_arg_backup_with_dest_a_symlink() {
    let (at, mut ucmd) = at_and_ucmd!();
    let source = "source";
    let source_content = "content";
    let symlink = "symlink";
    let original = "original";
    let backup = "symlink~";

    at.write(source, source_content);
    at.write(original, "original");
    at.symlink_file(original, symlink);

    ucmd.arg("-b").arg(source).arg(symlink).succeeds();

    assert!(!at.symlink_exists(symlink));
    assert_eq!(source_content, at.read(symlink));
    assert!(at.symlink_exists(backup));
    assert_eq!(original, at.resolve_link(backup));
}

#[test]
fn test_cp_arg_backup_with_dest_a_symlink_to_source() {
    let (at, mut ucmd) = at_and_ucmd!();
    let source = "source";
    let source_content = "content";
    let symlink = "symlink";
    let backup = "symlink~";

    at.write(source, source_content);
    at.symlink_file(source, symlink);

    ucmd.arg("-b").arg(source).arg(symlink).succeeds();

    assert!(!at.symlink_exists(symlink));
    assert_eq!(source_content, at.read(symlink));
    assert!(at.symlink_exists(backup));
    assert_eq!(source, at.resolve_link(backup));
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
#[cfg(not(target_os = "openbsd"))]
fn test_cp_parents_with_permissions_copy_file() {
    let (at, mut ucmd) = at_and_ucmd!();

    let dir = "dir";
    let file = "p1/p2/file";

    at.mkdir(dir);
    at.mkdir_all("p1/p2");
    at.touch(file);

    #[cfg(unix)]
    {
        let p1_mode = 0o0777;
        let p2_mode = 0o0711;
        let file_mode = 0o0702;

        at.set_mode("p1", p1_mode);
        at.set_mode("p1/p2", p2_mode);
        at.set_mode(file, file_mode);
    }

    ucmd.arg("-p")
        .arg("--parents")
        .arg(file)
        .arg(dir)
        .succeeds();

    #[cfg(all(unix, not(target_os = "freebsd")))]
    {
        let p1_metadata = at.metadata("p1");
        let p2_metadata = at.metadata("p1/p2");
        let file_metadata = at.metadata(file);

        assert_metadata_eq!(p1_metadata, at.metadata("dir/p1"));
        assert_metadata_eq!(p2_metadata, at.metadata("dir/p1/p2"));
        assert_metadata_eq!(file_metadata, at.metadata("dir/p1/p2/file"));
    }
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_cp_parents_with_permissions_copy_dir() {
    let (at, mut ucmd) = at_and_ucmd!();

    let dir1 = "dir";
    let dir2 = "p1/p2";
    let file = "p1/p2/file";

    at.mkdir(dir1);
    at.mkdir_all(dir2);
    at.touch(file);

    #[cfg(unix)]
    {
        let p1_mode = 0o0777;
        let p2_mode = 0o0711;
        let file_mode = 0o0702;

        at.set_mode("p1", p1_mode);
        at.set_mode("p1/p2", p2_mode);
        at.set_mode(file, file_mode);
    }

    ucmd.arg("-p")
        .arg("--parents")
        .arg("-r")
        .arg(dir2)
        .arg(dir1)
        .succeeds();

    #[cfg(all(unix, not(target_os = "freebsd")))]
    {
        let p1_metadata = at.metadata("p1");
        let p2_metadata = at.metadata("p1/p2");
        let file_metadata = at.metadata(file);

        assert_metadata_eq!(p1_metadata, at.metadata("dir/p1"));
        assert_metadata_eq!(p2_metadata, at.metadata("dir/p1/p2"));
        assert_metadata_eq!(file_metadata, at.metadata("dir/p1/p2/file"));
    }
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
#[cfg(not(target_os = "openbsd"))]
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
#[cfg(not(target_os = "openbsd"))]
fn test_cp_preserve_no_args_before_opts() {
    let (at, mut ucmd) = at_and_ucmd!();
    let src_file = "a";
    let dst_file = "b";

    // Prepare the source file
    at.touch(src_file);
    #[cfg(unix)]
    at.set_mode(src_file, 0o0500);

    // Copy
    ucmd.arg("--preserve")
        .arg(src_file)
        .arg(dst_file)
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
    for argument in ["--preserve=all", "--preserve=al"] {
        let (at, mut ucmd) = at_and_ucmd!();
        let src_file = "a";
        let dst_file = "b";

        // Prepare the source file
        at.touch(src_file);
        #[cfg(unix)]
        at.set_mode(src_file, 0o0500);

        // TODO: create a destination that does not allow copying of xattr and context
        // Copy
        ucmd.arg(src_file).arg(dst_file).arg(argument).succeeds();

        #[cfg(all(unix, not(target_os = "freebsd")))]
        {
            // Assert that the mode, ownership, and timestamps are preserved
            // NOTICE: the ownership is not modified on the src file, because that requires root permissions
            let metadata_src = at.metadata(src_file);
            let metadata_dst = at.metadata(dst_file);
            assert_metadata_eq!(metadata_src, metadata_dst);
        }
    }
}

#[test]
#[cfg(all(unix, not(any(target_os = "android", target_os = "openbsd"))))]
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
fn test_cp_preserve_link_parses() {
    // TODO: Also check whether --preserve=link did the right thing!
    for argument in [
        "--preserve=links",
        "--preserve=link",
        "--preserve=li",
        "--preserve=l",
    ] {
        new_ucmd!()
            .arg(argument)
            .arg(TEST_COPY_FROM_FOLDER_FILE)
            .arg(TEST_HELLO_WORLD_DEST)
            .succeeds()
            .no_output();
    }
}

#[test]
fn test_cp_preserve_invalid_rejected() {
    new_ucmd!()
        .arg("--preserve=invalid-value")
        .arg(TEST_COPY_FROM_FOLDER_FILE)
        .arg(TEST_HELLO_WORLD_DEST)
        .fails()
        .code_is(1)
        .no_stdout();
}

#[test]
#[cfg(target_os = "android")]
#[ignore = "disabled until fixed"] // FIXME: the test looks to .succeed on android
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
// android platform will causing stderr = cp: Permission denied (os error 13)
#[cfg(not(target_os = "android"))]
fn test_cp_preserve_links_case_1() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("a");
    at.hard_link("a", "b");
    at.mkdir("c");

    ucmd.arg("-d").arg("a").arg("b").arg("c").succeeds();

    assert!(at.dir_exists("c"));
    assert!(at.plus("c").join("a").exists());
    assert!(at.plus("c").join("b").exists());

    #[cfg(unix)]
    {
        let metadata_a = std::fs::metadata(at.subdir.join("c").join("a")).unwrap();
        let metadata_b = std::fs::metadata(at.subdir.join("c").join("b")).unwrap();

        assert_eq!(metadata_a.ino(), metadata_b.ino());
    }
}

#[test]
// android platform will causing stderr = cp: Permission denied (os error 13)
#[cfg(not(target_os = "android"))]
fn test_cp_preserve_links_case_2() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("a");
    at.symlink_file("a", "b");
    at.mkdir("c");

    ucmd.arg("--preserve=links")
        .arg("-R")
        .arg("-H")
        .arg("a")
        .arg("b")
        .arg("c")
        .succeeds();

    assert!(at.dir_exists("c"));
    assert!(at.plus("c").join("a").exists());
    assert!(at.plus("c").join("b").exists());

    #[cfg(unix)]
    {
        let metadata_a = std::fs::metadata(at.subdir.join("c").join("a")).unwrap();
        let metadata_b = std::fs::metadata(at.subdir.join("c").join("b")).unwrap();

        assert_eq!(metadata_a.ino(), metadata_b.ino());
    }
}

#[test]
// android platform will causing stderr = cp: Permission denied (os error 13)
#[cfg(not(target_os = "android"))]
fn test_cp_preserve_links_case_3() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("d");
    at.touch("d/a");
    at.symlink_file("d/a", "d/b");

    ucmd.arg("--preserve=links")
        .arg("-R")
        .arg("-L")
        .arg("d")
        .arg("c")
        .succeeds();

    assert!(at.dir_exists("c"));
    assert!(at.plus("c").join("a").exists());
    assert!(at.plus("c").join("b").exists());

    #[cfg(unix)]
    {
        let metadata_a = std::fs::metadata(at.subdir.join("c").join("a")).unwrap();
        let metadata_b = std::fs::metadata(at.subdir.join("c").join("b")).unwrap();

        assert_eq!(metadata_a.ino(), metadata_b.ino());
    }
}

#[test]
// android platform will causing stderr = cp: Permission denied (os error 13)
#[cfg(not(target_os = "android"))]
fn test_cp_preserve_links_case_4() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("d");
    at.touch("d/a");
    at.hard_link("d/a", "d/b");

    ucmd.arg("--preserve=links")
        .arg("-R")
        .arg("-L")
        .arg("d")
        .arg("c")
        .succeeds();

    assert!(at.dir_exists("c"));
    assert!(at.plus("c").join("a").exists());
    assert!(at.plus("c").join("b").exists());

    #[cfg(unix)]
    {
        let metadata_a = std::fs::metadata(at.subdir.join("c").join("a")).unwrap();
        let metadata_b = std::fs::metadata(at.subdir.join("c").join("b")).unwrap();

        assert_eq!(metadata_a.ino(), metadata_b.ino());
    }
}

#[test]
// android platform will causing stderr = cp: Permission denied (os error 13)
#[cfg(not(target_os = "android"))]
fn test_cp_preserve_links_case_5() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("d");
    at.touch("d/a");
    at.hard_link("d/a", "d/b");

    ucmd.arg("-dR")
        .arg("--no-preserve=links")
        .arg("d")
        .arg("c")
        .succeeds();

    assert!(at.dir_exists("c"));
    assert!(at.plus("c").join("a").exists());
    assert!(at.plus("c").join("b").exists());

    #[cfg(unix)]
    {
        let metadata_a = std::fs::metadata(at.subdir.join("c").join("a")).unwrap();
        let metadata_b = std::fs::metadata(at.subdir.join("c").join("b")).unwrap();

        assert_ne!(metadata_a.ino(), metadata_b.ino());
    }
}

#[test]
// android platform will causing stderr = cp: Permission denied (os error 13)
#[cfg(not(target_os = "android"))]
fn test_cp_preserve_links_case_6() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("a");
    at.hard_link("a", "b");
    at.mkdir("c");

    ucmd.arg("-d").arg("a").arg("b").arg("c").succeeds();

    assert!(at.dir_exists("c"));
    assert!(at.plus("c").join("a").exists());
    assert!(at.plus("c").join("b").exists());

    #[cfg(unix)]
    {
        let metadata_a = std::fs::metadata(at.subdir.join("c").join("a")).unwrap();
        let metadata_b = std::fs::metadata(at.subdir.join("c").join("b")).unwrap();

        assert_eq!(metadata_a.ino(), metadata_b.ino());
    }
}

#[test]
// android platform will causing stderr = cp: Permission denied (os error 13)
#[cfg(not(target_os = "android"))]
fn test_cp_preserve_links_case_7() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("src");
    at.touch("src/f");
    at.hard_link("src/f", "src/g");

    at.mkdir("dest");
    at.touch("dest/g");

    ucmd.arg("-n")
        .arg("--preserve=links")
        .arg("--debug")
        .arg("src/f")
        .arg("src/g")
        .arg("dest")
        .succeeds()
        .stdout_contains("skipped");

    assert!(at.dir_exists("dest"));
    assert!(at.plus("dest").join("f").exists());
    assert!(at.plus("dest").join("g").exists());
}

#[test]
#[cfg(unix)]
fn test_cp_no_preserve_mode() {
    use uucore::fs as uufs;
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("a");
    at.set_mode("a", 0o731);

    ucmd.arg("-a")
        .arg("--no-preserve=mode")
        .arg("a")
        .arg("b")
        .umask(0o077)
        .succeeds();

    assert!(at.file_exists("b"));

    let metadata_b = std::fs::metadata(at.subdir.join("b")).unwrap();
    let permission_b = uufs::display_permissions(&metadata_b, false);
    assert_eq!(permission_b, "rw-------".to_string());
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
        .arg(at.subdir.join(TEST_COPY_TO_FOLDER))
        .run();

    println!("ls dest {}", result.stdout_str());

    let result = scene2
        .cmd("ls")
        .arg("-al")
        .arg(at.subdir.join(TEST_COPY_TO_FOLDER_NEW))
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
    sleep(Duration::from_millis(100));

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
    use uutests::util::AtPath;
    use walkdir::WalkDir;

    let mut scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Test must be run as root (or with `sudo -E`)
    if scene.cmd("whoami").run().stdout_str() != "root\n" {
        return;
    }

    let at_src = AtPath::new(&at.plus(TEST_MOUNT_COPY_FROM_FOLDER));
    let at_dst = AtPath::new(&at.plus(TEST_COPY_TO_FOLDER_NEW));

    // Prepare the mount
    at_src.mkdir(TEST_MOUNT_MOUNTPOINT);
    let mountpoint_path = &at_src.plus_as_string(TEST_MOUNT_MOUNTPOINT);

    scene
        .mount_temp_fs(mountpoint_path)
        .expect("mounting tmpfs failed");

    at_src.touch(TEST_MOUNT_OTHER_FILESYSTEM_FILE);

    // Begin testing -x flag
    scene
        .ucmd()
        .arg("-rx")
        .arg(TEST_MOUNT_COPY_FROM_FOLDER)
        .arg(TEST_COPY_TO_FOLDER_NEW)
        .succeeds();

    // Ditch the mount before the asserts
    scene.umount_temp_fs();

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
    for argument in ["--reflink=never", "--reflink=neve", "--reflink=n"] {
        let (at, mut ucmd) = at_and_ucmd!();
        ucmd.arg(argument)
            .arg(TEST_HELLO_WORLD_SOURCE)
            .arg(TEST_EXISTING_FILE)
            .succeeds();

        // Check the content of the destination file
        assert_eq!(at.read(TEST_EXISTING_FILE), "Hello, World!\n");
    }
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
fn test_cp_conflicting_update() {
    new_ucmd!()
        .arg("-b")
        .arg("--update=none")
        .arg("a")
        .arg("b")
        .fails()
        .stderr_contains("--backup is mutually exclusive with -n or --update=none-fail");
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

#[cfg(target_os = "linux")]
#[test]
fn test_closes_file_descriptors() {
    use procfs::process::Process;
    use rlimit::Resource;
    let me = Process::myself().unwrap();

    // The test suite runs in parallel, we have pipe, sockets
    // opened by other tests.
    // So, we take in account the various fd to increase the limit
    let number_file_already_opened: u64 = me.fd_count().unwrap().try_into().unwrap();
    let limit_fd: u64 = number_file_already_opened + 9;

    // For debugging purposes:
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
    const BUFFER_SIZE: usize = 4096 * 4;
    let (at, mut ucmd) = at_and_ucmd!();

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
    const BUFFER_SIZE: usize = 4096 * 4;
    for argument in ["--sparse=always", "--sparse=alway", "--sparse=al"] {
        let (at, mut ucmd) = at_and_ucmd!();

        let buf: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

        at.make_file("src_file1");
        at.write_bytes("src_file1", &buf);

        ucmd.args(&[argument, "src_file1", "dst_file_sparse"])
            .succeeds();

        assert_eq!(at.read_bytes("dst_file_sparse"), buf);
        assert_eq!(at.metadata("dst_file_sparse").blocks(), 0);
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_cp_sparse_always_non_empty() {
    const BUFFER_SIZE: usize = 4096 * 16 + 3;
    let (at, mut ucmd) = at_and_ucmd!();

    let mut buf = vec![0; BUFFER_SIZE].into_boxed_slice();
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

    assert_eq!(at.read_bytes("dst_file_sparse").into_boxed_slice(), buf);
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
    const DISK: &str = "disk.img";
    const ROOTDIR: &str = "disk_root/";
    const USERDIR: &str = "dir/";
    const MOUNTPOINT: &str = "mountpoint/";
    let scene = TestScenario::new(util_name!());

    let src1_path: &str = &[MOUNTPOINT, USERDIR, "src1"].concat();
    let src2_path: &str = &[MOUNTPOINT, USERDIR, "src2"].concat();
    let dst_path: &str = &[MOUNTPOINT, USERDIR, "dst"].concat();

    scene.fixtures.mkdir(ROOTDIR);
    scene.fixtures.mkdir([ROOTDIR, USERDIR].concat());

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
#[cfg(unix)]
#[cfg(not(target_os = "openbsd"))]
fn test_no_preserve_mode() {
    use std::os::unix::prelude::MetadataExt;

    const PERMS_ALL: u32 = if cfg!(target_os = "freebsd") {
        // Only the superuser can set the sticky bit on a file.
        0o6777
    } else {
        0o7777
    };

    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file");
    set_permissions(at.plus("file"), PermissionsExt::from_mode(PERMS_ALL)).unwrap();
    let umask: u16 = 0o022;
    ucmd.arg("file")
        .arg("dest")
        .umask(libc::mode_t::from(umask))
        .succeeds()
        .no_stderr()
        .no_stdout();
    // remove sticky bit, setuid and setgid bit; apply umask
    let expected_perms = PERMS_ALL & !0o7000 & u32::from(!umask);
    assert_eq!(
        at.plus("dest").metadata().unwrap().mode() & 0o7777,
        expected_perms
    );
}

#[test]
#[cfg(unix)]
#[cfg(not(target_os = "openbsd"))]
fn test_preserve_mode() {
    use std::os::unix::prelude::MetadataExt;

    const PERMS_ALL: u32 = if cfg!(target_os = "freebsd") {
        // Only the superuser can set the sticky bit on a file.
        0o6777
    } else {
        0o7777
    };

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
        at.write("b/1", "hello");
        if create_t {
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
fn test_copy_through_dangling_symlink_posixly_correct() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file");
    at.symlink_file("nonexistent", "target");
    ucmd.arg("file")
        .arg("target")
        .env("POSIXLY_CORRECT", "1")
        .succeeds();
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

#[test]
fn test_cp_symlink_overwrite_detection() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.mkdir("good");
    at.mkdir("tmp");
    at.write("README", "file1");
    at.write("good/README", "file2");

    at.symlink_file("tmp/foo", "tmp/README");
    at.touch("tmp/foo");

    ts.ucmd()
        .arg("README")
        .arg("good/README")
        .arg("tmp")
        .fails()
        .stderr_only(if cfg!(target_os = "windows") {
            "cp: will not copy 'good/README' through just-created symlink 'tmp\\README'\n"
        } else if cfg!(target_os = "macos") {
            "cp: will not overwrite just-created 'tmp/README' with 'good/README'\n"
        } else {
            "cp: will not copy 'good/README' through just-created symlink 'tmp/README'\n"
        });
    let contents = at.read("tmp/foo");
    // None of the files seem to be copied in macos
    if cfg!(not(target_os = "macos")) {
        assert_eq!(contents, "file1");
    }
}

#[test]
fn test_cp_dangling_symlink_inside_directory() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.mkdir("good");
    at.mkdir("tmp");
    at.write("README", "file1");
    at.write("good/README", "file2");

    at.symlink_file("foo", "tmp/README");

    ts.ucmd()
        .arg("README")
        .arg("good/README")
        .arg("tmp")
        .fails()
        .stderr_only( if cfg!(target_os="windows") {
            "cp: not writing through dangling symlink 'tmp\\README'\ncp: not writing through dangling symlink 'tmp\\README'\n"
        } else {
            "cp: not writing through dangling symlink 'tmp/README'\ncp: not writing through dangling symlink 'tmp/README'\n"
            } );
}

/// Test for copying a dangling symbolic link and its permissions.
#[cfg(not(any(target_os = "freebsd", target_os = "openbsd")))] // FIXME: fix this test for FreeBSD/OpenBSD
#[test]
fn test_copy_through_dangling_symlink_no_dereference_permissions() {
    let (at, mut ucmd) = at_and_ucmd!();
    //               target name    link name
    at.symlink_file("no-such-file", "dangle");
    // to check if access time and modification time didn't change
    sleep(Duration::from_millis(100));
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

    at.mkdir_all("parent1/child");
    at.mkdir_all("parent2/child1/child2/child3");

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
    at.write("b/1", "hello");
    at.relative_symlink_file("../t", "c/1");
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
#[cfg(not(any(windows, target_os = "android", target_os = "openbsd")))]
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
    at.write("file", "f");
    at.relative_symlink_file("file", "slink");
    at.relative_symlink_file("slink", "slink2");
    ucmd.args(&["--link", "-P", "slink2", "z"]).succeeds();
    assert!(at.symlink_exists("z"));
    assert_eq!(at.read_symlink("z"), "slink");
}

#[cfg(not(any(windows, target_os = "android")))]
#[test]
fn test_remove_destination_with_destination_being_a_hardlink_to_source() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "file";
    let hardlink = "hardlink";

    at.touch(file);
    at.hard_link(file, hardlink);

    ucmd.args(&["--remove-destination", file, hardlink])
        .succeeds();

    assert_eq!("", at.resolve_link(hardlink));
    assert!(at.file_exists(file));
    assert!(at.file_exists(hardlink));
}

#[test]
fn test_remove_destination_with_destination_being_a_symlink_to_source() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "file";
    let symlink = "symlink";

    at.touch(file);
    at.symlink_file(file, symlink);

    ucmd.args(&["--remove-destination", file, symlink])
        .succeeds();
    assert!(!at.symlink_exists(symlink));
    assert!(at.file_exists(file));
    assert!(at.file_exists(symlink));
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

#[test]
#[cfg(not(windows))]
fn test_cp_symbolic_link_loop() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.symlink_file("loop", "loop");
    at.plus("loop");
    at.touch("f");
    ucmd.args(&["-f", "f", "loop"])
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
#[cfg(all(not(windows), not(target_os = "freebsd"), not(target_os = "openbsd")))]
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

/// cp should preserve attributes of subdirectories when copying recursively.
#[cfg(all(not(windows), not(target_os = "freebsd"), not(target_os = "openbsd")))]
#[test]
fn test_copy_dir_preserve_subdir_permissions() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("a1");
    at.mkdir("a1/a2");
    // Use different permissions for a better test
    at.set_mode("a1/a2", 0o0555);
    at.set_mode("a1", 0o0777);

    ucmd.args(&["-p", "-r", "a1", "b1"])
        .succeeds()
        .no_stderr()
        .no_stdout();

    // Make sure everything is preserved
    assert!(at.dir_exists("b1"));
    assert!(at.dir_exists("b1/a2"));
    assert_metadata_eq!(at.metadata("a1"), at.metadata("b1"));
    assert_metadata_eq!(at.metadata("a1/a2"), at.metadata("b1/a2"));
}

/// Test for preserving permissions when copying a directory, even in
/// the face of an inaccessible file in that directory.
#[cfg(all(not(windows), not(target_os = "freebsd"), not(target_os = "openbsd")))]
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
        .stderr_only("cp: cannot open 'd1/f' for reading: permission denied\n");
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
#[cfg(not(windows))]
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
#[cfg(unix)]
#[test]
fn test_copy_contents_fifo() {
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
#[cfg(not(any(target_os = "android", target_os = "openbsd")))]
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
    let at = &ts.fixtures;
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

#[test]
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
fn test_cp_debug_default() {
    #[cfg(target_os = "macos")]
    let expected = "copy offload: unknown, reflink: unsupported, sparse detection: unsupported";
    #[cfg(target_os = "linux")]
    let expected = "copy offload: unknown, reflink: unsupported, sparse detection: no";
    #[cfg(windows)]
    let expected = "copy offload: unsupported, reflink: unsupported, sparse detection: unsupported";

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");

    ts.ucmd()
        .arg("--debug")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains(expected);
}

#[test]
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
fn test_cp_debug_multiple_default() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let dir = "dir";
    at.touch("a");
    at.touch("b");
    at.mkdir(dir);

    let result = ts
        .ucmd()
        .arg("--debug")
        .arg("a")
        .arg("b")
        .arg(dir)
        .succeeds();

    #[cfg(target_os = "macos")]
    let expected = "copy offload: unknown, reflink: unsupported, sparse detection: unsupported";
    #[cfg(target_os = "linux")]
    let expected = "copy offload: unknown, reflink: unsupported, sparse detection: no";
    #[cfg(windows)]
    let expected = "copy offload: unsupported, reflink: unsupported, sparse detection: unsupported";

    // two files, two occurrences
    assert_eq!(result.stdout_str().matches(expected).count(), 2);
}

#[test]
#[cfg(target_os = "linux")]
fn test_cp_debug_sparse_reflink() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");

    ts.ucmd()
        .arg("--debug")
        .arg("--sparse=always")
        .arg("--reflink=never")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: no, sparse detection: zeros");
}

#[test]
fn test_cp_debug_no_update() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");
    at.touch("b");
    ts.ucmd()
        .arg("--debug")
        .arg("--update=none")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("skipped 'b'");
}

#[test]
#[cfg(target_os = "linux")]
fn test_cp_debug_sparse_always() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");

    ts.ucmd()
        .arg("--debug")
        .arg("--sparse=always")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: unsupported, sparse detection: zeros");
}

#[test]
#[cfg(target_os = "linux")]
fn test_cp_debug_sparse_never() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");

    ts.ucmd()
        .arg("--debug")
        .arg("--sparse=never")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: no, sparse detection: no");
}

#[test]
fn test_cp_debug_sparse_auto() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    ts.ucmd()
        .arg("--debug")
        .arg("--sparse=auto")
        .arg("a")
        .arg("b")
        .succeeds();

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        #[cfg(target_os = "macos")]
        let expected = "copy offload: unknown, reflink: unsupported, sparse detection: unsupported";
        #[cfg(target_os = "linux")]
        let expected = "copy offload: unknown, reflink: unsupported, sparse detection: no";

        ts.ucmd()
            .arg("--debug")
            .arg("--sparse=auto")
            .arg("a")
            .arg("b")
            .succeeds()
            .stdout_contains(expected);
    }
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn test_cp_debug_reflink_auto() {
    #[cfg(target_os = "macos")]
    let expected = "copy offload: unknown, reflink: unsupported, sparse detection: unsupported";
    #[cfg(target_os = "linux")]
    let expected = "copy offload: unknown, reflink: unsupported, sparse detection: no";

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");

    ts.ucmd()
        .arg("--debug")
        .arg("--reflink=auto")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains(expected);
}

#[test]
#[cfg(target_os = "linux")]
fn test_cp_debug_sparse_always_reflink_auto() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");
    ts.ucmd()
        .arg("--debug")
        .arg("--sparse=always")
        .arg("--reflink=auto")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: unsupported, sparse detection: zeros");
}

#[test]
fn test_cp_only_source_no_target() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");
    ts.ucmd()
        .arg("a")
        .fails()
        .stderr_contains("missing destination file operand after \"a\"");
}

#[test]
fn test_cp_dest_no_permissions() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.touch("valid.txt");
    at.touch("invalid_perms.txt");
    at.set_readonly("invalid_perms.txt");

    ts.ucmd()
        .args(&["valid.txt", "invalid_perms.txt"])
        .fails()
        .stderr_contains("invalid_perms.txt")
        .stderr_contains("denied");
}

#[test]
#[cfg(all(unix, not(target_os = "freebsd")))]
fn test_cp_attributes_only() {
    let (at, mut ucmd) = at_and_ucmd!();
    let a = "file_a";
    let b = "file_b";
    let mode_a = 0o0500;
    let mode_b = 0o0777;

    at.write(a, "a");
    at.write(b, "b");
    at.set_mode(a, mode_a);
    at.set_mode(b, mode_b);

    let mode_a = at.metadata(a).mode();
    let mode_b = at.metadata(b).mode();

    // --attributes-only doesn't do anything without other attribute preservation flags
    ucmd.arg("--attributes-only")
        .arg(a)
        .arg(b)
        .succeeds()
        .no_output();

    assert_eq!("a", at.read(a));
    assert_eq!("b", at.read(b));
    assert_eq!(mode_a, at.metadata(a).mode());
    assert_eq!(mode_b, at.metadata(b).mode());
}

#[test]
fn test_cp_seen_file() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.mkdir("a");
    at.mkdir("b");
    at.mkdir("c");
    at.write("a/f", "a");
    at.write("b/f", "b");

    let result = ts.ucmd().arg("a/f").arg("b/f").arg("c").fails();
    #[cfg(not(unix))]
    assert!(result
        .stderr_str()
        .contains("will not overwrite just-created 'c\\f' with 'b/f'"));
    #[cfg(unix)]
    assert!(result
        .stderr_str()
        .contains("will not overwrite just-created 'c/f' with 'b/f'"));

    assert!(at.plus("c").join("f").exists());

    ts.ucmd()
        .arg("--backup=numbered")
        .arg("a/f")
        .arg("b/f")
        .arg("c")
        .succeeds();
    assert!(at.plus("c").join("f").exists());
    assert!(at.plus("c").join("f.~1~").exists());
}

#[test]
fn test_cp_path_ends_with_terminator() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkdir("a");
    ts.ucmd().arg("-r").arg("-T").arg("a").arg("e/").succeeds();
}

#[test]
fn test_cp_no_such() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("b");
    ts.ucmd()
        .arg("b")
        .arg("no-such/")
        .fails()
        .stderr_is("cp: 'no-such/' is not a directory\n");
}

#[cfg(all(
    unix,
    not(any(target_os = "android", target_os = "macos", target_os = "openbsd"))
))]
#[test]
fn test_acl_preserve() {
    use std::process::Command;

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

    scene.ucmd().args(&["-p", &path, path2]).succeeds();

    assert!(compare_xattrs(&file, &file_target));
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_reflink_never_with_hole() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.write("a", "hello");
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(at.plus("a"))
        .unwrap();
    f.set_len(10000).unwrap();

    ts.ucmd()
        .arg("--debug")
        .arg("--reflink=never")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: no, sparse detection: SEEK_HOLE");

    let src_file_metadata = std::fs::metadata(at.plus("a")).unwrap();
    let dst_file_metadata = std::fs::metadata(at.plus("b")).unwrap();
    assert_eq!(src_file_metadata.blocks(), dst_file_metadata.blocks());
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_reflink_never_empty_file_with_hole() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(at.plus("a"))
        .unwrap();
    f.set_len(10000).unwrap();

    ts.ucmd()
        .arg("--debug")
        .arg("--reflink=never")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: unknown, reflink: no, sparse detection: SEEK_HOLE");

    let src_file_metadata = std::fs::metadata(at.plus("a")).unwrap();
    let dst_file_metadata = std::fs::metadata(at.plus("b")).unwrap();
    assert_eq!(src_file_metadata.blocks(), dst_file_metadata.blocks());
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_default_with_hole() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(at.plus("a"))
        .unwrap();
    f.set_len(10000).unwrap();

    at.append_bytes("a", "hello".as_bytes());

    ts.ucmd()
        .arg("--debug")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: yes, reflink: unsupported, sparse detection: SEEK_HOLE");

    let src_file_metadata = std::fs::metadata(at.plus("a")).unwrap();
    let dst_file_metadata = std::fs::metadata(at.plus("b")).unwrap();
    assert_eq!(src_file_metadata.blocks(), dst_file_metadata.blocks());
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_default_less_than_512_bytes() {
    let ts = TestScenario::new(util_name!());

    let at = &ts.fixtures;
    at.write_bytes("a", "hello".as_bytes());
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(at.plus("a"))
        .unwrap();
    f.set_len(400).unwrap();

    ts.ucmd()
        .arg("--debug")
        .arg("--reflink=auto")
        .arg("--sparse=auto")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: yes, reflink: unsupported, sparse detection: no");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_default_without_hole() {
    let ts = TestScenario::new(util_name!());

    let at = &ts.fixtures;
    at.write_bytes("a", "hello".as_bytes());

    let filler_bytes = [0_u8; 10000];

    at.append_bytes("a", &filler_bytes);

    ts.ucmd()
        .arg("--debug")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: yes, reflink: unsupported, sparse detection: no");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_default_empty_file_with_hole() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(at.plus("a"))
        .unwrap();
    f.set_len(10000).unwrap();

    ts.ucmd()
        .arg("--debug")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains(
            "copy offload: unknown, reflink: unsupported, sparse detection: SEEK_HOLE",
        );

    let src_file_metadata = std::fs::metadata(at.plus("a")).unwrap();
    let dst_file_metadata = std::fs::metadata(at.plus("b")).unwrap();
    assert_eq!(src_file_metadata.blocks(), dst_file_metadata.blocks());
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_reflink_never_sparse_always_with_hole() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.write("a", "hello");
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(at.plus("a"))
        .unwrap();
    f.set_len(10000).unwrap();

    ts.ucmd()
        .arg("--debug")
        .arg("--reflink=never")
        .arg("--sparse=always")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: no, sparse detection: SEEK_HOLE + zeros");

    let src_file_metadata = std::fs::metadata(at.plus("a")).unwrap();
    let dst_file_metadata = std::fs::metadata(at.plus("b")).unwrap();
    assert_eq!(src_file_metadata.blocks(), dst_file_metadata.blocks());
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_reflink_never_sparse_always_without_hole() {
    let ts = TestScenario::new(util_name!());
    let empty_bytes = [0_u8; 10000];
    let at = &ts.fixtures;
    at.write("a", "hello");
    at.append_bytes("a", &empty_bytes);

    ts.ucmd()
        .arg("--debug")
        .arg("--reflink=never")
        .arg("--sparse=always")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: no, sparse detection: zeros");

    let dst_file_metadata = std::fs::metadata(at.plus("b")).unwrap();
    assert_eq!(
        dst_file_metadata.blocks(),
        dst_file_metadata.blksize() / 512
    );
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_reflink_never_sparse_always_empty_file_with_hole() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(at.plus("a"))
        .unwrap();
    f.set_len(10000).unwrap();

    ts.ucmd()
        .arg("--debug")
        .arg("--reflink=never")
        .arg("--sparse=always")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: unknown, reflink: no, sparse detection: SEEK_HOLE");
}

#[test]
#[cfg(target_os = "linux")]
fn test_cp_default_virtual_file() {
    // This file has existed at least since 2008, so we assume that it is present on "all" Linux kernels.
    // https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-profiling

    use std::os::unix::prelude::MetadataExt;
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    ts.ucmd().arg("/sys/kernel/profiling").arg("b").succeeds();

    let dest_size = std::fs::metadata(at.plus("b"))
        .expect("Metadata of copied file cannot be read")
        .size();
    assert!(dest_size > 0);
}
#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_reflink_auto_sparse_always_non_sparse_file_with_long_zero_sequence() {
    let ts = TestScenario::new(util_name!());

    let buf: Vec<u8> = vec![0; 4096 * 4];
    let at = &ts.fixtures;
    at.touch("a");
    at.append_bytes("a", &buf);
    at.append_bytes("a", "hello".as_bytes());

    ts.ucmd()
        .arg("--debug")
        .arg("--sparse=always")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: unsupported, sparse detection: zeros");

    let dst_file_metadata = std::fs::metadata(at.plus("b")).unwrap();
    assert_eq!(
        dst_file_metadata.blocks(),
        dst_file_metadata.blksize() / 512
    );
}

#[test]
#[cfg(target_os = "linux")]
fn test_cp_debug_sparse_never_empty_sparse_file() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");

    ts.ucmd()
        .arg("--debug")
        .arg("--sparse=never")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: no, sparse detection: no");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_reflink_never_sparse_always_non_sparse_file_with_long_zero_sequence() {
    let ts = TestScenario::new(util_name!());

    let buf: Vec<u8> = vec![0; 4096 * 4];
    let at = &ts.fixtures;
    at.touch("a");
    at.append_bytes("a", &buf);
    at.append_bytes("a", "hello".as_bytes());

    ts.ucmd()
        .arg("--debug")
        .arg("--reflink=never")
        .arg("--sparse=always")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: no, sparse detection: zeros");

    let dst_file_metadata = std::fs::metadata(at.plus("b")).unwrap();
    assert_eq!(
        dst_file_metadata.blocks(),
        dst_file_metadata.blksize() / 512
    );
}

#[test]
#[cfg(target_os = "linux")]
fn test_cp_debug_sparse_always_sparse_virtual_file() {
    // This file has existed at least since 2008, so we assume that it is present on "all" Linux kernels.
    // https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-profiling
    let ts = TestScenario::new(util_name!());
    ts.ucmd()
        .arg("--debug")
        .arg("--sparse=always")
        .arg("/sys/kernel/profiling")
        .arg("b")
        .succeeds()
        .stdout_contains(
            "copy offload: avoided, reflink: unsupported, sparse detection: SEEK_HOLE + zeros",
        );
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_reflink_never_less_than_512_bytes() {
    let ts = TestScenario::new(util_name!());

    let at = &ts.fixtures;
    at.write_bytes("a", "hello".as_bytes());
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(at.plus("a"))
        .unwrap();
    f.set_len(400).unwrap();

    ts.ucmd()
        .arg("--debug")
        .arg("--reflink=never")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: no, sparse detection: no");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_reflink_never_sparse_never_empty_file_with_hole() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(at.plus("a"))
        .unwrap();
    f.set_len(10000).unwrap();

    ts.ucmd()
        .arg("--debug")
        .arg("--reflink=never")
        .arg("--sparse=never")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: unknown, reflink: no, sparse detection: SEEK_HOLE");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_reflink_never_file_with_hole() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(at.plus("a"))
        .unwrap();
    f.set_len(10000).unwrap();
    at.append_bytes("a", "hello".as_bytes());

    ts.ucmd()
        .arg("--debug")
        .arg("--reflink=never")
        .arg("--sparse=never")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: no, sparse detection: SEEK_HOLE");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_sparse_never_less_than_512_bytes() {
    let ts = TestScenario::new(util_name!());

    let at = &ts.fixtures;
    at.write_bytes("a", "hello".as_bytes());
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(at.plus("a"))
        .unwrap();
    f.set_len(400).unwrap();

    ts.ucmd()
        .arg("--debug")
        .arg("--reflink=auto")
        .arg("--sparse=never")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: no, sparse detection: no");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_sparse_never_without_hole() {
    let ts = TestScenario::new(util_name!());

    let at = &ts.fixtures;
    at.write_bytes("a", "hello".as_bytes());

    let filler_bytes = [0_u8; 10000];

    at.append_bytes("a", &filler_bytes);

    ts.ucmd()
        .arg("--reflink=auto")
        .arg("--sparse=never")
        .arg("--debug")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: no, sparse detection: no");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_sparse_never_empty_file_with_hole() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(at.plus("a"))
        .unwrap();
    f.set_len(10000).unwrap();

    ts.ucmd()
        .arg("--debug")
        .arg("--reflink=auto")
        .arg("--sparse=never")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: unknown, reflink: no, sparse detection: SEEK_HOLE");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_sparse_never_file_with_hole() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("a");
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(at.plus("a"))
        .unwrap();
    f.set_len(10000).unwrap();
    at.append_bytes("a", "hello".as_bytes());

    ts.ucmd()
        .arg("--debug")
        .arg("--reflink=auto")
        .arg("--sparse=never")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: no, sparse detection: SEEK_HOLE");
}

#[test]
#[cfg(target_os = "linux")]
fn test_cp_debug_default_sparse_virtual_file() {
    // This file has existed at least since 2008, so we assume that it is present on "all" Linux kernels.
    // https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-profiling
    let ts = TestScenario::new(util_name!());
    ts.ucmd()
        .arg("--debug")
        .arg("/sys/kernel/profiling")
        .arg("b")
        .succeeds()
        .stdout_contains(
            "copy offload: unsupported, reflink: unsupported, sparse detection: SEEK_HOLE",
        );
}

#[test]
#[cfg(target_os = "linux")]
fn test_cp_debug_sparse_never_zero_sized_virtual_file() {
    let ts = TestScenario::new(util_name!());
    ts.ucmd()
        .arg("--debug")
        .arg("--sparse=never")
        .arg("/proc/version")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: no, sparse detection: no");
}

#[test]
#[cfg(target_os = "linux")]
fn test_cp_debug_default_zero_sized_virtual_file() {
    let ts = TestScenario::new(util_name!());
    ts.ucmd()
        .arg("--debug")
        .arg("/proc/version")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: unsupported, reflink: unsupported, sparse detection: no");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_cp_debug_reflink_never_without_hole() {
    let ts = TestScenario::new(util_name!());
    let filler_bytes = [0_u8; 1000];
    let at = &ts.fixtures;
    at.write("a", "hello");
    at.append_bytes("a", &filler_bytes);

    ts.ucmd()
        .arg("--debug")
        .arg("--reflink=never")
        .arg("a")
        .arg("b")
        .succeeds()
        .stdout_contains("copy offload: avoided, reflink: no, sparse detection: no");
}

#[test]
fn test_cp_force_remove_destination_attributes_only_with_symlink() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("file1", "1");
    at.write("file2", "2");
    at.symlink_file("file1", "sym1");

    scene
        .ucmd()
        .args(&[
            "-a",
            "--remove-destination",
            "--attributes-only",
            "sym1",
            "file2",
        ])
        .succeeds();

    assert!(
        at.symlink_exists("file2"),
        "file2 is not a symbolic link as expected"
    );

    assert_eq!(
        at.read("file1"),
        at.read("file2"),
        "Contents of file1 and file2 do not match"
    );
}

#[test]
fn test_cp_no_dereference_attributes_only_with_symlink() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("file1", "1");
    at.write("file2", "2");
    at.write("file2.exp", "2");
    at.symlink_file("file1", "sym1");

    let result = scene
        .ucmd()
        .args(&["--no-dereference", "--attributes-only", "sym1", "file2"])
        .fails();

    assert_eq!(result.code(), 1, "cp command did not fail");

    assert_eq!(
        at.read("file2"),
        at.read("file2.exp"),
        "file2 content does not match expected"
    );
}
#[cfg(all(unix, not(target_os = "android")))]
#[cfg(test)]
/// contains the test for cp when the source and destination points to the same file
mod same_file {

    use uutests::util::TestScenario;
    use uutests::util_name;

    const FILE_NAME: &str = "foo";
    const SYMLINK_NAME: &str = "symlink";
    const CONTENTS: &str = "abcd";

    // the following tests tries to copy a file to the symlink of the same file with
    // various options
    #[test]
    fn test_same_file_from_file_to_symlink() {
        for option in ["-d", "-f", "-df"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, SYMLINK_NAME);
            scene
                .ucmd()
                .args(&[option, FILE_NAME, SYMLINK_NAME])
                .fails()
                .stderr_contains("'foo' and 'symlink' are the same file");
            assert!(at.symlink_exists(SYMLINK_NAME));
            assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
            assert!(at.file_exists(FILE_NAME));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_file_to_symlink_with_rem_option() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        at.write(FILE_NAME, CONTENTS);
        at.symlink_file(FILE_NAME, SYMLINK_NAME);
        scene
            .ucmd()
            .args(&["--rem", FILE_NAME, SYMLINK_NAME])
            .succeeds();
        assert!(at.file_exists(SYMLINK_NAME));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
        assert!(at.file_exists(FILE_NAME));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
    }

    #[test]
    fn test_same_file_from_file_to_symlink_with_backup_option() {
        for option in ["-b", "-bd", "-bf", "-bdf"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let backup = "symlink~";
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, SYMLINK_NAME);
            scene
                .ucmd()
                .args(&[option, FILE_NAME, SYMLINK_NAME])
                .succeeds();
            assert!(at.symlink_exists(backup));
            assert_eq!(FILE_NAME, at.resolve_link(backup));
            assert!(at.file_exists(SYMLINK_NAME));
            assert_eq!(at.read(SYMLINK_NAME), CONTENTS,);
            assert!(at.file_exists(FILE_NAME));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_file_to_symlink_with_link_option() {
        for option in ["-l", "-dl"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, SYMLINK_NAME);
            scene
                .ucmd()
                .args(&[option, FILE_NAME, SYMLINK_NAME])
                .fails()
                .stderr_contains("cp: cannot create hard link 'symlink' to 'foo'");
            assert!(at.symlink_exists(SYMLINK_NAME));
            assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
            assert!(at.file_exists(FILE_NAME));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_file_to_symlink_with_options_link_and_force() {
        for option in ["-fl", "-dfl"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, SYMLINK_NAME);
            scene
                .ucmd()
                .args(&[option, FILE_NAME, SYMLINK_NAME])
                .succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert!(at.file_exists(SYMLINK_NAME));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_file_to_symlink_with_options_backup_and_link() {
        for option in ["-bl", "-bdl", "-bfl", "-bdfl"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let backup = "symlink~";
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, SYMLINK_NAME);
            scene
                .ucmd()
                .args(&[option, FILE_NAME, SYMLINK_NAME])
                .succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert!(at.file_exists(SYMLINK_NAME));
            assert!(at.symlink_exists(backup));
            assert_eq!(FILE_NAME, at.resolve_link(backup));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_file_to_symlink_with_options_symlink() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        at.write(FILE_NAME, CONTENTS);
        at.symlink_file(FILE_NAME, SYMLINK_NAME);
        scene
            .ucmd()
            .args(&["-s", FILE_NAME, SYMLINK_NAME])
            .fails()
            .stderr_contains("cp: cannot create symlink 'symlink' to 'foo'");
        assert!(at.file_exists(FILE_NAME));
        assert!(at.symlink_exists(SYMLINK_NAME));
        assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
    }

    #[test]
    fn test_same_file_from_file_to_symlink_with_options_symlink_and_force() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        at.write(FILE_NAME, CONTENTS);
        at.symlink_file(FILE_NAME, SYMLINK_NAME);
        scene
            .ucmd()
            .args(&["-sf", FILE_NAME, SYMLINK_NAME])
            .succeeds();
        assert!(at.file_exists(FILE_NAME));
        assert!(at.symlink_exists(SYMLINK_NAME));
        assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
    }
    // the following tests tries to copy a symlink to the file that symlink points to with
    // various options
    #[test]
    fn test_same_file_from_symlink_to_file() {
        for option in ["-d", "-f", "-df", "--rem"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, SYMLINK_NAME);
            scene
                .ucmd()
                .args(&[option, SYMLINK_NAME, FILE_NAME])
                .fails()
                .stderr_contains("'symlink' and 'foo' are the same file");
            assert!(at.file_exists(FILE_NAME));
            assert!(at.symlink_exists(SYMLINK_NAME));
            assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_symlink_to_file_with_option_backup() {
        for option in ["-b", "-bf"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, SYMLINK_NAME);
            scene
                .ucmd()
                .args(&[option, SYMLINK_NAME, FILE_NAME])
                .fails()
                .stderr_contains("'symlink' and 'foo' are the same file");
            assert!(at.file_exists(FILE_NAME));
            assert!(at.symlink_exists(SYMLINK_NAME));
            assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }
    #[test]
    fn test_same_file_from_symlink_to_file_with_option_backup_without_deref() {
        for option in ["-bd", "-bdf"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let backup = "foo~";
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, SYMLINK_NAME);
            scene
                .ucmd()
                .args(&[option, SYMLINK_NAME, FILE_NAME])
                .succeeds();
            assert!(at.file_exists(backup));
            assert!(at.symlink_exists(FILE_NAME));
            // this doesn't makes sense but this is how gnu does it
            assert_eq!(FILE_NAME, at.resolve_link(FILE_NAME));
            assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
            assert_eq!(at.read(backup), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_symlink_to_file_with_options_link() {
        for option in ["-l", "-dl", "-fl", "-bl", "-bfl"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, SYMLINK_NAME);
            scene
                .ucmd()
                .args(&[option, SYMLINK_NAME, FILE_NAME])
                .succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert!(at.symlink_exists(SYMLINK_NAME));
            assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_symlink_to_file_with_option_symlink() {
        for option in ["-s", "-sf"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, SYMLINK_NAME);
            scene
                .ucmd()
                .args(&[option, SYMLINK_NAME, FILE_NAME])
                .fails()
                .stderr_contains("'symlink' and 'foo' are the same file");
            assert!(at.file_exists(FILE_NAME));
            assert!(at.symlink_exists(SYMLINK_NAME));
            assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    // the following tests tries to copy a file to the same file with various options
    #[test]
    fn test_same_file_from_file_to_file() {
        for option in ["-d", "-f", "-df", "--rem"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            at.write(FILE_NAME, CONTENTS);
            scene
                .ucmd()
                .args(&[option, FILE_NAME, FILE_NAME])
                .fails()
                .stderr_contains("'foo' and 'foo' are the same file");
            assert!(at.file_exists(FILE_NAME));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }
    #[test]
    fn test_same_file_from_file_to_file_with_backup() {
        for option in ["-b", "-bd"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            at.write(FILE_NAME, CONTENTS);
            scene
                .ucmd()
                .args(&[option, FILE_NAME, FILE_NAME])
                .fails()
                .stderr_contains("'foo' and 'foo' are the same file");
            assert!(at.file_exists(FILE_NAME));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_file_to_file_with_options_backup_and_no_deref() {
        for option in ["-bf", "-bdf"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let backup = "foo~";
            at.write(FILE_NAME, CONTENTS);
            scene
                .ucmd()
                .args(&[option, FILE_NAME, FILE_NAME])
                .succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert!(at.file_exists(backup));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
            assert_eq!(at.read(backup), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_file_to_file_with_options_link() {
        for option in ["-l", "-dl", "-fl", "-dfl", "-bl", "-bdl"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let backup = "foo~";
            at.write(FILE_NAME, CONTENTS);
            scene
                .ucmd()
                .args(&[option, FILE_NAME, FILE_NAME])
                .succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert!(!at.file_exists(backup));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_file_to_file_with_options_link_and_backup_and_force() {
        for option in ["-bfl", "-bdfl"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let backup = "foo~";
            at.write(FILE_NAME, CONTENTS);
            scene
                .ucmd()
                .args(&[option, FILE_NAME, FILE_NAME])
                .succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert!(at.file_exists(backup));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
            assert_eq!(at.read(backup), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_file_to_file_with_options_symlink() {
        for option in ["-s", "-sf"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            at.write(FILE_NAME, CONTENTS);
            scene
                .ucmd()
                .args(&[option, FILE_NAME, FILE_NAME])
                .fails()
                .stderr_contains("'foo' and 'foo' are the same file");
            assert!(at.file_exists(FILE_NAME));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    // the following tests tries to copy a symlink that points to a file to a symlink
    // that points to the same file with various options
    #[test]
    fn test_same_file_from_symlink_to_symlink_with_option_no_deref() {
        for option in ["-d", "-df"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let symlink1 = "sl1";
            let symlink2 = "sl2";
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, symlink1);
            at.symlink_file(FILE_NAME, symlink2);
            scene.ucmd().args(&[option, symlink1, symlink2]).succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
            assert_eq!(FILE_NAME, at.resolve_link(symlink1));
            assert_eq!(FILE_NAME, at.resolve_link(symlink2));
        }
    }

    #[test]
    fn test_same_file_from_symlink_to_symlink_with_option_force() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        let symlink1 = "sl1";
        let symlink2 = "sl2";
        at.write(FILE_NAME, CONTENTS);
        at.symlink_file(FILE_NAME, symlink1);
        at.symlink_file(FILE_NAME, symlink2);
        scene
            .ucmd()
            .args(&["-f", symlink1, symlink2])
            .fails()
            .stderr_contains("'sl1' and 'sl2' are the same file");
        assert!(at.file_exists(FILE_NAME));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
        assert_eq!(FILE_NAME, at.resolve_link(symlink1));
        assert_eq!(FILE_NAME, at.resolve_link(symlink2));
    }

    #[test]
    fn test_same_file_from_symlink_to_symlink_with_option_rem() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        let symlink1 = "sl1";
        let symlink2 = "sl2";
        at.write(FILE_NAME, CONTENTS);
        at.symlink_file(FILE_NAME, symlink1);
        at.symlink_file(FILE_NAME, symlink2);
        scene.ucmd().args(&["--rem", symlink1, symlink2]).succeeds();
        assert!(at.file_exists(FILE_NAME));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
        assert_eq!(FILE_NAME, at.resolve_link(symlink1));
        assert!(at.file_exists(symlink2));
        assert_eq!(at.read(symlink2), CONTENTS,);
    }

    #[test]
    fn test_same_file_from_symlink_to_symlink_with_option_backup() {
        for option in ["-b", "-bf"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let symlink1 = "sl1";
            let symlink2 = "sl2";
            let backup = "sl2~";
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, symlink1);
            at.symlink_file(FILE_NAME, symlink2);
            scene.ucmd().args(&[option, symlink1, symlink2]).succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
            assert_eq!(FILE_NAME, at.resolve_link(symlink1));
            assert!(at.file_exists(symlink2));
            assert_eq!(at.read(symlink2), CONTENTS,);
            assert_eq!(FILE_NAME, at.resolve_link(backup));
        }
    }

    #[test]
    fn test_same_file_from_symlink_to_symlink_with_option_backup_and_no_deref() {
        for option in ["-bd", "-bdf"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let symlink1 = "sl1";
            let symlink2 = "sl2";
            let backup = "sl2~";
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, symlink1);
            at.symlink_file(FILE_NAME, symlink2);
            scene.ucmd().args(&[option, symlink1, symlink2]).succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
            assert_eq!(FILE_NAME, at.resolve_link(symlink1));
            assert_eq!(FILE_NAME, at.resolve_link(symlink2));
            assert_eq!(FILE_NAME, at.resolve_link(backup));
        }
    }
    #[test]
    fn test_same_file_from_symlink_to_symlink_with_option_link() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        let symlink1 = "sl1";
        let symlink2 = "sl2";
        at.write(FILE_NAME, CONTENTS);
        at.symlink_file(FILE_NAME, symlink1);
        at.symlink_file(FILE_NAME, symlink2);
        scene
            .ucmd()
            .args(&["-l", symlink1, symlink2])
            .fails()
            .stderr_contains("cannot create hard link 'sl2' to 'sl1'");
        assert!(at.file_exists(FILE_NAME));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
        assert_eq!(FILE_NAME, at.resolve_link(symlink1));
        assert_eq!(FILE_NAME, at.resolve_link(symlink2));
    }

    #[test]
    fn test_same_file_from_symlink_to_symlink_with_option_force_link() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        let symlink1 = "sl1";
        let symlink2 = "sl2";
        at.write(FILE_NAME, CONTENTS);
        at.symlink_file(FILE_NAME, symlink1);
        at.symlink_file(FILE_NAME, symlink2);
        scene.ucmd().args(&["-fl", symlink1, symlink2]).succeeds();
        assert!(at.file_exists(FILE_NAME));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
        assert_eq!(FILE_NAME, at.resolve_link(symlink1));
        assert!(at.file_exists(symlink2));
        assert_eq!(at.read(symlink2), CONTENTS,);
    }

    #[test]
    fn test_same_file_from_symlink_to_symlink_with_option_backup_and_link() {
        for option in ["-bl", "-bfl"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let symlink1 = "sl1";
            let symlink2 = "sl2";
            let backup = "sl2~";
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, symlink1);
            at.symlink_file(FILE_NAME, symlink2);
            scene.ucmd().args(&[option, symlink1, symlink2]).succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
            assert_eq!(FILE_NAME, at.resolve_link(symlink1));
            assert!(at.file_exists(symlink2));
            assert_eq!(at.read(symlink2), CONTENTS,);
            assert_eq!(FILE_NAME, at.resolve_link(backup));
        }
    }

    #[test]
    fn test_same_file_from_symlink_to_symlink_with_option_symlink() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        let symlink1 = "sl1";
        let symlink2 = "sl2";
        at.write(FILE_NAME, CONTENTS);
        at.symlink_file(FILE_NAME, symlink1);
        at.symlink_file(FILE_NAME, symlink2);
        scene
            .ucmd()
            .args(&["-s", symlink1, symlink2])
            .fails()
            .stderr_contains("cannot create symlink 'sl2' to 'sl1'");
        assert!(at.file_exists(FILE_NAME));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
        assert_eq!(FILE_NAME, at.resolve_link(symlink1));
        assert_eq!(FILE_NAME, at.resolve_link(symlink2));
    }

    #[test]
    fn test_same_file_from_symlink_to_symlink_with_option_symlink_and_force() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        let symlink1 = "sl1";
        let symlink2 = "sl2";
        at.write(FILE_NAME, CONTENTS);
        at.symlink_file(FILE_NAME, symlink1);
        at.symlink_file(FILE_NAME, symlink2);
        scene.ucmd().args(&["-sf", symlink1, symlink2]).succeeds();
        assert!(at.file_exists(FILE_NAME));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
        assert_eq!(FILE_NAME, at.resolve_link(symlink1));
        assert_eq!(symlink1, at.resolve_link(symlink2));
    }

    // the following tests tries to copy file to a hardlink of the same file with
    // various options
    #[test]
    fn test_same_file_from_file_to_hardlink() {
        for option in ["-d", "-f", "-df"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let hardlink = "hardlink";
            at.write(FILE_NAME, CONTENTS);
            at.hard_link(FILE_NAME, hardlink);

            scene
                .ucmd()
                .args(&[option, FILE_NAME, hardlink])
                .fails()
                .stderr_contains("'foo' and 'hardlink' are the same file");
            assert!(at.file_exists(FILE_NAME));
            assert!(at.file_exists(hardlink));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_file_to_hardlink_with_option_rem() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        let hardlink = "hardlink";
        at.write(FILE_NAME, CONTENTS);
        at.hard_link(FILE_NAME, hardlink);
        scene
            .ucmd()
            .args(&["--rem", FILE_NAME, hardlink])
            .succeeds();
        assert!(at.file_exists(FILE_NAME));
        assert!(at.file_exists(hardlink));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
    }

    #[test]
    fn test_same_file_from_file_to_hardlink_with_option_backup() {
        for option in ["-b", "-bd", "-bf", "-bdf"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let hardlink = "hardlink";
            let backup = "hardlink~";
            at.write(FILE_NAME, CONTENTS);
            at.hard_link(FILE_NAME, hardlink);
            scene.ucmd().args(&[option, FILE_NAME, hardlink]).succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert!(at.file_exists(hardlink));
            assert!(at.file_exists(backup));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_file_to_hardlink_with_option_link() {
        for option in ["-l", "-dl", "-fl", "-dfl", "-bl", "-bdl", "-bfl", "-bdfl"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let hardlink = "hardlink";
            at.write(FILE_NAME, CONTENTS);
            at.hard_link(FILE_NAME, hardlink);
            scene.ucmd().args(&[option, FILE_NAME, hardlink]).succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert!(at.file_exists(hardlink));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_file_to_hardlink_with_option_symlink() {
        for option in ["-s", "-sf"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let hardlink = "hardlink";
            at.write(FILE_NAME, CONTENTS);
            at.hard_link(FILE_NAME, hardlink);
            scene
                .ucmd()
                .args(&[option, FILE_NAME, hardlink])
                .fails()
                .stderr_contains("'foo' and 'hardlink' are the same file");
            assert!(at.file_exists(FILE_NAME));
            assert!(at.file_exists(hardlink));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    // the following tests tries to copy symlink to a hardlink of the same symlink with
    // various options
    #[test]
    fn test_same_file_from_hard_link_of_symlink_to_symlink() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        let hardlink_to_symlink = "hlsl";
        at.write(FILE_NAME, CONTENTS);
        at.symlink_file(FILE_NAME, SYMLINK_NAME);
        at.hard_link(SYMLINK_NAME, hardlink_to_symlink);
        scene
            .ucmd()
            .args(&[hardlink_to_symlink, SYMLINK_NAME])
            .fails()
            .stderr_contains("cp: 'hlsl' and 'symlink' are the same file");
        assert!(at.file_exists(FILE_NAME));
        assert!(at.symlink_exists(SYMLINK_NAME));
        assert!(at.symlink_exists(hardlink_to_symlink));
        assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
        assert_eq!(FILE_NAME, at.resolve_link(hardlink_to_symlink));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
    }

    #[test]
    fn test_same_file_from_hard_link_of_symlink_to_symlink_with_option_force() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        let hardlink_to_symlink = "hlsl";
        at.write(FILE_NAME, CONTENTS);
        at.symlink_file(FILE_NAME, SYMLINK_NAME);
        at.hard_link(SYMLINK_NAME, hardlink_to_symlink);
        scene
            .ucmd()
            .args(&["-f", hardlink_to_symlink, SYMLINK_NAME])
            .fails()
            .stderr_contains("cp: 'hlsl' and 'symlink' are the same file");
        assert!(at.file_exists(FILE_NAME));
        assert!(at.symlink_exists(SYMLINK_NAME));
        assert!(at.symlink_exists(hardlink_to_symlink));
        assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
        assert_eq!(FILE_NAME, at.resolve_link(hardlink_to_symlink));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
    }

    #[test]
    fn test_same_file_from_hard_link_of_symlink_to_symlink_with_option_no_deref() {
        for option in ["-d", "-df"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let hardlink_to_symlink = "hlsl";
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, SYMLINK_NAME);
            at.hard_link(SYMLINK_NAME, hardlink_to_symlink);
            scene
                .ucmd()
                .args(&[option, hardlink_to_symlink, SYMLINK_NAME])
                .succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert!(at.symlink_exists(SYMLINK_NAME));
            assert!(at.symlink_exists(hardlink_to_symlink));
            assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
            assert_eq!(FILE_NAME, at.resolve_link(hardlink_to_symlink));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_hard_link_of_symlink_to_symlink_with_option_rem() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        let hardlink_to_symlink = "hlsl";
        at.write(FILE_NAME, CONTENTS);
        at.symlink_file(FILE_NAME, SYMLINK_NAME);
        at.hard_link(SYMLINK_NAME, hardlink_to_symlink);
        scene
            .ucmd()
            .args(&["--rem", hardlink_to_symlink, SYMLINK_NAME])
            .succeeds();
        assert!(at.file_exists(FILE_NAME));
        assert!(at.file_exists(SYMLINK_NAME));
        assert!(!at.symlink_exists(SYMLINK_NAME));
        assert!(at.symlink_exists(hardlink_to_symlink));
        assert_eq!(FILE_NAME, at.resolve_link(hardlink_to_symlink));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
        assert_eq!(at.read(SYMLINK_NAME), CONTENTS,);
    }

    #[test]
    fn test_same_file_from_hard_link_of_symlink_to_symlink_with_option_backup() {
        for option in ["-b", "-bf"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let backup = "symlink~";
            let hardlink_to_symlink = "hlsl";
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, SYMLINK_NAME);
            at.hard_link(SYMLINK_NAME, hardlink_to_symlink);
            scene
                .ucmd()
                .args(&[option, hardlink_to_symlink, SYMLINK_NAME])
                .succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert!(at.file_exists(SYMLINK_NAME));
            assert!(!at.symlink_exists(SYMLINK_NAME));
            assert!(at.symlink_exists(hardlink_to_symlink));
            assert_eq!(FILE_NAME, at.resolve_link(hardlink_to_symlink));
            assert!(at.symlink_exists(backup));
            assert_eq!(FILE_NAME, at.resolve_link(backup));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
            assert_eq!(at.read(SYMLINK_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_hard_link_of_symlink_to_symlink_with_option_backup_and_no_deref() {
        for option in ["-bd", "-bdf"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let backup = "symlink~";
            let hardlink_to_symlink = "hlsl";
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, SYMLINK_NAME);
            at.hard_link(SYMLINK_NAME, hardlink_to_symlink);
            scene
                .ucmd()
                .args(&[option, hardlink_to_symlink, SYMLINK_NAME])
                .succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert!(at.symlink_exists(SYMLINK_NAME));
            assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
            assert!(at.symlink_exists(hardlink_to_symlink));
            assert_eq!(FILE_NAME, at.resolve_link(hardlink_to_symlink));
            assert!(at.symlink_exists(backup));
            assert_eq!(FILE_NAME, at.resolve_link(backup));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_hard_link_of_symlink_to_symlink_with_option_link() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        let hardlink_to_symlink = "hlsl";
        at.write(FILE_NAME, CONTENTS);
        at.symlink_file(FILE_NAME, SYMLINK_NAME);
        at.hard_link(SYMLINK_NAME, hardlink_to_symlink);
        scene
            .ucmd()
            .args(&["-l", hardlink_to_symlink, SYMLINK_NAME])
            .fails()
            .stderr_contains("cannot create hard link 'symlink' to 'hlsl'");
        assert!(at.file_exists(FILE_NAME));
        assert!(at.symlink_exists(SYMLINK_NAME));
        assert!(at.symlink_exists(hardlink_to_symlink));
        assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
        assert_eq!(FILE_NAME, at.resolve_link(hardlink_to_symlink));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
    }

    #[test]
    fn test_same_file_from_hard_link_of_symlink_to_symlink_with_option_link_and_no_deref() {
        for option in ["-dl", "-dfl"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let hardlink_to_symlink = "hlsl";
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, SYMLINK_NAME);
            at.hard_link(SYMLINK_NAME, hardlink_to_symlink);
            scene
                .ucmd()
                .args(&[option, hardlink_to_symlink, SYMLINK_NAME])
                .succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert!(at.symlink_exists(SYMLINK_NAME));
            assert!(at.symlink_exists(hardlink_to_symlink));
            assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
            assert_eq!(FILE_NAME, at.resolve_link(hardlink_to_symlink));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_hard_link_of_symlink_to_symlink_with_option_link_and_force() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        let hardlink_to_symlink = "hlsl";
        at.write(FILE_NAME, CONTENTS);
        at.symlink_file(FILE_NAME, SYMLINK_NAME);
        at.hard_link(SYMLINK_NAME, hardlink_to_symlink);
        scene
            .ucmd()
            .args(&["-fl", hardlink_to_symlink, SYMLINK_NAME])
            .succeeds();
        assert!(at.file_exists(FILE_NAME));
        assert!(at.file_exists(SYMLINK_NAME));
        assert!(!at.symlink_exists(SYMLINK_NAME));
        assert!(at.symlink_exists(hardlink_to_symlink));
        assert_eq!(FILE_NAME, at.resolve_link(hardlink_to_symlink));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
    }

    #[test]
    fn test_same_file_from_hard_link_of_symlink_to_symlink_with_option_link_and_backup() {
        for option in ["-bl", "-bfl"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let backup = "symlink~";
            let hardlink_to_symlink = "hlsl";
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, SYMLINK_NAME);
            at.hard_link(SYMLINK_NAME, hardlink_to_symlink);
            scene
                .ucmd()
                .args(&[option, hardlink_to_symlink, SYMLINK_NAME])
                .succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert!(at.file_exists(SYMLINK_NAME));
            assert!(!at.symlink_exists(SYMLINK_NAME));
            assert!(at.symlink_exists(hardlink_to_symlink));
            assert_eq!(FILE_NAME, at.resolve_link(hardlink_to_symlink));
            assert!(at.symlink_exists(backup));
            assert_eq!(FILE_NAME, at.resolve_link(backup));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_hard_link_of_symlink_to_symlink_with_options_backup_link_no_deref() {
        for option in ["-bdl", "-bdfl"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            let hardlink_to_symlink = "hlsl";
            at.write(FILE_NAME, CONTENTS);
            at.symlink_file(FILE_NAME, SYMLINK_NAME);
            at.hard_link(SYMLINK_NAME, hardlink_to_symlink);
            scene
                .ucmd()
                .args(&[option, hardlink_to_symlink, SYMLINK_NAME])
                .succeeds();
            assert!(at.file_exists(FILE_NAME));
            assert!(at.symlink_exists(SYMLINK_NAME));
            assert!(at.symlink_exists(hardlink_to_symlink));
            assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
            assert_eq!(FILE_NAME, at.resolve_link(hardlink_to_symlink));
            assert_eq!(at.read(FILE_NAME), CONTENTS,);
        }
    }

    #[test]
    fn test_same_file_from_hard_link_of_symlink_to_symlink_with_option_symlink() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        let hardlink_to_symlink = "hlsl";
        at.write(FILE_NAME, CONTENTS);
        at.symlink_file(FILE_NAME, SYMLINK_NAME);
        at.hard_link(SYMLINK_NAME, hardlink_to_symlink);
        scene
            .ucmd()
            .args(&["-s", hardlink_to_symlink, SYMLINK_NAME])
            .fails()
            .stderr_contains("cannot create symlink 'symlink' to 'hlsl'");
        assert!(at.file_exists(FILE_NAME));
        assert!(at.symlink_exists(SYMLINK_NAME));
        assert!(at.symlink_exists(hardlink_to_symlink));
        assert_eq!(FILE_NAME, at.resolve_link(SYMLINK_NAME));
        assert_eq!(FILE_NAME, at.resolve_link(hardlink_to_symlink));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
    }

    #[test]
    fn test_same_file_from_hard_link_of_symlink_to_symlink_with_option_symlink_and_force() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        let hardlink_to_symlink = "hlsl";
        at.write(FILE_NAME, CONTENTS);
        at.symlink_file(FILE_NAME, SYMLINK_NAME);
        at.hard_link(SYMLINK_NAME, hardlink_to_symlink);
        scene
            .ucmd()
            .args(&["-sf", hardlink_to_symlink, SYMLINK_NAME])
            .succeeds();
        assert!(at.file_exists(FILE_NAME));
        assert!(at.symlink_exists(SYMLINK_NAME));
        assert!(at.symlink_exists(hardlink_to_symlink));
        assert_eq!(hardlink_to_symlink, at.resolve_link(SYMLINK_NAME));
        assert_eq!(FILE_NAME, at.resolve_link(hardlink_to_symlink));
        assert_eq!(at.read(FILE_NAME), CONTENTS,);
    }
}

// the following tests are for how the cp should behave when the source is a symlink
// and link option is given
#[cfg(all(unix, not(target_os = "android")))]
mod link_deref {

    use std::os::unix::fs::MetadataExt;
    use uutests::util::{AtPath, TestScenario};
    use uutests::util_name;

    const FILE: &str = "file";
    const FILE_LINK: &str = "file_link";
    const DIR: &str = "dir";
    const DIR_LINK: &str = "dir_link";
    const DANG_LINK: &str = "dang_link";
    const DST: &str = "dst";

    fn setup_link_deref_tests(source: &str, at: &AtPath) {
        match source {
            FILE_LINK => {
                at.touch(FILE);
                at.symlink_file(FILE, FILE_LINK);
            }
            DIR_LINK => {
                at.mkdir(DIR);
                at.symlink_dir(DIR, DIR_LINK);
            }
            DANG_LINK => at.symlink_file("nowhere", DANG_LINK),
            _ => {}
        }
    }

    // cp --link shouldn't deref source if -P is given
    #[test]
    fn test_cp_symlink_as_source_with_link_and_no_deref() {
        for src in [FILE_LINK, DIR_LINK, DANG_LINK] {
            for r in [false, true] {
                let scene = TestScenario::new(util_name!());
                let at = &scene.fixtures;
                setup_link_deref_tests(src, at);
                let mut args = vec!["--link", "-P", src, DST];
                if r {
                    args.push("-R");
                };
                scene.ucmd().args(&args).succeeds().no_stderr();
                at.is_symlink(DST);
                let src_ino = at.symlink_metadata(src).ino();
                let dest_ino = at.symlink_metadata(DST).ino();
                assert_eq!(src_ino, dest_ino);
            }
        }
    }

    // Dereferencing should fail for dangling symlink.
    #[test]
    fn test_cp_dang_link_as_source_with_link() {
        for option in ["", "-L", "-H"] {
            for r in [false, true] {
                let scene = TestScenario::new(util_name!());
                let at = &scene.fixtures;
                setup_link_deref_tests(DANG_LINK, at);
                let mut args = vec!["--link", DANG_LINK, DST];
                if r {
                    args.push("-R");
                };
                if !option.is_empty() {
                    args.push(option);
                }
                scene
                    .ucmd()
                    .args(&args)
                    .fails()
                    .stderr_contains("No such file or directory");
            }
        }
    }

    // Dereferencing should fail for the 'dir_link' without -R.
    #[test]
    fn test_cp_dir_link_as_source_with_link() {
        for option in ["", "-L", "-H"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            setup_link_deref_tests(DIR_LINK, at);
            let mut args = vec!["--link", DIR_LINK, DST];
            if !option.is_empty() {
                args.push(option);
            }
            scene
                .ucmd()
                .args(&args)
                .fails()
                .stderr_contains("cp: -r not specified; omitting directory");
        }
    }

    // cp --link -R 'dir_link' should create a new directory.
    #[test]
    fn test_cp_dir_link_as_source_with_link_and_r() {
        for option in ["", "-L", "-H"] {
            let scene = TestScenario::new(util_name!());
            let at = &scene.fixtures;
            setup_link_deref_tests(DIR_LINK, at);
            let mut args = vec!["--link", "-R", DIR_LINK, DST];
            if !option.is_empty() {
                args.push(option);
            }
            scene.ucmd().args(&args).succeeds();
            at.dir_exists(DST);
        }
    }

    //cp --link 'file_link' should create a hard link to the target.
    #[test]
    fn test_cp_file_link_as_source_with_link() {
        for option in ["", "-L", "-H"] {
            for r in [false, true] {
                let scene = TestScenario::new(util_name!());
                let at = &scene.fixtures;
                setup_link_deref_tests(FILE_LINK, at);
                let mut args = vec!["--link", "-R", FILE_LINK, DST];
                if !option.is_empty() {
                    args.push(option);
                }
                if r {
                    args.push("-R");
                }
                scene.ucmd().args(&args).succeeds();
                at.file_exists(DST);
                let src_ino = at.symlink_metadata(FILE).ino();
                let dest_ino = at.symlink_metadata(DST).ino();
                assert_eq!(src_ino, dest_ino);
            }
        }
    }
}

// The cp command might create directories with excessively permissive permissions temporarily,
// which could be problematic if we aim to preserve ownership or mode. For example, when
// copying a directory, the destination directory could temporarily be setgid on some filesystems.
// This temporary setgid status could grant access to other users who share the same group
// ownership as the newly created directory. To mitigate this issue, when creating a directory we
// disable these excessive permissions.
#[test]
#[cfg(unix)]
#[cfg(not(target_os = "openbsd"))]
fn test_dir_perm_race_with_preserve_mode_and_ownership() {
    const SRC_DIR: &str = "src";
    const DEST_DIR: &str = "dest";
    const FIFO: &str = "src/fifo";
    for attr in ["mode", "ownership"] {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        at.mkdir(SRC_DIR);
        at.mkdir(DEST_DIR);
        at.set_mode(SRC_DIR, 0o775);
        at.set_mode(DEST_DIR, 0o2775);
        at.mkfifo(FIFO);
        let child = scene
            .ucmd()
            .args(&[
                format!("--preserve={attr}").as_str(),
                "-R",
                "--copy-contents",
                "--parents",
                SRC_DIR,
                DEST_DIR,
            ])
            // make sure permissions weren't disabled because of umask.
            .umask(0)
            .run_no_wait();
        // while cp wait for fifo we could check the dirs created by cp
        let timeout = Duration::from_secs(10);
        let start_time = std::time::Instant::now();
        // wait for cp to create dirs
        loop {
            assert!(
                start_time.elapsed() < timeout,
                "timed out: cp took too long to create destination directory"
            );
            if at.dir_exists(&format!("{DEST_DIR}/{SRC_DIR}")) {
                break;
            }
            sleep(Duration::from_millis(100));
        }
        let mode = at.metadata(&format!("{DEST_DIR}/{SRC_DIR}")).mode();
        #[allow(clippy::unnecessary_cast, clippy::cast_lossless)]
        let mask = if attr == "mode" {
            libc::S_IWGRP | libc::S_IWOTH
        } else {
            libc::S_IRWXG | libc::S_IRWXO
        } as u32;
        assert_eq!(
            (mode & mask),
            0,
            "unwanted permissions are present - {attr}"
        );
        at.write(FIFO, "done");
        child.wait().unwrap().succeeded();
    }
}

#[test]
// when -d and -a are overridden with --preserve or --no-preserve make sure that it only
// overrides attributes not other flags like -r or --no_deref implied in -a and -d.
fn test_preserve_attrs_overriding_1() {
    const FILE: &str = "file";
    const SYMLINK: &str = "symlink";
    const DEST: &str = "dest";
    for f in ["-d", "-a"] {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        at.make_file(FILE);
        at.symlink_file(FILE, SYMLINK);
        scene
            .ucmd()
            .args(&[f, "--no-preserve=all", SYMLINK, DEST])
            .succeeds();
        at.symlink_exists(DEST);
    }
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_preserve_attrs_overriding_2() {
    const FILE1: &str = "file1";
    const FILE2: &str = "file2";
    const FOLDER: &str = "folder";
    const DEST: &str = "dest";
    for mut args in [
        // All of the following to args should tell cp to preserve mode and
        // timestamp, but not the link.
        vec!["-r", "--preserve=mode,link,timestamp", "--no-preserve=link"],
        vec![
            "-r",
            "--preserve=mode",
            "--preserve=link",
            "--preserve=timestamp",
            "--no-preserve=link",
        ],
        vec![
            "-r",
            "--preserve=mode,link",
            "--no-preserve=link",
            "--preserve=timestamp",
        ],
        vec!["-a", "--no-preserve=link"],
        vec!["-r", "--preserve", "--no-preserve=link"],
    ] {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        at.mkdir(FOLDER);
        at.make_file(&format!("{FOLDER}/{FILE1}"));
        at.set_mode(&format!("{FOLDER}/{FILE1}"), 0o775);
        at.hard_link(&format!("{FOLDER}/{FILE1}"), &format!("{FOLDER}/{FILE2}"));
        args.append(&mut vec![FOLDER, DEST]);
        let src_file1_metadata = at.metadata(&format!("{FOLDER}/{FILE1}"));
        scene.ucmd().args(&args).succeeds();
        at.dir_exists(DEST);
        let dest_file1_metadata = at.metadata(&format!("{DEST}/{FILE1}"));
        let dest_file2_metadata = at.metadata(&format!("{DEST}/{FILE2}"));
        assert_eq!(
            src_file1_metadata.modified().unwrap(),
            dest_file1_metadata.modified().unwrap()
        );
        assert_eq!(src_file1_metadata.mode(), dest_file1_metadata.mode());
        assert_ne!(dest_file1_metadata.ino(), dest_file2_metadata.ino());
    }
}

/// Test the behavior of preserving permissions when copying through a symlink
#[test]
#[cfg(unix)]
fn test_cp_symlink_permissions() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("a");
    at.set_mode("a", 0o700);
    at.symlink_file("a", "symlink");
    at.mkdir("dest");
    scene
        .ucmd()
        .args(&["--preserve", "symlink", "dest"])
        .succeeds();
    let dest_dir_metadata = at.metadata("dest/symlink");
    let src_dir_metadata = at.metadata("a");
    assert_eq!(
        src_dir_metadata.permissions().mode(),
        dest_dir_metadata.permissions().mode()
    );
}

/// Test the behavior of preserving permissions of parents when copying through a symlink
#[test]
#[cfg(unix)]
fn test_cp_parents_symlink_permissions_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("a");
    at.touch("a/file");
    at.set_mode("a", 0o700);
    at.symlink_dir("a", "symlink");
    at.mkdir("dest");
    scene
        .ucmd()
        .args(&["--parents", "-a", "symlink/file", "dest"])
        .succeeds();
    let dest_dir_metadata = at.metadata("dest/symlink");
    let src_dir_metadata = at.metadata("a");
    assert_eq!(
        src_dir_metadata.permissions().mode(),
        dest_dir_metadata.permissions().mode()
    );
}

/// Test the behavior of preserving permissions of parents when copying through
/// a symlink when source is a dir.
#[test]
#[cfg(unix)]
fn test_cp_parents_symlink_permissions_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir_all("a/b");
    at.set_mode("a", 0o755); // Set mode for the actual directory
    at.symlink_dir("a", "symlink");
    at.mkdir("dest");
    scene
        .ucmd()
        .args(&["--parents", "-a", "symlink/b", "dest"])
        .succeeds();
    let dest_dir_metadata = at.metadata("dest/symlink");
    let src_dir_metadata = at.metadata("a");
    assert_eq!(
        src_dir_metadata.permissions().mode(),
        dest_dir_metadata.permissions().mode()
    );
}

/// Test the behavior of copying a file to a destination with parents using absolute paths.
#[cfg(unix)]
#[test]
fn test_cp_parents_absolute_path() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir_all("a/b");
    at.touch("a/b/f");
    at.mkdir("dest");
    let src = format!("{}/a/b/f", at.root_dir_resolved());
    scene
        .ucmd()
        .args(&["--parents", src.as_str(), "dest"])
        .succeeds();
    let res = format!("dest{}/a/b/f", at.root_dir_resolved());
    at.file_exists(res);
}

#[test]
fn test_copy_symlink_overwrite() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("a");
    at.mkdir("b");
    at.mkdir("c");

    at.write("t", "hello");
    at.relative_symlink_file("../t", "a/1");
    at.relative_symlink_file("../t", "b/1");

    ucmd.arg("--no-dereference")
        .arg("a/1")
        .arg("b/1")
        .arg("c")
        .fails()
        .stderr_only(if cfg!(not(target_os = "windows")) {
            "cp: will not overwrite just-created 'c/1' with 'b/1'\n"
        } else {
            "cp: will not overwrite just-created 'c\\1' with 'b/1'\n"
        });
}

#[test]
fn test_symlink_mode_overwrite() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("a");
    at.mkdir("b");

    at.write("a/t", "hello");
    at.write("b/t", "hello");

    if cfg!(not(target_os = "windows")) {
        ucmd.arg("-s")
            .arg("a/t")
            .arg("b/t")
            .arg(".")
            .fails()
            .stderr_only("cp: will not overwrite just-created './t' with 'b/t'\n");

        assert_eq!(at.read("./t"), "hello");
    } else {
        ucmd.arg("-s")
            .arg("a\\t")
            .arg("b\\t")
            .arg(".")
            .fails()
            .stderr_only("cp: will not overwrite just-created '.\\t' with 'b\\t'\n");

        assert_eq!(at.read(".\\t"), "hello");
    }
}

// make sure that cp backup dest symlink before removing it.
#[test]
fn test_cp_with_options_backup_and_rem_when_dest_is_symlink() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("file", "xyz");
    at.mkdir("inner_dir");
    at.write("inner_dir/inner_file", "abc");
    at.relative_symlink_file("inner_file", "inner_dir/sl");
    scene
        .ucmd()
        .args(&["-b", "--rem", "file", "inner_dir/sl"])
        .succeeds();
    assert!(at.file_exists("inner_dir/inner_file"));
    assert_eq!(at.read("inner_dir/inner_file"), "abc");
    assert!(at.symlink_exists("inner_dir/sl~"));
    assert!(!at.symlink_exists("inner_dir/sl"));
    assert_eq!(at.read("inner_dir/sl"), "xyz");
}

#[test]
fn test_cp_single_file() {
    let (_at, mut ucmd) = at_and_ucmd!();
    ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .fails()
        .code_is(1)
        .stderr_contains("missing destination file");
}

#[test]
fn test_cp_no_file() {
    let (_at, mut ucmd) = at_and_ucmd!();
    ucmd.fails()
        .code_is(1)
        .stderr_contains("error: the following required arguments were not provided:");
}

#[test]
#[cfg(all(
    unix,
    not(any(target_os = "android", target_os = "macos", target_os = "openbsd"))
))]
fn test_cp_preserve_xattr_readonly_source() {
    use std::process::Command;
    use uutests::util::compare_xattrs;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let source_file = "a";
    let dest_file = "e";

    at.touch(source_file);

    let xattr_key = "user.test";
    match Command::new("setfattr")
        .args([
            "-n",
            xattr_key,
            "-v",
            "value",
            &at.plus_as_string(source_file),
        ])
        .status()
        .map(|status| status.code())
    {
        Ok(Some(0)) => {}
        Ok(_) => {
            println!("test skipped: setfattr failed");
            return;
        }
        Err(e) => {
            println!("test skipped: setfattr failed with {e}");
            return;
        }
    }

    let getfattr_output = Command::new("getfattr")
        .args([&at.plus_as_string(source_file)])
        .output()
        .expect("Failed to run `getfattr` on the destination file");

    assert!(
        getfattr_output.status.success(),
        "getfattr did not run successfully: {}",
        String::from_utf8_lossy(&getfattr_output.stderr)
    );

    let stdout = String::from_utf8_lossy(&getfattr_output.stdout);
    assert!(
        stdout.contains(xattr_key),
        "Expected '{}' not found in getfattr output:\n{}",
        xattr_key,
        stdout
    );

    at.set_readonly(source_file);
    assert!(scene
        .fixtures
        .metadata(source_file)
        .permissions()
        .readonly());

    scene
        .ucmd()
        .args(&[
            "--preserve=xattr",
            &at.plus_as_string(source_file),
            &at.plus_as_string(dest_file),
        ])
        .succeeds()
        .no_output();

    assert!(scene.fixtures.metadata(dest_file).permissions().readonly());
    assert!(
        compare_xattrs(&at.plus(source_file), &at.plus(dest_file)),
        "Extended attributes were not preserved"
    );
}

#[test]
#[cfg(unix)]
fn test_cp_from_stdin() {
    let (at, mut ucmd) = at_and_ucmd!();
    let target = "target";
    let test_string = "Hello, World!\n";

    ucmd.arg("/dev/fd/0")
        .arg(target)
        .pipe_in(test_string)
        .succeeds();

    assert!(at.file_exists(target));
    assert_eq!(at.read(target), test_string);
}

#[test]
fn test_cp_update_older_interactive_prompt_yes() {
    let (at, mut ucmd) = at_and_ucmd!();
    let old_file = "old";
    let new_file = "new";

    let f = at.make_file(old_file);
    f.set_modified(std::time::UNIX_EPOCH).unwrap();
    at.touch(new_file);

    ucmd.args(&["-i", "-v", "--update=older", new_file, old_file])
        .pipe_in("Y\n")
        .stderr_to_stdout()
        .succeeds()
        .stdout_is("cp: overwrite 'old'? 'new' -> 'old'\n");
}

#[test]
fn test_cp_update_older_interactive_prompt_no() {
    let (at, mut ucmd) = at_and_ucmd!();
    let old_file = "old";
    let new_file = "new";

    let f = at.make_file(old_file);
    f.set_modified(std::time::UNIX_EPOCH).unwrap();
    at.touch(new_file);

    ucmd.args(&["-i", "-v", "--update=older", new_file, old_file])
        .pipe_in("N\n")
        .stderr_to_stdout()
        .fails()
        .stdout_is("cp: overwrite 'old'? ");
}
