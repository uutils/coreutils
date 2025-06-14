// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore parenb parmrk ixany iuclc onlcr ofdel icanon noflsh econl igpar ispeed ospeed

use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
#[ignore = "Fails because cargo test does not run in a tty"]
fn runs() {
    new_ucmd!().succeeds();
}

#[test]
#[ignore = "Fails because cargo test does not run in a tty"]
fn print_all() {
    let res = new_ucmd!().succeeds();

    // Random selection of flags to check for
    for flag in [
        "parenb", "parmrk", "ixany", "iuclc", "onlcr", "ofdel", "icanon", "noflsh",
    ] {
        res.stdout_contains(flag);
    }
}

#[test]
fn save_and_setting() {
    new_ucmd!()
        .args(&["--save", "nl0"])
        .fails()
        .stderr_contains("when specifying an output style, modes may not be set");
}

#[test]
fn all_and_setting() {
    new_ucmd!()
        .args(&["--all", "nl0"])
        .fails()
        .stderr_contains("when specifying an output style, modes may not be set");
}

#[test]
fn save_and_all() {
    new_ucmd!()
        .args(&["--save", "--all"])
        .fails()
        .stderr_contains(
            "the options for verbose and stty-readable output styles are mutually exclusive",
        );

    new_ucmd!()
        .args(&["--all", "--save"])
        .fails()
        .stderr_contains(
            "the options for verbose and stty-readable output styles are mutually exclusive",
        );
}

#[test]
fn no_mapping() {
    new_ucmd!()
        .args(&["intr"])
        .fails()
        .stderr_contains("missing argument to 'intr'");
}

#[test]
fn invalid_mapping() {
    new_ucmd!()
        .args(&["intr", "cc"])
        .fails()
        .stderr_contains("invalid integer argument: 'cc'");

    new_ucmd!()
        .args(&["intr", "256"])
        .fails()
        .stderr_contains("invalid integer argument: '256': Value too large for defined data type");

    new_ucmd!()
        .args(&["intr", "0x100"])
        .fails()
        .stderr_contains(
            "invalid integer argument: '0x100': Value too large for defined data type",
        );

    new_ucmd!()
        .args(&["intr", "0400"])
        .fails()
        .stderr_contains("invalid integer argument: '0400': Value too large for defined data type");
}

#[test]
fn invalid_setting() {
    new_ucmd!()
        .args(&["-econl"])
        .fails()
        .stderr_contains("invalid argument '-econl'");

    new_ucmd!()
        .args(&["igpar"])
        .fails()
        .stderr_contains("invalid argument 'igpar'");
}

#[test]
fn invalid_baud_setting() {
    #[cfg(not(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    new_ucmd!()
        .args(&["100"])
        .fails()
        .stderr_contains("invalid argument '100'");

    new_ucmd!()
        .args(&["-1"])
        .fails()
        .stderr_contains("invalid argument '-1'");

    new_ucmd!()
        .args(&["ispeed"])
        .fails()
        .stderr_contains("missing argument to 'ispeed'");

    new_ucmd!()
        .args(&["ospeed"])
        .fails()
        .stderr_contains("missing argument to 'ospeed'");

    #[cfg(not(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    new_ucmd!()
        .args(&["ispeed", "995"])
        .fails()
        .stderr_contains("invalid ispeed '995'");

    #[cfg(not(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    new_ucmd!()
        .args(&["ospeed", "995"])
        .fails()
        .stderr_contains("invalid ospeed '995'");
}
