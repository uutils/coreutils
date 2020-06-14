use crate::common::util::*;

#[test]
fn test_hostname() {
    let ls_default_res = new_ucmd!().succeeds();
    let ls_short_res = new_ucmd!().arg("-s").succeeds();
    let ls_domain_res = new_ucmd!().arg("-d").succeeds();

    assert!(ls_default_res.stdout.len() >= ls_short_res.stdout.len());
    assert!(ls_default_res.stdout.len() >= ls_domain_res.stdout.len());
}

// FixME: fails for "MacOS"
#[cfg(not(target_os = "macos"))]
#[test]
fn test_hostname_ip() {
    let result = new_ucmd!().arg("-i").run();
    println!("{:#?}", result);
    assert!(result.success);
    assert!(!result.stdout.trim().is_empty());
}

#[test]
fn test_hostname_full() {
    let result = new_ucmd!().arg("-f").succeeds();
    assert!(!result.stdout.trim().is_empty());

    let ls_short_res = new_ucmd!().arg("-s").succeeds();
    assert!(result.stdout.trim().contains(ls_short_res.stdout.trim()));
}
