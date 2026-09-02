// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::new_ucmd;

#[test]
fn test_hostname() {
    let ls_default_res = new_ucmd!().succeeds();
    let ls_short_res = new_ucmd!().arg("-s").succeeds();
    let ls_domain_res = new_ucmd!().arg("-d").succeeds();

    assert!(ls_default_res.stdout().len() >= ls_short_res.stdout().len());
    assert!(ls_default_res.stdout().len() >= ls_domain_res.stdout().len());
}

#[test]
fn test_hostname_ip() {
    let result = new_ucmd!().arg("-i").succeeds();
    assert_ne!(result.stdout_str().trim(), "");
}

#[test]
fn test_hostname_full() {
    let ls_short_res = new_ucmd!().arg("-s").succeeds();
    assert_ne!(ls_short_res.stdout_str().trim(), "");

    new_ucmd!()
        .arg("-f")
        .succeeds()
        .stdout_contains(ls_short_res.stdout_str().trim());
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_hostname_domain_empty() {
    let fqdn = new_ucmd!().arg("-f").succeeds();
    let short = new_ucmd!().arg("-s").succeeds();
    let domain = new_ucmd!().arg("-d").succeeds();
    let domain_short = new_ucmd!().arg("-sd").succeeds();

    if fqdn.stdout() == short.stdout() {
        assert_eq!(domain.stdout(), []);
        assert_eq!(domain_short.stdout(), []);
    }
}
