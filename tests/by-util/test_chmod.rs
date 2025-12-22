// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) dirfd subdirs openat FDCWD

use std::fs::{OpenOptions, Permissions, metadata, set_permissions};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use uutests::at_and_ucmd;
use uutests::util::{AtPath, TestScenario, UCommand};

use uutests::new_ucmd;
use uutests::util_name;

static TEST_FILE: &str = "file";
static REFERENCE_FILE: &str = "reference";
static REFERENCE_PERMS: u32 = 0o247;

struct TestCase {
    args: Vec<&'static str>,
    before: u32,
    after: u32,
    umask: Option<libc::mode_t>,
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

    assert_eq!(
        perms, test.before,
        "{}: expected: {:o} got: {perms:o}",
        "setting permissions on test files before actual test run failed", test.after
    );

    for arg in &test.args {
        ucmd.arg(arg);
        if let Some(umask) = test.umask {
            ucmd.umask(umask);
        }
    }
    let r = ucmd.run();
    if !r.succeeded() {
        println!("{}", r.stderr_str());
        panic!("{ucmd}: failed");
    }

    let perms = at.metadata(TEST_FILE).permissions().mode();
    assert_eq!(
        perms, test.after,
        "{ucmd}: expected: {:o} got: {perms:o}",
        test.after
    );
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
            umask: None,
        },
        TestCase {
            args: vec!["0070", TEST_FILE],
            before: 0o100000,
            after: 0o100070,
            umask: None,
        },
        TestCase {
            args: vec!["0007", TEST_FILE],
            before: 0o100000,
            after: 0o100007,
            umask: None,
        },
        TestCase {
            args: vec!["-0700", TEST_FILE],
            before: 0o100700,
            after: 0o100000,
            umask: None,
        },
        TestCase {
            args: vec!["-0070", TEST_FILE],
            before: 0o100060,
            after: 0o100000,
            umask: None,
        },
        TestCase {
            args: vec!["-0007", TEST_FILE],
            before: 0o100001,
            after: 0o100000,
            umask: None,
        },
        TestCase {
            args: vec!["+0100", TEST_FILE],
            before: 0o100600,
            after: 0o100700,
            umask: None,
        },
        TestCase {
            args: vec!["+0020", TEST_FILE],
            before: 0o100050,
            after: 0o100070,
            umask: None,
        },
        TestCase {
            args: vec!["+0004", TEST_FILE],
            before: 0o100003,
            after: 0o100007,
            umask: None,
        },
    ];
    run_tests(tests);
}

