use common::util::*;

static UTIL_NAME: &'static str = "mkdir";
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

static TEST_DIR1: &'static str = "mkdir_test1";
static TEST_DIR2: &'static str = "mkdir_test2";
static TEST_DIR3: &'static str = "mkdir_test3";
static TEST_DIR4: &'static str = "mkdir_test4/mkdir_test4_1";
static TEST_DIR5: &'static str = "mkdir_test5/mkdir_test5_1";

#[test]
fn test_mkdir_mkdir() {
    new_ucmd().arg(TEST_DIR1).succeeds();
}

#[test]
fn test_mkdir_dup_dir() {
    let scene = TestScenario::new(UTIL_NAME);
    scene.ucmd().arg(TEST_DIR2).succeeds();
    scene.ucmd().arg(TEST_DIR2).fails();
}

#[test]
fn test_mkdir_mode() {
    new_ucmd()
        .arg("-m")
        .arg("755")
        .arg(TEST_DIR3)
        .succeeds();
}

#[test]
fn test_mkdir_parent() {
    new_ucmd().arg("-p").arg(TEST_DIR4).succeeds();
}

#[test]
fn test_mkdir_no_parent() {
    new_ucmd().arg(TEST_DIR5).fails();
}
