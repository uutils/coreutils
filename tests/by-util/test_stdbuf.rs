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
    let ts = TestScenario::new(util_name!());

    ts.ucmd().args(&["head"]).fails().stderr_is(&format!(
        "error: The following required arguments were not provided:\n    \
         --input <MODE>\n    \
         --output <MODE>\n    \
         --error <MODE>\n\n\
         USAGE:\n    \
         {1} {0} OPTION... COMMAND\n\n\
         For more information try --help",
        ts.util_name,
        ts.bin_path.to_string_lossy()
    ));
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
    new_ucmd!()
        .args(&["-i", "L", "head"])
        .fails()
        .usage_error("line buffering stdin is meaningless");
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_stdbuf_invalid_mode_fails() {
    let options = ["--input", "--output", "--error"];
    for option in &options {
        new_ucmd!()
            .args(&[*option, "1024R", "head"])
            .fails()
            .code_is(125)
            .stderr_only("stdbuf: invalid mode '1024R'");
        #[cfg(not(target_pointer_width = "128"))]
        new_ucmd!()
            .args(&[*option, "1Y", "head"])
            .fails()
            .code_is(125)
            .stderr_contains("stdbuf: invalid mode '1Y': Value too large for defined data type");
        #[cfg(target_pointer_width = "32")]
        {
            new_ucmd!()
                .args(&[*option, "5GB", "head"])
                .fails()
                .code_is(125)
                .stderr_contains(
                    "stdbuf: invalid mode '5GB': Value too large for defined data type",
                );
        }
    }
}
