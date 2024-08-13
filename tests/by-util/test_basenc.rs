// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//spell-checker: ignore (encodings) lsbf msbf
use crate::common::util::TestScenario;

#[test]
fn test_z85_not_padded() {
    // The z85 crate deviates from the standard in some cases; we have to catch those
    new_ucmd!()
        .args(&["--z85", "-d"])
        .pipe_in("##########")
        .fails()
        .stderr_only("basenc: error: invalid input\n");
    new_ucmd!()
        .args(&["--z85"])
        .pipe_in("123")
        .fails()
        .stderr_only("basenc: error: invalid input (length must be multiple of 4 characters)\n");
}

#[test]
fn test_invalid_input() {
    let error_message = if cfg!(windows) {
        "basenc: .: Permission denied\n"
    } else {
        "basenc: error: invalid input\n"
    };
    new_ucmd!()
        .args(&["--base32", "."])
        .fails()
        .stderr_only(error_message);
}

#[test]
fn test_base64() {
    new_ucmd!()
        .arg("--base64")
        .pipe_in("to>be?")
        .succeeds()
        .no_stderr()
        .stdout_only("dG8+YmU/\n");
}

#[test]
fn test_base64_decode() {
    new_ucmd!()
        .args(&["--base64", "-d"])
        .pipe_in("dG8+YmU/")
        .succeeds()
        .no_stderr()
        .stdout_only("to>be?");
}

#[test]
fn test_base64url() {
    new_ucmd!()
        .arg("--base64url")
        .pipe_in("to>be?")
        .succeeds()
        .no_stderr()
        .stdout_only("dG8-YmU_\n");
}

#[test]
fn test_base64url_decode() {
    new_ucmd!()
        .args(&["--base64url", "-d"])
        .pipe_in("dG8-YmU_")
        .succeeds()
        .no_stderr()
        .stdout_only("to>be?");
}

#[test]
fn test_base32() {
    new_ucmd!()
        .arg("--base32")
        .pipe_in("nice>base?")
        .succeeds()
        .no_stderr()
        .stdout_only("NZUWGZJ6MJQXGZJ7\n"); // spell-checker:disable-line
}

#[test]
fn test_base32_decode() {
    new_ucmd!()
        .args(&["--base32", "-d"])
        .pipe_in("NZUWGZJ6MJQXGZJ7") // spell-checker:disable-line
        .succeeds()
        .no_stderr()
        .stdout_only("nice>base?");
}

#[test]
fn test_base32hex() {
    new_ucmd!()
        .arg("--base32hex")
        .pipe_in("nice>base?")
        .succeeds()
        .no_stderr()
        .stdout_only("DPKM6P9UC9GN6P9V\n"); // spell-checker:disable-line
}

#[test]
fn test_base32hex_decode() {
    new_ucmd!()
        .args(&["--base32hex", "-d"])
        .pipe_in("DPKM6P9UC9GN6P9V") // spell-checker:disable-line
        .succeeds()
        .no_stderr()
        .stdout_only("nice>base?");
}

#[test]
fn test_base16() {
    new_ucmd!()
        .arg("--base16")
        .pipe_in("Hello, World!")
        .succeeds()
        .no_stderr()
        .stdout_only("48656C6C6F2C20576F726C6421\n");
}

#[test]
fn test_base16_decode() {
    new_ucmd!()
        .args(&["--base16", "-d"])
        .pipe_in("48656C6C6F2C20576F726C6421")
        .succeeds()
        .no_stderr()
        .stdout_only("Hello, World!");
}

#[test]
fn test_base2msbf() {
    new_ucmd!()
        .arg("--base2msbf")
        .pipe_in("msbf")
        .succeeds()
        .no_stderr()
        .stdout_only("01101101011100110110001001100110\n");
}

#[test]
fn test_base2msbf_decode() {
    new_ucmd!()
        .args(&["--base2msbf", "-d"])
        .pipe_in("01101101011100110110001001100110")
        .succeeds()
        .no_stderr()
        .stdout_only("msbf");
}

#[test]
fn test_base2lsbf() {
    new_ucmd!()
        .arg("--base2lsbf")
        .pipe_in("lsbf")
        .succeeds()
        .no_stderr()
        .stdout_only("00110110110011100100011001100110\n");
}

#[test]
fn test_base2lsbf_decode() {
    new_ucmd!()
        .args(&["--base2lsbf", "-d"])
        .pipe_in("00110110110011100100011001100110")
        .succeeds()
        .no_stderr()
        .stdout_only("lsbf");
}

#[test]
fn test_choose_last_encoding_z85() {
    new_ucmd!()
        .args(&[
            "--base2lsbf",
            "--base2msbf",
            "--base16",
            "--base32hex",
            "--base64url",
            "--base32",
            "--base64",
            "--z85",
        ])
        .pipe_in("Hello, World")
        .succeeds()
        .no_stderr()
        .stdout_only("nm=QNz.92jz/PV8\n");
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
        .no_stderr()
        .stdout_only("SGVsbG8sIFdvcmxkIQ==\n"); // spell-checker:disable-line
}

#[test]
fn test_choose_last_encoding_base2lsbf() {
    new_ucmd!()
        .args(&[
            "--base64url",
            "--base16",
            "--base2msbf",
            "--base32",
            "--base64",
            "--z85",
            "--base32hex",
            "--base2lsbf",
        ])
        .pipe_in("lsbf")
        .succeeds()
        .no_stderr()
        .stdout_only("00110110110011100100011001100110\n");
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
        .no_stderr()
        .stdout_only("nice>base?");
}
