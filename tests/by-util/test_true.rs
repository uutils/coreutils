use regex::Regex;
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "netbsd"))]
use std::fs::OpenOptions;
use uutests::new_ucmd;

#[test]
fn test_no_args() {
    new_ucmd!().succeeds().no_output();
}

#[test]
fn test_version() {
    let re = Regex::new(r"^true .*\d+\.\d+\.\d+\n$").unwrap();

    new_ucmd!()
        .args(&["--version"])
        .succeeds()
        .stdout_matches(&re);
}

#[test]
fn test_help() {
    new_ucmd!()
        .args(&["--help"])
        .succeeds()
        .stdout_contains("true");
}

#[test]
fn test_short_options() {
    for option in ["-h", "-V"] {
        new_ucmd!().arg(option).succeeds().no_output();
    }
}

#[test]
fn test_conflict() {
    new_ucmd!()
        .args(&["--help", "--version"])
        .succeeds()
        .no_output();
}

#[test]
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "netbsd"))]
fn test_full() {
    for option in ["--version", "--help"] {
        let dev_full = OpenOptions::new().write(true).open("/dev/full").unwrap();

        new_ucmd!()
            .arg(option)
            .set_stdout(dev_full)
            .fails()
            .stderr_contains("No space left on device");
    }
}
