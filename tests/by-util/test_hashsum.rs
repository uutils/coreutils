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
fn test_check_md5_ignore_missing() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("testf", "foobar\n");
    at.write(
        "testf.sha1",
        "14758f1afd44c09b7992073ccf00b43d  testf\n14758f1afd44c09b7992073ccf00b43d  testf2\n",
    );
    scene
        .ccmd("md5sum")
        .arg("-c")
        .arg(at.subdir.join("testf.sha1"))
        .fails()
        .stdout_contains("testf2: FAILED open or read");

    scene
        .ccmd("md5sum")
        .arg("-c")
        .arg("--ignore-missing")
        .arg(at.subdir.join("testf.sha1"))
        .succeeds()
        .stdout_is("testf: OK\n")
        .stderr_is("");

    scene
        .ccmd("md5sum")
        .arg("--ignore-missing")
        .arg(at.subdir.join("testf.sha1"))
        .fails()
        .stderr_contains("the --ignore-missing option is meaningful only when verifying checksums");
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
        .fails()
        .stdout_is("sha1sum: testf: No such file or directory\ntestf: FAILED open or read\n")
        .stderr_is("sha1sum: WARNING: 1 listed file could not be read\n");
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
fn test_conflicting_arg() {
    new_ucmd!()
        .arg("--tag")
        .arg("--check")
        .arg("--md5")
        .fails()
        .code_is(1);
    new_ucmd!()
        .arg("--tag")
        .arg("--text")
        .arg("--md5")
        .fails()
        .code_is(1);
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
#[cfg(not(windows))]
fn test_with_escape_filename() {
    let scene = TestScenario::new(util_name!());

    let at = &scene.fixtures;
    let filename = "a\nb";
    at.touch(filename);
    let result = scene.ccmd("md5sum").arg("--text").arg(filename).succeeds();
    let stdout = result.stdout_str();
    println!("stdout {}", stdout);
    assert!(stdout.starts_with('\\'));
    assert!(stdout.trim().ends_with("a\\nb"));
}

#[test]
#[cfg(not(windows))]
fn test_with_escape_filename_zero_text() {
    let scene = TestScenario::new(util_name!());

    let at = &scene.fixtures;
    let filename = "a\nb";
    at.touch(filename);
    let result = scene
        .ccmd("md5sum")
        .arg("--text")
        .arg("--zero")
        .arg(filename)
        .succeeds();
    let stdout = result.stdout_str();
    println!("stdout {}", stdout);
    assert!(!stdout.starts_with('\\'));
    assert!(stdout.contains("a\nb"));
}

#[test]
fn test_check_empty_line() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");
    at.write(
        "in.md5",
        "d41d8cd98f00b204e9800998ecf8427e  f\n\nd41d8cd98f00b204e9800998ecf8427e  f\ninvalid\n\n",
    );
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg(at.subdir.join("in.md5"))
        .succeeds()
        .stderr_contains("WARNING: 1 line is improperly formatted");
}

#[test]
#[cfg(not(windows))]
fn test_check_with_escape_filename() {
    let scene = TestScenario::new(util_name!());

    let at = &scene.fixtures;

    let filename = "a\nb";
    at.touch(filename);
    let result = scene.ccmd("md5sum").arg("--tag").arg(filename).succeeds();
    let stdout = result.stdout_str();
    println!("stdout {}", stdout);
    assert!(stdout.starts_with("\\MD5"));
    assert!(stdout.contains("a\\nb"));
    at.write("check.md5", stdout);
    let result = scene
        .ccmd("md5sum")
        .arg("--strict")
        .arg("-c")
        .arg("check.md5")
        .succeeds();
    result.stdout_is("\\a\\nb: OK\n");
}

#[test]
fn test_check_strict_error() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("f", "");
    at.write(
        "in.md5",
        "ERR\nERR\nd41d8cd98f00b204e9800998ecf8427e  f\nERR\n",
    );
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg("--strict")
        .arg(at.subdir.join("in.md5"))
        .fails()
        .stderr_contains("WARNING: 3 lines are improperly formatted");
}

#[test]
fn test_check_warn() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("f", "");
    at.write(
        "in.md5",
        "d41d8cd98f00b204e9800998ecf8427e  f\nd41d8cd98f00b204e9800998ecf8427e  f\ninvalid\n",
    );
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg("--warn")
        .arg(at.subdir.join("in.md5"))
        .succeeds()
        .stderr_contains("in.md5: 3: improperly formatted MD5 checksum line")
        .stderr_contains("WARNING: 1 line is improperly formatted");

    // with strict, we should fail the execution
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg("--strict")
        .arg(at.subdir.join("in.md5"))
        .fails();
}

#[test]
fn test_check_status() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("f", "");
    at.write("in.md5", "MD5(f)= d41d8cd98f00b204e9800998ecf8427f\n");
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg("--status")
        .arg(at.subdir.join("in.md5"))
        .fails()
        .no_output();
}

#[test]
fn test_check_status_code() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("f", "");
    at.write("in.md5", "d41d8cd98f00b204e9800998ecf8427f  f\n");
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg("--status")
        .arg(at.subdir.join("in.md5"))
        .fails()
        .stderr_is("")
        .stdout_is("");
}

#[test]
fn test_check_no_backslash_no_space() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("f", "");
    at.write("in.md5", "MD5(f)= d41d8cd98f00b204e9800998ecf8427e\n");
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg(at.subdir.join("in.md5"))
        .succeeds()
        .stdout_is("f: OK\n");
}

#[test]
fn test_check_check_ignore_no_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("f", "");
    at.write("in.md5", "d41d8cd98f00b204e9800998ecf8427f  missing\n");
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg("--ignore-missing")
        .arg(at.subdir.join("in.md5"))
        .fails()
        .stderr_contains("in.md5: no file was verified");
}
