// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;
// spell-checker:ignore checkfile, nonames, testf, ntestf
macro_rules! get_hash(
    ($str:expr) => (
        $str.split(' ').collect::<Vec<&str>>()[0]
    );
);

macro_rules! test_digest {
    ($($id:ident $t:ident $size:expr)*) => ($(

    mod $id {
        use uutests::util::*;
        use uutests::util_name;
        static DIGEST_ARG: &'static str = concat!("--", stringify!($t));
        static BITS_ARG: &'static str = concat!("--bits=", stringify!($size));
        static EXPECTED_FILE: &'static str = concat!(stringify!($id), ".expected");
        static CHECK_FILE: &'static str = concat!(stringify!($id), ".checkfile");
        static INPUT_FILE: &'static str = "input.txt";

        #[test]
        fn test_single_file() {
            let ts = TestScenario::new(util_name!());
            assert_eq!(ts.fixtures.read(EXPECTED_FILE),
                       get_hash!(ts.ucmd().arg(DIGEST_ARG).arg(BITS_ARG).arg(INPUT_FILE).succeeds().no_stderr().stdout_str()));
        }

        #[test]
        fn test_stdin() {
            let ts = TestScenario::new(util_name!());
            assert_eq!(ts.fixtures.read(EXPECTED_FILE),
                       get_hash!(ts.ucmd().arg(DIGEST_ARG).arg(BITS_ARG).pipe_in_fixture(INPUT_FILE).succeeds().no_stderr().stdout_str()));
        }

        #[test]
        fn test_nonames() {
            let ts = TestScenario::new(util_name!());
            // EXPECTED_FILE has no newline character at the end
            if DIGEST_ARG == "--b3sum" {
                // Option only available on b3sum
                assert_eq!(format!("{0}\n{0}\n", ts.fixtures.read(EXPECTED_FILE)),
                       ts.ucmd().arg(DIGEST_ARG).arg(BITS_ARG).arg("--no-names").arg(INPUT_FILE).arg("-").pipe_in_fixture(INPUT_FILE)
                       .succeeds().no_stderr().stdout_str()
                       );
                }
        }

        #[test]
        fn test_check() {
            let ts = TestScenario::new(util_name!());
            println!("File content='{}'", ts.fixtures.read(INPUT_FILE));
            println!("Check file='{}'", ts.fixtures.read(CHECK_FILE));

            ts.ucmd()
                .args(&[DIGEST_ARG, BITS_ARG, "--check", CHECK_FILE])
                .succeeds()
                .no_stderr()
                .stdout_is("input.txt: OK\n");
        }

        #[test]
        fn test_zero() {
            let ts = TestScenario::new(util_name!());
            assert_eq!(ts.fixtures.read(EXPECTED_FILE),
                       get_hash!(ts.ucmd().arg(DIGEST_ARG).arg(BITS_ARG).arg("--zero").arg(INPUT_FILE).succeeds().no_stderr().stdout_str()));
        }


        #[cfg(windows)]
        #[test]
        fn test_text_mode() {
            use uutests::new_ucmd;

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
            let expected = [&expected[..n - 3], b" -\n"].concat();
            new_ucmd!()
                .args(&[DIGEST_ARG, BITS_ARG, "-t"])
                .pipe_in("a\r\nb\r\nc\r\n")
                .succeeds()
                .no_stderr()
                .stdout_is(std::str::from_utf8(&expected).unwrap());
        }

        #[test]
        fn test_missing_file() {
            let ts = TestScenario::new(util_name!());
            let at = &ts.fixtures;

            at.write("a", "file1\n");
            at.write("c", "file3\n");

            #[cfg(unix)]
            let file_not_found_str = "No such file or directory";
            #[cfg(not(unix))]
            let file_not_found_str = "The system cannot find the file specified";

            ts.ucmd()
                .args(&[DIGEST_ARG, BITS_ARG, "a", "b", "c"])
                .fails()
                .stdout_contains("a\n")
                .stdout_contains("c\n")
                .stderr_contains(format!("b: {file_not_found_str}"));
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
fn test_check_b2sum_length_duplicate() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("testf", "foobar\n");

    scene
        .ccmd("b2sum")
        .arg("--length=123")
        .arg("--length=128")
        .arg("testf")
        .succeeds()
        .stdout_contains("d6d45901dec53e65d2b55fb6e2ab67b0");
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
        .fails_with_code(1);
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
        .fails_with_code(1);
}

#[test]
fn test_check_b2sum_tag_output() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");

    scene
        .ccmd("b2sum")
        .arg("--length=0")
        .arg("--tag")
        .arg("f")
        .succeeds()
        .stdout_only("BLAKE2b (f) = 786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce\n");

    scene
        .ccmd("b2sum")
        .arg("--length=128")
        .arg("--tag")
        .arg("f")
        .succeeds()
        .stdout_only("BLAKE2b-128 (f) = cae66941d9efbd404e4d88758ea67670\n");
}

#[test]
fn test_check_b2sum_verify() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("a", "a\n");

    scene
        .ccmd("b2sum")
        .arg("--tag")
        .arg("a")
        .succeeds()
        .stdout_only("BLAKE2b (a) = bedfbb90d858c2d67b7ee8f7523be3d3b54004ef9e4f02f2ad79a1d05bfdfe49b81e3c92ebf99b504102b6bf003fa342587f5b3124c205f55204e8c4b4ce7d7c\n");

    scene
        .ccmd("b2sum")
        .arg("--tag")
        .arg("-l")
        .arg("128")
        .arg("a")
        .succeeds()
        .stdout_only("BLAKE2b-128 (a) = b93e0fc7bb21633c08bba07c5e71dc00\n");
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
        .stdout_is("testf: FAILED open or read\n")
        .stderr_is("sha1sum: testf: No such file or directory\nsha1sum: WARNING: 1 listed file could not be read\n");
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

// GNU also supports one line sep
#[test]
fn test_check_md5sum_only_one_space() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    for f in ["a", " b", "c"] {
        at.write(f, &format!("{f}\n"));
    }
    at.write(
        "check.md5sum",
        "60b725f10c9c85c70d97880dfe8191b3 a\n\
        bf35d7536c785cf06730d5a40301eba2  b\n\
        2cd6ee2c70b0bde53fbe6cac3c8b8bb1 c\n",
    );
    scene
        .ccmd("md5sum")
        .arg("--strict")
        .arg("-c")
        .arg("check.md5sum")
        .succeeds()
        .stdout_only("a: OK\n b: OK\nc: OK\n");
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
        .fails_with_code(1);
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_conflicting_arg() {
    new_ucmd!()
        .arg("--tag")
        .arg("--check")
        .arg("--md5")
        .fails_with_code(1);
    new_ucmd!()
        .arg("--tag")
        .arg("--text")
        .arg("--md5")
        .fails_with_code(1);
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
    println!("stdout {stdout}");
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
    println!("stdout {stdout}");
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
    println!("stdout {stdout}");
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

    at.touch("f");
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

    at.touch("f");
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

    at.touch("f");
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

    at.touch("f");
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
fn test_sha1_with_md5sum_should_fail() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");
    at.write("f.sha1", "SHA1 (f) = d41d8cd98f00b204e9800998ecf8427e\n");
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg(at.subdir.join("f.sha1"))
        .fails()
        .stderr_contains("f.sha1: no properly formatted checksum lines found")
        .stderr_does_not_contain("WARNING: 1 line is improperly formatted");
}

#[test]
// Disabled on Windows because of the "*"
#[cfg(not(windows))]
fn test_check_one_two_space_star() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("empty");

    // with one space, the "*" is removed
    at.write("in.md5", "d41d8cd98f00b204e9800998ecf8427e *empty\n");

    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg(at.subdir.join("in.md5"))
        .succeeds()
        .stdout_is("empty: OK\n");

    // with two spaces, the "*" is not removed
    at.write("in.md5", "d41d8cd98f00b204e9800998ecf8427e  *empty\n");
    // First should fail as *empty doesn't exit
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg(at.subdir.join("in.md5"))
        .fails()
        .stdout_is("*empty: FAILED open or read\n");

    at.touch("*empty");
    // Should pass as we have the file
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg(at.subdir.join("in.md5"))
        .succeeds()
        .stdout_is("*empty: OK\n");
}

#[test]
// Disabled on Windows because of the "*"
#[cfg(not(windows))]
fn test_check_space_star_or_not() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("a");
    at.touch("*c");

    // with one space, the "*" is removed
    at.write(
        "in.md5",
        "d41d8cd98f00b204e9800998ecf8427e *c\n
        d41d8cd98f00b204e9800998ecf8427e a\n",
    );

    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg(at.subdir.join("in.md5"))
        .fails()
        .stdout_contains("c: FAILED")
        .stdout_does_not_contain("a: FAILED")
        .stderr_contains("WARNING: 1 line is improperly formatted");

    at.write(
        "in.md5",
        "d41d8cd98f00b204e9800998ecf8427e a\n
            d41d8cd98f00b204e9800998ecf8427e *c\n",
    );

    // First should fail as *empty doesn't exit
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg(at.subdir.join("in.md5"))
        .succeeds()
        .stdout_contains("a: OK")
        .stderr_contains("WARNING: 1 line is improperly formatted");
}

#[test]
fn test_check_no_backslash_no_space() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");
    at.write("in.md5", "MD5(f)= d41d8cd98f00b204e9800998ecf8427e\n");
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg(at.subdir.join("in.md5"))
        .succeeds()
        .stdout_is("f: OK\n");
}

