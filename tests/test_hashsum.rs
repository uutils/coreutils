macro_rules! get_hash(
    ($str:expr) => (
        $str.split(' ').collect::<Vec<&str>>()[0]
    );
);

macro_rules! test_digest {
    ($($id:ident $t:ident $size:expr)*) => ($(

    mod $id {
        use::common::util::*;
        static DIGEST_ARG: &'static str = concat!("--", stringify!($t));
        static BITS_ARG: &'static str = concat!("--bits=", stringify!($size));
        static EXPECTED_FILE: &'static str = concat!(stringify!($id), ".expected");

        #[test]
        fn test_single_file() {
            let ts = TestScenario::new("hashsum");
            assert_eq!(ts.fixtures.read(EXPECTED_FILE),
                       get_hash!(ts.ucmd().arg(DIGEST_ARG).arg(BITS_ARG).arg("input.txt").succeeds().no_stderr().stdout));
        }

        #[test]
        fn test_stdin() {
            let ts = TestScenario::new("hashsum");
            assert_eq!(ts.fixtures.read(EXPECTED_FILE),
                       get_hash!(ts.ucmd().arg(DIGEST_ARG).arg(BITS_ARG).pipe_in_fixture("input.txt").succeeds().no_stderr().stdout));
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
}
