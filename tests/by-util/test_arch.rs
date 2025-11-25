use uutests::new_ucmd;

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
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_arch_output_is_not_empty() {
    let result = new_ucmd!().succeeds();
    assert!(
        !result.stdout_str().trim().is_empty(),
        "arch output was empty"
    );
}
