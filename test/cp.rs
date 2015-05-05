#![feature(path_ext)]

use std::fs::{File, PathExt, remove_file};
use std::io::Read;
use std::path::{Path};
use std::process::Command;

static EXE: &'static str = "./cp";
static TEST_HELLO_WORLD_SOURCE: &'static str = "hello_world.txt";
static TEST_HELLO_WORLD_DEST: &'static str = "copy_of_hello_world.txt";

fn cleanup(filename: &'static str) {
    let path = Path::new(filename);
    if path.exists() {
        remove_file(&path).unwrap();
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
    let mut contents = String::new();
    let mut f = File::open(Path::new(TEST_HELLO_WORLD_DEST)).unwrap();
    let _ = f.read_to_string(&mut contents);
    assert_eq!(contents, "Hello, World!\n");

    cleanup(TEST_HELLO_WORLD_SOURCE);
    cleanup(TEST_HELLO_WORLD_DEST);
}
