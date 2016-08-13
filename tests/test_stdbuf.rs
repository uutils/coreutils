use common::util::*;

static UTIL_NAME: &'static str = "stdbuf";
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_stdbuf_unbuffered_stdout() {
    if cfg!(target_os="linux") {
        // This is a basic smoke test
        new_ucmd().args(&["-o0", "head"])
            .pipe_in("The quick brown fox jumps over the lazy dog.").run()
            .stdout_is("The quick brown fox jumps over the lazy dog.");
    }
}