#[test]
#[allow(clippy::unreadable_literal)]
#[allow(clippy::too_many_lines)]
// spell-checker:disable-next-line
fn test_chmod_ugoa() {
    let tests = vec![
        TestCase {
            args: vec!["u=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100700,
            umask: Some(0),
        },
        TestCase {
            args: vec!["g=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100070,
            umask: Some(0),
        },
        TestCase {
            args: vec!["o=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100007,
            umask: Some(0),
        },
        TestCase {
            args: vec!["a=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100777,
            umask: Some(0),
        },
        TestCase {
            args: vec!["-r", TEST_FILE],
            before: 0o100777,
            after: 0o100333,
            umask: Some(0),
        },
        TestCase {
            args: vec!["-w", TEST_FILE],
            before: 0o100777,
            after: 0o100555,
            umask: Some(0),
        },
        TestCase {
            args: vec!["-x", TEST_FILE],
            before: 0o100777,
            after: 0o100666,
            umask: Some(0),
        },
    ];
    run_tests(tests);

    let tests = vec![
        TestCase {
            args: vec!["u=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100700,
            umask: Some(0o022),
        },
        TestCase {
            args: vec!["g=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100070,
            umask: Some(0o022),
        },
        TestCase {
            args: vec!["o=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100007,
            umask: Some(0o022),
        },
        TestCase {
            args: vec!["a=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100777,
            umask: Some(0o022),
        },
        TestCase {
            args: vec!["+rw", TEST_FILE],
            before: 0o100000,
            after: 0o100644,
            umask: Some(0o022),
        },
        TestCase {
            args: vec!["=rwx", TEST_FILE],
            before: 0o100000,
            after: 0o100755,
            umask: Some(0o022),
        },
        TestCase {
            args: vec!["-x", TEST_FILE],
            before: 0o100777,
            after: 0o100666,
            umask: Some(0o022),
        },
    ];
    run_tests(tests);
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "android"))]
#[allow(clippy::cast_lossless)]
fn test_chmod_umask_expected() {
    // Get the actual system umask using libc
    let system_umask = unsafe {
        let mask = libc::umask(0);
        libc::umask(mask);
        mask
    };

    // Now verify that get_umask() returns the same value
    let current_umask = uucore::mode::get_umask();
    assert_eq!(
        current_umask, system_umask as u32,
        "get_umask() returned {current_umask:03o}, but system umask is {system_umask:03o}",
    );
}

fn get_expected_symlink_permissions() -> u32 {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        0o120_777
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        0o120_755
    }
}

#[test]
fn test_chmod_error_permissions() {
    // check that we print an error if umask prevents us from removing a permission
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("file");
    set_permissions(at.plus("file"), Permissions::from_mode(0o777)).unwrap();

    ucmd.args(&["-w", "file"])
        .umask(0o022)
        .fails_with_code(1)
        .stderr_is(
            // spell-checker:disable-next-line
            "chmod: file: new permissions are r-xrwxrwx, not r-xr-xr-x\n",
        );
    assert_eq!(
        metadata(at.plus("file")).unwrap().permissions().mode(),
        0o100_577
    );
}

#[test]
#[allow(clippy::unreadable_literal)]
fn test_chmod_ugo_copy() {
    let tests = vec![
        TestCase {
            args: vec!["u=g", TEST_FILE],
            before: 0o100070,
            after: 0o100770,
            umask: None,
        },
        TestCase {
            args: vec!["g=o", TEST_FILE],
            before: 0o100005,
            after: 0o100055,
            umask: None,
        },
        TestCase {
            args: vec!["o=u", TEST_FILE],
            before: 0o100200,
            after: 0o100202,
            umask: None,
        },
        TestCase {
            args: vec!["u-g", TEST_FILE],
            before: 0o100710,
            after: 0o100610,
            umask: None,
        },
        TestCase {
            args: vec!["u+g", TEST_FILE],
            before: 0o100250,
            after: 0o100750,
            umask: None,
        },
    ];
    run_tests(tests);
}

#[test]
#[allow(clippy::unreadable_literal)]
fn test_chmod_many_options() {
    let tests = vec![TestCase {
        args: vec!["-r,a+w", TEST_FILE],
        before: 0o100444,
        after: 0o100222,
        umask: Some(0),
    }];
    run_tests(tests);
}

#[test]
#[allow(clippy::unreadable_literal)]
fn test_chmod_reference_file() {
    let tests = [
        TestCase {
            args: vec!["--reference", REFERENCE_FILE, TEST_FILE],
            before: 0o100070,
            after: 0o100247,
            umask: None,
        },
        TestCase {
            args: vec!["a-w", "--reference", REFERENCE_FILE, TEST_FILE],
            before: 0o100070,
            after: 0o100247,
            umask: None,
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
        .stderr_is("chmod: 'd/no-x/y': Permission denied\n");
}

#[test]
#[allow(clippy::unreadable_literal)]
fn test_chmod_recursive() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("a");
    at.mkdir("a/b");
    at.mkdir("a/b/c");
    at.mkdir("z");

    // create expected permissions by removing read bits and write bits to the current perms
    let a_perms_expected = (at.metadata("a").permissions().mode() & !0o444) | 0o222;
    let z_perms_expected = (at.metadata("z").permissions().mode() & !0o444) | 0o222;

    make_file(&at.plus_as_string("a/a"), 0o100444);
    make_file(&at.plus_as_string("a/b/b"), 0o100444);
    make_file(&at.plus_as_string("a/b/c/c"), 0o100444);
    make_file(&at.plus_as_string("z/y"), 0o100444);
    #[cfg(not(target_os = "linux"))]
    let err_msg = "chmod: Permission denied\n";
    #[cfg(target_os = "linux")]
    let err_msg = "chmod: 'z': Permission denied\n";

    // only the permissions of folder `a` and `z` are changed
    // folder can't be read after read permission is removed
    ucmd.arg("-R")
        .arg("--verbose")
        .arg("-r,a+w")
        .arg("a")
        .arg("z")
        .umask(0)
        .fails()
        .stderr_is(err_msg);

    assert_eq!(at.metadata("z/y").permissions().mode(), 0o100444);
    assert_eq!(at.metadata("a/a").permissions().mode(), 0o100444);
    assert_eq!(at.metadata("a/b/b").permissions().mode(), 0o100444);
    assert_eq!(at.metadata("a/b/c/c").permissions().mode(), 0o100444);
    println!("mode {:o}", at.metadata("a").permissions().mode());
    assert_eq!(at.metadata("a").permissions().mode(), a_perms_expected);
    assert_eq!(at.metadata("z").permissions().mode(), z_perms_expected);
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
        .stderr_contains("cannot access 'does-not-exist': No such file or directory");
}

#[test]
fn test_chmod_non_existing_file_silent() {
    new_ucmd!()
        .arg("-R")
        .arg("--quiet")
        .arg("-r,a+w")
        .arg("does-not-exist")
        .fails_with_code(1)
        .no_stderr();
}

#[test]
fn test_chmod_preserve_root() {
    new_ucmd!()
        .arg("-R")
        .arg("--preserve-root")
        .arg("755")
        .arg("/")
        .fails_with_code(1)
        .stderr_contains("chmod: it is dangerous to operate recursively on '/'");
}

#[test]
fn test_chmod_symlink_non_existing_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let non_existing = "test_chmod_symlink_non_existing_file";
    let test_symlink = "test_chmod_symlink_non_existing_file_symlink";
    let expected_stdout = &format!(
        "failed to change mode of '{test_symlink}' from 0000 (---------) to 1500 (r-x-----T)"
    );
    let expected_stderr = &format!("cannot operate on dangling symlink '{test_symlink}'");

    at.symlink_file(non_existing, test_symlink);

    // this cannot succeed since the symbolic link dangles
    scene
        .ucmd()
        .arg("755")
        .arg("-v")
        .arg(test_symlink)
        .fails_with_code(1)
        .stdout_contains(expected_stdout)
        .stderr_contains(expected_stderr);

    // this should be the same than with just '-v' but without stderr
    scene
        .ucmd()
        .arg("755")
        .arg("-v")
        .arg("-f")
        .arg(test_symlink)
        .fails_with_code(1)
        .no_stderr()
        .stdout_contains(expected_stdout);

    // this should only include  the dangling symlink message
    // NOT the failure to change mode
    scene
        .ucmd()
        .arg("755")
        .arg(test_symlink)
        .fails_with_code(1)
        .no_stdout()
        .stderr_contains(expected_stderr);
}

#[test]
fn test_chmod_symlink_non_existing_file_recursive() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let non_existing = "test_chmod_symlink_non_existing_file_recursive";
    let test_symlink = "test_chmod_symlink_non_existing_file_recursive_symlink";
    let test_directory = "test_chmod_symlink_non_existing_file_directory";

    at.mkdir(test_directory);
    at.symlink_file(non_existing, &format!("{test_directory}/{test_symlink}"));

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
        "mode of '{test_directory}' retained as 0755 (rwxr-xr-x)"
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
        .fails_with_code(1)
        .usage_error("missing operand");
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
#[cfg(not(target_os = "android"))]
fn test_mode_after_dash_dash() {
    let (at, ucmd) = at_and_ucmd!();
    run_single_test(
        &TestCase {
            args: vec!["--", "-r", TEST_FILE],
            before: 0o100_777,
            after: 0o100_333,
            umask: None,
        },
        &at,
        ucmd,
    );
}

#[test]
fn test_chmod_file_after_non_existing_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(TEST_FILE);
    at.touch("file2");
    set_permissions(at.plus(TEST_FILE), Permissions::from_mode(0o664)).unwrap();
    set_permissions(at.plus("file2"), Permissions::from_mode(0o664)).unwrap();
    scene
        .ucmd()
        .arg("u+x")
        .arg("does-not-exist")
        .arg(TEST_FILE)
        .fails_with_code(1)
        .stderr_contains("chmod: cannot access 'does-not-exist': No such file or directory");

    assert_eq!(at.metadata(TEST_FILE).permissions().mode(), 0o100_764);

    scene
        .ucmd()
        .arg("u+x")
        .arg("--q")
        .arg("does-not-exist")
        .arg("file2")
        .fails_with_code(1)
        .no_stderr();
    assert_eq!(at.metadata("file2").permissions().mode(), 0o100_764);
}

#[test]
fn test_chmod_file_symlink_after_non_existing_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let existing = "file";
    let test_existing_symlink = "file_symlink";

    let non_existing = "test_chmod_symlink_non_existing_file";
    let test_dangling_symlink = "test_chmod_symlink_non_existing_file_symlink";
    let expected_stdout = &format!(
        "failed to change mode of '{test_dangling_symlink}' from 0000 (---------) to 1500 (r-x-----T)"
    );
    let expected_stderr = &format!("cannot operate on dangling symlink '{test_dangling_symlink}'");

    at.touch(existing);
    set_permissions(at.plus(existing), Permissions::from_mode(0o664)).unwrap();
    at.symlink_file(non_existing, test_dangling_symlink);
    at.symlink_file(existing, test_existing_symlink);

    // this cannot succeed since the symbolic link dangles
    // but the metadata for the existing target should change
    scene
        .ucmd()
        .arg("u+x")
        .arg("-v")
        .arg(test_dangling_symlink)
        .arg(test_existing_symlink)
        .fails_with_code(1)
        .stdout_contains(expected_stdout)
        .stderr_contains(expected_stderr);
    assert_eq!(
        at.metadata(test_existing_symlink).permissions().mode(),
        0o100_764,
        "Expected mode: {:o}, but got: {:o}",
        0o100_764,
        at.metadata(test_existing_symlink).permissions().mode()
    );
}

#[test]
fn test_quiet_n_verbose_used_multiple_times() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("file");
    scene
        .ucmd()
        .arg("u+x")
        .arg("--verbose")
        .arg("--verbose")
        .arg("file")
        .succeeds();
    scene
        .ucmd()
        .arg("u+x")
        .arg("--quiet")
        .arg("--quiet")
        .arg("file")
        .succeeds();
}

#[test]
fn test_changes_from_identical_reference() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("file");
    scene
        .ucmd()
        .arg("-c")
        .arg("--reference=file")
        .arg("file")
        .succeeds()
        .no_stdout();
}

#[test]
fn test_gnu_invalid_mode() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("file");
    scene.ucmd().arg("u+gr").arg("file").fails();
}

