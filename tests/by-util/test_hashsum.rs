// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
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

// Asterisk `*` is a reserved paths character on win32, nor the path can end with a whitespace.
// ref: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#naming-conventions
#[test]
fn test_check_md5sum() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    #[cfg(not(windows))]
    {
        for f in &["a", " b", "*c", "dd", " "] {
            at.write(f, &format!("{f}\n"));
        }
        at.write(
            "check.md5sum",
            "60b725f10c9c85c70d97880dfe8191b3  a\n\
             bf35d7536c785cf06730d5a40301eba2   b\n\
             f5b61709718c1ecf8db1aea8547d4698  *c\n\
             b064a020db8018f18ff5ae367d01b212  dd\n\
             d784fa8b6d98d27699781bd9a7cf19f0   ",
        );
        scene
            .ccmd("md5sum")
            .arg("--strict")
            .arg("-c")
            .arg("check.md5sum")
            .succeeds()
            .stdout_is("a: OK\n b: OK\n*c: OK\ndd: OK\n : OK\n")
            .stderr_is("");
    }
    #[cfg(windows)]
    {
        for f in &["a", " b", "dd"] {
            at.write(f, &format!("{f}\n"));
        }
        at.write(
            "check.md5sum",
            "60b725f10c9c85c70d97880dfe8191b3  a\n\
             bf35d7536c785cf06730d5a40301eba2   b\n\
             b064a020db8018f18ff5ae367d01b212  dd",
        );
        scene
            .ccmd("md5sum")
            .arg("--strict")
            .arg("-c")
            .arg("check.md5sum")
            .succeeds()
            .stdout_is("a: OK\n b: OK\ndd: OK\n")
            .stderr_is("");
    }
}

#[test]
fn test_check_md5sum_reverse_bsd() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    #[cfg(not(windows))]
    {
        for f in &["a", " b", "*c", "dd", " "] {
            at.write(f, &format!("{f}\n"));
        }
        at.write(
            "check.md5sum",
            "60b725f10c9c85c70d97880dfe8191b3 a\n\
             bf35d7536c785cf06730d5a40301eba2  b\n\
             f5b61709718c1ecf8db1aea8547d4698 *c\n\
             b064a020db8018f18ff5ae367d01b212 dd\n\
             d784fa8b6d98d27699781bd9a7cf19f0  ",
        );
        scene
            .ccmd("md5sum")
            .arg("--strict")
            .arg("-c")
            .arg("check.md5sum")
            .succeeds()
            .stdout_is("a: OK\n b: OK\n*c: OK\ndd: OK\n : OK\n")
            .stderr_is("");
    }
    #[cfg(windows)]
    {
        for f in &["a", " b", "dd"] {
            at.write(f, &format!("{f}\n"));
        }
        at.write(
            "check.md5sum",
            "60b725f10c9c85c70d97880dfe8191b3 a\n\
             bf35d7536c785cf06730d5a40301eba2  b\n\
             b064a020db8018f18ff5ae367d01b212 dd",
        );
        scene
            .ccmd("md5sum")
            .arg("--strict")
            .arg("-c")
            .arg("check.md5sum")
            .succeeds()
            .stdout_is("a: OK\n b: OK\ndd: OK\n")
            .stderr_is("");
    }
}

#[test]
fn test_check_md5sum_mixed_format() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    #[cfg(not(windows))]
    {
        for f in &[" b", "*c", "dd", " "] {
            at.write(f, &format!("{f}\n"));
        }
        at.write(
            "check.md5sum",
            "bf35d7536c785cf06730d5a40301eba2  b\n\
             f5b61709718c1ecf8db1aea8547d4698 *c\n\
             b064a020db8018f18ff5ae367d01b212 dd\n\
             d784fa8b6d98d27699781bd9a7cf19f0  ",
        );
    }
    #[cfg(windows)]
    {
        for f in &[" b", "dd"] {
            at.write(f, &format!("{f}\n"));
        }
        at.write(
            "check.md5sum",
            "bf35d7536c785cf06730d5a40301eba2  b\n\
             b064a020db8018f18ff5ae367d01b212 dd",
        );
    }
    scene
        .ccmd("md5sum")
        .arg("--strict")
        .arg("-c")
        .arg("check.md5sum")
        .fails()
        .code_is(1);
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
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
