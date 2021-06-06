// spell-checker:ignore (paths) sublink subwords

use crate::common::util::*;

const SUB_DIR: &str = "subdir/deeper";
const SUB_DIR_LINKS: &str = "subdir/links";
const SUB_FILE: &str = "subdir/links/subwords.txt";
const SUB_LINK: &str = "subdir/links/sublink.txt";

#[test]
fn test_du_basics() {
    new_ucmd!().succeeds().no_stderr();
}
#[cfg(target_vendor = "apple")]
fn _du_basics(s: &str) {
    let answer = "32\t./subdir
8\t./subdir/deeper
24\t./subdir/links
40\t./
";
    assert_eq!(s, answer);
}
#[cfg(not(target_vendor = "apple"))]
fn _du_basics(s: &str) {
    let answer = "28\t./subdir
8\t./subdir/deeper
16\t./subdir/links
36\t./
";
    assert_eq!(s, answer);
}

#[test]
fn test_du_basics_subdir() {
    let scene = TestScenario::new(util_name!());

    let result = scene.ucmd().arg(SUB_DIR).succeeds();

    #[cfg(target_os = "linux")]
    {
        let result_reference = scene.cmd("du").arg(SUB_DIR).run();
        if result_reference.succeeded() {
            assert_eq!(result.stdout_str(), result_reference.stdout_str());
            return;
        }
    }
    _du_basics_subdir(result.stdout_str());
}

#[cfg(target_vendor = "apple")]
fn _du_basics_subdir(s: &str) {
    assert_eq!(s, "4\tsubdir/deeper\n");
}
#[cfg(target_os = "windows")]
fn _du_basics_subdir(s: &str) {
    assert_eq!(s, "0\tsubdir/deeper\n");
}
#[cfg(target_os = "freebsd")]
fn _du_basics_subdir(s: &str) {
    assert_eq!(s, "8\tsubdir/deeper\n");
}
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "freebsd")
))]
fn _du_basics_subdir(s: &str) {
    // MS-WSL linux has altered expected output
    if !uucore::os::is_wsl_1() {
        assert_eq!(s, "8\tsubdir/deeper\n");
    } else {
        assert_eq!(s, "0\tsubdir/deeper\n");
    }
}

#[test]
fn test_du_basics_bad_name() {
    new_ucmd!()
        .arg("bad_name")
        .succeeds() // TODO: replace with ".fails()" once `du` is fixed
        .stderr_only("du: bad_name: No such file or directory\n");
}

#[test]
fn test_du_soft_link() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.symlink_file(SUB_FILE, SUB_LINK);

    let result = scene.ucmd().arg(SUB_DIR_LINKS).succeeds();

    #[cfg(target_os = "linux")]
    {
        let result_reference = scene.cmd("du").arg(SUB_DIR_LINKS).run();
        if result_reference.succeeded() {
            assert_eq!(result.stdout_str(), result_reference.stdout_str());
            return;
        }
    }
    _du_soft_link(result.stdout_str());
}

#[cfg(target_vendor = "apple")]
fn _du_soft_link(s: &str) {
    // 'macos' host variants may have `du` output variation for soft links
    assert!((s == "12\tsubdir/links\n") || (s == "16\tsubdir/links\n"));
}
#[cfg(target_os = "windows")]
fn _du_soft_link(s: &str) {
    assert_eq!(s, "8\tsubdir/links\n");
}
#[cfg(target_os = "freebsd")]
fn _du_soft_link(s: &str) {
    assert_eq!(s, "16\tsubdir/links\n");
}
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "freebsd")
))]
fn _du_soft_link(s: &str) {
    // MS-WSL linux has altered expected output
    if !uucore::os::is_wsl_1() {
        assert_eq!(s, "16\tsubdir/links\n");
    } else {
        assert_eq!(s, "8\tsubdir/links\n");
    }
}

#[test]
fn test_du_hard_link() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.hard_link(SUB_FILE, SUB_LINK);

    let result = scene.ucmd().arg(SUB_DIR_LINKS).succeeds();

    #[cfg(target_os = "linux")]
    {
        let result_reference = scene.cmd("du").arg(SUB_DIR_LINKS).run();
        if result_reference.succeeded() {
            assert_eq!(result.stdout_str(), result_reference.stdout_str());
            return;
        }
    }
    // We do not double count hard links as the inodes are identical
    _du_hard_link(result.stdout_str());
}

#[cfg(target_vendor = "apple")]
fn _du_hard_link(s: &str) {
    assert_eq!(s, "12\tsubdir/links\n")
}
#[cfg(target_os = "windows")]
fn _du_hard_link(s: &str) {
    assert_eq!(s, "8\tsubdir/links\n")
}
#[cfg(target_os = "freebsd")]
fn _du_hard_link(s: &str) {
    assert_eq!(s, "16\tsubdir/links\n")
}
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "freebsd")
))]
fn _du_hard_link(s: &str) {
    // MS-WSL linux has altered expected output
    if !uucore::os::is_wsl_1() {
        assert_eq!(s, "16\tsubdir/links\n");
    } else {
        assert_eq!(s, "8\tsubdir/links\n");
    }
}