#[test]
#[cfg(not(target_os = "android"))]
fn test_gnu_options() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("file");
    scene.ucmd().arg("-w").arg("file").succeeds();
    scene.ucmd().arg("file").arg("-w").succeeds();
    scene.ucmd().arg("-w").arg("--").arg("file").succeeds();
}

#[test]
fn test_gnu_repeating_options() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("file");
    scene.ucmd().arg("-w").arg("-w").arg("file").succeeds();
    scene
        .ucmd()
        .arg("-w")
        .arg("-w")
        .arg("-w")
        .arg("file")
        .succeeds();
}

#[test]
fn test_gnu_special_filenames() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let perms_before = Permissions::from_mode(0o100_640);
    let perms_after = Permissions::from_mode(0o100_440);

    make_file(&at.plus_as_string("--"), perms_before.mode());
    scene.ucmd().arg("-w").arg("--").arg("--").succeeds();
    assert_eq!(at.metadata("--").permissions(), perms_after);
    set_permissions(at.plus("--"), perms_before.clone()).unwrap();
    scene.ucmd().arg("--").arg("-w").arg("--").succeeds();
    assert_eq!(at.metadata("--").permissions(), perms_after);
    at.remove("--");

    make_file(&at.plus_as_string("-w"), perms_before.mode());
    scene.ucmd().arg("-w").arg("--").arg("-w").succeeds();
    assert_eq!(at.metadata("-w").permissions(), perms_after);
    set_permissions(at.plus("-w"), perms_before).unwrap();
    scene.ucmd().arg("--").arg("-w").arg("-w").succeeds();
    assert_eq!(at.metadata("-w").permissions(), perms_after);
}

