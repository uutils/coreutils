use common::util::*;
static UTIL_NAME: &'static str = "cp";
fn at_and_ucmd() -> (AtPath, UCommand) {
    let ts = TestScenario::new(UTIL_NAME);
    let ucmd = ts.ucmd();
    (ts.fixtures, ucmd)
}

static TEST_HELLO_WORLD_SOURCE: &'static str = "hello_world.txt";
static TEST_HELLO_WORLD_DEST: &'static str = "copy_of_hello_world.txt";
static TEST_COPY_TO_FOLDER: &'static str = "hello_dir/";
static TEST_COPY_TO_FOLDER_FILE: &'static str = "hello_dir/hello_world.txt";
static TEST_COPY_FROM_FOLDER_FILE: &'static str = "hello_dir_with_file/hello_world.txt";
static TEST_NOT_HELLO_WORLD: &'static str = "not_hello_world.txt";
static HELLO_WORLD_TEXT: &'static str = "Hello, World!\n";
static NOT_HELLO_WORLD_TEXT: &'static str = "This is not a Hello World!\n";
static HELLO_WORLD_IN_DIRECTORY: &'static str = "hello_dir_with_file/not_hello_world.txt";
//this file is misleadingly named - since it tests a particular situation (copying with noclobber
//into a directory with a file that has the same name as the source file)
static HELLO_FOLDER_WITH_FILES: &'static str = "hello_dir_with_file";

#[test]
fn test_cp_cp() {
    let (at, mut ucmd) = at_and_ucmd();
    // Invoke our binary to make the copy.
    let result = ucmd.arg(TEST_HELLO_WORLD_SOURCE)
                     .arg(TEST_HELLO_WORLD_DEST)
                     .run();

    // Check that the exit code represents a successful copy.
    let exit_success = result.success;
    assert!(exit_success);

    // Check the content of the destination file that was copied.
    assert_eq!(at.read(TEST_HELLO_WORLD_DEST), HELLO_WORLD_TEXT);
}

#[test]
fn test_cp_with_dirs_t() {
    let (at, mut ucmd) = at_and_ucmd();

    //using -t option
    let result_to_dir_t = ucmd
        .arg("-t")
        .arg(TEST_COPY_TO_FOLDER)
        .arg(TEST_HELLO_WORLD_SOURCE)
        .run();
    assert!(result_to_dir_t.success);
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), HELLO_WORLD_TEXT);
}

#[test]
fn test_cp_with_dirs() {
    let scene = TestScenario::new(UTIL_NAME);
    let at = &scene.fixtures;

    //using -t option
    let result_to_dir = scene.ucmd()
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_COPY_TO_FOLDER)
        .run();
    assert!(result_to_dir.success);
    assert_eq!(at.read(TEST_COPY_TO_FOLDER_FILE), HELLO_WORLD_TEXT);

    let result_from_dir = scene.ucmd()
        .arg(TEST_COPY_FROM_FOLDER_FILE)
        .arg(TEST_HELLO_WORLD_DEST)
        .run();
    assert!(result_from_dir.success);
    assert_eq!(at.read(TEST_HELLO_WORLD_DEST), HELLO_WORLD_TEXT);
}

#[test]
fn test_cp_no_clobber() {
    let scene = TestScenario::new(UTIL_NAME);
    let at = &scene.fixtures;

    //using -n option
    let result = scene.ucmd()
        .arg("-n")
        .arg(TEST_HELLO_WORLD_SOURCE)
        .arg(TEST_NOT_HELLO_WORLD)
        .run();
    assert!(result.success);
    assert_eq!(at.read(TEST_NOT_HELLO_WORLD), NOT_HELLO_WORLD_TEXT);
    assert_eq!(at.read(TEST_HELLO_WORLD_SOURCE), HELLO_WORLD_TEXT);

    //this time, copying to a directory
    let result_dir = scene.ucmd()
        .arg("-n")
        .arg(TEST_NOT_HELLO_WORLD)
        .arg(TEST_COPY_FROM_FOLDER_FILE)
        .run();
    assert!(result_dir.success);
    assert_eq!(at.read(TEST_COPY_FROM_FOLDER_FILE), HELLO_WORLD_TEXT);
    assert_eq!(at.read(TEST_NOT_HELLO_WORLD), NOT_HELLO_WORLD_TEXT);

    let result_implicit_dir = scene.ucmd()
        .arg("-n")
        .arg(TEST_NOT_HELLO_WORLD)
        .arg(HELLO_FOLDER_WITH_FILES)
        .run();
    assert!(result_implicit_dir.success);
    assert_eq!(at.read(HELLO_WORLD_IN_DIRECTORY), HELLO_WORLD_TEXT);
    assert_eq!(at.read(TEST_NOT_HELLO_WORLD), NOT_HELLO_WORLD_TEXT);
}
