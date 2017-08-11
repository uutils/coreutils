use common::util::*;
use std::fs::set_permissions;

static TEST_EXISTING_FILE:           &str = "existing_file.txt";
static TEST_HELLO_WORLD_SOURCE:      &str = "hello_world.txt";
static TEST_HELLO_WORLD_DEST:        &str = "copy_of_hello_world.txt";
static TEST_HOW_ARE_YOU_SOURCE:      &str = "how_are_you.txt";
static TEST_HOW_ARE_YOU_DEST:        &str = "hello_dir/how_are_you.txt";
static TEST_COPY_TO_FOLDER:          &str = "hello_dir/";
static TEST_COPY_TO_FOLDER_FILE:     &str = "hello_dir/hello_world.txt";
static TEST_COPY_FROM_FOLDER:        &str = "hello_dir_with_file/";
static TEST_COPY_FROM_FOLDER_FILE:   &str = "hello_dir_with_file/hello_world.txt";
static TEST_COPY_TO_FOLDER_NEW:      &str = "hello_dir_new/";
static TEST_COPY_TO_FOLDER_NEW_FILE: &str = "hello_dir_new/hello_world.txt";

#[test]
fn test_cp_cp() {
    let (at, mut ucmd) = at_and_ucmd!();
    // Invoke our binary to make the copy.
    let result = ucmd.arg(TEST_HELLO_WORLD_SOURCE)
                     .arg(TEST_HELLO_WORLD_DEST)
                     .run();

    // Check that the exit code represents a successful copy.
    let exit_success = result.success;
    assert!(exit_success);

    // Check the content of the destination file that was copied.
    assert_eq!(at.read(TEST_HELLO_WORLD_DEST), "Hello, World!\n");
}


#[test]
fn test_cp_duplicate_files() {
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg(TEST_HELLO_WORLD_SOURCE)
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
    let result = ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_EXISTING_FILE)
        .run();

    assert!(!result.success);
    assert!(result.stderr.contains("not a directory"));
}

#[test]
fn test_cp_directory_not_recursive() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg(TEST_COPY_TO_FOLDER)
        .arg(TEST_HELLO_WORLD_DEST)
        .run();

    assert!(!result.success);
    assert!(result.stderr.contains("omitting directory"));
}


#[test]
fn test_cp_multiple_files() {
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg(TEST_HELLO_WORLD_SOURCE)
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
    let result_to_dir = scene.ucmd()
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_COPY_TO_FOLDER)
        .run();
    assert!(result_to_dir.success);
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");

    let result_from_dir = scene.ucmd()
        .arg(TEST_COPY_FROM_FOLDER_FILE)
        .arg(TEST_HELLO_WORLD_DEST)
        .run();
    assert!(result_from_dir.success);
    assert_eq!(at.read(TEST_HELLO_WORLD_DEST), "Hello, World!\n");
}

#[test]
fn test_cp_arg_target_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("-t")
        .arg(TEST_COPY_TO_FOLDER)
        .run();

    assert!(result.success);
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), "Hello, World!\n");
}

#[test]
fn test_cp_arg_no_target_directory() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg(TEST_HELLO_WORLD_SOURCE)
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
    let result = ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .arg("-i")
        .pipe_in("N\n")
        .run();

    assert!(result.success);
    assert!(result.stderr.contains("Not overwriting"));
}

#[test]
#[cfg(target_os="unix")]
fn test_cp_arg_link() {
    use std::os::linux::fs::MetadataExt;

    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--link")
        .arg(TEST_HELLO_WORLD_DEST)
        .run();

    assert!(result.success);
    assert_eq!(at.metadata(TEST_HELLO_WORLD_SOURCE).st_nlink(), 2);
}

#[test]
fn test_cp_arg_symlink() {
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--symbolic-link")
        .arg(TEST_HELLO_WORLD_DEST)
        .run();

    assert!(result.success);
    assert!(at.is_symlink(TEST_HELLO_WORLD_DEST));
}


#[test]
fn test_cp_arg_no_clobber() {
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg(TEST_HELLO_WORLD_SOURCE)
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
    let mut permissions = at.make_file(TEST_HELLO_WORLD_DEST).metadata().unwrap().permissions();
    permissions.set_readonly(true);
    set_permissions(at.plus(TEST_HELLO_WORLD_DEST), permissions).unwrap();

    let result = ucmd.arg(TEST_HELLO_WORLD_SOURCE)
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
    let mut permissions = at.make_file(TEST_HELLO_WORLD_DEST).metadata().unwrap().permissions();
    permissions.set_readonly(true);
    set_permissions(at.plus(TEST_HELLO_WORLD_DEST), permissions).unwrap();

    let result = ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--remove-destination")
        .arg(TEST_HELLO_WORLD_DEST)
        .run();

    assert!(result.success);
    assert_eq!(at.read(TEST_HELLO_WORLD_DEST), "Hello, World!\n");
}

#[test]
fn test_cp_arg_backup() {
    let (at, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--backup")
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .run();

    assert!(result.success);
    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert_eq!(at.read(&*format!("{}~", TEST_HOW_ARE_YOU_SOURCE)), "How are you?\n");
}

#[test]
fn test_cp_arg_suffix() {
    let (at, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg(TEST_HELLO_WORLD_SOURCE)
        .arg("--suffix")
        .arg(".bak")
        .arg(TEST_HOW_ARE_YOU_SOURCE)
        .run();

    assert!(result.success);
    assert_eq!(at.read(TEST_HOW_ARE_YOU_SOURCE), "Hello, World!\n");
    assert_eq!(at.read(&*format!("{}.bak", TEST_HOW_ARE_YOU_SOURCE)), "How are you?\n");
}
