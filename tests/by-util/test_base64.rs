// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore unpadded, QUJD, baddecode

#[cfg(target_os = "linux")]
use uutests::at_and_ucmd;
use uutests::new_ucmd;

#[test]
fn test_version() {
    new_ucmd!()
        .arg("--version")
        .succeeds()
        .no_stderr()
        .stdout_is(format!("base64 {}\n", uucore::crate_version!()));
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: argv/filenames must be valid UTF-8")]
fn test_base64_non_utf8_paths() {
    use std::os::unix::ffi::OsStringExt;
    let (at, mut ucmd) = at_and_ucmd!();

    let filename = std::ffi::OsString::from_vec(vec![0xFF, 0xFE]);
    std::fs::write(at.plus(&filename), b"hello world").unwrap();

    ucmd.arg(&filename)
        .succeeds()
        .stdout_is("aGVsbG8gd29ybGQ=\n");
}

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
    for decode_param in ["-d", "--decode", "--dec", "-D"] {
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
fn test_decode_padded_block_followed_by_unpadded_tail() {
    new_ucmd!()
        .arg("--decode")
        .pipe_in("MTIzNA==MTIzNA")
        .succeeds()
        .stdout_only("12341234");
}

#[test]
fn test_decode_padded_block_followed_by_aligned_tail() {
    new_ucmd!()
        .arg("--decode")
        .pipe_in("MTIzNA==QUJD")
        .succeeds()
        .stdout_only("1234ABC");
}

#[test]
fn test_decode_unpadded_stream_without_equals() {
    new_ucmd!()
        .arg("--decode")
        .pipe_in("MTIzNA")
        .succeeds()
        .stdout_only("1234");
}

#[test]
fn test_garbage() {
    let input = "aGVsbG8sIHdvcmxkIQ==\0"; // spell-checker:disable-line
    new_ucmd!()
        .arg("-d")
        .pipe_in(input)
        .fails()
        .stdout_is("hello, world!")
        .stderr_is("base64: error: invalid input\n");
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
fn test_ignore_garbage_decodes_prefix_before_final_error() {
    // https://github.com/uutils/coreutils/issues/12923
    // With --ignore-garbage, GNU skips the '.' and keeps decoding the rest of
    // the stream, only erroring once it hits the truncated trailing quantum.
    let input = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJqdGkiOiJjMjQ5MmF";
    new_ucmd!()
        .arg("-di")
        .pipe_in(input)
        .fails()
        .stdout_is("{\"alg\":\"RS256\",\"typ\":\"JWT\"}{\"jti\":\"c2492a")
        .stderr_is("base64: error: invalid input\n");
}

#[test]
fn test_decode_trailing_remainder_canonical() {
    // A trailing 2-character group whose unused padding bits are already
    // zero decodes cleanly, with no error, even without explicit '=' padding.
    new_ucmd!()
        .arg("-d")
        .pipe_in("aQ")
        .succeeds()
        .stdout_only("i");
}

#[test]
fn test_decode_trailing_remainder_non_canonical() {
    // A trailing 2-character group whose unused padding bits are non-zero
    // still decodes (GNU doesn't discard the byte it managed to produce),
    // but is reported as invalid input.
    new_ucmd!()
        .arg("-d")
        .pipe_in("XY")
        .fails()
        .stdout_is("]")
        .stderr_is("base64: error: invalid input\n");
}

#[test]
fn test_decode_explicit_padding_non_canonical() {
    // Same leniency as the trailing-remainder case, but for a quantum that's
    // already a full 4 characters with an explicit '=' (from GNU's own
    // basenc/base64.pl: baddecode6/baddecode7).
    new_ucmd!()
        .arg("-d")
        .pipe_in("SB==")
        .fails()
        .stdout_is("H")
        .stderr_is("base64: error: invalid input\n");

    new_ucmd!()
        .arg("-d")
        .pipe_in("SGVsbG9=")
        .fails()
        .stdout_is("Hello")
        .stderr_is("base64: error: invalid input\n");
}

#[test]
fn test_decode_insufficient_padding_recovers_preceding_data() {
    // A trailing '=' without enough characters left to complete the padded
    // quantum (e.g. "NA=" would need to be "NA==") isn't a valid quantum at
    // all, but GNU still recovers whatever precedes it by decoding that much
    // as an unpadded remainder (from GNU's own basenc/base64.pl: baddecode8).
    new_ucmd!()
        .arg("-d")
        .pipe_in("MTIzNA=")
        .fails()
        .stdout_is("1234")
        .stderr_is("base64: error: invalid input\n");
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
            .stderr_contains("error: a value is required for '--wrap <COLS>' but none was supplied")
            .stderr_contains("For more information, try '--help'.")
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

#[test]
#[cfg(all(target_os = "linux", not(target_env = "musl")))]
#[cfg_attr(wasi_runner, ignore = "WASI sandbox: host paths not visible")]
fn test_read_error() {
    new_ucmd!()
        .arg("/proc/self/mem")
        .fails()
        .stderr_is("base64: read error: Input/output error\n");
}

#[test]
#[cfg(target_os = "linux")]
fn test_base64_file_with_trailing_slash() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write("a", "b");

    ucmd.arg("a/")
        .fails()
        .stderr_only("base64: a/: Not a directory\n");
}
