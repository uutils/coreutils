use crate::common::util::*;
use std::fs::{metadata, set_permissions, OpenOptions, Permissions};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::sync::Mutex;

extern crate libc;
use uucore::mode::strip_minus_from_mode;
extern crate chmod;
use self::libc::umask;

static TEST_FILE: &str = "file";
static REFERENCE_FILE: &str = "reference";
static REFERENCE_PERMS: u32 = 0o247;
lazy_static! {
    static ref UMASK_MUTEX: Mutex<()> = Mutex::new(());
}

struct TestCase {
    args: Vec<&'static str>,
    before: u32,
    after: u32,
}

fn make_file(file: &str, mode: u32) {
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

fn run_single_test(test: &TestCase, at: &AtPath, mut ucmd: UCommand) {
    make_file(&at.plus_as_string(TEST_FILE), test.before);
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
    if !r.succeeded() {
        println!("{}", r.stderr_str());
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
        run_single_test(&test, &at, ucmd);
    }
}

#[test]
#[allow(clippy::unreadable_literal)]
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
#[allow(clippy::unreadable_literal)]
// spell-checker:disable-next-line
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
            args: vec!["-x", TEST_FILE],
            before: 0o100777,
            after: 0o100666,
        },
    ];
    run_tests(tests);

    // check that we print an error if umask prevents us from removing a permission
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file");
    set_permissions(at.plus("file"), Permissions::from_mode(0o777)).unwrap();
    ucmd.args(&["-w", "file"])
        .fails()
        .code_is(1)
        // spell-checker:disable-next-line
        .stderr_is("chmod: file: new permissions are r-xrwxrwx, not r-xr-xr-x");
    assert_eq!(
        metadata(at.plus("file")).unwrap().permissions().mode(),
        0o100577
    );

    unsafe {
        umask(last);
    }
}

#[test]
#[allow(clippy::unreadable_literal)]
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
#[allow(clippy::unreadable_literal)]
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
#[allow(clippy::unreadable_literal)]
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
    make_file(&at.plus_as_string(REFERENCE_FILE), REFERENCE_PERMS);
    run_single_test(&tests[0], &at, ucmd);
}

#[test]
fn test_permission_denied() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("d/");
    at.mkdir("d/no-x");
    at.mkdir("d/no-x/y");

    scene.ucmd().arg("u=rw").arg("d/no-x").succeeds();

    scene
        .ucmd()
        .arg("-R")
        .arg("o=r")
        .arg("d")
        .fails()
        .stderr_is("chmod: 'd/no-x/y': Permission denied");
}

#[test]
#[allow(clippy::unreadable_literal)]
fn test_chmod_recursive() {
    let _guard = UMASK_MUTEX.lock();

    let original_umask = unsafe { umask(0) };
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("a");
    at.mkdir("a/b");
    at.mkdir("a/b/c");
    at.mkdir("z");
    make_file(&at.plus_as_string("a/a"), 0o100444);
    make_file(&at.plus_as_string("a/b/b"), 0o100444);
    make_file(&at.plus_as_string("a/b/c/c"), 0o100444);
    make_file(&at.plus_as_string("z/y"), 0o100444);

    // only the permissions of folder `a` and `z` are changed
    // folder can't be read after read permission is removed
    ucmd.arg("-R")
        .arg("--verbose")
        .arg("-r,a+w")
        .arg("a")
        .arg("z")
        .fails()
        .stderr_is("chmod: Permission denied");

    assert_eq!(at.metadata("z/y").permissions().mode(), 0o100444);
    assert_eq!(at.metadata("a/a").permissions().mode(), 0o100444);
    assert_eq!(at.metadata("a/b/b").permissions().mode(), 0o100444);
    assert_eq!(at.metadata("a/b/c/c").permissions().mode(), 0o100444);
    println!("mode {:o}", at.metadata("a").permissions().mode());
    assert_eq!(at.metadata("a").permissions().mode(), 0o40333);
    assert_eq!(at.metadata("z").permissions().mode(), 0o40333);

    unsafe {
        umask(original_umask);
    }
}

#[test]
#[allow(clippy::unreadable_literal)]
fn test_chmod_recursive_read_permission() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("a");
    at.mkdir("a/b");
    let mut perms = at.metadata("a/b").permissions();
    perms.set_mode(0o311);
    set_permissions(at.plus_as_string("a/b"), perms.clone()).unwrap();
    set_permissions(at.plus_as_string("a"), perms).unwrap();

    ucmd.arg("-R").arg("u+r").arg("a").succeeds();

    assert_eq!(at.metadata("a").permissions().mode(), 0o40711);
    assert_eq!(at.metadata("a/b").permissions().mode(), 0o40711);
}

