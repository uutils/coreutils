use common::util::*;
extern crate tempdir;
use self::tempdir::TempDir;

static UTIL_NAME: &'static str = "mktemp";

static TEST_TEMPLATE1: &'static str = "tempXXXXXX";
static TEST_TEMPLATE2: &'static str = "temp";
static TEST_TEMPLATE3: &'static str = "tempX";
static TEST_TEMPLATE4: &'static str = "tempXX";
static TEST_TEMPLATE5: &'static str = "tempXXX";
static TEST_TEMPLATE6: &'static str = "tempXXXlate";
static TEST_TEMPLATE7: &'static str = "XXXtemplate";
#[cfg(unix)]
static TEST_TEMPLATE8: &'static str = "tempXXXla/te";
#[cfg(windows)]
static TEST_TEMPLATE8: &'static str = "tempXXXla\\te";

const TMPDIR: &'static str = "TMPDIR";

#[test]
fn test_mktemp_mktemp() {
    let scene = TestScenario::new(UTIL_NAME);

    let pathname = scene.fixtures.as_string();

    let exit_success1 = scene.ucmd().env(TMPDIR, &pathname).arg(TEST_TEMPLATE1).run().success;
    let exit_success2 = scene.ucmd().env(TMPDIR, &pathname).arg(TEST_TEMPLATE2).run().success;
    let exit_success3 = scene.ucmd().env(TMPDIR, &pathname).arg(TEST_TEMPLATE3).run().success;
    let exit_success4 = scene.ucmd().env(TMPDIR, &pathname).arg(TEST_TEMPLATE4).run().success;
    let exit_success5 = scene.ucmd().env(TMPDIR, &pathname).arg(TEST_TEMPLATE5).run().success;
    let exit_success6 = scene.ucmd().env(TMPDIR, &pathname).arg(TEST_TEMPLATE6).run().success;
    let exit_success7 = scene.ucmd().env(TMPDIR, &pathname).arg(TEST_TEMPLATE7).run().success;
    let exit_success8 = scene.ucmd().env(TMPDIR, &pathname).arg(TEST_TEMPLATE8).run().success;


    assert!(exit_success1);
    assert!(!exit_success2);
    assert!(!exit_success3);
    assert!(!exit_success4);
    assert!(exit_success5);
    assert!(exit_success6);
    assert!(exit_success7);
    assert!(!exit_success8);
}

#[test]
fn test_mktemp_make_temp_dir() {
    let scene = TestScenario::new(UTIL_NAME);

    let pathname = scene.fixtures.as_string();

    let exit_success1 = scene.ucmd().env(TMPDIR, &pathname).arg("-d").arg(TEST_TEMPLATE1).run().success;
    let exit_success2 = scene.ucmd().env(TMPDIR, &pathname).arg("-d").arg(TEST_TEMPLATE2).run().success;
    let exit_success3 = scene.ucmd().env(TMPDIR, &pathname).arg("-d").arg(TEST_TEMPLATE3).run().success;
    let exit_success4 = scene.ucmd().env(TMPDIR, &pathname).arg("-d").arg(TEST_TEMPLATE4).run().success;
    let exit_success5 = scene.ucmd().env(TMPDIR, &pathname).arg("-d").arg(TEST_TEMPLATE5).run().success;
    let exit_success6 = scene.ucmd().env(TMPDIR, &pathname).arg("-d").arg(TEST_TEMPLATE6).run().success;
    let exit_success7 = scene.ucmd().env(TMPDIR, &pathname).arg("-d").arg(TEST_TEMPLATE7).run().success;
    let exit_success8 = scene.ucmd().env(TMPDIR, &pathname).arg("-d").arg(TEST_TEMPLATE8).run().success;

    assert!(exit_success1);
    assert!(!exit_success2);
    assert!(!exit_success3);
    assert!(!exit_success4);
    assert!(exit_success5);
    assert!(exit_success6);
    assert!(exit_success7);
    assert!(!exit_success8);
}

