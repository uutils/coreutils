use crate::common::util::*;
use std::fs::{metadata, set_permissions, OpenOptions};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::sync::Mutex;

extern crate libc;
use self::chmod::strip_minus_from_mode;
extern crate chmod;
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
    after: u32,
}

fn mkfile(file: &str, mode: u32) {
    OpenOptions::new()
        .mode(mode)
        .create(true)
        .write(true)
        .open(file)
        .unwrap();
    let mut perms = metadata(file).unwrap().permissions();
    perms.set_mode(mode);
    set_permissions(file, perms).unwrap();
}

fn run_single_test(test: &TestCase, at: AtPath, mut ucmd: UCommand) {
    mkfile(&at.plus_as_string(TEST_FILE), test.before);
    let perms = at.metadata(TEST_FILE).permissions().mode();
    if perms != test.before {
        panic!(
            "{}: expected: {:o} got: {:o}",
            "setting permissions on test files before actual test run failed", test.after, perms
        );
    }

    for arg in &test.args {
        ucmd.arg(arg);
    }
    let r = ucmd.run();
    if !r.success {
        println!("{}", r.stderr);
        panic!("{:?}: failed", ucmd.raw);
    }

    let perms = at.metadata(TEST_FILE).permissions().mode();
    if perms != test.after {
        panic!(
            "{:?}: expected: {:o} got: {:o}",
            ucmd.raw, test.after, perms
        );
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
    let tests = vec![
        TestCase {
            args: vec!["0700", TEST_FILE],
            before: 0o100000,
            after: 0o100700,
        },
        TestCase {
            args: vec!["0070", TEST_FILE],
            before: 0o100000,
            after: 0o100070,
        },
        TestCase {
            args: vec!["0007", TEST_FILE],
            before: 0o100000,
            after: 0o100007,
        },
        TestCase {
            args: vec!["-0700", TEST_FILE],
            before: 0o100700,
            after: 0o100000,
        },
        TestCase {
            args: vec!["-0070", TEST_FILE],
            before: 0o100060,
            after: 0o100000,
        },
        TestCase {
            args: vec!["-0007", TEST_FILE],
            before: 0o100001,
            after: 0o100000,
        },
        TestCase {
            args: vec!["+0100", TEST_FILE],
            before: 0o100600,
            after: 0o100700,
        },
        TestCase {
            args: vec!["+0020", TEST_FILE],
            before: 0o100050,
            after: 0o100070,
        },
        TestCase {
            args: vec!["+0004", TEST_FILE],
            before: 0o100003,
            after: 0o100007,
        },
    ];
    run_tests(tests);
}

#[test]
fn test_chmod_ugoa() {
    let _guard = UMASK_MUTEX.lock();

    let last = unsafe { umask(0) };
    let tests = vec![
        TestCase {
            args: vec!["u=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100700,
        },
        TestCase {
            args: vec!["g=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100070,
        },
        TestCase {
            args: vec!["o=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100007,
        },
        TestCase {
            args: vec!["a=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100777,
        },
        TestCase {
            args: vec!["-r", TEST_FILE],
            before: 0o100777,
            after: 0o100333,
        },
        TestCase {
            args: vec!["-w", TEST_FILE],
            before: 0o100777,
            after: 0o100555,
        },
        TestCase {
            args: vec!["-x", TEST_FILE],
            before: 0o100777,
            after: 0o100666,
        },
    ];
    run_tests(tests);

    unsafe {
        umask(0o022);
    }
    let tests = vec![
        TestCase {
            args: vec!["u=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100700,
        },
        TestCase {
            args: vec!["g=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100070,
        },
        TestCase {
            args: vec!["o=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100007,
        },
        TestCase {
            args: vec!["a=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100777,
        },
        TestCase {
            args: vec!["+rw", TEST_FILE],
            before: 0o100000,
            after: 0o100644,
        },
        TestCase {
            args: vec!["=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100755,
        },
        TestCase {
            args: vec!["-w", TEST_FILE],
            before: 0o100777,
            after: 0o100577,
        },
        TestCase {
            args: vec!["-x", TEST_FILE],
            before: 0o100777,
            after: 0o100666,
        },
    ];
    run_tests(tests);
    unsafe {
        umask(last);
    }
}

#[test]
fn test_chmod_ugo_copy() {
    let tests = vec![
        TestCase {
            args: vec!["u=g", TEST_FILE],
            before: 0o100070,
            after: 0o100770,
        },
        TestCase {
            args: vec!["g=o", TEST_FILE],
            before: 0o100005,
            after: 0o100055,
        },
        TestCase {
            args: vec!["o=u", TEST_FILE],
            before: 0o100200,
            after: 0o100202,
        },
        TestCase {
            args: vec!["u-g", TEST_FILE],
            before: 0o100710,
            after: 0o100610,
        },
        TestCase {
            args: vec!["u+g", TEST_FILE],
            before: 0o100250,
            after: 0o100750,
        },
    ];
    run_tests(tests);
}

#[test]
fn test_chmod_many_options() {
    let _guard = UMASK_MUTEX.lock();

    let original_umask = unsafe { umask(0) };
    let tests = vec![TestCase {
        args: vec!["-r,a+w", TEST_FILE],
        before: 0o100444,
        after: 0o100222,
    }];
    run_tests(tests);
    unsafe {
        umask(original_umask);
    }
}

#[test]
fn test_chmod_reference_file() {
    let tests = vec![
        TestCase {
            args: vec!["--reference", REFERENCE_FILE, TEST_FILE],
            before: 0o100070,
            after: 0o100247,
        },
        TestCase {
            args: vec!["a-w", "--reference", REFERENCE_FILE, TEST_FILE],
            before: 0o100070,
            after: 0o100247,
        },
    ];
    let (at, ucmd) = at_and_ucmd!();
    mkfile(&at.plus_as_string(REFERENCE_FILE), REFERENCE_PERMS);
    run_single_test(&tests[0], at, ucmd);
}

#[test]
fn test_chmod_recursive() {
    let _guard = UMASK_MUTEX.lock();

    let original_umask = unsafe { umask(0) };
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("a");
    at.mkdir("a/b");
    at.mkdir("a/b/c");
    at.mkdir("z");
    mkfile(&at.plus_as_string("a/a"), 0o100444);
    mkfile(&at.plus_as_string("a/b/b"), 0o100444);
    mkfile(&at.plus_as_string("a/b/c/c"), 0o100444);
    mkfile(&at.plus_as_string("z/y"), 0o100444);

    let result = ucmd
        .arg("-R")
        .arg("--verbose")
        .arg("-r,a+w")
        .arg("a")
        .arg("z")
        .succeeds();

    assert_eq!(at.metadata("z/y").permissions().mode(), 0o100222);
    assert_eq!(at.metadata("a/a").permissions().mode(), 0o100222);
    assert_eq!(at.metadata("a/b/b").permissions().mode(), 0o100222);
    assert_eq!(at.metadata("a/b/c/c").permissions().mode(), 0o100222);
    println!("mode {:o}", at.metadata("a").permissions().mode());
    assert_eq!(at.metadata("a").permissions().mode(), 0o40333);
    assert_eq!(at.metadata("z").permissions().mode(), 0o40333);
    assert!(result.stderr.contains("to 333 (-wx-wx-wx)"));
    assert!(result.stderr.contains("to 222 (-w--w--w-)"));

    unsafe {
        umask(original_umask);
    }
}

#[test]
fn test_chmod_non_existing_file() {
    let (_at, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .arg("-R")
        .arg("--verbose")
        .arg("-r,a+w")
        .arg("dont-exist")
        .fails();
    assert!(result
        .stderr
        .contains("cannot access 'dont-exist': No such file or directory"));
}

#[test]
fn test_chmod_preserve_root() {
    let (_at, mut ucmd) = at_and_ucmd!();
    let result = ucmd
        .arg("-R")
        .arg("--preserve-root")
        .arg("755")
        .arg("/")
        .fails();
    assert!(result
        .stderr
        .contains("chmod: error: it is dangerous to operate recursively on '/'"));
}

#[test]
fn test_chmod_symlink_non_existing_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let non_existing = "test_chmod_symlink_non_existing_file";
    let test_symlink = "test_chmod_symlink_non_existing_file_symlink";
    let expected_stdout = &format!(
        "failed to change mode of '{}' from 0000 (---------) to 0000 (---------)",
        test_symlink
    );
    let expected_stderr = &format!("cannot operate on dangling symlink '{}'", test_symlink);

    at.symlink_file(non_existing, test_symlink);
    let mut result;

    // this cannot succeed since the symbolic link dangles
    result = scene.ucmd().arg("755").arg("-v").arg(test_symlink).fails();

    println!("stdout = {:?}", result.stdout);
    println!("stderr = {:?}", result.stderr);

    assert!(result.stdout.contains(expected_stdout));
    assert!(result.stderr.contains(expected_stderr));
    assert_eq!(result.code, Some(1));

    // this should be the same than with just '-v' but without stderr
    result = scene
        .ucmd()
        .arg("755")
        .arg("-v")
        .arg("-f")
        .arg(test_symlink)
        .fails();

    println!("stdout = {:?}", result.stdout);
    println!("stderr = {:?}", result.stderr);

    assert!(result.stdout.contains(expected_stdout));
    assert!(result.stderr.is_empty());
    assert_eq!(result.code, Some(1));
}

#[test]
fn test_chmod_symlink_non_existing_file_recursive() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let non_existing = "test_chmod_symlink_non_existing_file_recursive";
    let test_symlink = "test_chmod_symlink_non_existing_file_recursive_symlink";
    let test_directory = "test_chmod_symlink_non_existing_file_directory";

    at.mkdir(test_directory);
    at.symlink_file(
        non_existing,
        &format!("{}/{}", test_directory, test_symlink),
    );
    let mut result;

    // this should succeed
    result = scene
        .ucmd()
        .arg("-R")
        .arg("755")
        .arg(test_directory)
        .succeeds();
    assert_eq!(result.code, Some(0));
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());

    let expected_stdout = &format!(
        "mode of '{}' retained as 0755 (rwxr-xr-x)\nneither symbolic link '{}/{}' nor referent has been changed",
        test_directory, test_directory, test_symlink
    );

    // '-v': this should succeed without stderr
    result = scene
        .ucmd()
        .arg("-R")
        .arg("-v")
        .arg("755")
        .arg(test_directory)
        .succeeds();

    println!("stdout = {:?}", result.stdout);
    println!("stderr = {:?}", result.stderr);

    assert!(result.stdout.contains(expected_stdout));
    assert!(result.stderr.is_empty());
    assert_eq!(result.code, Some(0));

    // '-vf': this should be the same than with just '-v'
    result = scene
        .ucmd()
        .arg("-R")
        .arg("-v")
        .arg("-f")
        .arg("755")
        .arg(test_directory)
        .succeeds();

    println!("stdout = {:?}", result.stdout);
    println!("stderr = {:?}", result.stderr);

    assert!(result.stdout.contains(expected_stdout));
    assert!(result.stderr.is_empty());
    assert_eq!(result.code, Some(0));
}

#[test]
fn test_chmod_strip_minus_from_mode() {
    let tests = vec![
        // ( before, after )
        ("chmod -v -xw -R FILE", "chmod -v xw -R FILE"),
        ("chmod g=rwx FILE -c", "chmod g=rwx FILE -c"),
        (
            "chmod -c -R -w,o+w FILE --preserve-root",
            "chmod -c -R w,o+w FILE --preserve-root",
        ),
        ("chmod -c -R +w FILE ", "chmod -c -R +w FILE "),
        ("chmod a=r,=xX FILE", "chmod a=r,=xX FILE"),
        (
            "chmod -v --reference RFILE -R FILE",
            "chmod -v --reference RFILE -R FILE",
        ),
        ("chmod -Rvc -w-x FILE", "chmod -Rvc w-x FILE"),
        ("chmod 755 -v FILE", "chmod 755 -v FILE"),
        ("chmod -v +0004 FILE -R", "chmod -v +0004 FILE -R"),
        ("chmod -v -0007 FILE -R", "chmod -v 0007 FILE -R"),
    ];

    for test in tests {
        let mut args: Vec<String> = test.0.split(" ").map(|v| v.to_string()).collect();
        let _mode_had_minus_prefix = strip_minus_from_mode(&mut args);
        assert_eq!(test.1, args.join(" "));
    }
}