#[test]
fn test_gnu_special_options() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("file");
    scene.ucmd().arg("--").arg("--").arg("file").succeeds();
    scene.ucmd().arg("--").arg("--").fails();
}

#[test]
fn test_chmod_dereference_symlink() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let target = "file";
    let symlink = "symlink";

    at.touch(target);
    set_permissions(at.plus(target), Permissions::from_mode(0o664)).unwrap();
    at.symlink_file(target, symlink);

    // Use --dereference: should modify the target file's permissions
    scene
        .ucmd()
        .arg("--dereference")
        .arg("u+x")
        .arg(symlink)
        .succeeds()
        .no_stderr();
    assert_eq!(at.metadata(target).permissions().mode(), 0o100_764);
    assert_eq!(
        at.symlink_metadata(symlink).permissions().mode(),
        get_expected_symlink_permissions()
    );
}

#[test]
fn test_chmod_no_dereference_symlink() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let target = "file";
    let symlink = "symlink";

    at.touch(target);
    set_permissions(at.plus(target), Permissions::from_mode(0o664)).unwrap();
    at.symlink_file(target, symlink);

    scene
        .ucmd()
        .arg("--no-dereference")
        .arg("u+x")
        .arg(symlink)
        .succeeds()
        .no_stderr();
    assert_eq!(at.metadata(target).permissions().mode(), 0o100_664);
    assert_eq!(
        at.symlink_metadata(symlink).permissions().mode(),
        get_expected_symlink_permissions()
    );
}

