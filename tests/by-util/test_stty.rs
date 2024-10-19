// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore parenb parmrk ixany iuclc onlcr ofdel icanon noflsh ixon

use crate::common::util::TestScenario;
use regex::Regex;

#[test]
fn test_invalid_arg() {
    new_ucmd!()
        .args(&["--file=/dev/tty", "--definitely-invalid"])
        .fails()
        .code_is(1);
}

#[test]
fn runs() {
    new_ucmd!().arg("--file=/dev/tty").succeeds();
}

#[test]
fn print_all() {
    let cmd_result = new_ucmd!().args(&["--file=/dev/tty", "-a"]).succeeds();

    // "iuclc" removed due to this comment in stty.rs:
    //
    // not supported by nix
    // Flag::new("iuclc", I::IUCLC),

    // Random selection of flags to check for
    for flag in [
        "parenb", "parmrk", "ixany", "onlcr", "ofdel", "icanon", "noflsh",
    ] {
        cmd_result.stdout_contains(flag);
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

// Make sure the "allow_hyphen_values" clap function has been called with true
#[test]
fn negation() {
    new_ucmd!()
        .args(&["--file=/dev/tty", "-ixon"])
        .succeeds()
        .stdout_is_bytes([])
        .stderr_is_bytes([]);
}

fn succeeds_test_with_regex(args: &[&str], stdout_regex: &Regex) {
    new_ucmd!()
        .args(args)
        .succeeds()
        .stdout_str_check(|st| {
            let Some(str) = st.lines().next() else {
                return false;
            };

            stdout_regex.is_match(str)
        })
        .no_stderr();
}

// The end of options delimiter ("--") and everything after must be ignored
#[test]
fn ignore_end_of_options_and_after() {
    {
        // e.g.:
        // speed 38400 baud; rows 54; columns 216; line = 0;
        let regex =
            Regex::new("speed [0-9]+ baud; rows [0-9]+; columns [0-9]+; line = [0-9]+;").unwrap();

        // "stty -a -- -ixon" should behave like "stty -a"
        // Should not abort with an error complaining about passing both "-a" and "-ixon" (since "-ixon" is after "--")
        succeeds_test_with_regex(&["--file=/dev/tty", "-a", "--", "-ixon"], &regex);
    }

    {
        // e.g.:
        // speed 38400 baud; line = 0;
        let regex = Regex::new("speed [0-9]+ baud; line = [0-9]+;").unwrap();

        // "stty -- non-existent-option-that-must-be-ignore" should behave like "stty"
        // Should not abort with an error complaining about an invalid argument, since the invalid argument is after "--"
        succeeds_test_with_regex(
            &[
                "--file=/dev/tty",
                "--",
                "non-existent-option-that-must-be-ignored",
            ],
            &regex,
        );
    }
}
