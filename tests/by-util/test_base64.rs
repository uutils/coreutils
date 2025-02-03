// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_encode() {
    let input = "hello, world!";
    new_ucmd!()
        .pipe_in(input)
        .succeeds()
        .stdout_only("aGVsbG8sIHdvcmxkIQ==\n"); // spell-checker:disable-line

    // Using '-' as our file
    new_ucmd!()
        .arg("-")
        .pipe_in(input)
        .succeeds()
        .stdout_only("aGVsbG8sIHdvcmxkIQ==\n"); // spell-checker:disable-line
}

#[test]
fn test_encode_repeat_flags_later_wrap_10() {
    let input = "hello, world!";
    new_ucmd!()
        .args(&["-ii", "-w15", "-w10"])
        .pipe_in(input)
        .succeeds()
        .stdout_only("aGVsbG8sIH\ndvcmxkIQ==\n"); // spell-checker:disable-line
}

#[test]
fn test_encode_repeat_flags_later_wrap_15() {
    let input = "hello, world!";
    new_ucmd!()
        .args(&["-ii", "-w10", "-w15"])
        .pipe_in(input)
        .succeeds()
        .stdout_only("aGVsbG8sIHdvcmx\nkIQ==\n"); // spell-checker:disable-line
}

#[test]
fn test_decode_short() {
    let input = "aQ";
    new_ucmd!()
        .args(&["--decode"])
        .pipe_in(input)
        .succeeds()
        .stdout_only("i");
}

#[test]
fn test_multi_lines() {
    let input = ["aQ\n\n\n", "a\nQ==\n\n\n"];
    for i in input {
        new_ucmd!()
            .args(&["--decode"])
            .pipe_in(i)
            .succeeds()
            .stdout_only("i");
    }
}

#[test]
fn test_base64_encode_file() {
    new_ucmd!()
        .arg("input-simple.txt")
        .succeeds()
        .stdout_only("SGVsbG8sIFdvcmxkIQo=\n"); // spell-checker:disable-line
}

#[test]
fn test_decode() {
    for decode_param in ["-d", "--decode", "--dec"] {
        let input = "aGVsbG8sIHdvcmxkIQ=="; // spell-checker:disable-line
        new_ucmd!()
            .arg(decode_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("hello, world!");
    }
}

#[test]
fn test_decode_repeat_flags() {
    let input = "aGVsbG8sIHdvcmxkIQ==\n"; // spell-checker:disable-line
    new_ucmd!()
        .args(&["-didiw80", "--wrap=17", "--wrap", "8"]) // spell-checker:disable-line
        .pipe_in(input)
        .succeeds()
        .stdout_only("hello, world!");
}

#[test]
fn test_garbage() {
    let input = "aGVsbG8sIHdvcmxkIQ==\0"; // spell-checker:disable-line
    new_ucmd!()
        .arg("-d")
        .pipe_in(input)
        .fails()
        .stderr_only("base64: error: invalid input\n");
}

#[test]
fn test_ignore_garbage() {
    for ignore_garbage_param in ["-i", "--ignore-garbage", "--ig"] {
        let input = "aGVsbG8sIHdvcmxkIQ==\0"; // spell-checker:disable-line
        new_ucmd!()
            .arg("-d")
            .arg(ignore_garbage_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("hello, world!");
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
            // spell-checker:disable-next-line
            .stdout_only("VGhlIHF1aWNrIGJyb3du\nIGZveCBqdW1wcyBvdmVy\nIHRoZSBsYXp5IGRvZy4=\n");
    }
    let input = "hello, world";
    new_ucmd!()
        .args(&["--wrap", "0"])
        .pipe_in(input)
        .succeeds()
        .stdout_only("aGVsbG8sIHdvcmxk"); // spell-checker:disable-line
    new_ucmd!()
        .args(&["--wrap", "30"])
        .pipe_in(input)
        .succeeds()
        .stdout_only("aGVsbG8sIHdvcmxk\n"); // spell-checker:disable-line
}

#[test]
fn test_wrap_no_arg() {
    for wrap_param in ["-w", "--wrap"] {
        new_ucmd!()
            .arg(wrap_param)
            .fails()
            .stderr_contains("a value is required for '--wrap <COLS>' but none was supplied")
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
            .stderr_only("base64: invalid wrap size: 'b'\n");
    }
}

#[test]
fn test_base64_extra_operand() {
    // Expect a failure when multiple files are specified.
    new_ucmd!()
        .arg("a.txt")
        .arg("b.txt")
        .fails()
        .usage_error("extra operand 'b.txt'");
}

#[test]
fn test_base64_file_not_found() {
    new_ucmd!()
        .arg("a.txt")
        .fails()
        .stderr_only("base64: a.txt: No such file or directory\n");
}

#[test]
fn test_no_repeated_trailing_newline() {
    new_ucmd!()
        .args(&["--wrap", "10", "--", "-"])
        .pipe_in("The quick brown fox jumps over the lazy dog.")
        .succeeds()
        .stdout_only(
            // cSpell:disable
            "\
VGhlIHF1aW
NrIGJyb3du
IGZveCBqdW
1wcyBvdmVy
IHRoZSBsYX
p5IGRvZy4=
",
            // cSpell:enable
        );
}

#[test]
fn test_wrap_default() {
    const PIPE_IN: &str = "The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog.";

    new_ucmd!()
        .args(&["--", "-"])
        .pipe_in(PIPE_IN)
        .succeeds()
        .stdout_only(
            // cSpell:disable
            "\
VGhlIHF1aWNrIGJyb3duIGZveCBqdW1wcyBvdmVyIHRoZSBsYXp5IGRvZy4gVGhlIHF1aWNrIGJy
b3duIGZveCBqdW1wcyBvdmVyIHRoZSBsYXp5IGRvZy4gVGhlIHF1aWNrIGJyb3duIGZveCBqdW1w
cyBvdmVyIHRoZSBsYXp5IGRvZy4=
",
            // cSpell:enable
        );
}

// Prevent regression to:
//
// ❯ coreutils manpage base64 | rg --fixed-strings -- 'base32'
// The data are encoded as described for the base32 alphabet in RFC 4648.
// to the bytes of the formal base32 alphabet. Use \-\-ignore\-garbage
// The data are encoded as described for the base32 alphabet in RFC 4648.
// to the bytes of the formal base32 alphabet. Use \-\-ignore\-garbage
#[test]
fn test_manpage() {
    use std::process::{Command, Stdio};

    let test_scenario = TestScenario::new("");

    let child = Command::new(&test_scenario.bin_path)
        .arg("manpage")
        .arg("base64")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let output = child.wait_with_output().unwrap();

    assert_eq!(output.status.code().unwrap(), 0);

    assert!(output.stderr.is_empty());

    let stdout_str = std::str::from_utf8(&output.stdout).unwrap();

    assert!(stdout_str.contains("base64 alphabet"));

    assert!(!stdout_str.to_ascii_lowercase().contains("base32"));
}
