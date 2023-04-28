use crate::common::util::TestScenario;
// spell-checker:ignore checkfile, nonames, testf, ntestf
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

        #[test]
        fn test_zero() {
            let ts = TestScenario::new("hashsum");
            assert_eq!(ts.fixtures.read(EXPECTED_FILE),
                       get_hash!(ts.ucmd().arg(DIGEST_ARG).arg(BITS_ARG).arg("--zero").arg("input.txt").succeeds().no_stderr().stdout_str()));
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
fn test_check_b2sum_length_option_0() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("testf", "foobar\n");
    at.write("testf.b2sum", "9e2bf63e933e610efee4a8d6cd4a9387e80860edee97e27db3b37a828d226ab1eb92a9cdd8ca9ca67a753edaf8bd89a0558496f67a30af6f766943839acf0110  testf\n");

    scene
        .ccmd("b2sum")
        .arg("--length=0")
        .arg("-c")
        .arg(at.subdir.join("testf.b2sum"))
        .succeeds()
        .stdout_only("testf: OK\n");
}

#[test]
fn test_check_b2sum_length_option_8() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("testf", "foobar\n");
    at.write("testf.b2sum", "6a  testf\n");

    scene
        .ccmd("b2sum")
        .arg("--length=8")
        .arg("-c")
        .arg(at.subdir.join("testf.b2sum"))
        .succeeds()
        .stdout_only("testf: OK\n");
}

#[test]
fn test_invalid_b2sum_length_option_not_multiple_of_8() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("testf", "foobar\n");

    scene
        .ccmd("b2sum")
        .arg("--length=9")
        .arg(at.subdir.join("testf"))
        .fails()
        .code_is(1);
}

#[test]
fn test_invalid_b2sum_length_option_too_large() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("testf", "foobar\n");

    scene
        .ccmd("b2sum")
        .arg("--length=513")
        .arg(at.subdir.join("testf"))
        .fails()
        .code_is(1);
}

#[test]
fn test_check_file_not_found_warning() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("testf", "foobar\n");
    at.write(
        "testf.sha1",
        "988881adc9fc3655077dc2d4d757d480b5ea0e11  testf\n",
    );
    at.remove("testf");
    scene
        .ccmd("sha1sum")
        .arg("-c")
        .arg(at.subdir.join("testf.sha1"))
        .succeeds()
        .stdout_is("sha1sum: testf: No such file or directory\ntestf: FAILED open or read\n")
        .stderr_is("sha1sum: warning: 1 listed file could not be read\n");
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}
