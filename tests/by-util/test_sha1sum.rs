// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;
// spell-checker:ignore checkfile, testf, ntestf
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

test_digest! {sha1}

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
fn test_help_shows_correct_utility_name() {
    // Test that help output shows the actual utility name instead of "hashsum"

    new_ucmd!()
        .arg("--help")
        .succeeds()
        .stdout_contains("Usage: sha1sum")
        .stdout_does_not_contain("Usage: hashsum");
}
