#[macro_use]
mod common;

use common::util::*;

static UTIL_NAME: &'static str = "stdbuf";

#[test]
fn test_stdbuf_unbuffered_stdout() {
    if cfg!(target_os="linux") {
        let (_, mut ucmd) = testing(UTIL_NAME);
        // This is a basic smoke test
        let result = ucmd.args(&["-o0", "head"])
                         .run_piped_stdin("The quick brown fox jumps over the lazy dog.");
        assert_eq!(result.stdout,
                   "The quick brown fox jumps over the lazy dog.");
    }
}
