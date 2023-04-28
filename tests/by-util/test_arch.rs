use crate::common::util::TestScenario;

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

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}
