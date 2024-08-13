// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::common::util::TestScenario;
use regex::Regex;

#[test]
fn test_normal() {
    let re = Regex::new(r"^[0-9a-f]{8}").unwrap();
    new_ucmd!().succeeds().stdout_matches(&re);
}

#[test]
fn test_invalid_flag() {
    new_ucmd!()
        .arg("--invalid-argument")
        .fails()
        .no_stdout()
        .code_is(1);
}