#[test]
fn test_chmod_non_existing_file() {
    new_ucmd!()
        .arg("-R")
        .arg("-r,a+w")
        .arg("does-not-exist")
        .fails()
        .stderr_contains(&"cannot access 'does-not-exist': No such file or directory");
}

#[test]
fn test_chmod_non_existing_file_silent() {
    new_ucmd!()
        .arg("-R")
        .arg("--quiet")
        .arg("-r,a+w")
        .arg("does-not-exist")
        .fails()
        .no_stderr()
        .code_is(1);
}

#[test]
fn test_chmod_preserve_root() {
    new_ucmd!()
        .arg("-R")
        .arg("--preserve-root")
        .arg("755")
        .arg("/")
        .fails()
        .stderr_contains(&"chmod: it is dangerous to operate recursively on '/'");
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

    // this cannot succeed since the symbolic link dangles
    scene
        .ucmd()
        .arg("755")
        .arg("-v")
        .arg(test_symlink)
        .fails()
        .code_is(1)
        .stdout_contains(expected_stdout)
        .stderr_contains(expected_stderr);

    // this should be the same than with just '-v' but without stderr
    scene
        .ucmd()
        .arg("755")
        .arg("-v")
        .arg("-f")
        .arg(test_symlink)
        .run()
        .code_is(1)
        .no_stderr()
        .stdout_contains(expected_stdout);
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

    // this should succeed
    scene
        .ucmd()
        .arg("-R")
        .arg("755")
        .arg(test_directory)
        .succeeds()
        .no_stderr()
        .no_stdout();

    let expected_stdout = &format!(
        // spell-checker:disable-next-line
        "mode of '{}' retained as 0755 (rwxr-xr-x)",
        test_directory
    );

    // '-v': this should succeed without stderr
    scene
        .ucmd()
        .arg("-R")
        .arg("-v")
        .arg("755")
        .arg(test_directory)
        .succeeds()
        .stdout_contains(expected_stdout)
        .no_stderr();

    // '-vf': this should be the same than with just '-v'
    scene
        .ucmd()
        .arg("-R")
        .arg("-v")
        .arg("-f")
        .arg("755")
        .arg(test_directory)
        .succeeds()
        .stdout_contains(expected_stdout)
        .no_stderr();
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
            "chmod -v --reference REF_FILE -R FILE",
            "chmod -v --reference REF_FILE -R FILE",
        ),
        ("chmod -Rvc -w-x FILE", "chmod -Rvc w-x FILE"),
        ("chmod 755 -v FILE", "chmod 755 -v FILE"),
        ("chmod -v +0004 FILE -R", "chmod -v +0004 FILE -R"),
        ("chmod -v -0007 FILE -R", "chmod -v 0007 FILE -R"),
    ];

    for test in tests {
        let mut args: Vec<String> = test.0.split(' ').map(|v| v.to_string()).collect();
        let _mode_had_minus_prefix = strip_minus_from_mode(&mut args);
        assert_eq!(test.1, args.join(" "));
    }
}

#[test]
fn test_chmod_keep_setgid() {
    for (from, arg, to) in [
        (0o7777, "777", 0o46777),
        (0o7777, "=777", 0o40777),
        (0o7777, "0777", 0o46777),
        (0o7777, "=0777", 0o40777),
        (0o7777, "00777", 0o40777),
        (0o2444, "a+wx", 0o42777),
        (0o2444, "a=wx", 0o42333),
        (0o1444, "g+s", 0o43444),
        (0o4444, "u-s", 0o40444),
        (0o7444, "a-s", 0o41444),
    ] {
        let (at, mut ucmd) = at_and_ucmd!();
        at.mkdir("dir");
        set_permissions(at.plus("dir"), Permissions::from_mode(from)).unwrap();
        let r = ucmd.arg(arg).arg("dir").succeeds();
        println!("{}", r.stderr_str());
        assert_eq!(at.metadata("dir").permissions().mode(), to);
    }
}

#[test]
fn test_no_operands() {
    new_ucmd!()
        .arg("777")
        .fails()
        .code_is(1)
        .usage_error("missing operand");
}

#[test]
fn test_mode_after_dash_dash() {
    let (at, ucmd) = at_and_ucmd!();
    run_single_test(
        &TestCase {
            args: vec!["--", "-r", TEST_FILE],
            before: 0o100777,
            after: 0o100333,
        },
        &at,
        ucmd,
    );
}