#[test]
fn test_incomplete_format() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");
    at.write("in.md5", "MD5 (\n");
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg(at.subdir.join("in.md5"))
        .fails()
        .stderr_contains("no properly formatted checksum lines found");
}

#[test]
fn test_start_error() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");
    at.write("in.md5", "ERR\nd41d8cd98f00b204e9800998ecf8427e  f\n");
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg("--strict")
        .arg(at.subdir.join("in.md5"))
        .fails()
        .stdout_is("f: OK\n")
        .stderr_contains("WARNING: 1 line is improperly formatted");
}

#[test]
fn test_check_check_ignore_no_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");
    at.write("in.md5", "d41d8cd98f00b204e9800998ecf8427f  missing\n");
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg("--ignore-missing")
        .arg(at.subdir.join("in.md5"))
        .fails()
        .stderr_contains("in.md5: no file was verified");
}

#[test]
fn test_check_directory_error() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("d");
    at.write("in.md5", "d41d8cd98f00b204e9800998ecf8427f  d\n");
    #[cfg(not(windows))]
    let err_msg = "md5sum: d: Is a directory\n";
    #[cfg(windows)]
    let err_msg = "md5sum: d: Permission denied\n";
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg(at.subdir.join("in.md5"))
        .fails()
        .stderr_contains(err_msg);
}

