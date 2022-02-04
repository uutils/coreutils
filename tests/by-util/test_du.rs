//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (paths) sublink subwords

use crate::common::util::*;

const SUB_DIR: &str = "subdir/deeper";
const SUB_DEEPER_DIR: &str = "subdir/deeper/deeper_dir";
const SUB_DIR_LINKS: &str = "subdir/links";
const SUB_DIR_LINKS_DEEPER_SYM_DIR: &str = "subdir/links/deeper_dir";
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
40\t.
";
    assert_eq!(s, answer);
}
#[cfg(not(target_vendor = "apple"))]
fn _du_basics(s: &str) {
    let answer = "28\t./subdir
8\t./subdir/deeper
16\t./subdir/links
36\t.
";
    assert_eq!(s, answer);
}

#[test]
fn test_du_basics_subdir() {
    let ts = TestScenario::new(util_name!());

    let result = ts.ucmd().arg(SUB_DIR).succeeds();

    #[cfg(target_os = "linux")]
    {
        let result_reference = unwrap_or_return!(expected_result(&ts, &[SUB_DIR]));
        if result_reference.succeeded() {
            assert_eq!(result.stdout_str(), result_reference.stdout_str());
            return;
        }
    }
    _du_basics_subdir(result.stdout_str());
}

#[cfg(target_vendor = "apple")]
fn _du_basics_subdir(s: &str) {
    assert_eq!(s, "4\tsubdir/deeper/deeper_dir\n8\tsubdir/deeper\n");
}
#[cfg(target_os = "windows")]
fn _du_basics_subdir(s: &str) {
    assert_eq!(s, "0\tsubdir/deeper\\deeper_dir\n0\tsubdir/deeper\n");
}
#[cfg(target_os = "freebsd")]
fn _du_basics_subdir(s: &str) {
    assert_eq!(s, "8\tsubdir/deeper/deeper_dir\n16\tsubdir/deeper\n");
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
fn test_du_invalid_size() {
    let args = &["block-size", "threshold"];
    let ts = TestScenario::new(util_name!());
    for s in args {
        ts.ucmd()
            .arg(format!("--{}=1fb4t", s))
            .arg("/tmp")
            .fails()
            .code_is(1)
            .stderr_only(format!("du: invalid --{} argument '1fb4t'", s));
        #[cfg(not(target_pointer_width = "128"))]
        ts.ucmd()
            .arg(format!("--{}=1Y", s))
            .arg("/tmp")
            .fails()
            .code_is(1)
            .stderr_only(format!("du: --{} argument '1Y' too large", s));
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
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.symlink_file(SUB_FILE, SUB_LINK);

    let result = ts.ucmd().arg(SUB_DIR_LINKS).succeeds();

    #[cfg(target_os = "linux")]
    {
        let result_reference = unwrap_or_return!(expected_result(&ts, &[SUB_DIR_LINKS]));
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
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.hard_link(SUB_FILE, SUB_LINK);

    let result = ts.ucmd().arg(SUB_DIR_LINKS).succeeds();

    #[cfg(target_os = "linux")]
    {
        let result_reference = unwrap_or_return!(expected_result(&ts, &[SUB_DIR_LINKS]));
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
    assert_eq!(s, "12\tsubdir/links\n");
}
#[cfg(target_os = "windows")]
fn _du_hard_link(s: &str) {
    assert_eq!(s, "8\tsubdir/links\n");
}
#[cfg(target_os = "freebsd")]
fn _du_hard_link(s: &str) {
    assert_eq!(s, "16\tsubdir/links\n");
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
    let ts = TestScenario::new(util_name!());

    let result = ts.ucmd().arg("-d1").succeeds();

    #[cfg(target_os = "linux")]
    {
        let result_reference = unwrap_or_return!(expected_result(&ts, &["-d1"]));
        if result_reference.succeeded() {
            assert_eq!(result.stdout_str(), result_reference.stdout_str());
            return;
        }
    }
    _du_d_flag(result.stdout_str());
}

#[cfg(target_vendor = "apple")]
fn _du_d_flag(s: &str) {
    assert_eq!(s, "20\t./subdir\n24\t.\n");
}
#[cfg(target_os = "windows")]
fn _du_d_flag(s: &str) {
    assert_eq!(s, "8\t.\\subdir\n8\t.\n");
}
#[cfg(target_os = "freebsd")]
fn _du_d_flag(s: &str) {
    assert_eq!(s, "36\t./subdir\n44\t.\n");
}
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "freebsd")
))]
fn _du_d_flag(s: &str) {
    // MS-WSL linux has altered expected output
    if !uucore::os::is_wsl_1() {
        assert_eq!(s, "28\t./subdir\n36\t.\n");
    } else {
        assert_eq!(s, "8\t./subdir\n8\t.\n");
    }
}

