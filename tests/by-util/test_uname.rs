// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_uname() {
    new_ucmd!().succeeds();
}

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
    new_ucmd!()
        .arg("-i")
        .succeeds()
        .stdout_str_apply(str::trim_end)
        .stdout_only("unknown");
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

#[test]
fn test_uname_operating_system() {
    #[cfg(target_os = "android")]
    new_ucmd!()
        .arg("--operating-system")
        .succeeds()
        .stdout_is("Android\n");
    #[cfg(target_vendor = "apple")]
    new_ucmd!()
        .arg("--operating-system")
        .succeeds()
        .stdout_is("Darwin\n");
    #[cfg(target_os = "freebsd")]
    new_ucmd!()
        .arg("--operating-system")
        .succeeds()
        .stdout_is("FreeBSD\n");
    #[cfg(target_os = "fuchsia")]
    new_ucmd!()
        .arg("--operating-system")
        .succeeds()
        .stdout_is("Fuchsia\n");
    #[cfg(all(target_os = "linux", any(target_env = "gnu", target_env = "")))]
    new_ucmd!()
        .arg("--operating-system")
        .succeeds()
        .stdout_is("GNU/Linux\n");
    #[cfg(all(target_os = "linux", not(any(target_env = "gnu", target_env = ""))))]
    new_ucmd!()
        .arg("--operating-system")
        .succeeds()
        .stdout_is("Linux\n");
    #[cfg(target_os = "netbsd")]
    new_ucmd!()
        .arg("--operating-system")
        .succeeds()
        .stdout_is("NetBSD\n");
    #[cfg(target_os = "openbsd")]
    new_ucmd!()
        .arg("--operating-system")
        .succeeds()
        .stdout_is("OpenBSD\n");
    #[cfg(target_os = "redox")]
    new_ucmd!()
        .arg("--operating-system")
        .succeeds()
        .stdout_is("Redox\n");
    #[cfg(target_os = "windows")]
    {
        let result = new_ucmd!().arg("--operating-system").succeeds();
        println!("{:?}", result.stdout_str());
        assert!(result.stdout_str().starts_with("MS/Windows"));
    }
}

#[test]
fn test_uname_help() {
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .stdout_contains("system information");
}

#[test]
fn test_uname_output_for_invisible_chars() {
    // let re = regex::Regex::new("[^[[:print:]]]").unwrap(); // matches invisible (and emojis)
    let re = regex::Regex::new("[^[[:print:]]\\p{Other_Symbol}]").unwrap(); // matches invisible (not emojis)
    let result = new_ucmd!().arg("--all").succeeds();
    assert_eq!(re.find(result.stdout_str().trim_end()), None);
}
