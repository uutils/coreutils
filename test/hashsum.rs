static PROGNAME: &'static str = "./hashsum";

#[path = "common/util.rs"]
#[macro_use]
mod util;

macro_rules! get_hash(
    ($str:expr) => (
        $str.split(' ').collect::<Vec<&str>>()[0]
    );
);

macro_rules! test_digest {
    ($($t:ident)*) => ($(

    mod $t {
        use std::process::Command;
        use util::*;

        static DIGEST_ARG: &'static str = concat!("--", stringify!($t));
        static EXPECTED_FILE: &'static str = concat!(stringify!($t), ".expected");

        #[test]
        fn test_single_file() {
            let mut cmd = Command::new(::PROGNAME);
            let result = run(&mut cmd.arg(DIGEST_ARG).arg("input.txt"));

            assert_empty_stderr!(result);
            assert!(result.success);
            assert_eq!(get_hash!(result.stdout), get_file_contents(EXPECTED_FILE));
        }

        #[test]
        fn test_stdin() {
            let input = get_file_contents("input.txt");
            let mut cmd = Command::new(::PROGNAME);
            let result = run_piped_stdin(&mut cmd.arg(DIGEST_ARG), input);

            assert_empty_stderr!(result);
            assert!(result.success);
            assert_eq!(get_hash!(result.stdout), get_file_contents(EXPECTED_FILE));
        }
    }
    )*)
}

test_digest! { md5 sha1 sha224 sha256 sha384 sha512 }
