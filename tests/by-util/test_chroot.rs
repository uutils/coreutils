use crate::common::util::*;

#[test]
fn test_missing_operand() {
    new_ucmd!()
        .fails()
        .stderr_is(
            "chroot: error: Missing operand: NEWROOT\nTry `chroot --help` for more information.",
        )
        .status_code(1);
}

#[test]
fn test_no_such_directory() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch(&at.plus_as_string("a"));

    ucmd.arg("a")
        .fails()
        .stderr_is("chroot: error: cannot change root directory to `a`: no such directory");
}
