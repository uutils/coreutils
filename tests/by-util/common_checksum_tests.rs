//! This file centralizes common tests for standalone checksum uutils:
//! - b2sum
//! - md5sum
//! - sha1sum
//! - sha224sum
//! - sha256sum
//! - sha384sum
//! - sha512sum

// spell-checker:ignore checkfile

macro_rules! test_digest_inner {
    ($id:ident, $len:literal, $args:expr) => {
        mod $id {
            use uutests::util::*;
            use uutests::util_name;
            #[allow(unused)]
            static LENGTH_ARG: &'static str = concat!("--length=", stringify!($len));
            static EXPECTED_FILE: &'static str = concat!(stringify!($id), ".expected");
            static CHECK_FILE: &'static str = concat!(stringify!($id), ".checkfile");
            static INPUT_FILE: &'static str = "input.txt";

            #[test]
            fn test_single_file() {
                let ts = TestScenario::new(util_name!());
                assert_eq!(
                    ts.fixtures.read(EXPECTED_FILE),
                    ts.ucmd()
                        .arg(INPUT_FILE)
                        .args($args)
                        .succeeds()
                        .no_stderr()
                        .stdout_str()
                        .split(' ')
                        .next()
                        .unwrap()
                );
            }

            #[test]
            fn test_stdin() {
                let ts = TestScenario::new(util_name!());
                assert_eq!(
                    ts.fixtures.read(EXPECTED_FILE),
                    ts.ucmd()
                        .args($args)
                        .pipe_in_fixture(INPUT_FILE)
                        .succeeds()
                        .no_stderr()
                        .stdout_str()
                        .split(' ')
                        .next()
                        .unwrap()
                );
            }

            #[test]
            fn test_check() {
                let ts = TestScenario::new(util_name!());
                println!("File content='{}'", ts.fixtures.read(INPUT_FILE));
                println!("Check file='{}'", ts.fixtures.read(CHECK_FILE));

                ts.ucmd()
                    .args($args)
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
                    ts.ucmd()
                        .args($args)
                        .arg("--zero")
                        .arg(INPUT_FILE)
                        .succeeds()
                        .no_stderr()
                        .stdout_str()
                        .split(' ')
                        .next()
                        .unwrap()
                );
            }

            #[test]
            fn test_stdin_with_dash_directory() {
                let ts = TestScenario::new(util_name!());
                ts.fixtures.mkdir("-");
                assert_eq!(
                    ts.fixtures.read(EXPECTED_FILE),
                    ts.ucmd()
                        .args($args)
                        .pipe_in_fixture(INPUT_FILE)
                        .succeeds()
                        .no_stderr()
                        .stdout_str()
                        .split(' ')
                        .next()
                        .unwrap()
                );
            }

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
        }
    };
}
pub(crate) use test_digest_inner;

macro_rules! test_digest {
    ($id:ident) => {
        crate::common_checksum_tests::test_digest_inner!($id, 0, &[] as &[&str]);
    };
    ($id:ident,$len:literal) => {
        crate::common_checksum_tests::test_digest_inner!($id, $len, &[LENGTH_ARG]);
    };
}
pub(crate) use test_digest;