#[test]
fn test_chmod_symlink_to_dangling_target_dereference() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let dangling_target = "nonexistent_file";
    let symlink = "symlink";

    at.symlink_file(dangling_target, symlink);

    // Use --dereference: should fail due to dangling symlink
    scene
        .ucmd()
        .arg("--dereference")
        .arg("u+x")
        .arg(symlink)
        .fails()
        .stderr_contains(format!("cannot operate on dangling symlink '{symlink}'"));
}

#[test]
fn test_chmod_symlink_target_no_dereference() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file = "a";
    let symlink = "symlink";
    at.touch(file);
    at.symlink_file(file, symlink);
    set_permissions(at.plus(file), Permissions::from_mode(0o644)).unwrap();

    scene
        .ucmd()
        .arg("--no-dereference")
        .arg("755")
        .arg(symlink)
        .succeeds()
        .no_stderr();
    assert_eq!(
        at.symlink_metadata(file).permissions().mode(),
        0o100_644,
        "Expected symlink permissions: {:o}, but got: {:o}",
        0o100_644,
        at.symlink_metadata(file).permissions().mode()
    );
}

#[test]
fn test_chmod_symlink_recursive_final_traversal_flag() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let dangling_target = "nonexistent_file";
    let symlink = "symlink";

    at.symlink_file(dangling_target, symlink);

    scene
        .ucmd()
        .arg("755")
        .arg("-R")
        .arg("-H")
        .arg("-L")
        .arg("-H")
        .arg("-L")
        .arg("-P")
        .arg(symlink)
        .succeeds()
        .no_output();
    assert_eq!(
        at.symlink_metadata(symlink).permissions().mode(),
        get_expected_symlink_permissions(),
        "Expected symlink permissions: {:o}, but got: {:o}",
        get_expected_symlink_permissions(),
        at.symlink_metadata(symlink).permissions().mode()
    );
}

#[test]
fn test_chmod_symlink_to_dangling_recursive_no_traverse() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let dangling_target = "nonexistent_file";
    let symlink = "symlink";

    at.symlink_file(dangling_target, symlink);

    scene
        .ucmd()
        .arg("755")
        .arg("-R")
        .arg("-P")
        .arg(symlink)
        .succeeds()
        .no_output();
    assert_eq!(
        at.symlink_metadata(symlink).permissions().mode(),
        get_expected_symlink_permissions(),
        "Expected symlink permissions: {:o}, but got: {:o}",
        get_expected_symlink_permissions(),
        at.symlink_metadata(symlink).permissions().mode()
    );
}

