#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "cp";

static TEST_HELLO_WORLD_SOURCE: &'static str = "hello_world.txt";
static TEST_HELLO_WORLD_DEST: &'static str = "copy_of_hello_world.txt";

#[test]
fn test_cp_cp() {
    let (at, mut ucmd) = testing(UTIL_NAME);
    // Invoke our binary to make the copy.
    let result = ucmd.arg(TEST_HELLO_WORLD_SOURCE)
                     .arg(TEST_HELLO_WORLD_DEST)
                     .run();

    // Check that the exit code represents a successful copy.
    let exit_success = result.success;
    assert_eq!(exit_success, true);

    // Check the content of the destination file that was copied.
    assert_eq!(at.read(TEST_HELLO_WORLD_DEST), "Hello, World!\n");
}
