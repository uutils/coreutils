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
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_hostname_domain_empty() {
    let fqdn = new_ucmd!().arg("-f").succeeds();
    let short = new_ucmd!().arg("-s").succeeds();
    let domain = new_ucmd!().arg("-d").succeeds();
    let domain_short = new_ucmd!().arg("-sd").succeeds();

    if fqdn.stdout() == short.stdout() {
        assert!(domain.stdout().is_empty());
        assert!(domain_short.stdout().is_empty());
    }
}

#[cfg(all(target_os = "linux", not(wasi_runner)))]
#[rstest::rstest]
#[case(b"node\xff.example\xfe", &[], b"node\xff.example\xfe\n")]
#[case(b"node\xff.example\xfe", &["-f"], b"node\xff.example\xfe\n")]
#[case(b"node\xff.example\xfe", &["-s"], b"node\xff\n")]
#[case(b"node\xff.example\xfe", &["-d"], b"example\xfe\n")]
#[case(b"node\xff", &[], b"node\xff\n")]
#[case(b"node\xff", &["-s"], b"node\xff\n")]
#[case(b"node\xff", &["-d"], b"")]
fn test_hostname_non_utf8(#[case] hostname: &[u8], #[case] args: &[&str], #[case] expected: &[u8]) {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::process::Command;
    use uutests::util::TestScenario;

    if !Command::new("unshare")
        .args(["-Ur", "-u", "true"])
        .output()
        .is_ok_and(|output| output.status.success())
    {
        eprintln!("Skipping test: user and UTS namespaces are unavailable");
        return;
    }

    let ts = TestScenario::new("hostname");
    ts.cmd("unshare")
        .args(&["-Ur", "-u", "sh", "-c"])
        // Set the hostname inside the isolated namespace, then query it with the requested flags.
        .arg(r#"bin=$1; "$bin" hostname "$2" || exit; shift 2; exec "$bin" hostname "$@""#)
        .arg("sh")
        .arg(&ts.bin_path)
        .arg(OsStr::from_bytes(hostname))
        .args(args)
        .succeeds()
        .stdout_only_bytes(expected);
}
