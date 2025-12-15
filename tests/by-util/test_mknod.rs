// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore nconfined

use std::os::unix::fs::PermissionsExt;

#[cfg(feature = "feat_selinux")]
use uucore::selinux::get_getfattr_output;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util::run_ucmd_as_root;
use uutests::util_name;

#[test]
fn test_mknod_invalid_arg() {
    new_ucmd!()
        .arg("--foo")
        .fails_with_code(1)
        .no_stdout()
        .stderr_contains("unexpected argument '--foo' found");
}

#[test]
fn test_mknod_help() {
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .no_stderr()
        .stdout_contains("Usage:");
}

#[test]
fn test_mknod_version() {
    assert!(
        new_ucmd!()
            .arg("--version")
            .succeeds()
            .no_stderr()
            .stdout_str()
            .starts_with("mknod")
    );
}

#[test]
fn test_mknod_fifo_default_writable() {
    let ts = TestScenario::new(util_name!());
    ts.ucmd().arg("test_file").arg("p").succeeds();
    assert!(ts.fixtures.is_fifo("test_file"));
    assert!(!ts.fixtures.metadata("test_file").permissions().readonly());
}

#[test]
fn test_mknod_fifo_mnemonic_usage() {
    let ts = TestScenario::new(util_name!());
    ts.ucmd().arg("test_file").arg("pipe").succeeds();
    assert!(ts.fixtures.is_fifo("test_file"));
}

#[test]
fn test_mknod_fifo_read_only() {
    let ts = TestScenario::new(util_name!());
    ts.ucmd()
        .arg("-m")
        .arg("a=r")
        .arg("test_file")
        .arg("p")
        .succeeds();
    assert!(ts.fixtures.is_fifo("test_file"));
    assert!(ts.fixtures.metadata("test_file").permissions().readonly());
}

#[test]
fn test_mknod_fifo_invalid_extra_operand() {
    new_ucmd!()
        .arg("test_file")
        .arg("p")
        .arg("1")
        .arg("2")
        .fails()
        .stderr_contains("Fifos do not have major and minor device numbers");
}

#[test]
fn test_mknod_character_device_requires_major_and_minor() {
    new_ucmd!()
        .arg("test_file")
        .arg("c")
        .fails_with_code(1)
        .stderr_contains("Special files require major and minor device numbers.");
    new_ucmd!()
        .arg("test_file")
        .arg("c")
        .arg("1")
        .fails_with_code(1)
        .stderr_contains("Special files require major and minor device numbers.");
    new_ucmd!()
        .arg("test_file")
        .arg("c")
        .arg("1")
        .arg("c")
        .fails()
        .stderr_contains("invalid value 'c'");
    new_ucmd!()
        .arg("test_file")
        .arg("c")
        .arg("c")
        .arg("1")
        .fails()
        .stderr_contains("invalid value 'c'");
}

#[test]
fn test_mknod_invalid_mode() {
    new_ucmd!()
        .arg("--mode")
        .arg("rw")
        .arg("test_file")
        .arg("p")
        .fails()
        .no_stdout()
        .code_is(1)
        .stderr_contains("invalid mode");
}

#[test]
fn test_mknod_mode_permissions() {
    for test_mode in [0o0666, 0o0000, 0o0444, 0o0004, 0o0040, 0o0400, 0o0644] {
        let ts = TestScenario::new(util_name!());
        let filename = format!("null_file-{test_mode:04o}");

        if let Ok(result) = run_ucmd_as_root(
            &ts,
            &[
                "--mode",
                &format!("{test_mode:04o}"),
                &filename,
                "c",
                "1",
                "3",
            ],
        ) {
            result.success().no_stdout();
        } else {
            print!("Test skipped; `mknod c 1 3` for null char dev requires root user");
            break;
        }

        assert!(ts.fixtures.is_char_device(&filename));
        let permissions = ts.fixtures.metadata(&filename).permissions();
        assert_eq!(test_mode, PermissionsExt::mode(&permissions) & 0o777);
    }
}

#[test]
fn test_mknod_mode_comma_separated() {
    let ts = TestScenario::new(util_name!());
    ts.ucmd()
        .arg("-m")
        .arg("u=rwx,g=rx,o=")
        .arg("test_file")
        .arg("p")
        .succeeds();
    assert!(ts.fixtures.is_fifo("test_file"));
    assert_eq!(
        ts.fixtures.metadata("test_file").permissions().mode() & 0o777,
        0o750
    );
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_mknod_selinux() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let dest = "test_file";
    let args = [
        "-Z",
        "--context",
        "--context=unconfined_u:object_r:user_tmp_t:s0",
    ];
    for arg in args {
        ts.ucmd()
            .arg(arg)
            .arg("-m")
            .arg("a=r")
            .arg(dest)
            .arg("p")
            .succeeds();
        assert!(ts.fixtures.is_fifo("test_file"));
        assert!(ts.fixtures.metadata("test_file").permissions().readonly());

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
fn test_mknod_selinux_invalid() {
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
            .arg("-m")
            .arg("a=r")
            .arg(dest)
            .arg("p")
            .fails()
            .stderr_contains("failed to");
        if at.file_exists(dest) {
            at.remove(dest);
        }
    }
}