#[test]
fn test_chmod_dangling_symlink_recursive_combos() {
    let error_scenarios = [vec!["-R"], vec!["-R", "-H"], vec!["-R", "-L"]];

    for flags in error_scenarios {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        let dangling_target = "nonexistent_file";
        let symlink = "symlink";

        at.symlink_file(dangling_target, symlink);

        let mut ucmd = scene.ucmd();
        ucmd.args(&flags)
            .arg("u+x")
            .umask(0o022)
            .arg(symlink)
            .fails()
            .stderr_is("chmod: cannot operate on dangling symlink 'symlink'\n");
        assert_eq!(
            at.symlink_metadata(symlink).permissions().mode(),
            get_expected_symlink_permissions(),
            "Expected symlink permissions: {:o}, but got: {:o}",
            get_expected_symlink_permissions(),
            at.symlink_metadata(symlink).permissions().mode()
        );
    }
}

#[test]
fn test_chmod_traverse_symlink_combo() {
    let scenarios = [
        (
            vec!["-R"], // Should default to "-H"
            0o100_664,
            get_expected_symlink_permissions(),
        ),
        (
            vec!["-R", "-H"],
            0o100_664,
            get_expected_symlink_permissions(),
        ),
        (
            vec!["-R", "-L"],
            0o100_764,
            get_expected_symlink_permissions(),
        ),
        (
            vec!["-R", "-P"],
            0o100_664,
            get_expected_symlink_permissions(),
        ),
    ];

    for (flags, expected_target_perms, expected_symlink_perms) in scenarios {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        let directory = "dir";
        let target = "file";
        let symlink = "symlink";

        at.mkdir(directory);
        at.touch(target);
        at.symlink_file(target, &format!("{directory}/{symlink}"));

        set_permissions(at.plus(target), Permissions::from_mode(0o664)).unwrap();

        let mut ucmd = scene.ucmd();
        ucmd.args(&flags)
            .arg("u+x")
            .umask(0o022)
            .arg(directory)
            .succeeds()
            .no_stderr();

        let actual_target = at.metadata(target).permissions().mode();
        assert_eq!(
            actual_target, expected_target_perms,
            "For flags {flags:?}, expected target perms = {expected_target_perms:o}, got = {actual_target:o}",
        );

        let actual_symlink = at
            .symlink_metadata(&format!("{directory}/{symlink}"))
            .permissions()
            .mode();
        assert_eq!(
            actual_symlink, expected_symlink_perms,
            "For flags {flags:?}, expected symlink perms = {expected_symlink_perms:o}, got = {actual_symlink:o}",
        );
    }
}

#[test]
fn test_chmod_recursive_symlink_to_directory_command_line() {
    // Test behavior when the symlink itself is a command-line argument
    let scenarios = [
        (vec!["-R"], true), // Default behavior (-H): follow symlinks that are command line args
        (vec!["-R", "-H"], true), // Explicit -H: follow symlinks that are command line args
        (vec!["-R", "-L"], true), // -L: follow all symlinks
        (vec!["-R", "-P"], false), // -P: never follow symlinks
    ];

    for (flags, should_follow_symlink_dir) in scenarios {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        let target_dir = "target_dir";
        let symlink_to_dir = "link_dir";
        let file_in_target = "file_in_target";

        at.mkdir(target_dir);
        at.touch(format!("{target_dir}/{file_in_target}"));
        at.symlink_dir(target_dir, symlink_to_dir);

        set_permissions(
            at.plus(format!("{target_dir}/{file_in_target}")),
            Permissions::from_mode(0o644),
        )
        .unwrap();

        let mut ucmd = scene.ucmd();
        ucmd.args(&flags)
            .arg("go-rwx")
            .arg(symlink_to_dir) // The symlink itself is the command-line argument
            .succeeds()
            .no_stderr();

        let actual_file_perms = at
            .metadata(&format!("{target_dir}/{file_in_target}"))
            .permissions()
            .mode();

        if should_follow_symlink_dir {
            // When following symlinks, the file inside the target directory should have its permissions changed
            assert_eq!(
                actual_file_perms, 0o100_600,
                "For flags {flags:?}, expected file perms when following symlinks = 600, got = {actual_file_perms:o}",
            );
        } else {
            // When not following symlinks, the file inside the target directory should be unchanged
            assert_eq!(
                actual_file_perms, 0o100_644,
                "For flags {flags:?}, expected file perms when not following symlinks = 644, got = {actual_file_perms:o}",
            );
        }
    }
}

