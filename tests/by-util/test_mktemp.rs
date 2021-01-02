use crate::common::util::*;

extern crate tempfile;
use self::tempfile::tempdir;

static TEST_TEMPLATE1: &'static str = "tempXXXXXX";
static TEST_TEMPLATE2: &'static str = "temp";
static TEST_TEMPLATE3: &'static str = "tempX";
static TEST_TEMPLATE4: &'static str = "tempXX";
static TEST_TEMPLATE5: &'static str = "tempXXX";
static TEST_TEMPLATE6: &'static str = "tempXXXlate";
static TEST_TEMPLATE7: &'static str = "XXXtemplate";
#[cfg(unix)]
static TEST_TEMPLATE8: &'static str = "tempXXXl/ate";
#[cfg(windows)]
static TEST_TEMPLATE8: &'static str = "tempXXXl\\ate";

const TMPDIR: &'static str = "TMPDIR";

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
    let result = scene
        .ucmd()
        .env(TMPDIR, &pathname)
        .arg("-t")
        .arg(TEST_TEMPLATE8)
        .fails();
    println!("stdout {}", result.stdout);
    println!("stderr {}", result.stderr);
    assert!(result
        .stderr
        .contains("error: suffix cannot contain any path separators"));
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
