// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
#[cfg(not(windows))]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[cfg(not(windows))]
#[test]
fn test_mknod_help() {
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .no_stderr()
        .stdout_contains("Usage:");
}

#[test]
#[cfg(not(windows))]
fn test_mknod_version() {
    assert!(new_ucmd!()
        .arg("--version")
        .succeeds()
        .no_stderr()
        .stdout_str()
        .starts_with("mknod"));
}

#[test]
#[cfg(not(windows))]
fn test_mknod_fifo_default_writable() {
    let ts = TestScenario::new(util_name!());
    ts.ucmd().arg("test_file").arg("p").succeeds();
    assert!(ts.fixtures.is_fifo("test_file"));
    assert!(!ts.fixtures.metadata("test_file").permissions().readonly());
}

#[test]
#[cfg(not(windows))]
fn test_mknod_fifo_mnemonic_usage() {
    let ts = TestScenario::new(util_name!());
    ts.ucmd().arg("test_file").arg("pipe").succeeds();
    assert!(ts.fixtures.is_fifo("test_file"));
}

#[test]
#[cfg(not(windows))]
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
#[cfg(not(windows))]
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
#[cfg(not(windows))]
fn test_mknod_character_device_requires_major_and_minor() {
    new_ucmd!()
        .arg("test_file")
        .arg("c")
        .fails()
        .code_is(1)
        .stderr_contains("Special files require major and minor device numbers.");
    new_ucmd!()
        .arg("test_file")
        .arg("c")
        .arg("1")
        .fails()
        .code_is(1)
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
#[cfg(not(windows))]
fn test_mknod_invalid_arg() {
    new_ucmd!()
        .arg("--foo")
        .fails()
        .no_stdout()
        .stderr_contains("unexpected argument '--foo' found");
}

#[test]
#[cfg(not(windows))]
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
