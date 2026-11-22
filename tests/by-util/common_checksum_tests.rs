// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! This file centralizes common tests for standalone checksum uutils:
//! - b2sum
//! - md5sum
//! - sha1sum
//! - sha224sum
//! - sha256sum
//! - sha384sum
//! - sha512sum

// spell-checker:ignore checkfile, Xsum

pub fn get_untagged_hash(s: &str) -> &str {
    s.split(' ').next().unwrap()
}

macro_rules! test_digest_inner {
    ($id:ident, $args:expr) => {
        mod $id {
            use uutests::util::*;
            use uutests::util_name;

            static HASH_FIXTURE: &'static str = concat!(stringify!($id), ".expected");
            static UNTAGGED_CHECK_FILE: &'static str =
                concat!(stringify!($id), ".untagged.checkfile");
            static TAGGED_CHECK_FILE: &'static str = concat!(stringify!($id), ".tagged.checkfile");
            static INPUT_FILE: &'static str = "input.txt";

            // Test a simple `Xsum INPUT_FILE` command, check both untagged and
            // tagged outputs.
            #[rstest::rstest]
            #[case::untagged(UNTAGGED_CHECK_FILE, &[])]
            #[case::tagged(TAGGED_CHECK_FILE, &["--tag"])]
            fn test_single_file(#[case] expected_output: &str, #[case] extra_args: &[&str]) {
                let ts = TestScenario::new(util_name!());
                ts.ucmd()
                    .arg(INPUT_FILE)
                    .args($args)
                    .args(extra_args)
                    .succeeds()
                    .no_stderr()
                    .stdout_is_fixture(expected_output);
            }

            // Test sending a file content through stdin,
            // and check the expected result.
            #[test]
            fn test_stdin() {
                let ts = TestScenario::new(util_name!());
                assert_eq!(
                    ts.fixtures.read(HASH_FIXTURE),
                    crate::common_checksum_tests::get_untagged_hash(
                        ts.ucmd()
                            .args($args)
                            .pipe_in_fixture(INPUT_FILE)
                            .succeeds()
                            .no_stderr()
                            .stdout_str()
                    )
                );
            }

            #[rstest_reuse::template]
            #[rstest::rstest]
            #[case::untagged(UNTAGGED_CHECK_FILE)]
            #[case::tagged(TAGGED_CHECK_FILE)]
            fn test_check(#[case] fixture_path: &str) {}

            // Test checking a checksum by giving it as a file
            // `Xsum --check CHECK_FILE`, and make sure that the
            // checksum is correctly decoded.
            #[rstest_reuse::apply(test_check)]
            fn test_check_file(#[case] fixture_path: &str) {
                let ts = TestScenario::new(util_name!());
                println!("File content='{}'", ts.fixtures.read(INPUT_FILE));
                println!("Check file='{}'", ts.fixtures.read(fixture_path));

                ts.ucmd()
                    .args($args)
                    .args(&["--check", fixture_path])
                    .succeeds()
                    .stdout_only("input.txt: OK\n");
            }

            // Test checking a checksum by piping it through stdin,
            // and make sure that the checksum is correctly decoded.
            #[rstest_reuse::apply(test_check)]
            fn test_check_stdin(#[case] fixture_path: &str) {
                let ts = TestScenario::new(util_name!());
                println!("File content='{}'", ts.fixtures.read(INPUT_FILE));
                println!("Check file='{}'", ts.fixtures.read(fixture_path));

                ts.ucmd()
                    .args($args)
                    .arg("--check")
                    .pipe_in_fixture(fixture_path)
                    .succeeds()
                    .stdout_only("input.txt: OK\n");
            }

            // Test that passing --zero still yields the expected hash,
            // and that it ends with a `\0` instead of a `\n`.
            #[test]
            fn test_zero() {
                let ts = TestScenario::new(util_name!());
                let result = ts
                    .ucmd()
                    .args($args)
                    .arg("--zero")
                    .arg(INPUT_FILE)
                    .succeeds();
                result.no_stderr();
                let stdout = result.stdout_str();

                assert_eq!(
                    ts.fixtures.read(HASH_FIXTURE),
                    crate::common_checksum_tests::get_untagged_hash(stdout)
                );
                assert!(stdout.ends_with("\0"));
            }

            // Check that when giving no file argument, opening "-" for
            // stdin doesn't resolve to opening a directory named "-".
            #[test]
            fn test_stdin_with_dash_directory() {
                let ts = TestScenario::new(util_name!());
                ts.fixtures.mkdir("-");
                assert_eq!(
                    ts.fixtures.read(HASH_FIXTURE),
                    crate::common_checksum_tests::get_untagged_hash(
                        ts.ucmd()
                            .args($args)
                            .pipe_in_fixture(INPUT_FILE)
                            .succeeds()
                            .no_stderr()
                            .stdout_str()
                    )
                );
            }

            // Test behavior upon trying to sum a non-existing file.
            // Make sure it does not prevent other files from being processed.
            #[test]
            fn test_missing_file() {
                let ts = TestScenario::new(util_name!());
                let at = &ts.fixtures;

                at.write("a", "file1\n");
                at.write("c", "file3\n");

                ts.ucmd()
                    .args($args)
                    .args(&["a", "b", "c"])
                    .fails()
                    .stdout_contains("a\n")
                    .stdout_contains("c\n")
                    .stderr_contains("b: No such file or directory");
            }

            // Check behavior when an invalid arg is passed
            #[test]
            fn test_invalid_arg() {
                uutests::new_ucmd!()
                    .arg("--invalid-arg")
                    .args($args)
                    .fails_with_code(1);
            }

            // Check behavior when conflicting args are passed
            #[test]
            fn test_conflicting_arg() {
                uutests::new_ucmd!()
                    .arg("--tag")
                    .arg("--check")
                    .args($args)
                    .fails_with_code(1);
                uutests::new_ucmd!()
                    .arg("--tag")
                    .arg("--text")
                    .args($args)
                    .fails_with_code(1);
            }
        }
    };
}
pub(crate) use test_digest_inner;

macro_rules! test_digest {
    ($id:ident) => {
        crate::common_checksum_tests::test_digest_inner!($id, &[] as &[&str]);
    };
    ($id:ident,$len:literal) => {
        crate::common_checksum_tests::test_digest_inner!(
            $id,
            &[concat!("--length=", stringify!($len))]
        );
    };
}
pub(crate) use test_digest;
