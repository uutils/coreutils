use common::util::*;

static UTIL_NAME: &'static str = "stdbuf";

fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_stdbuf_unbuffered_stdout() {
    if cfg!(target_os="linux") {
        // This is a basic smoke test
        let result = new_ucmd().args(&["-o0", "head"])
                         .run_piped_stdin("The quick brown fox jumps over the lazy dog.");
        assert_eq!(result.stdout,
                   "The quick brown fox jumps over the lazy dog.");
    }
}
