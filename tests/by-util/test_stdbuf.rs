#[cfg(not(target_os = "windows"))]
use crate::common::util::*;

#[cfg(not(target_os = "windows"))]
#[test]
fn test_stdbuf_unbuffered_stdout() {
    // This is a basic smoke test
    new_ucmd!()
        .args(&["-o0", "head"])
        .pipe_in("The quick brown fox jumps over the lazy dog.")
        .run()
        .stdout_is("The quick brown fox jumps over the lazy dog.");
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_stdbuf_line_buffered_stdout() {
    new_ucmd!()
        .args(&["-oL", "head"])
        .pipe_in("The quick brown fox jumps over the lazy dog.")
        .run()
        .stdout_is("The quick brown fox jumps over the lazy dog.");
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_stdbuf_no_buffer_option_fails() {
    new_ucmd!().args(&["head"]).fails().stderr_is(
        "error: The following required arguments were not provided:\n    \
         --error <MODE>\n    \
         --input <MODE>\n    \
         --output <MODE>\n\n\
         USAGE:\n    \
         stdbuf OPTION... COMMAND\n\n\
         For more information try --help",
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_stdbuf_trailing_var_arg() {
    new_ucmd!()
        .args(&["-i", "1024", "tail", "-1"])
        .pipe_in("The quick brown fox\njumps over the lazy dog.")
        .run()
        .stdout_is("jumps over the lazy dog.");
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_stdbuf_line_buffering_stdin_fails() {
    new_ucmd!().args(&["-i", "L", "head"]).fails().stderr_is(
        "stdbuf: line buffering stdin is meaningless\nTry 'stdbuf --help' for more information.",
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_stdbuf_invalid_mode_fails() {
    new_ucmd!()
        .args(&["-i", "1024R", "head"])
        .fails()
        .stderr_is("stdbuf: invalid mode 1024R\nTry 'stdbuf --help' for more information.");
}
