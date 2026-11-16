// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use regex::Regex;
use uutests::new_ucmd;

#[test]
fn test_output_format() {
    // Output must be exactly 8 lowercase hex digits followed by a newline.
    let re = Regex::new(r"^[0-9a-f]{8}\n$").unwrap();
    new_ucmd!().succeeds().stdout_matches(&re);
}

#[test]
fn test_help() {
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .stdout_contains("Print the numeric identifier");
}

#[test]
fn test_invalid_flag() {
    new_ucmd!()
        .arg("--invalid-argument")
        .fails_with_code(1)
        .no_stdout();
}

#[cfg(target_os = "linux")]
#[cfg(not(wasi_runner))]
#[test]
fn test_hostname_is_ipv4_address() {
    use std::process::Command;
    use uutests::{util::TestScenario, util_name};

    let can_unshare = Command::new("unshare")
        .args(["-rmu", "hostname", "127.0.0.1"])
        .status()
        .is_ok_and(|status| status.success());
    if !can_unshare {
        eprintln!("TEST SKIPPED: unshare -rmu hostname does not work");
        return;
    }

    let ts = TestScenario::new(util_name!());
    let output = Command::new("unshare")
        .args(["-rmu", "sh", "-c"])
        .arg(r#"hostname 127.0.0.1 && mount -t tmpfs tmpfs /etc && exec "$0" hostid"#)
        .arg(&ts.bin_path)
        .output()
        .expect("failed to spawn unshare");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "007f0100\n");
}
