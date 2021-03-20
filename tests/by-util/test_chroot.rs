use crate::common::util::*;

#[test]
fn test_missing_operand() {
    new_ucmd!()
        .fails()
        .stderr_is("Missing operand: NEWROOT\nTry `chroot --help` for more information.");
}
