use common::util::*;

#[test]
fn test_uname_compatible() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("-a").run();
    assert!(result.success);
}

#[test]
fn test_uname_name() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("-n").run();
    assert!(result.success);
}

#[test]
fn test_uname_kernel() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("-o").run();
    assert!(result.success);
    #[cfg(target_os = "linux")]
    assert!(result.stdout.to_lowercase().contains("linux"));
}
