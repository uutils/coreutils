use common::util::*;

static TEST_DIR1: &'static str = "mkdir_test1";
static TEST_DIR2: &'static str = "mkdir_test2";
static TEST_DIR3: &'static str = "mkdir_test3";
static TEST_DIR4: &'static str = "mkdir_test4/mkdir_test4_1";
static TEST_DIR5: &'static str = "mkdir_test5/mkdir_test5_1";
static TEST_DIR6: &'static str = "mkdir_test6";
static TEST_FILE7: &'static str = "mkdir_test7";

#[test]
fn test_mkdir_mkdir() {
    new_ucmd!().arg(TEST_DIR1).succeeds();
}

#[test]
fn test_mkdir_dup_dir() {
    let scene = TestScenario::new(util_name!());
    scene.ucmd().arg(TEST_DIR2).succeeds();
    scene.ucmd().arg(TEST_DIR2).fails();
}

#[test]
fn test_mkdir_mode() {
    new_ucmd!()
        .arg("-m")
        .arg("755")
        .arg(TEST_DIR3)
        .succeeds();
}

#[test]
fn test_mkdir_parent() {
    let scene = TestScenario::new(util_name!());
    scene.ucmd().arg("-p").arg(TEST_DIR4).succeeds();
    scene.ucmd().arg("-p").arg(TEST_DIR4).succeeds();
}

#[test]
fn test_mkdir_no_parent() {
    new_ucmd!().arg(TEST_DIR5).fails();
}

#[test]
fn test_mkdir_dup_dir_parent() {
    let scene = TestScenario::new(util_name!());
    scene.ucmd().arg(TEST_DIR6).succeeds();
    scene.ucmd().arg("-p").arg(TEST_DIR6).succeeds();
}

#[test]
fn test_mkdir_dup_file() {
    let scene = TestScenario::new(util_name!());
    scene.fixtures.touch(TEST_FILE7);
    scene.ucmd().arg(TEST_FILE7).fails();
    // mkdir should fail for a file even if -p is specified.
    scene.ucmd().arg("-p").arg(TEST_FILE7).fails();
}