#[test]
fn test_du_d_flag() {
    let scene = TestScenario::new(util_name!());

    let result = scene.ucmd().arg("-d1").succeeds();

    #[cfg(target_os = "linux")]
    {
        let result_reference = scene.cmd("du").arg("-d1").run();
        if result_reference.succeeded() {
            assert_eq!(
                // TODO: gnu `du` doesn't use trailing "/" here
                // result.stdout_str(), result_reference.stdout_str()
                result.stdout_str().trim_end_matches("/\n"),
                result_reference.stdout_str().trim_end_matches('\n')
            );
            return;
        }
    }
    _du_d_flag(result.stdout_str());
}

#[cfg(target_vendor = "apple")]
fn _du_d_flag(s: &str) {
    assert_eq!(s, "16\t./subdir\n20\t./\n");
}
#[cfg(target_os = "windows")]
fn _du_d_flag(s: &str) {
    assert_eq!(s, "8\t./subdir\n8\t./\n");
}
#[cfg(target_os = "freebsd")]
fn _du_d_flag(s: &str) {
    assert_eq!(s, "28\t./subdir\n36\t./\n");
}
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "freebsd")
))]
fn _du_d_flag(s: &str) {
    // MS-WSL linux has altered expected output
    if !uucore::os::is_wsl_1() {
        assert_eq!(s, "28\t./subdir\n36\t./\n");
    } else {
        assert_eq!(s, "8\t./subdir\n8\t./\n");
    }
}

#[test]
fn test_du_h_flag_empty_file() {
    new_ucmd!()
        .arg("-h")
        .arg("empty.txt")
        .succeeds()
        .stdout_only("0\tempty.txt\n");
}

#[cfg(feature = "touch")]
#[test]
fn test_du_time() {
    let scene = TestScenario::new(util_name!());

    scene
        .ccmd("touch")
        .arg("-a")
        .arg("-t")
        .arg("201505150000")
        .arg("date_test")
        .succeeds();

    scene
        .ccmd("touch")
        .arg("-m")
        .arg("-t")
        .arg("201606160000")
        .arg("date_test")
        .succeeds();

    let result = scene.ucmd().arg("--time").arg("date_test").succeeds();
    result.stdout_only("0\t2016-06-16 00:00\tdate_test\n");

    let result = scene.ucmd().arg("--time=atime").arg("date_test").succeeds();
    result.stdout_only("0\t2015-05-15 00:00\tdate_test\n");

    let result = scene.ucmd().arg("--time=ctime").arg("date_test").succeeds();
    result.stdout_only("0\t2016-06-16 00:00\tdate_test\n");

    #[cfg(not(target_env = "musl"))]
    {
        use regex::Regex;

        let re_birth =
            Regex::new(r"0\t[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}\tdate_test").unwrap();
        let result = scene.ucmd().arg("--time=birth").arg("date_test").succeeds();
        result.stdout_matches(&re_birth);
    }
}

#[cfg(not(target_os = "windows"))]
#[cfg(feature = "chmod")]
#[test]
fn test_du_no_permission() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir_all(SUB_DIR_LINKS);

    scene.ccmd("chmod").arg("-r").arg(SUB_DIR_LINKS).succeeds();

    let result = scene.ucmd().arg(SUB_DIR_LINKS).run(); // TODO: replace with ".fails()" once `du` is fixed
    result.stderr_contains(
        "du: cannot read directory ‘subdir/links‘: Permission denied (os error 13)",
    );

    #[cfg(target_os = "linux")]
    {
        let result_reference = scene.cmd("du").arg(SUB_DIR_LINKS).fails();
        if result_reference
            .stderr_str()
            .contains("du: cannot read directory 'subdir/links': Permission denied")
        {
            assert_eq!(result.stdout_str(), result_reference.stdout_str());
            return;
        }
    }

    _du_no_permission(result.stdout_str());
}

#[cfg(target_vendor = "apple")]
fn _du_no_permission(s: &str) {
    assert_eq!(s, "0\tsubdir/links\n");
}
#[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))]
fn _du_no_permission(s: &str) {
    assert_eq!(s, "4\tsubdir/links\n");
}

#[test]
fn test_du_one_file_system() {
    let scene = TestScenario::new(util_name!());

    let result = scene.ucmd().arg("-x").arg(SUB_DIR).succeeds();

    #[cfg(target_os = "linux")]
    {
        let result_reference = scene.cmd("du").arg("-x").arg(SUB_DIR).run();
        if result_reference.succeeded() {
            assert_eq!(result.stdout_str(), result_reference.stdout_str());
            return;
        }
    }
    _du_basics_subdir(result.stdout_str());
}
