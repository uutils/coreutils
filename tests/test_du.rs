use common::util::*;
use std::fs::set_permissions;

static SUB_DIR: &str = "subdir";
static SUB_FILE: &str = "subdir/subwords.txt";
static SUB_LINK: &str = "subdir/sublink.txt";

#[test]
fn test_du_basics() {
    let (_at, mut ucmd) = at_and_ucmd!();

    let result = ucmd.run();
    assert!(result.success);
    assert_eq!(result.stderr, "");
    assert_eq!(result.stdout, "24\t./subdir\n32\t./\n");
}

#[test]
fn test_du_basics_subdir() {
    let (_at, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg(SUB_DIR).run();
    assert!(result.success);
    assert_eq!(result.stderr, "");
    assert_eq!(result.stdout, "24\tsubdir\n");
}

#[test]
fn test_du_basics_bad_name() {
    let (_at, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("bad_name").run();
    assert_eq!(result.stdout, "");
    assert_eq!(result.stderr, "du: bad_name: No such file or directory\n");
}

#[test]
fn test_du_soft_link() {
    let ts = TestScenario::new("du");

    let link = ts.cmd("ln").arg("-s").arg(SUB_FILE).arg(SUB_LINK).run();
    assert!(link.success);

    let result = ts.ucmd().arg(SUB_DIR).run();
    assert!(result.success);
    assert_eq!(result.stderr, "");
    assert_eq!(result.stdout, "32\tsubdir\n");
}

#[test]
fn test_du_hard_link() {
    let ts = TestScenario::new("du");

    let link = ts.cmd("ln").arg(SUB_FILE).arg(SUB_LINK).run();
    assert!(link.success);

    let result = ts.ucmd().arg(SUB_DIR).run();
    assert!(result.success);
    assert_eq!(result.stderr, "");
    // We do not double count hard links as the inodes are identicle
    assert_eq!(result.stdout, "24\tsubdir\n");
}

// todo:
// du on file with no permissions
// du on multi dir with '-d'
//
/*
 * let mut permissions = at.make_file(TEST_HELLO_WORLD_DEST)
 * .metadata()
 * .unwrap()
 * .permissions();
 * permissions.set_readonly(true);
 */
