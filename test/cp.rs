use std::io::process::Command;
use std::io::File;
use std::io::fs::{unlink, PathExtensions};

static EXE: &'static str = "./cp";
static TEST_HELLO_WORLD_SOURCE: &'static str = "hello_world.txt";
static TEST_HELLO_WORLD_DEST: &'static str = "copy_of_hello_world.txt";

fn cleanup(filename: &'static str) {
    let path = Path::new(filename);
    if path.exists() {
        unlink(&path).unwrap();
    }
}


#[test]
fn test_cp_cp() {
    // Invoke our binary to make the copy.
    let prog = Command::new(EXE)
                            .arg(TEST_HELLO_WORLD_SOURCE)
                            .arg(TEST_HELLO_WORLD_DEST)
                            .status();

    // Check that the exit code represents a successful copy.
    let exit_success = prog.unwrap().success();
    assert_eq!(exit_success, true);

    // Check the content of the destination file that was copied.
    let contents = File::open(&Path::new(TEST_HELLO_WORLD_DEST))
                            .read_to_string()
                            .unwrap();
    assert_eq!(contents.as_slice(), "Hello, World!\n");

    cleanup(TEST_HELLO_WORLD_SOURCE);
    cleanup(TEST_HELLO_WORLD_DEST);
}
