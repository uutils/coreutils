// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::common::util::TestScenario;

#[test]
fn test_hostname() {
    let ls_default_res = new_ucmd!().succeeds();
    let ls_short_res = new_ucmd!().arg("-s").succeeds();
    let ls_domain_res = new_ucmd!().arg("-d").succeeds();

    assert!(ls_default_res.stdout().len() >= ls_short_res.stdout().len());
    assert!(ls_default_res.stdout().len() >= ls_domain_res.stdout().len());
}

// FixME: fails for "MacOS", "freebsd" and "openbsd" "failed to lookup address information: Name does not resolve"
#[cfg(not(any(target_os = "macos", target_os = "freebsd", target_os = "openbsd")))]
#[test]
fn test_hostname_ip() {
    let result = new_ucmd!().arg("-i").succeeds();
    assert!(!result.stdout_str().trim().is_empty());
}

#[test]
fn test_hostname_full() {
    let ls_short_res = new_ucmd!().arg("-s").succeeds();
    assert!(!ls_short_res.stdout_str().trim().is_empty());

    new_ucmd!()
        .arg("-f")
        .succeeds()
        .stdout_contains(ls_short_res.stdout_str().trim());
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}
