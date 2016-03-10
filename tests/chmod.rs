#[macro_use]
mod common;

use std::os::unix::raw::mode_t;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::fs::OpenOptionsExt;
use common::util::*;

static UTIL_NAME: &'static str = "chmod";
static TEST_FILE: &'static str = "file";
static REFERENCE_FILE: &'static str = "reference";
static REFERENCE_PERMS: mode_t = 0o247;

struct TestCase {
    args: Vec<&'static str>,
    before: mode_t,
    after: mode_t
}

fn mkfile(file: &str, mode: mode_t) {
    std::fs::OpenOptions::new().mode(mode).create(true).write(true).open(file).unwrap();
    let mut perms = std::fs::metadata(file).unwrap().permissions();
    perms.set_mode(mode);
    std::fs::set_permissions(file, perms).unwrap();
}

fn run_tests(tests: Vec<TestCase>) {
    for test in tests {
        let (at, mut ucmd) = testing(UTIL_NAME);

        mkfile(&at.plus_as_string(TEST_FILE), test.before);
        mkfile(&at.plus_as_string(REFERENCE_FILE), REFERENCE_PERMS);
        let perms = at.metadata(TEST_FILE).permissions().mode();
        if perms != test.before{
            panic!(format!("{}: expected: {:o} got: {:o}", "setting permissions failed", test.after, perms));
        }

        for arg in test.args {
            ucmd.arg(arg);
        }
        let r = ucmd.run();
        if !r.success {
            println!("{}", r.stderr);
            panic!(format!("{:?}: failed", ucmd.raw));
        }

        let perms = at.metadata(TEST_FILE).permissions().mode();
        if perms != test.after {
            panic!(format!("{:?}: expected: {:o} got: {:o}", ucmd.raw, test.after, perms));
        }
    }
}

#[test]
fn test_chmod_octal() {
    let tests = vec!{
        TestCase{args: vec!{"0700",  TEST_FILE}, before: 0o000, after: 0o700},
        TestCase{args: vec!{"0070",  TEST_FILE}, before: 0o000, after: 0o070},
        TestCase{args: vec!{"0007",  TEST_FILE}, before: 0o000, after: 0o007},
        // Known failues: #788
        // TestCase{args: vec!{"-0700", TEST_FILE}, before: 0o700, after: 0o000},
        // TestCase{args: vec!{"-0070", TEST_FILE}, before: 0o060, after: 0o000},
        // TestCase{args: vec!{"-0007", TEST_FILE}, before: 0o001, after: 0o000},
        TestCase{args: vec!{"+0100", TEST_FILE}, before: 0o600, after: 0o700},
        TestCase{args: vec!{"+0020", TEST_FILE}, before: 0o050, after: 0o070},
        TestCase{args: vec!{"+0004", TEST_FILE}, before: 0o003, after: 0o007},
    };
    run_tests(tests);
}

#[test]
fn test_chmod_ugoa() {
    let tests = vec!{
        TestCase{args: vec!{"u=rwx", TEST_FILE}, before: 0o000, after: 0o700},
        TestCase{args: vec!{"g=rwx", TEST_FILE}, before: 0o000, after: 0o070},
        TestCase{args: vec!{"o=rwx", TEST_FILE}, before: 0o000, after: 0o007},
        TestCase{args: vec!{"a=rwx", TEST_FILE}, before: 0o000, after: 0o777},
    };
    run_tests(tests);
}

#[test]
fn test_chmod_ugo_copy() {
    let tests = vec!{
        TestCase{args: vec!{"u=g", TEST_FILE}, before: 0o070, after: 0o770},
        TestCase{args: vec!{"g=o", TEST_FILE}, before: 0o005, after: 0o055},
        TestCase{args: vec!{"o=u", TEST_FILE}, before: 0o200, after: 0o202},
        TestCase{args: vec!{"u-g", TEST_FILE}, before: 0o710, after: 0o610},
        TestCase{args: vec!{"u+g", TEST_FILE}, before: 0o250, after: 0o750},
    };
    run_tests(tests);
}

#[test]
fn test_chmod_reference_file() {
    let tests = vec!{
        TestCase{args: vec!{"--reference", REFERENCE_FILE, TEST_FILE}, before: 0o070, after: 0o247},
    };
    run_tests(tests);
}
