use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./stdbuf";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn test_stdbuf_unbuffered_stdout() {
    // This is a basic smoke test
    let mut cmd = Command::new(PROGNAME);
    let result = run_piped_stdin(&mut cmd.args(&["-o0", "head"]), "The quick brown fox jumps over the lazy dog.");
    assert_eq!(result.stdout, "The quick brown fox jumps over the lazy dog.");
}
