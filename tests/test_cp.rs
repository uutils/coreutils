use common::util::*;

static TEST_HELLO_WORLD_SOURCE: &'static str = "hello_world.txt";
static TEST_HELLO_WORLD_DEST: &'static str = "copy_of_hello_world.txt";
static TEST_COPY_TO_FOLDER: &'static str = "hello_dir/";
static TEST_COPY_TO_FOLDER_FILE: &'static str = "hello_dir/hello_world.txt";
static TEST_COPY_FROM_FOLDER_FILE: &'static str = "hello_dir_with_file/hello_world.txt";

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
