use crate::common::util::*;

#[test]
fn test_hostname() {
    let ls_default_res = new_ucmd!().succeeds();
    let ls_short_res = new_ucmd!().arg("-s").succeeds();
    let ls_domain_res = new_ucmd!().arg("-d").succeeds();

    assert!(ls_default_res.stdout().len() >= ls_short_res.stdout().len());
    assert!(ls_default_res.stdout().len() >= ls_domain_res.stdout().len());
}

// FixME: fails for "MacOS"
#[cfg(not(target_vendor = "apple"))]
#[test]
fn test_hostname_ip() {
    let result = new_ucmd!().arg("-i").run();
    println!("{:#?}", result);
    assert!(result.success);
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