#[test]
fn test_chmod_recursive_symlink_during_traversal() {
    // Test behavior when symlinks are encountered during directory traversal
    let scenarios = [
        (vec!["-R"], false), // Default behavior (-H): don't follow symlinks encountered during traversal
        (vec!["-R", "-H"], false), // Explicit -H: don't follow symlinks encountered during traversal
        (vec!["-R", "-L"], true),  // -L: follow all symlinks including those found during traversal
        (vec!["-R", "-P"], false), // -P: never follow symlinks
    ];

    for (flags, should_follow_symlink_dir) in scenarios {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        let directory = "dir";
        let target_dir = "target_dir";
        let symlink_to_dir = "link_dir";
        let file_in_target = "file_in_target";

        at.mkdir(directory);
        at.mkdir(target_dir);
        at.touch(format!("{target_dir}/{file_in_target}"));
        at.symlink_dir(target_dir, &format!("{directory}/{symlink_to_dir}"));

        set_permissions(
            at.plus(format!("{target_dir}/{file_in_target}")),
            Permissions::from_mode(0o644),
        )
        .unwrap();

        let mut ucmd = scene.ucmd();
        ucmd.args(&flags)
            .arg("go-rwx")
            .arg(directory) // The directory is the command-line argument
            .succeeds()
            .no_stderr();

        let actual_file_perms = at
            .metadata(&format!("{target_dir}/{file_in_target}"))
            .permissions()
            .mode();

        if should_follow_symlink_dir {
            // When following symlinks, the file inside the target directory should have its permissions changed
            assert_eq!(
                actual_file_perms, 0o100_600,
                "For flags {flags:?}, expected file perms when following symlinks = 600, got = {actual_file_perms:o}",
            );
        } else {
            // When not following symlinks, the file inside the target directory should be unchanged
            assert_eq!(
                actual_file_perms, 0o100_644,
                "For flags {flags:?}, expected file perms when not following symlinks = 644, got = {actual_file_perms:o}",
            );
        }
    }
}

#[test]
fn test_chmod_recursive_symlink_combinations() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let directory = "dir";
    let target_dir = "target_dir";
    let target_file = "target_file";
    let symlink_to_dir = "link_dir";
    let symlink_to_file = "link_file";
    let file_in_target = "file";

    at.mkdir(directory);
    at.mkdir(target_dir);
    at.touch(target_file);
    at.touch(format!("{target_dir}/{file_in_target}"));
    at.symlink_dir(target_dir, &format!("{directory}/{symlink_to_dir}"));
    at.symlink_file(target_file, &format!("{directory}/{symlink_to_file}"));

    set_permissions(at.plus(target_file), Permissions::from_mode(0o644)).unwrap();
    set_permissions(
        at.plus(format!("{target_dir}/{file_in_target}")),
        Permissions::from_mode(0o644),
    )
    .unwrap();

    // Test with -R -L (follow all symlinks)
    scene
        .ucmd()
        .arg("-R")
        .arg("-L")
        .arg("go-rwx")
        .arg(directory)
        .succeeds()
        .no_stderr();

    // Both target file and file in target directory should have permissions changed
    assert_eq!(at.metadata(target_file).permissions().mode(), 0o100_600);
    assert_eq!(
        at.metadata(&format!("{target_dir}/{file_in_target}"))
            .permissions()
            .mode(),
        0o100_600
    );
}

