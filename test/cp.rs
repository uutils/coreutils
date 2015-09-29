use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./cp";
static TEST_HELLO_WORLD_SOURCE: &'static str = "hello_world.txt";
static TEST_HELLO_WORLD_DEST: &'static str = "copy_of_hello_world.txt";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_cp_cp() {
    // Invoke our binary to make the copy.
    let prog = Command::new(PROGNAME)
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
