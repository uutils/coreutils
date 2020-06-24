use crate::common::util::*;
#[cfg(not(windows))]
use std::fs::set_permissions;

#[cfg(not(windows))]
use std::os::unix::fs;

#[cfg(windows)]
use std::os::windows::fs::symlink_file;

#[cfg(not(windows))]
use std::env;

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

#[test]
fn test_cp_cp() {
    let (at, mut ucmd) = at_and_ucmd!();
    // Invoke our binary to make the copy.
    let result = ucmd
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HELLO_WORLD_DEST)
        .run();

    // Check that the exit code represents a successful copy.
    let exit_success = result.success;
    assert!(exit_success);

    // Check the content of the destination file that was copied.
    assert_eq!(at.read(TEST_HELLO_WORLD_DEST), "Hello, World!\n");
}

#[test]
fn test_cp_existing_target() {
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_EXISTING_FILE)
        .run();

    assert!(result.success);

    // Check the content of the destination file
    assert_eq!(at.read(TEST_EXISTING_FILE), "Hello, World!\n");

    // No backup should have been created
    assert!(!at.file_exists(&*format!("{}~", TEST_EXISTING_FILE)));
}

#[test]
fn test_cp_duplicate_files() {
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_COPY_TO_FOLDER)
        .run();

    assert!(result.success);
    assert!(result.stderr.contains("specified more than once"));
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");
}

#[test]
fn test_cp_multiple_files_target_is_file() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_EXISTING_FILE)
        .run();

    assert!(!result.success);
    assert!(result.stderr.contains("not a directory"));
}

#[test]
fn test_cp_directory_not_recursive() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .arg(TEST_COPY_TO_FOLDER)
        .arg(TEST_HELLO_WORLD_DEST)
        .run();

    assert!(!result.success);
    assert!(result.stderr.contains("omitting directory"));
}

#[test]
fn test_cp_multiple_files() {
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg(TEST_COPY_TO_FOLDER)
        .run();

    assert!(result.success);
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");
    assert_eq!(at.read(TEST_HOW_ARE_YOU_DEST), "How are you?\n");
}

#[test]
fn test_cp_recurse() {
    let (at, mut ucmd) = at_and_ucmd!();

    let result = ucmd
        .arg("-r")
        .arg(TEST_COPY_FROM_FOLDER)
        .arg(TEST_COPY_TO_FOLDER_NEW)
        .run();

    assert!(result.success);
    // Check the content of the destination file that was copied.
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_NEW_FILE), "Hello, World!\n");
}

#[test]
fn test_cp_with_dirs_t() {
    let (at, mut ucmd) = at_and_ucmd!();

    //using -t option
    let result_to_dir_t = ucmd
        .arg("-t")
        .arg(TEST_COPY_TO_FOLDER)
        .arg(TEST_HELLO_WORLD_SOURCE)
        .run();
    assert!(result_to_dir_t.success);
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");
}

#[test]
fn test_cp_with_dirs() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    //using -t option
    let result_to_dir = scene
        .ucmd()
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_COPY_TO_FOLDER)
        .run();
    assert!(result_to_dir.success);
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");

    let result_from_dir = scene
        .ucmd()
        .arg(TEST_COPY_FROM_FOLDER_FILE)
        .arg(TEST_HELLO_WORLD_DEST)
        .run();
    assert!(result_from_dir.success);
    assert_eq!(at.read(TEST_HELLO_WORLD_DEST), "Hello, World!\n");
}

#[test]
fn test_cp_arg_target_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("-t")
        .arg(TEST_COPY_TO_FOLDER)
        .run();

    assert!(result.success);
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");
}

#[test]
fn test_cp_arg_no_target_directory() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("-v")
        .arg("-T")
        .arg(TEST_COPY_TO_FOLDER)
        .run();

    assert!(!result.success);
    assert!(result.stderr.contains("cannot overwrite directory"));
}

#[test]
fn test_cp_arg_interactive() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg("-i")
        .pipe_in("N\n")
        .run();

    assert!(result.success);
    assert!(result.stderr.contains("Not overwriting"));
}

#[test]
#[cfg(target_os = "unix")]
fn test_cp_arg_link() {
    use std::os::linux::fs::MetadataExt;

    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--link")
        .arg(TEST_HELLO_WORLD_DEST)
        .run();

    assert!(result.success);
    assert_eq!(at.metadata(TEST_HELLO_WORLD_SOURCE).st_nlink(), 2);
}

#[test]
fn test_cp_arg_symlink() {
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--symbolic-link")
        .arg(TEST_HELLO_WORLD_DEST)
        .run();

    assert!(result.success);
    assert!(at.is_symlink(TEST_HELLO_WORLD_DEST));
}

#[test]
fn test_cp_arg_no_clobber() {
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--no-clobber")
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .run();

    assert!(result.success);
    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "How are you?\n");
    assert!(result.stderr.contains("Not overwriting"));
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

    let result = ucmd
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--force")
        .arg(TEST_HELLO_WORLD_DEST)
        .run();

    println!("{:?}", result.stderr);
    println!("{:?}", result.stdout);

    assert!(result.success);
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

    let result = ucmd
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--remove-destination")
        .arg(TEST_HELLO_WORLD_DEST)
        .run();

    assert!(result.success);
    assert_eq!(at.read(TEST_HELLO_WORLD_DEST), "Hello, World!\n");
}

#[test]
fn test_cp_arg_backup() {
    let (at, mut ucmd) = at_and_ucmd!();

    let result = ucmd
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--backup")
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .run();

    assert!(result.success);
    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert_eq!(
        at.read(&*format!("{}~", TEST_HOW_ARE_YOU_SOURCE)),
        "How are you?\n"
    );
}

#[test]
fn test_cp_arg_suffix() {
    let (at, mut ucmd) = at_and_ucmd!();

    let result = ucmd
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--suffix")
        .arg(".bak")
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .run();

    assert!(result.success);
    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert_eq!(
        at.read(&*format!("{}.bak", TEST_HOW_ARE_YOU_SOURCE)),
        "How are you?\n"
    );
}

#[test]
fn test_cp_no_deref() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

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
    let result = scene
        .ucmd()
        .arg("-P")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HELLO_WORLD_SOURCE_SYMLINK)
        .arg(TEST_COPY_TO_FOLDER)
        .run();

    // Check that the exit code represents a successful copy.
    let exit_success = result.success;
    assert!(exit_success);
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
    let result = scene
        .ucmd()
        .arg("-P")
        .arg("-R")
        .arg("-v")
        .arg(TEST_COPY_FROM_FOLDER)
        .arg(TEST_COPY_TO_FOLDER_NEW)
        .run();
    println!("cp output {}", result.stdout);

    // Check that the exit code represents a successful copy.
    let exit_success = result.success;
    assert!(exit_success);

    #[cfg(not(windows))]
    {
        let scene2 = TestScenario::new("ls");
        let result = scene2.cmd("ls").arg("-al").arg(path_to_new_symlink).run();
        println!("ls source {}", result.stdout);

        let path_to_new_symlink = at.subdir.join(TEST_COPY_TO_FOLDER_NEW);

        let result = scene2.cmd("ls").arg("-al").arg(path_to_new_symlink).run();
        println!("ls dest {}", result.stdout);
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
