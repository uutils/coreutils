// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use regex::Regex;
use uutests::new_ucmd;

#[test]
fn test_normal() {
    let re = Regex::new(r"^[0-9a-f]{8}").unwrap();
    new_ucmd!().succeeds().stdout_matches(&re);
}

#[test]
fn test_output_format() {
    // Output must be exactly 8 lowercase hex digits followed by a newline.
    // The stricter anchored regex catches outputs like "00000000garbage"
    // that the existing test_normal regex would incorrectly accept.
    let re = Regex::new(r"^[0-9a-f]{8}\n$").unwrap();
    new_ucmd!().succeeds().stdout_matches(&re);
}

#[test]
fn test_help() {
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .stdout_contains("Print the numeric identifier");
}

#[test]
fn test_invalid_flag() {
    new_ucmd!()
        .arg("--invalid-argument")
        .fails_with_code(1)
        .no_stdout();
}
