macro_rules! get_hash(
    ($str:expr) => (
        $str.split(' ').collect::<Vec<&str>>()[0]
    );
);

macro_rules! test_digest {
    ($($t:ident)*) => ($(

    mod $t {
        use::common::util::*;
        static UTIL_NAME: &'static str = "hashsum";
        fn at_and_ucmd() -> (AtPath, UCommand) {
            let ts = TestScenario::new(UTIL_NAME);
            let ucmd = ts.ucmd();
            (ts.fixtures, ucmd)
        }
        static DIGEST_ARG: &'static str = concat!("--", stringify!($t));
        static EXPECTED_FILE: &'static str = concat!(stringify!($t), ".expected");

        #[test]
        fn test_single_file() {
            let (at, mut ucmd) = at_and_ucmd();
            assert_eq!(at.read(EXPECTED_FILE),
                       get_hash!(ucmd.arg(DIGEST_ARG).arg("input.txt").succeeds().no_stderr().stdout));
        }

        #[test]
        fn test_stdin() {
            let (at, mut ucmd) = at_and_ucmd();
            let input = at.read("input.txt");
            assert_eq!(at.read(EXPECTED_FILE),
                       get_hash!(ucmd.arg(DIGEST_ARG).pipe_in(input).succeeds().no_stderr().stdout));
        }
    }
    )*)
}

test_digest! { md5 sha1 sha224 sha256 sha384 sha512 }
