// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_encode() {
    let input = "Hello, World!";
    new_ucmd!()
        .pipe_in(input)
        .succeeds()
        .stdout_only("JBSWY3DPFQQFO33SNRSCC===\n"); // spell-checker:disable-line

    // Using '-' as our file
    new_ucmd!()
        .arg("-")
        .pipe_in(input)
        .succeeds()
        .stdout_only("JBSWY3DPFQQFO33SNRSCC===\n"); // spell-checker:disable-line
}

#[test]
fn test_encode_repeat_flags_later_wrap_10() {
    let input = "Hello, World!\n";
    new_ucmd!()
        .args(&["-ii", "-w17", "-w10"])
        .pipe_in(input)
        .succeeds()
        .stdout_only("JBSWY3DPFQ\nQFO33SNRSC\nCCQ=\n"); // spell-checker:disable-line
}

#[test]
fn test_encode_repeat_flags_later_wrap_17() {
    let input = "Hello, World!\n";
    new_ucmd!()
        .args(&["-ii", "-w10", "-w17"])
        .pipe_in(input)
        .succeeds()
        .stdout_only("JBSWY3DPFQQFO33SN\nRSCCCQ=\n"); // spell-checker:disable-line
}

#[test]
fn test_base32_encode_file() {
    new_ucmd!()
        .arg("input-simple.txt")
        .succeeds()
        .stdout_only("JBSWY3DPFQQFO33SNRSCCCQ=\n"); // spell-checker:disable-line
}

#[test]
fn test_decode() {
    for decode_param in ["-d", "--decode", "--dec"] {
        let input = "JBSWY3DPFQQFO33SNRSCC===\n"; // spell-checker:disable-line
        new_ucmd!()
            .arg(decode_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("Hello, World!");
    }
}

#[test]
fn test_decode_repeat_flags() {
    let input = "JBSWY3DPFQQFO33SNRSCC===\n"; // spell-checker:disable-line
    new_ucmd!()
        .args(&["-didiw80", "--wrap=17", "--wrap", "8"]) // spell-checker:disable-line
        .pipe_in(input)
        .succeeds()
        .stdout_only("Hello, World!");
}

#[test]
fn test_garbage() {
    let input = "aGVsbG8sIHdvcmxkIQ==\0"; // spell-checker:disable-line
    new_ucmd!()
        .arg("-d")
        .pipe_in(input)
        .fails()
        .stderr_only("base32: error: invalid input\n");
}

#[test]
fn test_ignore_garbage() {
    for ignore_garbage_param in ["-i", "--ignore-garbage", "--ig"] {
        let input = "JBSWY\x013DPFQ\x02QFO33SNRSCC===\n"; // spell-checker:disable-line
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
    for wrap_param in ["-w", "--wrap", "--wr"] {
        let input = "The quick brown fox jumps over the lazy dog.";
        new_ucmd!()
            .arg(wrap_param)
            .arg("20")
            .pipe_in(input)
            .succeeds()
            .stdout_only(
                "KRUGKIDROVUWG2ZAMJZG\n653OEBTG66BANJ2W24DT\nEBXXMZLSEB2GQZJANRQX\nU6JAMRXWOLQ=\n", // spell-checker:disable-line
            );
    }
}

#[test]
fn test_wrap_no_arg() {
    for wrap_param in ["-w", "--wrap"] {
        let ts = TestScenario::new(util_name!());
        let expected_stderr = "a value is required for '--wrap <COLS>' but none was supplied";
        ts.ucmd()
            .arg(wrap_param)
            .fails()
            .stderr_contains(expected_stderr)
            .no_stdout();
    }
}

#[test]
fn test_wrap_bad_arg() {
    for wrap_param in ["-w", "--wrap"] {
        new_ucmd!()
            .arg(wrap_param)
            .arg("b")
            .fails()
            .stderr_only("base32: invalid wrap size: 'b'\n");
    }
}

#[test]
fn test_base32_extra_operand() {
    // Expect a failure when multiple files are specified.
    new_ucmd!()
        .arg("a.txt")
        .arg("b.txt")
        .fails()
        .usage_error("extra operand 'b.txt'");
}

#[test]
fn test_base32_file_not_found() {
    new_ucmd!()
        .arg("a.txt")
        .fails()
        .stderr_only("base32: a.txt: No such file or directory\n");
}
