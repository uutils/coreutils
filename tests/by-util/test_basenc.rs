// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (encodings) lsbf msbf
// spell-checker:ignore autopad MFRGG MFRGGZDF abcdeabc baddecode CPNMUO

use rstest::rstest;
use uutests::{at_and_ucmd, new_ucmd};

#[test]
fn test_version() {
    new_ucmd!()
        .arg("--version")
        .succeeds()
        .no_stderr()
        .stdout_is(format!("basenc {}\n", uucore::crate_version!()));
}

#[test]
fn test_z85_not_padded_decode() {
    // The z85 crate deviates from the standard in some cases; we have to catch those
    new_ucmd!()
        .args(&["--z85", "-d"])
        .pipe_in("##########")
        .fails()
        .stderr_only("basenc: invalid input\n");
}

#[test]
fn test_z85_not_padded_encode() {
    // The z85 crate deviates from the standard in some cases; we have to catch those
    new_ucmd!()
        .args(&["--z85"])
        .pipe_in("123")
        .fails()
        .stderr_only("basenc: invalid input (length must be multiple of 4 characters)\n");
}

#[test]
fn test_invalid_input() {
    let cmd = new_ucmd!().args(&["--base32", "."]).fails();
    if cfg!(windows) {
        cmd.stderr_only("basenc: .: Permission denied\n");
    } else if std::env::var("UUTESTS_WASM_RUNNER").is_ok() {
        // wasi-libc may report a different error string than the host libc
        cmd.stderr_contains("basenc: read error:");
    } else {
        cmd.stderr_only("basenc: read error: Is a directory\n");
    }
}

#[rstest]
#[case::base64(&["--base64"], "to>be?", "dG8+YmU/\n")]
#[case::base64_decode(&["--base64", "-d"], "dG8+YmU/", "to>be?")]
#[case::base64url(&["--base64url"], "to>be?", "dG8-YmU_\n")]
#[case::base64url_decode(&["--base64url", "-d"], "dG8-YmU_", "to>be?")]
#[case::base32_autopad_short_quantum(&["--base32", "--decode"], "MFRGG", "abc")]
#[case::base32_autopad_multiline_stream(&["--base32", "--decode"], "MFRGGZDF\nMFRGG", "abcdeabc")]
#[case::base32hex_autopad_short_quantum(&["--base32hex", "--decode"], "C5H66", "abc")]
#[case::base16(&["--base16"], "Hello, World!", "48656C6C6F2C20576F726C6421\n")]
#[case::base16_decode(&["--base16", "-d"], "48656C6C6F2C20576F726C6421", "Hello, World!")]
#[case::base16_decode_lowercase(&["--base16", "-d"], "48656c6c6f2c20576f726c6421", "Hello, World!")]
#[case::base16_decode_and_ignore_garbage_lowercase(&["--base16", "-d", "-i"], "48656c6c6f2c20576f726c6421", "Hello, World!")]
#[case::base2msbf(&["--base2msbf"], "msbf", "01101101011100110110001001100110\n")]
#[case::base2msbf_decode(&["--base2msbf", "-d"], "01101101011100110110001001100110", "msbf")]
#[case::base2lsbf(&["--base2lsbf"], "lsbf", "00110110110011100100011001100110\n")]
#[case::base2lsbf_decode(&["--base2lsbf", "-d"], "00110110110011100100011001100110", "lsbf")]
#[case::z85_decode(&["--z85", "-d"], "nm=QNz.92jz/PV8", "Hello, World")]
#[case::base58(&["--base58"], "Hello, World!", "72k1xXWG59fYdzSNoA\n")]
#[case::base58_decode(&["--base58", "-d"], "72k1xXWG59fYdzSNoA", "Hello, World!")]
#[case::choose_last_encoding_base2lsbf(&["--base64url", "--base16", "--base2msbf", "--base32", "--base64", "--z85", "--base32hex", "--base2lsbf"], "lsbf", "00110110110011100100011001100110\n")]
#[case::choose_last_encoding_base58(&["--base64", "--base32", "--base16", "--z85", "--base58"], "Hello!", "d3yC1LKr\n")]
fn test_basenc_stdin(#[case] args: &[&str], #[case] input: &str, #[case] expected: &str) {
    new_ucmd!()
        .args(args)
        .pipe_in(input)
        .succeeds()
        .stdout_only(expected);
}

#[test]
fn test_base32() {
    new_ucmd!()
        .arg("--base32")
        .pipe_in("nice>base?")
        .succeeds()
        .stdout_only("NZUWGZJ6MJQXGZJ7\n"); // spell-checker:disable-line
}

#[test]
fn test_base32_decode() {
    new_ucmd!()
        .args(&["--base32", "-d"])
        .pipe_in("NZUWGZJ6MJQXGZJ7") // spell-checker:disable-line
        .succeeds()
        .stdout_only("nice>base?");
}