#[test]
fn test_du_dereference() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.symlink_dir(SUB_DEEPER_DIR, SUB_DIR_LINKS_DEEPER_SYM_DIR);

    let result = ts.ucmd().arg("-L").arg(SUB_DIR_LINKS).succeeds();

    #[cfg(target_os = "linux")]
    {
        let result_reference = unwrap_or_return!(expected_result(&ts, &["-L", SUB_DIR_LINKS]));

        if result_reference.succeeded() {
            assert_eq!(result.stdout_str(), result_reference.stdout_str());
            return;
        }
    }

    _du_dereference(result.stdout_str());
}

#[cfg(target_vendor = "apple")]
fn _du_dereference(s: &str) {
    assert_eq!(s, "4\tsubdir/links/deeper_dir\n16\tsubdir/links\n");
}
#[cfg(target_os = "windows")]
fn _du_dereference(s: &str) {
    assert_eq!(s, "0\tsubdir/links\\deeper_dir\n8\tsubdir/links\n");
}
#[cfg(target_os = "freebsd")]
fn _du_dereference(s: &str) {
    assert_eq!(s, "8\tsubdir/links/deeper_dir\n24\tsubdir/links\n");
}
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "freebsd")
))]
fn _du_dereference(s: &str) {
    // MS-WSL linux has altered expected output
    if !uucore::os::is_wsl_1() {
        assert_eq!(s, "8\tsubdir/links/deeper_dir\n24\tsubdir/links\n");
    } else {
        assert_eq!(s, "0\tsubdir/links/deeper_dir\n8\tsubdir/links\n");
    }
}

#[test]
fn test_du_inodes_basic() {
    let ts = TestScenario::new(util_name!());
    let result = ts.ucmd().arg("--inodes").succeeds();

    #[cfg(target_os = "linux")]
    {
        let result_reference = unwrap_or_return!(expected_result(&ts, &["--inodes"]));
        assert_eq!(result.stdout_str(), result_reference.stdout_str());
    }

    #[cfg(not(target_os = "linux"))]
    _du_inodes_basic(result.stdout_str());
}

#[cfg(target_os = "windows")]
fn _du_inodes_basic(s: &str) {
    assert_eq!(
        s,
        "2\t.\\subdir\\deeper\\deeper_dir
4\t.\\subdir\\deeper
3\t.\\subdir\\links
8\t.\\subdir
11\t.
"
    );
}

#[cfg(not(target_os = "windows"))]
fn _du_inodes_basic(s: &str) {
    assert_eq!(
        s,
        "2\t./subdir/deeper/deeper_dir
4\t./subdir/deeper
3\t./subdir/links
8\t./subdir
11\t.
"
    );
}

#[test]
fn test_du_inodes() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd()
        .arg("--summarize")
        .arg("--inodes")
        .succeeds()
        .stdout_only("11\t.\n");

    let result = ts.ucmd().arg("--separate-dirs").arg("--inodes").succeeds();

    #[cfg(target_os = "windows")]
    result.stdout_contains("3\t.\\subdir\\links\n");
    #[cfg(not(target_os = "windows"))]
    result.stdout_contains("3\t./subdir/links\n");
    result.stdout_contains("3\t.\n");

    #[cfg(target_os = "linux")]
    {
        let result_reference =
            unwrap_or_return!(expected_result(&ts, &["--separate-dirs", "--inodes"]));
        assert_eq!(result.stdout_str(), result_reference.stdout_str());
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
    let ts = TestScenario::new(util_name!());

    ts.ccmd("touch")
        .arg("-a")
        .arg("-t")
        .arg("201505150000")
        .arg("date_test")
        .succeeds();

    ts.ccmd("touch")
        .arg("-m")
        .arg("-t")
        .arg("201606160000")
        .arg("date_test")
        .succeeds();

    let result = ts.ucmd().arg("--time").arg("date_test").succeeds();
    result.stdout_only("0\t2016-06-16 00:00\tdate_test\n");

    let result = ts.ucmd().arg("--time=atime").arg("date_test").succeeds();
    result.stdout_only("0\t2015-05-15 00:00\tdate_test\n");

    let result = ts.ucmd().arg("--time=ctime").arg("date_test").succeeds();
    result.stdout_only("0\t2016-06-16 00:00\tdate_test\n");

    if birth_supported() {
        use regex::Regex;

        let re_birth =
            Regex::new(r"0\t[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}\tdate_test").unwrap();
        let result = ts.ucmd().arg("--time=birth").arg("date_test").succeeds();
        result.stdout_matches(&re_birth);
    }
}

