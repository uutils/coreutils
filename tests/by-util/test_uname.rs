use crate::common::util::*;

#[test]
fn test_uname_compatible() {
    new_ucmd!().arg("-a").succeeds();
}

#[test]
fn test_uname_name() {
    new_ucmd!().arg("-n").succeeds();
}

#[test]
fn test_uname_processor() {
    let result = new_ucmd!().arg("-p").succeeds();
    assert_eq!(result.stdout_str().trim_end(), "unknown");
}

#[test]
fn test_uname_hardware_platform() {
    let result = new_ucmd!().arg("-i").succeeds();
    assert_eq!(result.stdout_str().trim_end(), "unknown");
}

#[test]
fn test_uname_machine() {
    new_ucmd!().arg("-m").succeeds();
}

#[test]
fn test_uname_kernel_version() {
    new_ucmd!().arg("-v").succeeds();
}

#[test]
fn test_uname_kernel() {
    let (_, mut ucmd) = at_and_ucmd!();

    #[cfg(target_os = "linux")]
    {
        let result = ucmd.arg("-o").succeeds();
        assert!(result.stdout_str().to_lowercase().contains("linux"));
    }

    #[cfg(not(target_os = "linux"))]
    ucmd.arg("-o").succeeds();
}
