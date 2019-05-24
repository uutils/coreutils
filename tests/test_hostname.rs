use common::util::*;

#[test]
fn test_hostname() {
    let ls_default_res = new_ucmd!().succeeds();
    let ls_short_res = new_ucmd!().arg("-s").succeeds();
    let ls_domain_res = new_ucmd!().arg("-d").succeeds();

    assert!(ls_default_res.stdout.len() >= ls_short_res.stdout.len());
    assert!(ls_default_res.stdout.len() >= ls_domain_res.stdout.len());
}

