// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore nconfined

#[cfg(feature = "feat_selinux")]
use uucore::selinux::get_getfattr_output;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_create_fifo_missing_operand() {
    new_ucmd!().fails().stderr_is("mkfifo: missing operand\n");
}

#[test]
fn test_create_one_fifo() {
    new_ucmd!().arg("abc").succeeds();
}

#[test]
fn test_create_one_fifo_with_invalid_mode() {
    new_ucmd!()
        .arg("abcd")
        .arg("-m")
        .arg("invalid")
        .fails()
        .stderr_contains("invalid mode");

    new_ucmd!()
        .arg("abcd")
        .arg("-m")
        .arg("0999")
        .fails()
        .stderr_contains("invalid mode");
}

#[test]
fn test_create_multiple_fifos() {
    new_ucmd!()
        .arg("abcde")
        .arg("def")
        .arg("sed")
        .arg("dum")
        .succeeds();
}

#[test]
fn test_create_one_fifo_with_mode() {
    new_ucmd!().arg("abcde").arg("-m600").succeeds();
}

#[test]
fn test_create_one_fifo_already_exists() {
    new_ucmd!()
        .arg("abcdef")
        .arg("abcdef")
        .fails()
        .stderr_is("mkfifo: cannot create fifo 'abcdef': File exists\n");
}

#[test]
fn test_create_fifo_with_mode_and_umask() {
    use uucore::fs::display_permissions;
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let test_fifo_creation = |mode: &str, umask: u16, expected: &str| {
        scene
            .ucmd()
            .arg("-m")
            .arg(mode)
            .arg(format!("fifo_test_{mode}"))
            .umask(libc::mode_t::from(umask))
            .succeeds();

        let metadata = std::fs::metadata(at.subdir.join(format!("fifo_test_{mode}"))).unwrap();
        let permissions = display_permissions(&metadata, true);
        assert_eq!(permissions, expected.to_string());
    };

    test_fifo_creation("734", 0o077, "prwx-wxr--"); // spell-checker:disable-line
    test_fifo_creation("706", 0o777, "prwx---rw-"); // spell-checker:disable-line
    test_fifo_creation("a=rwx", 0o022, "prwxrwxrwx"); // spell-checker:disable-line
    test_fifo_creation("a=rx", 0o022, "pr-xr-xr-x"); // spell-checker:disable-line
    test_fifo_creation("a=r", 0o022, "pr--r--r--"); // spell-checker:disable-line
    test_fifo_creation("=rwx", 0o022, "prwxr-xr-x"); // spell-checker:disable-line
    test_fifo_creation("u+w", 0o022, "prw-rw-rw-"); // spell-checker:disable-line
    test_fifo_creation("u-w", 0o022, "pr--rw-rw-"); // spell-checker:disable-line
    test_fifo_creation("u+x", 0o022, "prwxrw-rw-"); // spell-checker:disable-line
    test_fifo_creation("u-r,g-w,o+x", 0o022, "p-w-r--rwx"); // spell-checker:disable-line
    test_fifo_creation("a=rwx,o-w", 0o022, "prwxrwxr-x"); // spell-checker:disable-line
    test_fifo_creation("=rwx,o-w", 0o022, "prwxr-xr-x"); // spell-checker:disable-line
}

#[test]
fn test_create_fifo_with_umask() {
    use uucore::fs::display_permissions;
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let test_fifo_creation = |umask: u16, expected: &str| {
        scene
            .ucmd()
            .arg("fifo_test")
            .umask(libc::mode_t::from(umask))
            .succeeds();

        let metadata = std::fs::metadata(at.subdir.join("fifo_test")).unwrap();
        let permissions = display_permissions(&metadata, true);
        assert_eq!(permissions, expected.to_string());
        at.remove("fifo_test");
    };

    test_fifo_creation(0o022, "prw-r--r--"); // spell-checker:disable-line
    test_fifo_creation(0o777, "p---------"); // spell-checker:disable-line
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_mkfifo_selinux() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let dest = "test_file";
    let args = [
        "-Z",
        "--context",
        "--context=unconfined_u:object_r:user_tmp_t:s0",
    ];
    for arg in args {
        ts.ucmd().arg(arg).arg(dest).succeeds();
        assert!(at.is_fifo("test_file"));

        let context_value = get_getfattr_output(&at.plus_as_string(dest));
        assert!(
            context_value.contains("unconfined_u"),
            "Expected 'unconfined_u' not found in getfattr output:\n{context_value}"
        );
        at.remove(&at.plus_as_string(dest));
    }
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_mkfifo_selinux_invalid() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let dest = "orig";

    let args = [
        "--context=a",
        "--context=unconfined_u:object_r:user_tmp_t:s0:a",
        "--context=nconfined_u:object_r:user_tmp_t:s0",
    ];
    for arg in args {
        new_ucmd!()
            .arg(arg)
            .arg(dest)
            .fails()
            .stderr_contains("failed to");
        if at.file_exists(dest) {
            at.remove(dest);
        }
    }
}
