use crate::common::util::TestScenario;
use is_terminal::IsTerminal;

#[test]
fn test_more_no_arg() {
    // Reading from stdin is now supported, so this must succeed
    if std::io::stdout().is_terminal() {
        new_ucmd!().succeeds();
    }
}

#[test]
fn test_valid_arg() {
    if std::io::stdout().is_terminal() {
        new_ucmd!().arg("-c").succeeds();
        new_ucmd!().arg("--print-over").succeeds();

        new_ucmd!().arg("-p").succeeds();
        new_ucmd!().arg("--clean-print").succeeds();

        new_ucmd!().arg("-s").succeeds();
        new_ucmd!().arg("--squeeze").succeeds();

        new_ucmd!().arg("-n").arg("10").succeeds();
        new_ucmd!().arg("--lines").arg("0").succeeds();
        new_ucmd!().arg("--number").arg("0").succeeds();
    }
}

#[test]
fn test_invalid_arg() {
    if std::io::stdout().is_terminal() {
        new_ucmd!().arg("--invalid").fails();

        new_ucmd!().arg("--lines").arg("-10").fails();
        new_ucmd!().arg("--number").arg("-10").fails();
    }
}

#[test]
fn test_more_dir_arg() {
    // Run the test only if there's a valid terminal, else do nothing
    // Maybe we could capture the error, i.e. "Device not found" in that case
    // but I am leaving this for later
    if std::io::stdout().is_terminal() {
        new_ucmd!()
            .arg(".")
            .fails()
            .usage_error("'.' is a directory.");
    }
}

#[test]
#[cfg(target_family = "unix")]
fn test_more_invalid_file_perms() {
    use std::fs::{set_permissions, Permissions};
    use std::os::unix::fs::PermissionsExt;

    if std::io::stdout().is_terminal() {
        let (at, mut ucmd) = at_and_ucmd!();
        let permissions = Permissions::from_mode(0o244);
        at.make_file("invalid-perms.txt");
        set_permissions(at.plus("invalid-perms.txt"), permissions).unwrap();
        ucmd.arg("invalid-perms.txt")
            .fails()
            .code_is(1)
            .stderr_contains("permission denied");
    }
}

#[test]
fn test_more_error_on_multiple_files() {
    if std::io::stdout().is_terminal() {
        let ts = TestScenario::new("more");
        ts.fixtures.mkdir_all("folder");
        ts.fixtures.make_file("file1");
        ts.ucmd()
            .arg("folder")
            .arg("file2")
            .arg("file1")
            .succeeds()
            .stdout_contains("folder")
            .stdout_contains("file2")
            .stdout_contains("file1");
        ts.ucmd()
            .arg("file2")
            .arg("file3")
            .succeeds()
            .stdout_contains("file2")
            .stdout_contains("file3");
    }
}