#[cfg(feature = "touch")]
fn birth_supported() -> bool {
    let ts = TestScenario::new(util_name!());
    let m = match std::fs::metadata(ts.fixtures.subdir) {
        Ok(m) => m,
        Err(e) => panic!("{}", e),
    };
    m.created().is_ok()
}

#[cfg(not(target_os = "windows"))]
#[cfg(feature = "chmod")]
#[test]
fn test_du_no_permission() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.mkdir_all(SUB_DIR_LINKS);

    ts.ccmd("chmod").arg("-r").arg(SUB_DIR_LINKS).succeeds();

    let result = ts.ucmd().arg(SUB_DIR_LINKS).run(); // TODO: replace with ".fails()" once `du` is fixed
    result.stderr_contains(
        "du: cannot read directory 'subdir/links': Permission denied (os error 13)",
    );

    #[cfg(target_os = "linux")]
    {
        let result_reference = unwrap_or_return!(expected_result(&ts, &[SUB_DIR_LINKS]));
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
    let ts = TestScenario::new(util_name!());

    let result = ts.ucmd().arg("-x").arg(SUB_DIR).succeeds();

    #[cfg(target_os = "linux")]
    {
        let result_reference = unwrap_or_return!(expected_result(&ts, &["-x", SUB_DIR]));
        if result_reference.succeeded() {
            assert_eq!(result.stdout_str(), result_reference.stdout_str());
            return;
        }
    }
    _du_basics_subdir(result.stdout_str());
}

#[test]
fn test_du_threshold() {
    let ts = TestScenario::new(util_name!());

    let threshold = if cfg!(windows) { "7K" } else { "10K" };

    ts.ucmd()
        .arg(format!("--threshold={}", threshold))
        .succeeds()
        .stdout_contains("links")
        .stdout_does_not_contain("deeper_dir");

    ts.ucmd()
        .arg(format!("--threshold=-{}", threshold))
        .succeeds()
        .stdout_does_not_contain("links")
        .stdout_contains("deeper_dir");
}

#[test]
fn test_du_apparent_size() {
    let ts = TestScenario::new(util_name!());
    let result = ts.ucmd().arg("--apparent-size").succeeds();

    #[cfg(target_os = "linux")]
    {
        let result_reference = unwrap_or_return!(expected_result(&ts, &["--apparent-size"]));
        assert_eq!(result.stdout_str(), result_reference.stdout_str());
    }

    #[cfg(not(target_os = "linux"))]
    _du_apparent_size(result.stdout_str());
}

#[cfg(target_os = "windows")]
fn _du_apparent_size(s: &str) {
    assert_eq!(
        s,
        "1\t.\\subdir\\deeper\\deeper_dir
1\t.\\subdir\\deeper
6\t.\\subdir\\links
6\t.\\subdir
6\t.
"
    );
}
#[cfg(target_vendor = "apple")]
fn _du_apparent_size(s: &str) {
    assert_eq!(
        s,
        "1\t./subdir/deeper/deeper_dir
1\t./subdir/deeper
6\t./subdir/links
6\t./subdir
6\t.
"
    );
}
#[cfg(target_os = "freebsd")]
fn _du_apparent_size(s: &str) {
    assert_eq!(
        s,
        "1\t./subdir/deeper/deeper_dir
2\t./subdir/deeper
6\t./subdir/links
8\t./subdir
8\t.
"
    );
}
#[cfg(all(
    not(target_vendor = "apple"),
    not(target_os = "windows"),
    not(target_os = "freebsd")
))]
fn _du_apparent_size(s: &str) {
    assert_eq!(
        s,
        "5\t./subdir/deeper/deeper_dir
9\t./subdir/deeper
10\t./subdir/links
22\t./subdir
26\t.
"
    );
}

#[test]
fn test_du_bytes() {
    let ts = TestScenario::new(util_name!());
    let result = ts.ucmd().arg("--bytes").succeeds();

    #[cfg(target_os = "linux")]
    {
        let result_reference = unwrap_or_return!(expected_result(&ts, &["--bytes"]));
        assert_eq!(result.stdout_str(), result_reference.stdout_str());
    }

    #[cfg(target_os = "windows")]
    result.stdout_contains("5145\t.\\subdir\n");
    #[cfg(target_vendor = "apple")]
    result.stdout_contains("5625\t./subdir\n");
    #[cfg(target_os = "freebsd")]
    result.stdout_contains("7193\t./subdir\n");
    #[cfg(all(
        not(target_vendor = "apple"),
        not(target_os = "windows"),
        not(target_os = "freebsd"),
        not(target_os = "linux")
    ))]
    result.stdout_contains("21529\t./subdir\n");
}