#[test]
#[cfg(not(windows))]
fn test_continue_after_directory_error() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("d");
    at.touch("file");
    at.touch("no_read_perms");
    at.set_mode("no_read_perms", 200);

    let (out, err_msg) = (
        "d41d8cd98f00b204e9800998ecf8427e  file\n",
        [
            "md5sum: d: Is a directory",
            "md5sum: dne: No such file or directory",
            "md5sum: no_read_perms: Permission denied\n",
        ]
        .join("\n"),
    );

    scene
        .ccmd("md5sum")
        .arg("d")
        .arg("dne")
        .arg("no_read_perms")
        .arg("file")
        .fails()
        .stdout_is(out)
        .stderr_is(err_msg);
}

#[test]
fn test_check_quiet() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");
    at.write("in.md5", "d41d8cd98f00b204e9800998ecf8427e  f\n");
    scene
        .ccmd("md5sum")
        .arg("--quiet")
        .arg("--check")
        .arg(at.subdir.join("in.md5"))
        .succeeds()
        .no_output();

    // incorrect md5
    at.write("in.md5", "d41d8cd98f00b204e9800998ecf8427f  f\n");
    scene
        .ccmd("md5sum")
        .arg("--quiet")
        .arg("--check")
        .arg(at.subdir.join("in.md5"))
        .fails()
        .stdout_contains("f: FAILED")
        .stderr_contains("WARNING: 1 computed checksum did NOT match");

    scene
        .ccmd("md5sum")
        .arg("--quiet")
        .arg(at.subdir.join("in.md5"))
        .fails()
        .stderr_contains("md5sum: the --quiet option is meaningful only when verifying checksums");
    scene
        .ccmd("md5sum")
        .arg("--strict")
        .arg(at.subdir.join("in.md5"))
        .fails()
        .stderr_contains("md5sum: the --strict option is meaningful only when verifying checksums");
}

#[test]
fn test_star_to_start() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");
    at.write("in.md5", "d41d8cd98f00b204e9800998ecf8427e *f\n");
    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg(at.subdir.join("in.md5"))
        .succeeds()
        .stdout_only("f: OK\n");
}

