// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::common::util::TestScenario;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_create_fifo_missing_operand() {
    new_ucmd!().fails().stderr_is("mkfifo: missing operand\n");
}

#[test]
fn test_create_one_fifo() {
    new_ucmd!().arg("abc").succeeds();
}

#[test]
fn test_create_one_fifo_with_invalid_mode() {
    new_ucmd!()
        .arg("abcd")
        .arg("-m")
        .arg("invalid")
        .fails()
        .stderr_contains("invalid mode");
}

#[test]
fn test_create_multiple_fifos() {
    new_ucmd!()
        .arg("abcde")
        .arg("def")
        .arg("sed")
        .arg("dum")
        .succeeds();
}

#[test]
fn test_create_one_fifo_with_mode() {
    new_ucmd!().arg("abcde").arg("-m600").succeeds();
}

#[test]
fn test_create_one_fifo_already_exists() {
    new_ucmd!()
        .arg("abcdef")
        .arg("abcdef")
        .fails()
        .stderr_is("mkfifo: cannot create fifo 'abcdef': File exists\n");
}
