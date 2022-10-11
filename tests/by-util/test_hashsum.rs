use crate::common::util::*;
// spell-checker:ignore checkfile, nonames, testf
macro_rules! get_hash(
    ($str:expr) => (
        $str.split(' ').collect::<Vec<&str>>()[0]
    );
);

macro_rules! test_digest {
    ($($id:ident $t:ident $size:expr)*) => ($(

    mod $id {
        use crate::common::util::*;
        static DIGEST_ARG: &'static str = concat!("--", stringify!($t));
        static BITS_ARG: &'static str = concat!("--bits=", stringify!($size));
        static EXPECTED_FILE: &'static str = concat!(stringify!($id), ".expected");
        static CHECK_FILE: &'static str = concat!(stringify!($id), ".checkfile");

        #[test]
        fn test_single_file() {
            let ts = TestScenario::new("hashsum");
            assert_eq!(ts.fixtures.read(EXPECTED_FILE),
                       get_hash!(ts.ucmd().arg(DIGEST_ARG).arg(BITS_ARG).arg("input.txt").succeeds().no_stderr().stdout_str()));
        }

        #[test]
        fn test_stdin() {
            let ts = TestScenario::new("hashsum");
            assert_eq!(ts.fixtures.read(EXPECTED_FILE),
                       get_hash!(ts.ucmd().arg(DIGEST_ARG).arg(BITS_ARG).pipe_in_fixture("input.txt").succeeds().no_stderr().stdout_str()));
        }

        #[test]
        fn test_nonames() {
            let ts = TestScenario::new("hashsum");
            // EXPECTED_FILE has no newline character at the end
            if DIGEST_ARG == "--b3sum" {
                // Option only available on b3sum
                assert_eq!(format!("{0}\n{0}\n", ts.fixtures.read(EXPECTED_FILE)),
                       ts.ucmd().arg(DIGEST_ARG).arg(BITS_ARG).arg("--no-names").arg("input.txt").arg("-").pipe_in_fixture("input.txt")
                       .succeeds().no_stderr().stdout_str()
                       );
                }
        }

        #[test]
        fn test_check() {
            let ts = TestScenario::new("hashsum");
            ts.ucmd()
                .args(&[DIGEST_ARG, BITS_ARG, "--check", CHECK_FILE])
                .succeeds()
                .no_stderr()
                .stdout_is("input.txt: OK\n");
        }

        #[cfg(windows)]
        #[test]
        fn test_text_mode() {
            // TODO Replace this with hard-coded files that store the
            // expected output of text mode on an input file that has
            // "\r\n" line endings.
            let result = new_ucmd!()
                .args(&[DIGEST_ARG, BITS_ARG, "-b"])
                .pipe_in("a\nb\nc\n")
                .succeeds();
            let expected = result.no_stderr().stdout();
            // Replace the "*-\n" at the end of the output with " -\n".
            // The asterisk indicates that the digest was computed in
            // binary mode.
            let n = expected.len();
            let expected = [&expected[..n - 3], &[b' ', b'-', b'\n']].concat();
            new_ucmd!()
                .args(&[DIGEST_ARG, BITS_ARG, "-t"])
                .pipe_in("a\r\nb\r\nc\r\n")
                .succeeds()
                .no_stderr()
                .stdout_is(std::str::from_utf8(&expected).unwrap());
        }
    }
    )*)
}

test_digest! {
    md5 md5 128
    sha1 sha1 160
    sha224 sha224 224
    sha256 sha256 256
    sha384 sha384 384
    sha512 sha512 512
    sha3_224 sha3 224
    sha3_256 sha3 256
    sha3_384 sha3 384
    sha3_512 sha3 512
    shake128_256 shake128 256
    shake256_512 shake256 512
    b2sum b2sum 512
    b3sum b3sum 256
}

#[test]
fn test_check_sha1() {
    // To make sure that #3815 doesn't happen again
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("testf", "foobar\n");
    at.write(
        "testf.sha1",
        "988881adc9fc3655077dc2d4d757d480b5ea0e11  testf\n",
    );
    scene
        .ccmd("sha1sum")
        .arg("-c")
        .arg(at.subdir.join("testf.sha1"))
        .succeeds()
        .stdout_is("testf: OK\n")
        .stderr_is("");
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}
