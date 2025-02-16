// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) gpghome

use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

use uucore::display::Quotable;

use std::path::PathBuf;
#[cfg(not(windows))]
use std::path::MAIN_SEPARATOR;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static TEST_TEMPLATE1: &str = "tempXXXXXX";
static TEST_TEMPLATE2: &str = "temp";
static TEST_TEMPLATE3: &str = "tempX";
static TEST_TEMPLATE4: &str = "tempXX";
static TEST_TEMPLATE5: &str = "tempXXX";
static TEST_TEMPLATE6: &str = "tempXXXlate";
static TEST_TEMPLATE7: &str = "XXXtemplate";
#[cfg(unix)]
static TEST_TEMPLATE8: &str = "tempXXXl/ate";
#[cfg(windows)]
static TEST_TEMPLATE8: &str = "tempXXXl\\ate";

#[cfg(not(windows))]
const TMPDIR: &str = "TMPDIR";
#[cfg(windows)]
const TMPDIR: &str = "TMP";

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

/// Like [`assert_matches_template`] but for the suffix of a string.
#[cfg(windows)]
macro_rules! assert_suffix_matches_template {
    ($template:expr, $s:expr) => {{
        let n = ($s).len();
        let m = ($template).len();
        let suffix = &$s[n - m..n];
        assert!(
            matches_template($template, suffix),
            "\"{}\" does not end with \"{}\"",
            $template,
            suffix
        );
    }};
}

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
        .ucmd()
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
        .ucmd()
        .arg("--directory")
        .arg("--tmpdir")
        .arg("apt-key-gpghome.XXXXXXXXXX")
        .succeeds();
    result.no_stderr().stdout_contains("apt-key-gpghome.");
    assert!(PathBuf::from(result.stdout_str().trim()).is_dir());
}

/// Test for combining `--tmpdir` and a template with a subdirectory.
#[test]
fn test_tmpdir_template_has_subdirectory() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("a");
    #[cfg(not(windows))]
    let (template, joined) = ("a/bXXXX", "./a/bXXXX");
    #[cfg(windows)]
    let (template, joined) = (r"a\bXXXX", r".\a\bXXXX");
    let result = ucmd.args(&["--tmpdir=.", template]).succeeds();
    let filename = result.no_stderr().stdout_str().trim_end();
    assert_matches_template!(joined, filename);
    assert!(at.file_exists(filename));
}

