// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use regex::Regex;
use uutests::new_ucmd;
#[cfg(target_os = "linux")]
use uutests::{util::TestScenario, util_name};

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

/// With the hostname set to an IPv4 address, the id is derived from that address.
#[cfg(target_os = "linux")]
#[test]
fn test_hostname_is_ipv4_address() {
    use std::process::Command;

    // /etc/hostid takes precedence over the hostname.
    if std::path::Path::new("/etc/hostid").exists() {
        println!("TEST SKIPPED: /etc/hostid exists");
        return;
    }
    // Unprivileged user namespaces are disabled in some kernels and sandboxes.
    let can_unshare = Command::new("unshare")
        .args(["-ru", "hostname", "127.0.0.1"])
        .status()
        .is_ok_and(|status| status.success());
    if !can_unshare {
        println!("TEST SKIPPED: no unprivileged UTS namespace available");
        return;
    }

    let ts = TestScenario::new(util_name!());
    let output = Command::new("unshare")
        .args(["-ru", "sh", "-c"])
        .arg(r#"hostname 127.0.0.1 && exec "$0" hostid"#)
        .arg(&ts.bin_path)
        .output()
        .expect("failed to spawn unshare");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "007f0100\n");
}
