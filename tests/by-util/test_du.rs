use crate::common::util::*;

const SUB_DIR: &str = "subdir/deeper";
const SUB_DIR_LINKS: &str = "subdir/links";
const SUB_FILE: &str = "subdir/links/subwords.txt";
const SUB_LINK: &str = "subdir/links/sublink.txt";

#[test]
fn test_du_basics() {
    let (_at, mut ucmd) = at_and_ucmd!();
    let result = ucmd.run();
    assert!(result.success);
    assert_eq!(result.stderr, "");
}
#[cfg(target_vendor = "apple")]
fn _du_basics(s: String) {
    let answer = "32\t./subdir
8\t./subdir/deeper
24\t./subdir/links
40\t./
";
    assert_eq!(s, answer);
}
#[cfg(not(target_vendor = "apple"))]
fn _du_basics(s: String) {
    let answer = "28\t./subdir
8\t./subdir/deeper
16\t./subdir/links
36\t./
";
    assert_eq!(s, answer);
}

#[test]
fn test_du_basics_subdir() {
    let (_at, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg(SUB_DIR).run();
    assert!(result.success);
    assert_eq!(result.stderr, "");
    _du_basics_subdir(result.stdout);
}

#[cfg(target_vendor = "apple")]
fn _du_basics_subdir(s: String) {
    assert_eq!(s, "4\tsubdir/deeper\n");
}
#[cfg(target_os = "windows")]
fn _du_basics_subdir(s: String) {
    assert_eq!(s, "0\tsubdir/deeper\n");
}
#[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))]
fn _du_basics_subdir(s: String) {
    // MS-WSL linux has altered expected output
    if !is_wsl() {
        assert_eq!(s, "8\tsubdir/deeper\n");
    } else {
        assert_eq!(s, "0\tsubdir/deeper\n");
    }
}

#[test]
fn test_du_basics_bad_name() {
    let (_at, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("bad_name").run();
    assert_eq!(result.stdout, "");
    assert_eq!(
        result.stderr,
        "du: error: bad_name: No such file or directory\n"
    );
}

#[test]
fn test_du_soft_link() {
    let ts = TestScenario::new("du");

    let link = ts.ccmd("ln").arg("-s").arg(SUB_FILE).arg(SUB_LINK).run();
    assert!(link.success);

    let result = ts.ucmd().arg(SUB_DIR_LINKS).run();
    assert!(result.success);
    assert_eq!(result.stderr, "");
    _du_soft_link(result.stdout);
}

#[cfg(target_vendor = "apple")]
fn _du_soft_link(s: String) {
    // 'macos' host variants may have `du` output variation for soft links
    assert!((s == "12\tsubdir/links\n") || (s == "16\tsubdir/links\n"));
}
#[cfg(target_os = "windows")]
fn _du_soft_link(s: String) {
    assert_eq!(s, "8\tsubdir/links\n");
}
#[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))]
fn _du_soft_link(s: String) {
    // MS-WSL linux has altered expected output
    if !is_wsl() {
        assert_eq!(s, "16\tsubdir/links\n");
    } else {
        assert_eq!(s, "8\tsubdir/links\n");
    }
}

#[test]
fn test_du_hard_link() {
    let ts = TestScenario::new("du");

    let link = ts.ccmd("ln").arg(SUB_FILE).arg(SUB_LINK).run();
    assert!(link.success);

    let result = ts.ucmd().arg(SUB_DIR_LINKS).run();
    assert!(result.success);
    assert_eq!(result.stderr, "");
    // We do not double count hard links as the inodes are identical
    _du_hard_link(result.stdout);
}

#[cfg(target_vendor = "apple")]
fn _du_hard_link(s: String) {
    assert_eq!(s, "12\tsubdir/links\n")
}
#[cfg(target_os = "windows")]
fn _du_hard_link(s: String) {
    assert_eq!(s, "8\tsubdir/links\n")
}
#[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))]
fn _du_hard_link(s: String) {
    // MS-WSL linux has altered expected output
    if !is_wsl() {
        assert_eq!(s, "16\tsubdir/links\n");
    } else {
        assert_eq!(s, "8\tsubdir/links\n");
    }
}

#[test]
fn test_du_d_flag() {
    let ts = TestScenario::new("du");

    let result = ts.ucmd().arg("-d").arg("1").run();
    assert!(result.success);
    assert_eq!(result.stderr, "");
    _du_d_flag(result.stdout);
}

#[cfg(target_vendor = "apple")]
fn _du_d_flag(s: String) {
    assert_eq!(s, "16\t./subdir\n20\t./\n");
}
#[cfg(target_os = "windows")]
fn _du_d_flag(s: String) {
    assert_eq!(s, "8\t./subdir\n8\t./\n");
}
#[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))]
fn _du_d_flag(s: String) {
    // MS-WSL linux has altered expected output
    if !is_wsl() {
        assert_eq!(s, "28\t./subdir\n36\t./\n");
    } else {
        assert_eq!(s, "8\t./subdir\n8\t./\n");
    }
}

#[test]
fn test_du_h_flag_empty_file() {
    let ts = TestScenario::new("du");

    let result = ts.ucmd().arg("-h").arg("empty.txt").run();
    assert!(result.success);
    assert_eq!(result.stderr, "");
    assert_eq!(result.stdout, "0\tempty.txt\n");
}
