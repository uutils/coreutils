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
