// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;
// spell-checker:ignore checkfile, testf, ntestf, heic
macro_rules! get_hash(
    ($str:expr) => (
        $str.split(' ').collect::<Vec<&str>>()[0]
    );
);

macro_rules! test_digest {
    ($id:ident) => {
        mod $id {
            use uutests::util::*;
            use uutests::util_name;
            static EXPECTED_FILE: &'static str = concat!(stringify!($id), ".expected");
            static CHECK_FILE: &'static str = concat!(stringify!($id), ".checkfile");
            static INPUT_FILE: &'static str = "input.txt";

            #[test]
            fn test_single_file() {
                let ts = TestScenario::new(util_name!());
                assert_eq!(
                    ts.fixtures.read(EXPECTED_FILE),
                    get_hash!(
                        ts.ucmd()
                            .arg(INPUT_FILE)
                            .succeeds()
                            .no_stderr()
                            .stdout_str()
                    )
                );
            }

            #[test]
            fn test_stdin() {
                let ts = TestScenario::new(util_name!());
                assert_eq!(
                    ts.fixtures.read(EXPECTED_FILE),
                    get_hash!(
                        ts.ucmd()
                            .pipe_in_fixture(INPUT_FILE)
                            .succeeds()
                            .no_stderr()
                            .stdout_str()
                    )
                );
            }

            #[test]
            fn test_stdin_with_dash_directory() {
                let ts = TestScenario::new(util_name!());
                ts.fixtures.mkdir("-");
                assert_eq!(
                    ts.fixtures.read(EXPECTED_FILE),
                    get_hash!(
                        ts.ucmd()
                            .pipe_in_fixture(INPUT_FILE)
                            .succeeds()
                            .no_stderr()
                            .stdout_str()
                    )
                );
            }

            #[test]
            fn test_check() {
                let ts = TestScenario::new(util_name!());
                println!("File content='{}'", ts.fixtures.read(INPUT_FILE));
                println!("Check file='{}'", ts.fixtures.read(CHECK_FILE));

                ts.ucmd()
                    .args(&["--check", CHECK_FILE])
                    .succeeds()
                    .no_stderr()
                    .stdout_is("input.txt: OK\n");
            }

            #[test]
            fn test_zero() {
                let ts = TestScenario::new(util_name!());
                assert_eq!(
                    ts.fixtures.read(EXPECTED_FILE),
                    get_hash!(
                        ts.ucmd()
                            .arg("--zero")
                            .arg(INPUT_FILE)
                            .succeeds()
                            .no_stderr()
                            .stdout_str()
                    )
                );
            }

            #[test]
            fn test_missing_file() {
                let ts = TestScenario::new(util_name!());
                let at = &ts.fixtures;

                at.write("a", "file1\n");
                at.write("c", "file3\n");

                ts.ucmd()
                    .args(&["a", "b", "c"])
                    .fails()
                    .stdout_contains("a\n")
                    .stdout_contains("c\n")
                    .stderr_contains("b: No such file or directory");
            }
        }
    };
}

test_digest! {sha256}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_conflicting_arg() {
    new_ucmd!().arg("--tag").arg("--check").fails_with_code(1);
    new_ucmd!().arg("--tag").arg("--text").fails_with_code(1);
}

#[test]
fn test_tag() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("foobar", "foo bar\n");
    scene
        .ccmd("sha256sum")
        .arg("--tag")
        .arg("foobar")
        .succeeds()
        .stdout_is(
            "SHA256 (foobar) = 1f2ec52b774368781bed1d1fb140a92e0eb6348090619c9291f9a5a3c8e8d151\n",
        );
}

#[test]
fn test_sha256_binary() {
    let ts = TestScenario::new(util_name!());
    assert_eq!(
        ts.fixtures.read("binary.sha256.expected"),
        get_hash!(
            ts.ucmd()
                .arg("binary.png")
                .succeeds()
                .no_stderr()
                .stdout_str()
        )
    );
}

#[test]
fn test_sha256_stdin_binary() {
    let ts = TestScenario::new(util_name!());
    assert_eq!(
        ts.fixtures.read("binary.sha256.expected"),
        get_hash!(
            ts.ucmd()
                .pipe_in_fixture("binary.png")
                .succeeds()
                .no_stderr()
                .stdout_str()
        )
    );
}

#[test]
fn test_check_sha256_binary() {
    new_ucmd!()
        .args(&["--check", "binary.sha256.checkfile"])
        .succeeds()
        .no_stderr()
        .stdout_is("binary.png: OK\n");
}

// Regression test for https://github.com/uutils/coreutils/issues/6655
// On Windows, `sha256sum --check` applied CRLF -> LF conversion while
// generation hashed raw bytes, so verifying any binary file containing
// "\r\n" (HEIC, TTF, PNG, ...) against a checksum file generated by the
// same tool always reported FAILED.
#[test]
fn test_check_binary_files_with_crlf_bytes() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    // Bytes around offset 43134 of a real TTF file, taken from the issue.
    at.write_bytes("a.fake-ttf", b"\x00\x00\r\n\x00\x00");
    at.write_bytes("b.fake-heic", b"HEIC\r\n\xff\x00\r\n*");

    let untagged = ts.ucmd().args(&["a.fake-ttf", "b.fake-heic"]).succeeds();
    untagged.stdout_is(
        "1c671d7322d49cd2726475f4b8a8b50f27b454789e23a31c6ac14014740d8e58  a.fake-ttf\n\
         a54c776c4b43597b7f043ff59cd0a36753764eb6edec3c00a99dc54adcfbccbc  b.fake-heic\n",
    );
    at.write_bytes("hash.256", untagged.stdout());

    let tagged = ts
        .ucmd()
        .args(&["--tag", "a.fake-ttf", "b.fake-heic"])
        .succeeds();
    at.write_bytes("hash-tag.256", tagged.stdout());

    for checksum_file in ["hash.256", "hash-tag.256"] {
        ts.ucmd()
            .args(&["--check", checksum_file])
            .succeeds()
            .stdout_only("a.fake-ttf: OK\nb.fake-heic: OK\n");
    }
}
