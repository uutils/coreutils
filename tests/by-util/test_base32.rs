// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

use crate::common::util::*;

#[test]
fn test_encode() {
    let input = "Hello, World!";
    new_ucmd!()
        .pipe_in(input)
        .succeeds()
        .stdout_only("JBSWY3DPFQQFO33SNRSCC===\n");

    // Using '-' as our file
    new_ucmd!()
        .arg("-")
        .pipe_in(input)
        .succeeds()
        .stdout_only("JBSWY3DPFQQFO33SNRSCC===\n");
}

#[test]
fn test_base32_encode_file() {
    new_ucmd!()
        .arg("input-simple.txt")
        .succeeds()
        .stdout_only("JBSWY3DPFQQFO33SNRSCCCQ=\n");
}

#[test]
fn test_decode() {
    for decode_param in vec!["-d", "--decode"] {
        let input = "JBSWY3DPFQQFO33SNRSCC===\n";
        new_ucmd!()
            .arg(decode_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("Hello, World!");
    }
}

#[test]
fn test_garbage() {
    let input = "aGVsbG8sIHdvcmxkIQ==\0";
    new_ucmd!()
        .arg("-d")
        .pipe_in(input)
        .fails()
        .stderr_only("base32: error: invalid input\n");
}

#[test]
fn test_ignore_garbage() {
    for ignore_garbage_param in vec!["-i", "--ignore-garbage"] {
        let input = "JBSWY\x013DPFQ\x02QFO33SNRSCC===\n";
        new_ucmd!()
            .arg("-d")
            .arg(ignore_garbage_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("Hello, World!");
    }
}

#[test]
fn test_wrap() {
    for wrap_param in vec!["-w", "--wrap"] {
        let input = "The quick brown fox jumps over the lazy dog.";
        new_ucmd!()
            .arg(wrap_param)
            .arg("20")
            .pipe_in(input)
            .succeeds()
            .stdout_only(
                "KRUGKIDROVUWG2ZAMJZG\n653OEBTG66BANJ2W24DT\nEBXXMZLSEB2GQZJANRQX\nU6JAMRXWOLQ=\n",
            );
    }
}

#[test]
fn test_wrap_no_arg() {
    for wrap_param in vec!["-w", "--wrap"] {
        new_ucmd!().arg(wrap_param).fails().stderr_only(format!(
            "error: The argument '--wrap <wrap>\' requires a value but none was supplied\n\nUSAGE:\n    base32 [OPTION]... [FILE]\n\nFor more information try --help"
        ));
    }
}

#[test]
fn test_wrap_bad_arg() {
    for wrap_param in vec!["-w", "--wrap"] {
        new_ucmd!()
            .arg(wrap_param)
            .arg("b")
            .fails()
            .stderr_only("base32: Invalid wrap size: ‘b’: invalid digit found in string\n");
    }
}

#[test]
fn test_base32_extra_operand() {
    // Expect a failure when multiple files are specified.
    new_ucmd!()
        .arg("a.txt")
        .arg("a.txt")
        .fails()
        .stderr_only("base32: extra operand ‘a.txt’");
}

#[test]
fn test_base32_file_not_found() {
    new_ucmd!()
        .arg("a.txt")
        .fails()
        .stderr_only("base32: a.txt: No such file or directory");
}
