use crate::common::util::*;

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
fn test_uname_processor() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("-p").run();
    assert!(result.success);
    assert_eq!(result.stdout.trim_end(), "unknown");
}

#[test]
fn test_uname_hwplatform() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("-i").run();
    assert!(result.success);
    assert_eq!(result.stdout.trim_end(), "unknown");
}

#[test]
fn test_uname_machine() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("-m").run();
    assert!(result.success);
}

#[test]
fn test_uname_kernel_version() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("-v").run();
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
