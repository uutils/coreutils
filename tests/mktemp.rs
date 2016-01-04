extern crate tempdir;

use tempdir::TempDir;

#[macro_use]
mod common;

use common::util::*;

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
    let ts = TestSet::new(UTIL_NAME);

    let pathname = ts.fixtures.as_string();

    let exit_success1 = ts.util_cmd().env(TMPDIR, &pathname).arg(TEST_TEMPLATE1).run().success;
    let exit_success2 = ts.util_cmd().env(TMPDIR, &pathname).arg(TEST_TEMPLATE2).run().success;
    let exit_success3 = ts.util_cmd().env(TMPDIR, &pathname).arg(TEST_TEMPLATE3).run().success;
    let exit_success4 = ts.util_cmd().env(TMPDIR, &pathname).arg(TEST_TEMPLATE4).run().success;
    let exit_success5 = ts.util_cmd().env(TMPDIR, &pathname).arg(TEST_TEMPLATE5).run().success;
    let exit_success6 = ts.util_cmd().env(TMPDIR, &pathname).arg(TEST_TEMPLATE6).run().success;
    let exit_success7 = ts.util_cmd().env(TMPDIR, &pathname).arg(TEST_TEMPLATE7).run().success;
    let exit_success8 = ts.util_cmd().env(TMPDIR, &pathname).arg(TEST_TEMPLATE8).run().success;


    assert!(exit_success1);
    assert!(!exit_success2);
    assert!(!exit_success3);
    assert!(!exit_success4);
    assert!(exit_success5);
    assert!(exit_success6);
    assert!(exit_success7);
    assert!(!exit_success8);
}

// TODO: test directory option when implemented

#[test]
fn test_mktemp_dry_run() {
    let ts = TestSet::new(UTIL_NAME);

    let pathname = ts.fixtures.as_string();

    let exit_success1 = ts.util_cmd().env(TMPDIR, &pathname).arg("-u").arg(TEST_TEMPLATE1).run().success;
    let exit_success2 = ts.util_cmd().env(TMPDIR, &pathname).arg("-u").arg(TEST_TEMPLATE2).run().success;
    let exit_success3 = ts.util_cmd().env(TMPDIR, &pathname).arg("-u").arg(TEST_TEMPLATE3).run().success;
    let exit_success4 = ts.util_cmd().env(TMPDIR, &pathname).arg("-u").arg(TEST_TEMPLATE4).run().success;
    let exit_success5 = ts.util_cmd().env(TMPDIR, &pathname).arg("-u").arg(TEST_TEMPLATE5).run().success;
    let exit_success6 = ts.util_cmd().env(TMPDIR, &pathname).arg("-u").arg(TEST_TEMPLATE6).run().success;
    let exit_success7 = ts.util_cmd().env(TMPDIR, &pathname).arg("-u").arg(TEST_TEMPLATE7).run().success;
    let exit_success8 = ts.util_cmd().env(TMPDIR, &pathname).arg("-u").arg(TEST_TEMPLATE8).run().success;


    assert!(exit_success1);
    assert!(!exit_success2);
    assert!(!exit_success3);
    assert!(!exit_success4);
    assert!(exit_success5);
    assert!(exit_success6);
    assert!(exit_success7);
    assert!(!exit_success8);
}

// TOOD: test quiet option when correctry implemented

#[test]
fn test_mktemp_suffix() {
    let ts = TestSet::new(UTIL_NAME);

    let pathname = ts.fixtures.as_string();

    let exit_success1 = ts.util_cmd().env(TMPDIR, &pathname).arg("--suffix").arg("suf").arg(TEST_TEMPLATE1).run().success;
    let exit_success2 = ts.util_cmd().env(TMPDIR, &pathname).arg("--suffix").arg("suf").arg(TEST_TEMPLATE2).run().success;
    let exit_success3 = ts.util_cmd().env(TMPDIR, &pathname).arg("--suffix").arg("suf").arg(TEST_TEMPLATE3).run().success;
    let exit_success4 = ts.util_cmd().env(TMPDIR, &pathname).arg("--suffix").arg("suf").arg(TEST_TEMPLATE4).run().success;
    let exit_success5 = ts.util_cmd().env(TMPDIR, &pathname).arg("--suffix").arg("suf").arg(TEST_TEMPLATE5).run().success;
    let exit_success6 = ts.util_cmd().env(TMPDIR, &pathname).arg("--suffix").arg("suf").arg(TEST_TEMPLATE6).run().success;
    let exit_success7 = ts.util_cmd().env(TMPDIR, &pathname).arg("--suffix").arg("suf").arg(TEST_TEMPLATE7).run().success;
    let exit_success8 = ts.util_cmd().env(TMPDIR, &pathname).arg("--suffix").arg("suf").arg(TEST_TEMPLATE8).run().success;


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
    let ts = TestSet::new(UTIL_NAME);

   let path = TempDir::new_in(ts.fixtures.as_string(), UTIL_NAME).unwrap();
   let pathname = path.path().as_os_str();

    let exit_success1 = ts.util_cmd().arg("-p").arg(pathname).arg(TEST_TEMPLATE1).run().success;
    let exit_success2 = ts.util_cmd().arg("-p").arg(pathname).arg(TEST_TEMPLATE2).run().success;
    let exit_success3 = ts.util_cmd().arg("-p").arg(pathname).arg(TEST_TEMPLATE3).run().success;
    let exit_success4 = ts.util_cmd().arg("-p").arg(pathname).arg(TEST_TEMPLATE4).run().success;
    let exit_success5 = ts.util_cmd().arg("-p").arg(pathname).arg(TEST_TEMPLATE5).run().success;
    let exit_success6 = ts.util_cmd().arg("-p").arg(pathname).arg(TEST_TEMPLATE6).run().success;
    let exit_success7 = ts.util_cmd().arg("-p").arg(pathname).arg(TEST_TEMPLATE7).run().success;
    let exit_success8 = ts.util_cmd().arg("-p").arg(pathname).arg(TEST_TEMPLATE8).run().success;


    assert!(exit_success1);
    assert!(!exit_success2);
    assert!(!exit_success3);
    assert!(!exit_success4);
    assert!(exit_success5);
    assert!(exit_success6);
    assert!(exit_success7);
    assert!(!exit_success8);
}
