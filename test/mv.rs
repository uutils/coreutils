use std::io::process::Command;
use std::io::fs::{PathExtensions};

static EXE: &'static str = "./mv";
static TEST_HELLO_WORLD_SOURCE: &'static str = "hello_world.txt";
static TEST_HELLO_WORLD_DEST: &'static str = "move_of_hello_world.txt";

#[test]
fn test_mv() {
    let prog = Command::new(EXE)
                            .arg(TEST_HELLO_WORLD_SOURCE)
                            .arg(TEST_HELLO_WORLD_DEST)
                            .status();

    let exit_success = prog.unwrap().success();
    assert_eq!(exit_success, true);

    let dest = Path::new(TEST_HELLO_WORLD_DEST);
    assert!(dest.exists() == true);

    let source = Path::new(TEST_HELLO_WORLD_SOURCE);
    assert!(source.exists() == false);
}