#[test]
fn test_mktemp_dry_run() {
    let scene = TestScenario::new(UTIL_NAME);

    let pathname = scene.fixtures.as_string();

    let exit_success1 = scene.ucmd().env(TMPDIR, &pathname).arg("-u").arg(TEST_TEMPLATE1).run().success;
    let exit_success2 = scene.ucmd().env(TMPDIR, &pathname).arg("-u").arg(TEST_TEMPLATE2).run().success;
    let exit_success3 = scene.ucmd().env(TMPDIR, &pathname).arg("-u").arg(TEST_TEMPLATE3).run().success;
    let exit_success4 = scene.ucmd().env(TMPDIR, &pathname).arg("-u").arg(TEST_TEMPLATE4).run().success;
    let exit_success5 = scene.ucmd().env(TMPDIR, &pathname).arg("-u").arg(TEST_TEMPLATE5).run().success;
    let exit_success6 = scene.ucmd().env(TMPDIR, &pathname).arg("-u").arg(TEST_TEMPLATE6).run().success;
    let exit_success7 = scene.ucmd().env(TMPDIR, &pathname).arg("-u").arg(TEST_TEMPLATE7).run().success;
    let exit_success8 = scene.ucmd().env(TMPDIR, &pathname).arg("-u").arg(TEST_TEMPLATE8).run().success;


    assert!(exit_success1);
    assert!(!exit_success2);
    assert!(!exit_success3);
    assert!(!exit_success4);
    assert!(exit_success5);
    assert!(exit_success6);
    assert!(exit_success7);
    assert!(!exit_success8);
}

#[test]
fn test_mktemp_quiet() {
    let scene = TestScenario::new(UTIL_NAME);

    let result1 = scene.ucmd().arg("-p").arg("/definitely/not/exist/I/promise").arg("-q").run();
    let result2 = scene.ucmd().arg("-d").arg("-p").arg("/definitely/not/exist/I/promise").arg("-q").run();

    assert!(result1.stderr.is_empty() && result1.stdout.is_empty() && !result1.success);
    assert!(result2.stderr.is_empty() && result2.stdout.is_empty() && !result2.success);
}

#[test]
fn test_mktemp_suffix() {
    let scene = TestScenario::new(UTIL_NAME);

    let pathname = scene.fixtures.as_string();

    let exit_success1 = scene.ucmd().env(TMPDIR, &pathname).arg("--suffix").arg("suf").arg(TEST_TEMPLATE1).run().success;
    let exit_success2 = scene.ucmd().env(TMPDIR, &pathname).arg("--suffix").arg("suf").arg(TEST_TEMPLATE2).run().success;
    let exit_success3 = scene.ucmd().env(TMPDIR, &pathname).arg("--suffix").arg("suf").arg(TEST_TEMPLATE3).run().success;
    let exit_success4 = scene.ucmd().env(TMPDIR, &pathname).arg("--suffix").arg("suf").arg(TEST_TEMPLATE4).run().success;
    let exit_success5 = scene.ucmd().env(TMPDIR, &pathname).arg("--suffix").arg("suf").arg(TEST_TEMPLATE5).run().success;
    let exit_success6 = scene.ucmd().env(TMPDIR, &pathname).arg("--suffix").arg("suf").arg(TEST_TEMPLATE6).run().success;
    let exit_success7 = scene.ucmd().env(TMPDIR, &pathname).arg("--suffix").arg("suf").arg(TEST_TEMPLATE7).run().success;
    let exit_success8 = scene.ucmd().env(TMPDIR, &pathname).arg("--suffix").arg("suf").arg(TEST_TEMPLATE8).run().success;


    assert!(exit_success1);
    assert!(!exit_success2);
    assert!(!exit_success3);
    assert!(!exit_success4);
    assert!(exit_success5);
    assert!(!exit_success6);
    assert!(!exit_success7);
    assert!(!exit_success8);
}

#[test]
fn test_mktemp_tmpdir() {
    let scene = TestScenario::new(UTIL_NAME);

   let path = TempDir::new_in(scene.fixtures.as_string(), UTIL_NAME).unwrap();
   let pathname = path.path().as_os_str();

    let exit_success1 = scene.ucmd().arg("-p").arg(pathname).arg(TEST_TEMPLATE1).run().success;
    let exit_success2 = scene.ucmd().arg("-p").arg(pathname).arg(TEST_TEMPLATE2).run().success;
    let exit_success3 = scene.ucmd().arg("-p").arg(pathname).arg(TEST_TEMPLATE3).run().success;
    let exit_success4 = scene.ucmd().arg("-p").arg(pathname).arg(TEST_TEMPLATE4).run().success;
    let exit_success5 = scene.ucmd().arg("-p").arg(pathname).arg(TEST_TEMPLATE5).run().success;
    let exit_success6 = scene.ucmd().arg("-p").arg(pathname).arg(TEST_TEMPLATE6).run().success;
    let exit_success7 = scene.ucmd().arg("-p").arg(pathname).arg(TEST_TEMPLATE7).run().success;
    let exit_success8 = scene.ucmd().arg("-p").arg(pathname).arg(TEST_TEMPLATE8).run().success;


    assert!(exit_success1);
    assert!(!exit_success2);
    assert!(!exit_success3);
    assert!(!exit_success4);
    assert!(exit_success5);
    assert!(exit_success6);
    assert!(exit_success7);
    assert!(!exit_success8);
}
