// spell-checker:ignore (words) gpghome

use crate::common::util::*;

use uucore::display::Quotable;

use std::path::PathBuf;
use tempfile::tempdir;

static TEST_TEMPLATE1: &str = "tempXXXXXX";
static TEST_TEMPLATE2: &str = "temp";
static TEST_TEMPLATE3: &str = "tempX";
static TEST_TEMPLATE4: &str = "tempXX";
static TEST_TEMPLATE5: &str = "tempXXX";
static TEST_TEMPLATE6: &str = "tempXXXlate"; // spell-checker:disable-line
static TEST_TEMPLATE7: &str = "XXXtemplate"; // spell-checker:disable-line
#[cfg(unix)]
static TEST_TEMPLATE8: &str = "tempXXXl/ate";
#[cfg(windows)]
static TEST_TEMPLATE8: &str = "tempXXXl\\ate";

#[cfg(not(windows))]
const TMPDIR: &str = "TMPDIR";
#[cfg(windows)]
const TMPDIR: &str = "TMP";

#[test]
fn test_mktemp_mktemp() {
    let scene = TestScenario::new(util_name!());

    let pathname = scene.fixtures.as_string();

    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg(TEST_TEMPLATE1)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg(TEST_TEMPLATE2)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg(TEST_TEMPLATE3)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg(TEST_TEMPLATE4)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg(TEST_TEMPLATE5)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg(TEST_TEMPLATE6)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg(TEST_TEMPLATE7)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg(TEST_TEMPLATE8)
        .fails();
}

#[test]
fn test_mktemp_mktemp_t() {
    let scene = TestScenario::new(util_name!());

    let pathname = scene.fixtures.as_string();

    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-t")
        .arg(TEST_TEMPLATE1)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-t")
        .arg(TEST_TEMPLATE2)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-t")
        .arg(TEST_TEMPLATE3)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-t")
        .arg(TEST_TEMPLATE4)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-t")
        .arg(TEST_TEMPLATE5)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-t")
        .arg(TEST_TEMPLATE6)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-t")
        .arg(TEST_TEMPLATE7)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-t")
        .arg(TEST_TEMPLATE8)
        .fails()
        .no_stdout()
        .stderr_contains("invalid suffix")
        .stderr_contains("contains directory separator");
}

#[test]
fn test_mktemp_make_temp_dir() {
    let scene = TestScenario::new(util_name!());

    let pathname = scene.fixtures.as_string();

    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-d")
        .arg(TEST_TEMPLATE1)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-d")
        .arg(TEST_TEMPLATE2)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-d")
        .arg(TEST_TEMPLATE3)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-d")
        .arg(TEST_TEMPLATE4)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-d")
        .arg(TEST_TEMPLATE5)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-d")
        .arg(TEST_TEMPLATE6)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-d")
        .arg(TEST_TEMPLATE7)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-d")
        .arg(TEST_TEMPLATE8)
        .fails();
}

#[test]
fn test_mktemp_dry_run() {
    let scene = TestScenario::new(util_name!());

    let pathname = scene.fixtures.as_string();

    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-u")
        .arg(TEST_TEMPLATE1)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-u")
        .arg(TEST_TEMPLATE2)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-u")
        .arg(TEST_TEMPLATE3)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-u")
        .arg(TEST_TEMPLATE4)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-u")
        .arg(TEST_TEMPLATE5)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-u")
        .arg(TEST_TEMPLATE6)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-u")
        .arg(TEST_TEMPLATE7)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-u")
        .arg(TEST_TEMPLATE8)
        .fails();
}

#[test]
fn test_mktemp_quiet() {
    let scene = TestScenario::new(util_name!());

    scene
        .ucmd()
        .arg("-p")
        .arg("/definitely/not/exist/I/promise")
        .arg("-q")
        .fails()
        .no_stdout()
        .no_stderr();
    scene
        .ucmd()
        .arg("-d")
        .arg("-p")
        .arg("/definitely/not/exist/I/promise")
        .arg("-q")
        .fails()
        .no_stdout()
        .no_stderr();
}

#[test]
fn test_mktemp_suffix() {
    let scene = TestScenario::new(util_name!());

    let pathname = scene.fixtures.as_string();

    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("--suffix")
        .arg("suf")
        .arg(TEST_TEMPLATE1)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("--suffix")
        .arg("suf")
        .arg(TEST_TEMPLATE2)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("--suffix")
        .arg("suf")
        .arg(TEST_TEMPLATE3)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("--suffix")
        .arg("suf")
        .arg(TEST_TEMPLATE4)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("--suffix")
        .arg("suf")
        .arg(TEST_TEMPLATE5)
        .succeeds();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("--suffix")
        .arg("suf")
        .arg(TEST_TEMPLATE6)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("--suffix")
        .arg("suf")
        .arg(TEST_TEMPLATE7)
        .fails();
    scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("--suffix")
        .arg("suf")
        .arg(TEST_TEMPLATE8)
        .fails();
}