/// Test that an absolute path is disallowed when --tmpdir is provided.
#[test]
fn test_tmpdir_absolute_path() {
    #[cfg(windows)]
    let path = r"C:\XXX";
    #[cfg(not(windows))]
    let path = "/XXX";
    new_ucmd!()
        .args(&["--tmpdir=a", path])
        .fails()
        .stderr_only(format!(
            "mktemp: invalid template, '{path}'; with --tmpdir, it may not be absolute\n"
        ));
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

#[cfg(unix)]
#[test]
fn test_directory_permissions() {
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd.args(&["-d", "XXX"]).succeeds();
    let dirname = result.no_stderr().stdout_str().trim_end();
    assert_matches_template!("XXX", dirname);
    let metadata = at.metadata(dirname);
    assert!(metadata.is_dir());
    assert_eq!(metadata.permissions().mode(), 0o40700);
}

/// Test that a template with a path separator is invalid.
#[test]
fn test_template_path_separator() {
    #[cfg(not(windows))]
    new_ucmd!()
        .args(&["-t", "a/bXXX"])
        .fails()
        .stderr_only(format!(
            "mktemp: invalid template, {}, contains directory separator\n",
            "a/bXXX".quote()
        ));
    #[cfg(windows)]
    new_ucmd!()
        .args(&["-t", r"a\bXXX"])
        .fails()
        .stderr_only(format!(
            "mktemp: invalid template, {}, contains directory separator\n",
            r"a\bXXX".quote()
        ));
}

/// Test that a prefix with a point is valid.
#[test]
fn test_prefix_template_separator() {
    new_ucmd!().args(&["-p", ".", "-t", "a.XXXX"]).succeeds();
}

#[test]
fn test_prefix_template_with_path_separator() {
    #[cfg(not(windows))]
    new_ucmd!()
        .args(&["-t", "a/XXX"])
        .fails()
        .stderr_only(format!(
            "mktemp: invalid template, {}, contains directory separator\n",
            "a/XXX".quote()
        ));
    #[cfg(windows)]
    new_ucmd!()
        .args(&["-t", r"a\XXX"])
        .fails()
        .stderr_only(format!(
            "mktemp: invalid template, {}, contains directory separator\n",
            r"a\XXX".quote()
        ));
}

/// Test that a suffix with a path separator is invalid.
#[test]
fn test_suffix_path_separator() {
    #[cfg(not(windows))]
    new_ucmd!()
        .arg("aXXX/b")
        .fails()
        .stderr_only("mktemp: invalid suffix '/b', contains directory separator\n");
    #[cfg(windows)]
    new_ucmd!()
        .arg(r"aXXX\b")
        .fails()
        .stderr_only("mktemp: invalid suffix '\\b', contains directory separator\n");
    #[cfg(not(windows))]
    new_ucmd!()
        .arg("XXX/..")
        .fails()
        .stderr_only("mktemp: invalid suffix '/..', contains directory separator\n");
    #[cfg(windows)]
    new_ucmd!()
        .arg(r"XXX\..")
        .fails()
        .stderr_only("mktemp: invalid suffix '\\..', contains directory separator\n");
}

#[test]
fn test_too_few_xs_suffix() {
    new_ucmd!()
        .args(&["--suffix=X", "aXX"])
        .fails()
        .stderr_only("mktemp: too few X's in template 'aXX'\n");
}

#[test]
fn test_too_few_xs_suffix_directory() {
    new_ucmd!()
        .args(&["-d", "--suffix=X", "aXX"])
        .fails()
        .stderr_only("mktemp: too few X's in template 'aXX'\n");
}

#[test]
fn test_too_many_arguments() {
    new_ucmd!()
        .args(&["-q", "a", "b"])
        .fails()
        .code_is(1)
        .usage_error("too many templates");
}

#[test]
fn test_two_contiguous_wildcard_blocks() {
    let (at, mut ucmd) = at_and_ucmd!();
    let template = "XXX_XXX";
    let result = ucmd.arg(template).succeeds();
    let filename = result.no_stderr().stdout_str().trim_end();
    assert_eq!(&filename[..4], "XXX_");
    assert_matches_template!(template, filename);
    assert!(at.file_exists(filename));
}

#[test]
fn test_three_contiguous_wildcard_blocks() {
    let (at, mut ucmd) = at_and_ucmd!();
    let template = "XXX_XXX_XXX";
    let result = ucmd.arg(template).succeeds();
    let filename = result.no_stderr().stdout_str().trim_end();
    assert_eq!(&filename[..8], "XXX_XXX_");
    assert_matches_template!(template, filename);
    assert!(at.file_exists(filename));
}

/// Test that template must end in X even if `--suffix` is the empty string.
#[test]
fn test_suffix_must_end_in_x() {
    new_ucmd!()
        .args(&["--suffix=", "aXXXb"])
        .fails()
        .stderr_is("mktemp: with --suffix, template 'aXXXb' must end in X\n");
}

#[test]
fn test_suffix_empty_template() {
    new_ucmd!()
        .args(&["--suffix=aXXXb", ""])
        .fails()
        .stderr_is("mktemp: with --suffix, template '' must end in X\n");

    new_ucmd!()
        .args(&["-d", "--suffix=aXXXb", ""])
        .fails()
        .stderr_is("mktemp: with --suffix, template '' must end in X\n");
}

#[test]
fn test_mktemp_with_posixly_correct() {
    let scene = TestScenario::new(util_name!());

    scene
        .ucmd()
        .env("POSIXLY_CORRECT", "1")
        .args(&["aXXXX", "--suffix=b"])
        .fails()
        .usage_error("too many templates");

    scene
        .ucmd()
        .env("POSIXLY_CORRECT", "1")
        .args(&["--suffix=b", "aXXXX"])
        .succeeds();
}

/// Test that files are created relative to `TMPDIR` environment variable.
#[test]
fn test_tmpdir_env_var() {
    // `TMPDIR=. mktemp`
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd.env(TMPDIR, ".").succeeds();
    let filename = result.no_stderr().stdout_str().trim_end();
    #[cfg(not(windows))]
    {
        let template = format!(".{MAIN_SEPARATOR}tmp.XXXXXXXXXX");
        assert_matches_template!(&template, filename);
    }
    // On Windows, `env::temp_dir()` seems to give an absolute path
    // regardless of the value of `TMPDIR`; see
    // * https://github.com/uutils/coreutils/pull/3552#issuecomment-1211804981
    // * https://doc.rust-lang.org/std/env/fn.temp_dir.html
    // * https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-gettemppath2w
    #[cfg(windows)]
    assert_suffix_matches_template!("tmp.XXXXXXXXXX", filename);
    assert!(at.file_exists(filename));

    // `TMPDIR=. mktemp --tmpdir`
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd.env(TMPDIR, ".").arg("--tmpdir").succeeds();
    let filename = result.no_stderr().stdout_str().trim_end();
    #[cfg(not(windows))]
    {
        let template = format!(".{MAIN_SEPARATOR}tmp.XXXXXXXXXX");
        assert_matches_template!(&template, filename);
    }
    #[cfg(windows)]
    assert_suffix_matches_template!("tmp.XXXXXXXXXX", filename);
    assert!(at.file_exists(filename));

    // `TMPDIR=. mktemp --tmpdir XXX`
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd.env(TMPDIR, ".").args(&["--tmpdir", "XXX"]).succeeds();
    let filename = result.no_stderr().stdout_str().trim_end();
    #[cfg(not(windows))]
    {
        let template = format!(".{MAIN_SEPARATOR}XXX");
        assert_matches_template!(&template, filename);
    }
    #[cfg(windows)]
    assert_suffix_matches_template!("XXX", filename);
    assert!(at.file_exists(filename));

    // `TMPDIR=. mktemp XXX` - in this case `TMPDIR` is ignored.
    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd.env(TMPDIR, ".").arg("XXX").succeeds();
    let filename = result.no_stderr().stdout_str().trim_end();
    let template = "XXX";
    assert_matches_template!(template, filename);
    assert!(at.file_exists(filename));
}

#[test]
fn test_nonexistent_tmpdir_env_var() {
    #[cfg(not(windows))]
    new_ucmd!().env(TMPDIR, "no/such/dir").fails().stderr_only("mktemp: failed to create file via template 'no/such/dir/tmp.XXXXXXXXXX': No such file or directory\n");
    #[cfg(windows)]
    {
        let result = new_ucmd!().env(TMPDIR, r"no\such\dir").fails();
        result.no_stdout();
        let stderr = result.stderr_str();
        assert!(
            stderr.starts_with("mktemp: failed to create file via template"),
            "{}",
            stderr
        );
        assert!(
            stderr.ends_with("no\\such\\dir\\tmp.XXXXXXXXXX': No such file or directory\n"),
            "{}",
            stderr
        );
    }

    #[cfg(not(windows))]
    new_ucmd!().env(TMPDIR, "no/such/dir").arg("-d").fails().stderr_only("mktemp: failed to create directory via template 'no/such/dir/tmp.XXXXXXXXXX': No such file or directory\n");
    #[cfg(windows)]
    {
        let result = new_ucmd!().env(TMPDIR, r"no\such\dir").arg("-d").fails();
        result.no_stdout();
        let stderr = result.stderr_str();
        assert!(
            stderr.starts_with("mktemp: failed to create directory via template"),
            "{}",
            stderr
        );
        assert!(
            stderr.ends_with("no\\such\\dir\\tmp.XXXXXXXXXX': No such file or directory\n"),
            "{}",
            stderr
        );
    }
}

#[test]
fn test_nonexistent_dir_prefix() {
    #[cfg(not(windows))]
    new_ucmd!().arg("d/XXX").fails().stderr_only(
        "mktemp: failed to create file via template 'd/XXX': No such file or directory\n",
    );
    #[cfg(windows)]
    {
        let result = new_ucmd!().arg(r"d\XXX").fails();
        result.no_stdout();
        let stderr = result.stderr_str();
        assert!(
            stderr.starts_with("mktemp: failed to create file via template"),
            "{}",
            stderr
        );
        assert!(
            stderr.ends_with("d\\XXX': No such file or directory\n"),
            "{}",
            stderr
        );
    }

    #[cfg(not(windows))]
    new_ucmd!().arg("-d").arg("d/XXX").fails().stderr_only(
        "mktemp: failed to create directory via template 'd/XXX': No such file or directory\n",
    );
    #[cfg(windows)]
    {
        let result = new_ucmd!().arg("-d").arg(r"d\XXX").fails();
        result.no_stdout();
        let stderr = result.stderr_str();
        assert!(
            stderr.starts_with("mktemp: failed to create directory via template"),
            "{}",
            stderr
        );
        assert!(
            stderr.ends_with("d\\XXX': No such file or directory\n"),
            "{}",
            stderr
        );
    }
}

#[test]
fn test_default_missing_value() {
    new_ucmd!().arg("-d").arg("--tmpdir").succeeds();
}

#[test]
fn test_default_issue_4821_t_tmpdir() {
    let scene = TestScenario::new(util_name!());
    let pathname = scene.fixtures.as_string();
    let result = scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-t")
        .arg("foo.XXXX")
        .succeeds();
    let stdout = result.stdout_str();
    println!("stdout = {stdout}");
    assert!(stdout.contains(&pathname));
}

#[test]
fn test_default_issue_4821_t_tmpdir_p() {
    let scene = TestScenario::new(util_name!());
    let pathname = scene.fixtures.as_string();
    let result = scene
        .ucmd()
        .arg("-t")
        .arg("-p")
        .arg(&pathname)
        .arg("foo.XXXX")
        .succeeds();
    let stdout = result.stdout_str();
    println!("stdout = {stdout}");
    assert!(stdout.contains(&pathname));
}

#[test]
fn test_t_ensure_tmpdir_has_higher_priority_than_p() {
    let scene = TestScenario::new(util_name!());
    let pathname = scene.fixtures.as_string();
    let result = scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-t")
        .arg("-p")
        .arg("should_not_attempt_to_write_in_this_nonexisting_dir")
        .arg("foo.XXXX")
        .succeeds();
    let stdout = result.stdout_str();
    println!("stdout = {stdout}");
    assert!(stdout.contains(&pathname));
}

#[test]
fn test_missing_xs_tmpdir_template() {
    let scene = TestScenario::new(util_name!());
    scene
        .ucmd()
        .arg("--tmpdir")
        .arg(TEST_TEMPLATE3)
        .fails()
        .no_stdout()
        .stderr_contains("too few X's in template");
    scene
        .ucmd()
        .arg("--tmpdir=foobar")
        .fails()
        .no_stdout()
        .stderr_contains("failed to create file via template");
}

#[test]
fn test_both_tmpdir_flags_present() {
    let scene = TestScenario::new(util_name!());

    #[cfg(not(windows))]
    let template = format!(".{MAIN_SEPARATOR}foobarXXXX");

    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .env(TMPDIR, ".")
        .arg("-p")
        .arg("nonsense")
        .arg("--tmpdir")
        .arg("foobarXXXX")
        .succeeds();
    let filename = result.no_stderr().stdout_str().trim_end();

    #[cfg(not(windows))]
    assert_matches_template!(&template, filename);
    #[cfg(windows)]
    assert_suffix_matches_template!("foobarXXXX", filename);

    assert!(at.file_exists(filename));

    scene
        .ucmd()
        .arg("-p")
        .arg(".")
        .arg("--tmpdir=does_not_exist")
        .fails()
        .no_stdout()
        .stderr_contains("failed to create file via template");

    let (at, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .arg("--tmpdir")
        .arg("foobarXXXX")
        .arg("-p")
        .arg(".")
        .succeeds();
    let filename = result.no_stderr().stdout_str().trim_end();

    #[cfg(not(windows))]
    assert_matches_template!(&template, filename);
    #[cfg(windows)]
    assert_suffix_matches_template!("foobarXXXX", filename);

    assert!(at.file_exists(filename));
}

#[test]
fn test_missing_short_tmpdir_flag() {
    let scene = TestScenario::new(util_name!());
    scene
        .ucmd()
        .arg("-p")
        .fails()
        .no_stdout()
        .stderr_contains("a value is required for '-p <DIR>' but none was supplied");
}
