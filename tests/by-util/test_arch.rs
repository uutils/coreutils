use crate::common::util::*;

#[test]
fn test_arch() {
    new_ucmd!().succeeds();
}

#[test]
fn test_arch_help() {
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .stdout_contains("architecture name");
}
