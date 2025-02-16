// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::new_ucmd;
#[cfg(not(target_os = "windows"))]
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn invalid_input() {
    new_ucmd!().arg("-/").fails().code_is(125);
}

#[test]
fn test_permission() {
    new_ucmd!()
        .arg("-o1")
        .arg(".")
        .fails()
        .code_is(126)
        .stderr_contains("Permission denied");
}

#[test]
fn test_no_such() {
    new_ucmd!()
        .arg("-o1")
        .arg("no_such")
        .fails()
        .code_is(127)
        .stderr_contains("No such file or directory");
}

#[cfg(all(not(target_os = "windows"), not(target_os = "openbsd")))]
#[test]
fn test_stdbuf_unbuffered_stdout() {
    // This is a basic smoke test
    new_ucmd!()
        .args(&["-o0", "head"])
        .pipe_in("The quick brown fox jumps over the lazy dog.")
        .run()
        .stdout_is("The quick brown fox jumps over the lazy dog.");
}

#[cfg(all(not(target_os = "windows"), not(target_os = "openbsd")))]
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

    ts.ucmd()
        .args(&["head"])
        .fails()
        .stderr_contains("the following required arguments were not provided:");
}

#[cfg(all(not(target_os = "windows"), not(target_os = "openbsd")))]
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
            .usage_error("invalid mode '1024R': Value too large for defined data type");
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