#[test]
fn test_mktemp_tmpdir() {
    let scene = TestScenario::new(util_name!());
    let dir = tempdir().unwrap();
    let path = dir.path().join(scene.fixtures.as_string());
    let pathname = path.as_os_str();

    scene
        .ucmd()
        .arg("-p")
        .arg(pathname)
        .arg(TEST_TEMPLATE1)
        .succeeds();
    scene
        .ucmd()
        .arg("-p")
        .arg(pathname)
        .arg(TEST_TEMPLATE2)
        .fails();
    scene
        .ucmd()
        .arg("-p")
        .arg(pathname)
        .arg(TEST_TEMPLATE3)
        .fails();
    scene
        .ucmd()
        .arg("-p")
        .arg(pathname)
        .arg(TEST_TEMPLATE4)
        .fails();
    scene
        .ucmd()
        .arg("-p")
        .arg(pathname)
        .arg(TEST_TEMPLATE5)
        .succeeds();
    scene
        .ucmd()
        .arg("-p")
        .arg(pathname)
        .arg(TEST_TEMPLATE6)
        .succeeds();
    scene
        .ucmd()
        .arg("-p")
        .arg(pathname)
        .arg(TEST_TEMPLATE7)
        .succeeds();
    scene
        .ucmd()
        .arg("-p")
        .arg(pathname)
        .arg(TEST_TEMPLATE8)
        .fails();
}

#[test]
fn test_mktemp_tmpdir_one_arg() {
    let scene = TestScenario::new(util_name!());

    let result = scene
        .ucmd_keepenv()
        .arg("--tmpdir")
        .arg("apt-key-gpghome.XXXXXXXXXX")
        .succeeds();
    result.no_stderr().stdout_contains("apt-key-gpghome.");
    assert!(PathBuf::from(result.stdout_str().trim()).is_file());
}

#[test]
fn test_mktemp_directory_tmpdir() {
    let scene = TestScenario::new(util_name!());

    let result = scene
        .ucmd_keepenv()
        .arg("--directory")
        .arg("--tmpdir")
        .arg("apt-key-gpghome.XXXXXXXXXX")
        .succeeds();
    result.no_stderr().stdout_contains("apt-key-gpghome.");
    assert!(PathBuf::from(result.stdout_str().trim()).is_dir());
}

/// Decide whether a string matches a given template.
///
/// In the template, the character `'X'` is treated as a wildcard,
/// that is, it matches anything. All other characters in `template`
/// and `s` must match exactly.
///
/// # Examples
///
/// ```rust,ignore
/// # These all match.
/// assert!(matches_template("abc", "abc"));
/// assert!(matches_template("aXc", "abc"));
/// assert!(matches_template("XXX", "abc"));
///
/// # None of these match
/// assert!(matches_template("abc", "abcd"));
/// assert!(matches_template("abc", "ab"));
/// assert!(matches_template("aXc", "abd"));
/// assert!(matches_template("XXX", "abcd"));
/// ```
///
fn matches_template(template: &str, s: &str) -> bool {
    if template.len() != s.len() {
        return false;
    }
    for (a, b) in template.chars().zip(s.chars()) {
        if !(a == 'X' || a == b) {
            return false;
        }
    }
    true
}

/// An assertion that uses [`matches_template`] and adds a helpful error message.
macro_rules! assert_matches_template {
    ($template:expr, $s:expr) => {{
        assert!(
            matches_template($template, $s),
            "\"{}\" != \"{}\"",
            $template,
            $s
        );
    }};
}

/// Test that the file is created in the directory given by the template.
#[test]
fn test_respect_template() {
    let (at, mut ucmd) = at_and_ucmd!();
    let template = "XXX";
    let result = ucmd.arg(template).succeeds();
    let filename = result.no_stderr().stdout_str().trim_end();
    assert_matches_template!(template, filename);
    assert!(at.file_exists(filename));
}

/// Test that the file is created in the directory given by the template.
#[test]
fn test_respect_template_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("d");
    #[cfg(not(windows))]
    let template = "d/XXX";
    #[cfg(windows)]
    let template = r"d\XXX";
    let result = ucmd.arg(template).succeeds();
    let filename = result.no_stderr().stdout_str().trim_end();
    assert_matches_template!(template, filename);
    assert!(at.file_exists(filename));
}

/// Test that a template with a path separator is invalid.
#[test]
fn test_template_path_separator() {
    new_ucmd!()
        .args(&["-t", "a/bXXX"])
        .fails()
        .stderr_only(format!(
            "mktemp: invalid template, {}, contains directory separator\n",
            "a/bXXX".quote()
        ));
}