#[test]
fn test_base32hex() {
    new_ucmd!()
        .arg("--base32hex")
        .pipe_in("nice>base?")
        .succeeds()
        .stdout_only("DPKM6P9UC9GN6P9V\n"); // spell-checker:disable-line
}

#[test]
fn test_base32hex_decode() {
    new_ucmd!()
        .args(&["--base32hex", "-d"])
        .pipe_in("DPKM6P9UC9GN6P9V") // spell-checker:disable-line
        .succeeds()
        .stdout_only("nice>base?");
}

#[test]
fn test_base32_baddecode_keeps_prefix() {
    new_ucmd!()
        .args(&["--base32", "--decode"])
        .pipe_in("MFRGGZDF=")
        .fails()
        .stdout_is("abcde")
        .stderr_is("basenc: invalid input\n");
}

#[test]
fn test_base32hex_rejects_trailing_garbage() {
    new_ucmd!()
        .args(&["--base32hex", "-d"])
        .pipe_in("VNC0FKD5W")
        .fails()
        .stdout_is_bytes(b"\xFD\xD8\x07\xD1\xA5")
        .stderr_is("basenc: invalid input\n");
}

#[test]
fn test_base32hex_truncated_block_keeps_prefix() {
    new_ucmd!()
        .args(&["--base32hex", "-d"])
        .pipe_in("CPNMUO")
        .fails()
        .stdout_is_bytes(b"foo")
        .stderr_is("basenc: invalid input\n");
}

// This test fails if the Base16 output buffer is not explicitly flushed.
#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI sandbox: host paths not visible")]
fn test_base16_write_error_is_reported() {
    new_ucmd!()
        .arg("--base16")
        .pipe_in("Hello, World!")
        .set_stdout(std::fs::File::create("/dev/full").unwrap())
        .fails()
        .stderr_is("basenc: No space left on device\n");
}

#[test]
fn test_base58_large_file_no_chunking() {
    // Regression test: base58 must process entire input as one big integer,
    // not in 1024-byte chunks. This test ensures files >1024 bytes work correctly.
    let (at, mut ucmd) = at_and_ucmd!();
    let filename = "large_file.txt";

    // spell-checker:disable
    let input = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(50);
    // spell-checker:enable
    at.write(filename, &input);

    let result = ucmd.arg("--base58").arg(filename).succeeds();
    let encoded = result.stdout_str();

    // Verify the output ends with the expected suffix (matches GNU basenc output)
    // spell-checker:disable
    assert!(
        encoded
            .trim_end()
            .ends_with("ZNRRacEnhrY83ZEYkpwWVZNFK5DFRasr\nw693NsNGtiQ9fYAj")
    );
    // spell-checker:enable
}

#[test]
fn test_choose_last_encoding_base64() {
    new_ucmd!()
        .args(&[
            "--base2msbf",
            "--base2lsbf",
            "--base64url",
            "--base32hex",
            "--base32",
            "--base16",
            "--z85",
            "--base64",
        ])
        .pipe_in("Hello, World!")
        .succeeds()
        .stdout_only("SGVsbG8sIFdvcmxkIQ==\n"); // spell-checker:disable-line
}

#[test]
fn test_base32_decode_repeated() {
    new_ucmd!()
        .args(&[
            "--ignore",
            "--wrap=80",
            "--base32hex",
            "--z85",
            "--ignore",
            "--decode",
            "--z85",
            "--base32",
            "-w",
            "10",
        ])
        .pipe_in("NZUWGZJ6MJQXGZJ7") // spell-checker:disable-line
        .succeeds()
        .stdout_only("nice>base?");
}

// The restriction that input length has to be divisible by 4 only applies to data being encoded with Z85, not to the
// decoding of Z85-encoded data
#[test]
fn test_z85_length_check() {
    new_ucmd!()
        .args(&["--decode", "--z85"])
        // Input has length 10, not divisible by 4
        // spell-checker:disable-next-line
        .pipe_in("f!$Kwh8WxM")
        .succeeds()
        .stdout_only("12345678");
}

#[test]
fn test_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let filename = "file";

    at.write(filename, "foo");

    ucmd.arg(filename)
        .arg("--base64")
        .succeeds()
        .stdout_is("Zm9v\n");
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: argv/filenames must be valid UTF-8")]
fn test_file_with_non_utf8_name() {
    use std::os::unix::ffi::OsStringExt;
    let (at, mut ucmd) = at_and_ucmd!();

    let filename = std::ffi::OsString::from_vec(vec![0xFF, 0xFE]);
    std::fs::write(at.plus(&filename), b"foo").unwrap();

    ucmd.arg(filename)
        .arg("--base64")
        .succeeds()
        .stdout_is("Zm9v\n");
}
