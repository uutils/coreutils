use common::util::*;
use std::fs::{metadata, OpenOptions, set_permissions};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::sync::Mutex;

extern crate libc;
use self::libc::umask;


static TEST_FILE: &'static str = "file";
static REFERENCE_FILE: &'static str = "reference";
static REFERENCE_PERMS: u32 = 0o247;
lazy_static! {
    static ref UMASK_MUTEX: Mutex<()> = Mutex::new(());
}

struct TestCase {
    args: Vec<&'static str>,
    before: u32,
    after: u32
}

fn mkfile(file: &str, mode: u32) {
    OpenOptions::new().mode(mode).create(true).write(true).open(file).unwrap();
    let mut perms = metadata(file).unwrap().permissions();
    perms.set_mode(mode);
    set_permissions(file, perms).unwrap();
}

fn run_single_test(test: &TestCase, at: AtPath, mut ucmd: UCommand) {
        mkfile(&at.plus_as_string(TEST_FILE), test.before);
        let perms = at.metadata(TEST_FILE).permissions().mode();
        if perms != test.before {
            panic!(format!("{}: expected: {:o} got: {:o}", "setting permissions failed", test.after, perms));
        }

        for arg in &test.args {
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

fn run_tests(tests: Vec<TestCase>) {
    for test in tests {
        let (at, ucmd) = at_and_ucmd!();
        run_single_test(&test, at, ucmd);
    }
}

#[test]
fn test_chmod_octal() {
    let tests = vec!{
        TestCase{args: vec!{"0700",  TEST_FILE}, before: 0o000, after: 0o700},
        TestCase{args: vec!{"0070",  TEST_FILE}, before: 0o000, after: 0o070},
        TestCase{args: vec!{"0007",  TEST_FILE}, before: 0o000, after: 0o007},
        TestCase{args: vec!{"-0700", TEST_FILE}, before: 0o700, after: 0o000},
        TestCase{args: vec!{"-0070", TEST_FILE}, before: 0o060, after: 0o000},
        TestCase{args: vec!{"-0007", TEST_FILE}, before: 0o001, after: 0o000},
        TestCase{args: vec!{"+0100", TEST_FILE}, before: 0o600, after: 0o700},
        TestCase{args: vec!{"+0020", TEST_FILE}, before: 0o050, after: 0o070},
        TestCase{args: vec!{"+0004", TEST_FILE}, before: 0o003, after: 0o007},
    };
    run_tests(tests);
}

#[test]
fn test_chmod_ugoa() {
    let _guard = UMASK_MUTEX.lock();

    let last = unsafe {
        umask(0)
    };
    let tests = vec!{
        TestCase{args: vec!{"u=rwx", TEST_FILE}, before: 0o000, after: 0o700},
        TestCase{args: vec!{"g=rwx", TEST_FILE}, before: 0o000, after: 0o070},
        TestCase{args: vec!{"o=rwx", TEST_FILE}, before: 0o000, after: 0o007},
        TestCase{args: vec!{"a=rwx", TEST_FILE}, before: 0o000, after: 0o777},
        TestCase{args: vec!{"-r", TEST_FILE}, before: 0o777, after: 0o333},
        TestCase{args: vec!{"-w", TEST_FILE}, before: 0o777, after: 0o555},
        TestCase{args: vec!{"-x", TEST_FILE}, before: 0o777, after: 0o666},
    };
    run_tests(tests);

    unsafe {
        umask(0o022);
    }
    let tests = vec!{
        TestCase{args: vec!{"u=rwx", TEST_FILE}, before: 0o000, after: 0o700},
        TestCase{args: vec!{"g=rwx", TEST_FILE}, before: 0o000, after: 0o070},
        TestCase{args: vec!{"o=rwx", TEST_FILE}, before: 0o000, after: 0o007},
        TestCase{args: vec!{"a=rwx", TEST_FILE}, before: 0o000, after: 0o777},
        TestCase{args: vec!{"+rw", TEST_FILE}, before: 0o000, after: 0o644},
        TestCase{args: vec!{"=rwx", TEST_FILE}, before: 0o000, after: 0o755},
        TestCase{args: vec!{"-w", TEST_FILE}, before: 0o777, after: 0o577},
        TestCase{args: vec!{"-x", TEST_FILE}, before: 0o777, after: 0o666},
    };
    run_tests(tests);
    unsafe {
        umask(last);
    }
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
fn test_chmod_many_options() {
    let _guard = UMASK_MUTEX.lock();

    let original_umask = unsafe {
        umask(0)
    };
    let tests = vec!{
        TestCase{args: vec!{"-r,a+w", TEST_FILE}, before: 0o444, after: 0o222},
    };
    run_tests(tests);
    unsafe {
        umask(original_umask);
    }
}

#[test]
fn test_chmod_reference_file() {
    let tests = vec!{
        TestCase{args: vec!{"--reference", REFERENCE_FILE, TEST_FILE}, before: 0o070, after: 0o247},
        TestCase{args: vec!{"a-w", "--reference", REFERENCE_FILE, TEST_FILE}, before: 0o070, after: 0o247},
    };
    let (at, ucmd) = at_and_ucmd!();
    mkfile(&at.plus_as_string(REFERENCE_FILE), REFERENCE_PERMS);
    run_single_test(&tests[0], at, ucmd);
}