#[test]
#[cfg(target_os = "linux")]
fn test_chmod_non_utf8_paths() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Create a file with non-UTF-8 name
    // Using bytes that form an invalid UTF-8 sequence
    let non_utf8_bytes = b"test_\xFF\xFE.txt";
    let non_utf8_name = OsStr::from_bytes(non_utf8_bytes);

    // Create the file using OpenOptions with the non-UTF-8 name
    OpenOptions::new()
        .mode(0o644)
        .create(true)
        .write(true)
        .truncate(true)
        .open(at.plus(non_utf8_name))
        .unwrap();

    // Verify initial permissions
    let initial_perms = metadata(at.plus(non_utf8_name))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(initial_perms & 0o777, 0o644);

    // Test chmod with the non-UTF-8 filename
    scene
        .ucmd()
        .arg("755")
        .arg(non_utf8_name)
        .succeeds()
        .no_stderr();

    // Verify permissions were changed
    let new_perms = metadata(at.plus(non_utf8_name))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(new_perms & 0o777, 0o755);

    // Test with multiple non-UTF-8 files
    let non_utf8_bytes2 = b"file_\xC0\x80.dat";
    let non_utf8_name2 = OsStr::from_bytes(non_utf8_bytes2);

    OpenOptions::new()
        .mode(0o666)
        .create(true)
        .write(true)
        .truncate(true)
        .open(at.plus(non_utf8_name2))
        .unwrap();

    // Change permissions on both files at once
    scene
        .ucmd()
        .arg("644")
        .arg(non_utf8_name)
        .arg(non_utf8_name2)
        .succeeds()
        .no_stderr();

    // Verify both files have the new permissions
    assert_eq!(
        metadata(at.plus(non_utf8_name))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o644
    );
    assert_eq!(
        metadata(at.plus(non_utf8_name2))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o644
    );
}

#[cfg(all(target_os = "linux", feature = "chmod"))]
#[test]
#[ignore = "covered by util/check-safe-traversal.sh"]
fn test_chmod_recursive_uses_dirfd_for_subdirs() {
    use std::process::Command;
    use uutests::get_tests_binary;

    // strace is required; fail fast if it is missing or not runnable
    let output = Command::new("strace")
        .arg("-V")
        .output()
        .expect("strace not found; install strace to run this test");
    assert!(
        output.status.success(),
        "strace -V failed; ensure strace is installed and usable"
    );

    let (at, _ucmd) = at_and_ucmd!();
    at.mkdir("x");
    at.mkdir("x/y");
    at.mkdir("x/y/z");

    let log_path = at.plus_as_string("strace.log");

    let status = Command::new("strace")
        .arg("-e")
        .arg("openat")
        .arg("-o")
        .arg(&log_path)
        .arg(get_tests_binary!())
        .args(["chmod", "-R", "+x", "x"])
        .current_dir(&at.subdir)
        .status()
        .expect("failed to run strace");
    assert!(status.success(), "strace run failed");

    let log = at.read("strace.log");

    // Regression guard: ensure recursion uses dirfd-relative openat instead of AT_FDCWD with a multi-component path
    assert!(
        !log.contains("openat(AT_FDCWD, \"x/y"),
        "chmod recursed using AT_FDCWD with a multi-component path; expected dirfd-relative openat"
    );
}

#[test]
fn test_chmod_colored_output() {
    // Test colored help message
    new_ucmd!()
        .arg("--help")
        .env("CLICOLOR_FORCE", "1")
        .env("LANG", "en_US.UTF-8")
        .succeeds()
        .stdout_contains("\x1b[1m\x1b[4mUsage:\x1b[0m") // Bold+underline "Usage:"
        .stdout_contains("\x1b[1m\x1b[4mArguments:\x1b[0m"); // Bold+underline "Arguments:"

    // Test colored error message for invalid option
    new_ucmd!()
        .arg("--invalid-option")
        .env("CLICOLOR_FORCE", "1")
        .env("LANG", "en_US.UTF-8")
        .fails()
        .code_is(1)
        .stderr_contains("\x1b[31merror\x1b[0m") // Red "error"
        .stderr_contains("\x1b[33m--invalid-option\x1b[0m"); // Yellow invalid option

    // Test French localized colored error message
    new_ucmd!()
        .arg("--invalid-option")
        .env("CLICOLOR_FORCE", "1")
        .env("LANG", "fr_FR.UTF-8")
        .fails()
        .code_is(1)
        .stderr_contains("\x1b[31merreur\x1b[0m") // Red "erreur" in French
        .stderr_contains("\x1b[33m--invalid-option\x1b[0m"); // Yellow invalid option
}
