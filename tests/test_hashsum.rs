macro_rules! get_hash(
    ($str:expr) => (
        $str.split(' ').collect::<Vec<&str>>()[0]
    );
);

macro_rules! test_digest {
    ($($t:ident)*) => ($(

    mod $t {
        use::common::util::*;
        static DIGEST_ARG: &'static str = concat!("--", stringify!($t));
        static EXPECTED_FILE: &'static str = concat!(stringify!($t), ".expected");

        #[test]
        fn test_single_file() {
            let ts = TestScenario::new("hashsum");
            assert_eq!(ts.fixtures.read(EXPECTED_FILE),
                       get_hash!(ts.ucmd().arg(DIGEST_ARG).arg("input.txt").succeeds().no_stderr().stdout));
        }

        #[test]
        fn test_stdin() {
            let ts = TestScenario::new("hashsum");
            assert_eq!(ts.fixtures.read(EXPECTED_FILE),
                       get_hash!(ts.ucmd().arg(DIGEST_ARG).pipe_in_fixture("input.txt").succeeds().no_stderr().stdout));
        }
    }
    )*)
}

test_digest! { md5 sha1 sha224 sha256 sha384 sha512 }
