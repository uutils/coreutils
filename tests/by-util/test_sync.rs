use crate::common::util::*;

#[test]
fn test_sync_default() {
    new_ucmd!().run();
}

#[test]
fn test_sync_incorrect_arg() {
    new_ucmd!().arg("--foo").fails();
}