#[test]
fn test_check_b2sum_strict_check() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("f");

    let checksums = [
        "2e  f\n",
        "e4a6a0577479b2b4  f\n",
        "cae66941d9efbd404e4d88758ea67670  f\n",
        "246c0442cd564aced8145b8b60f1370aa7  f\n",
        "0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8  f\n",
        "4ded8c5fc8b12f3273f877ca585a44ad6503249a2b345d6d9c0e67d85bcb700db4178c0303e93b8f4ad758b8e2c9fd8b3d0c28e585f1928334bb77d36782e8  f\n",
        "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce  f\n",
    ];

    at.write("ck", &checksums.join(""));

    let output = "f: OK\n".to_string().repeat(checksums.len());

    scene
        .ccmd("b2sum")
        .arg("-c")
        .arg(at.subdir.join("ck"))
        .succeeds()
        .stdout_only(&output);

    scene
        .ccmd("b2sum")
        .arg("--strict")
        .arg("-c")
        .arg(at.subdir.join("ck"))
        .succeeds()
        .stdout_only(&output);
}

#[test]
fn test_check_md5_comment_line() {
    // A comment in a checksum file shall be discarded unnoticed.

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("foo", "foo-content\n");
    at.write(
        "MD5SUM",
        "\
        # This is a comment\n\
        8411029f3f5b781026a93db636aca721  foo\n\
        # next comment is empty\n#",
    );

    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg("MD5SUM")
        .succeeds()
        .stdout_contains("foo: OK")
        .no_stderr();
}

#[test]
fn test_check_md5_comment_only() {
    // A file only filled with comments is equivalent to an empty file,
    // and therefore produces an error.

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("foo", "foo-content\n");
    at.write("MD5SUM", "# This is a comment\n");

    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg("MD5SUM")
        .fails()
        .stderr_contains("no properly formatted checksum lines found");
}

#[test]
fn test_check_md5_comment_leading_space() {
    // A file only filled with comments is equivalent to an empty file,
    // and therefore produces an error.

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("foo", "foo-content\n");
    at.write(
        "MD5SUM",
        " # This is a comment\n\
        8411029f3f5b781026a93db636aca721  foo\n",
    );

    scene
        .ccmd("md5sum")
        .arg("--check")
        .arg("MD5SUM")
        .succeeds()
        .stdout_contains("foo: OK")
        .stderr_contains("WARNING: 1 line is improperly formatted");
}

#[test]
fn test_sha256_binary() {
    let ts = TestScenario::new(util_name!());
    assert_eq!(
        ts.fixtures.read("binary.sha256.expected"),
        get_hash!(
            ts.ucmd()
                .arg("--sha256")
                .arg("--bits=256")
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
                .arg("--sha256")
                .arg("--bits=256")
                .pipe_in_fixture("binary.png")
                .succeeds()
                .no_stderr()
                .stdout_str()
        )
    );
}

#[test]
fn test_check_sha256_binary() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd()
        .args(&[
            "--sha256",
            "--bits=256",
            "--check",
            "binary.sha256.checkfile",
        ])
        .succeeds()
        .no_stderr()
        .stdout_is("binary.png: OK\n");
}

#[test]
fn test_help_shows_correct_utility_name() {
    // Test that help output shows the actual utility name instead of "hashsum"
    let scene = TestScenario::new(util_name!());

    // Test md5sum
    scene
        .ccmd("md5sum")
        .arg("--help")
        .succeeds()
        .stdout_contains("Usage: md5sum")
        .stdout_does_not_contain("Usage: hashsum");

    // Test sha256sum
    scene
        .ccmd("sha256sum")
        .arg("--help")
        .succeeds()
        .stdout_contains("Usage: sha256sum")
        .stdout_does_not_contain("Usage: hashsum");

    // Test b2sum
    scene
        .ccmd("b2sum")
        .arg("--help")
        .succeeds()
        .stdout_contains("Usage: b2sum")
        .stdout_does_not_contain("Usage: hashsum");

    // Test b3sum
    scene
        .ccmd("b3sum")
        .arg("--help")
        .succeeds()
        .stdout_contains("Usage: b3sum")
        .stdout_does_not_contain("Usage: hashsum");

    // Test that generic hashsum still shows the correct usage
    scene
        .ccmd("hashsum")
        .arg("--help")
        .succeeds()
        .stdout_contains("Usage: hashsum --<digest>");
}
