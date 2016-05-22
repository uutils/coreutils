macro_rules! get_hash(
    ($str:expr) => (
        $str.split(' ').collect::<Vec<&str>>()[0]
    );
);

macro_rules! test_digest {
    ($($t:ident)*) => ($(

    mod $t {
        use common::util::*;
        static UTIL_NAME: &'static str = "hashsum";
        static DIGEST_ARG: &'static str = concat!("--", stringify!($t));
        static EXPECTED_FILE: &'static str = concat!(stringify!($t), ".expected");

        #[test]
        fn test_single_file() {
            let (at, mut ucmd) = testing(UTIL_NAME);
            let result = ucmd.arg(DIGEST_ARG).arg("input.txt").run();

            assert_empty_stderr!(result);
            assert!(result.success);
            assert_eq!(get_hash!(result.stdout), at.read(EXPECTED_FILE));
        }

        #[test]
        fn test_stdin() {
            let (at, mut ucmd) = testing(UTIL_NAME);
            let input = at.read("input.txt");
            let result = ucmd.arg(DIGEST_ARG).run_piped_stdin(input);

            assert_empty_stderr!(result);
            assert!(result.success);
            assert_eq!(get_hash!(result.stdout), at.read(EXPECTED_FILE));
        }
    }
    )*)
}

test_digest! { md5 sha1 sha224 sha256 sha384 sha512 }
