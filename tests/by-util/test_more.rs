use crate::common::util::TestScenario;
use is_terminal::IsTerminal;
use std::fs::{set_permissions, Permissions};
use std::os::unix::fs::PermissionsExt;

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
fn test_more_invalid_file_perms() {
    let (at, mut ucmd) = at_and_ucmd!();
    let permissions = Permissions::from_mode(0o244);
    at.make_file("invalid-perms.txt").metadata().unwrap();
    set_permissions(at.plus("invalid-perms.txt"), permissions).unwrap();
    ucmd.arg("invalid-perms.txt").fails();
    //.code_is(1)
    //.stderr_is("more: cannot open 'invalid-perms.txt': permission denied");
}
