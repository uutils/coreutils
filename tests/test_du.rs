use common::util::*;

static SUB_DIR: &str = "subdir/deeper";
static SUB_DIR_LINKS: &str = "subdir/links";
static SUB_FILE: &str = "subdir/links/subwords.txt";
static SUB_LINK: &str = "subdir/links/sublink.txt";

#[test]
fn test_du_basics() {
    let (_at, mut ucmd) = at_and_ucmd!();
    let answer = "32\t./subdir
8\t./subdir/deeper
24\t./subdir/links
40\t./
";
    let result = ucmd.run();
    assert!(result.success);
    assert_eq!(result.stderr, "");
    assert_eq!(result.stdout, answer);
}

#[test]
fn test_du_basics_subdir() {
    let (_at, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg(SUB_DIR).run();
    assert!(result.success);
    assert_eq!(result.stderr, "");
    assert_eq!(result.stdout, "8\tsubdir/deeper\n");
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

    let result = ts.ucmd().arg(SUB_DIR_LINKS).run();
    assert!(result.success);
    assert_eq!(result.stderr, "");
    assert_eq!(result.stdout, "32\tsubdir/links\n");
}

#[test]
fn test_du_hard_link() {
    let ts = TestScenario::new("du");

    let link = ts.cmd("ln").arg(SUB_FILE).arg(SUB_LINK).run();
    assert!(link.success);

    let result = ts.ucmd().arg(SUB_DIR_LINKS).run();
    assert!(result.success);
    assert_eq!(result.stderr, "");
    // We do not double count hard links as the inodes are identicle
    assert_eq!(result.stdout, "24\tsubdir/links\n");
}

#[test]
fn test_du_d_flag() {
    let ts = TestScenario::new("du");

    let result = ts.ucmd().arg("-d").arg("1").run();
    assert!(result.success);
    assert_eq!(result.stderr, "");
    assert_eq!(result.stdout, "32\t./subdir\n40\t./\n");
}
