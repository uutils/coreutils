// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

use common::util::*;


#[test]
fn test_encode() {
    let input = "Hello, World!";
    new_ucmd!()
        .pipe_in(input)
        .succeeds()
        .stdout_only("JBSWY3DPFQQFO33SNRSCC===\n");
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
            .stdout_only("KRUGKIDROVUWG2ZAMJZG\n653OEBTG66BANJ2W24DT\nEBXXMZLSEB2GQZJANRQX\nU6JAMRXWOLQ=\n");
    }
}

#[test]
fn test_wrap_no_arg() {
    for wrap_param in vec!["-w", "--wrap"] {
        new_ucmd!()
            .arg(wrap_param)
            .fails()
            .stderr_only(format!("base32: error: Argument to option '{}' missing.\n",
                                 if wrap_param == "-w" { "w" } else { "wrap" }));
    }
}

#[test]
fn test_wrap_bad_arg() {
    for wrap_param in vec!["-w", "--wrap"] {
        new_ucmd!()
            .arg(wrap_param).arg("b")
            .fails()
            .stderr_only("base32: error: invalid wrap size: ‘b’: invalid digit found in string\n");
    }
}
