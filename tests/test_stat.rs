use common::util::*;

static UTIL_NAME: &'static str = "stat";

use std::process::Command;

#[test]
fn test_invalid_option() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-w").arg("-q").arg("/");
    ucmd.fails();
}

#[test]
#[cfg(target_os = "linux")]
fn test_terse_fs_format() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let args = ["-f", "-t", "/proc"];
    ucmd.args(&args);
    assert_eq!(ucmd.run().stdout, expected_result(&args));
}

#[test]
#[cfg(target_os = "linux")]
fn test_fs_format() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let args = ["-f", "--format=%n %i 0x%t %T", "/dev/shm"];
    ucmd.args(&args);
    assert_eq!(ucmd.run().stdout, "/dev/shm 0 0x1021994 tmpfs\n");
}

#[test]
#[cfg(target_os = "linux")]
fn test_terse_normal_format() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let args = ["-t", "/"];
    ucmd.args(&args);
    assert_eq!(ucmd.run().stdout, expected_result(&args));
}

#[test]
#[cfg(target_os = "linux")]
fn test_normal_format() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let args = ["/boot"];
    ucmd.args(&args);
    assert_eq!(ucmd.run().stdout, expected_result(&args));
}

#[test]
#[cfg(target_os = "linux")]
fn test_follow_symlink() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let args = ["-L", "/dev/cdrom"];
    ucmd.args(&args);
    assert_eq!(ucmd.run().stdout, expected_result(&args));
}

#[test]
#[cfg(target_os = "linux")]
fn test_symlink() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let args = ["/dev/cdrom"];
    ucmd.args(&args);
    assert_eq!(ucmd.run().stdout, expected_result(&args));
}

#[test]
#[cfg(target_os = "linux")]
fn test_char() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let args = ["/dev/zero"];
    ucmd.args(&args);
    assert_eq!(ucmd.run().stdout, expected_result(&args));
}

#[test]
#[cfg(target_os = "linux")]
fn test_multi_files() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let args = ["/dev", "/usr/lib", "/etc/fstab", "/var"];
    ucmd.args(&args);
    assert_eq!(ucmd.run().stdout, expected_result(&args));
}

#[test]
#[cfg(target_os = "linux")]
fn test_printf() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let args = ["--printf=123%-# 15q\\r\\\"\\\\\\a\\b\\e\\f\\v%+020.23m\\x12\\167\\132\\112\\n", "/"];
    ucmd.args(&args);
    assert_eq!(ucmd.run().stdout, "123?\r\"\\\x07\x08\x1B\x0C\x0B                   /\x12wZJ\n");
}

fn expected_result(args: &[&str]) -> String {
    let output = Command::new(UTIL_NAME).args(args).output().unwrap();
    String::from_utf8_lossy(&output.stdout).into_owned()
}
