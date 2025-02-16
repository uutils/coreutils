// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_arch() {
    new_ucmd!().succeeds();
}

#[test]
fn test_arch_help() {
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .stdout_contains("architecture name");
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}
