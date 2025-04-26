// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore nconfined

use uutests::new_ucmd;
use uutests::util::TestScenario;
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
#[cfg(feature = "feat_selinux")]
fn test_mknod_selinux() {
    use std::process::Command;
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

        let getfattr_output = Command::new("getfattr")
            .arg(at.plus_as_string(dest))
            .arg("-n")
            .arg("security.selinux")
            .output()
            .expect("Failed to run `getfattr` on the destination file");
        println!("{:?}", getfattr_output);
        assert!(
            getfattr_output.status.success(),
            "getfattr did not run successfully: {}",
            String::from_utf8_lossy(&getfattr_output.stderr)
        );

        let stdout = String::from_utf8_lossy(&getfattr_output.stdout);
        assert!(
            stdout.contains("unconfined_u"),
            "Expected '{}' not found in getfattr output:\n{}",
            "foo",
            stdout
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
