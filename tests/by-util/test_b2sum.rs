// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use rstest::rstest;

use uutests::util::TestScenario;
use uutests::{new_ucmd, util_name};
// spell-checker:ignore checkfile, nonames, testf, ntestf
macro_rules! get_hash(
    ($str:expr) => (
        $str.split(' ').collect::<Vec<&str>>()[0]
    );
);

macro_rules! test_digest_with_len {
    ($id:ident, $t:ident, $size:expr) => {
        mod $id {
            use uutests::util::*;
            use uutests::util_name;
            static LENGTH_ARG: &'static str = concat!("--length=", stringify!($size));
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
                            .arg(LENGTH_ARG)
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
                            .arg(LENGTH_ARG)
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
                    .args(&[LENGTH_ARG, "--check", CHECK_FILE])
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
                            .arg(LENGTH_ARG)
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
                    .args(&[LENGTH_ARG, "a", "b", "c"])
                    .fails()
                    .stdout_contains("a\n")
                    .stdout_contains("c\n")
                    .stderr_contains("b: No such file or directory");
            }
        }
    };
}

test_digest_with_len! {b2sum, b2sum, 512}

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
        .fails_with_code(1)
        .stderr_contains("b2sum: invalid length: '9'")
        .stderr_contains("b2sum: length is not a multiple of 8");
}

#[rstest]
#[case("513")]
#[case("1024")]
#[case("18446744073709552000")]
fn test_invalid_b2sum_length_option_too_large(#[case] len: &str) {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("testf", "foobar\n");

    scene
        .ccmd("b2sum")
        .arg("--length")
        .arg(len)
        .arg(at.subdir.join("testf"))
        .fails_with_code(1)
        .no_stdout()
        .stderr_contains(format!("b2sum: invalid length: '{len}'"))
        .stderr_contains("b2sum: maximum digest length for 'BLAKE2b' is 512 bits");
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
fn test_help_shows_correct_utility_name() {
    // Test b2sum
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .stdout_contains("Usage: b2sum")
        .stdout_does_not_contain("Usage: hashsum");
}
