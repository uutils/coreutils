// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
#![allow(clippy::borrow_as_ptr)]

use uutests::util::TestScenario;
use uutests::{at_and_ucmd, new_ucmd, util_name};

use regex::Regex;
#[cfg(target_os = "linux")]
use std::fmt::Write;

// tests for basic tee functionality.
// inspired by:
// https://github.com/coreutils/coreutils/tests/misc/tee.sh

// spell-checker:ignore nopipe

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_short_help_is_long_help() {
    // I can't believe that this test is necessary.
    let help_short = new_ucmd!()
        .arg("-h")
        .succeeds()
        .no_stderr()
        .stdout_str()
        .to_owned();
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .no_stderr()
        .stdout_is(help_short);
}

#[test]
fn test_tee_processing_multiple_operands() {
    // POSIX says: "Processing of at least 13 file operands shall be supported."

    let content = "tee_sample_content";
    for n in [1, 2, 12, 13] {
        let files = (1..=n).map(|x| x.to_string()).collect::<Vec<_>>();
        let (at, mut ucmd) = at_and_ucmd!();

        ucmd.args(&files)
            .pipe_in(content)
            .succeeds()
            .stdout_is(content);

        for file in &files {
            assert!(at.file_exists(file));
            assert_eq!(at.read(file), content);
        }
    }
}

#[test]
fn test_tee_treat_minus_as_filename() {
    // Ensure tee treats '-' as the name of a file, as mandated by POSIX.

    let (at, mut ucmd) = at_and_ucmd!();
    let content = "tee_sample_content";
    let file = "-";

    ucmd.arg("-").pipe_in(content).succeeds().stdout_is(content);

    assert!(at.file_exists(file));
    assert_eq!(at.read(file), content);
}

#[test]
fn test_tee_append() {
    let (at, mut ucmd) = at_and_ucmd!();
    let content = "tee_sample_content";
    let file = "tee_out";

    at.touch(file);
    at.write(file, content);
    assert_eq!(at.read(file), content);

    ucmd.arg("-a")
        .arg(file)
        .pipe_in(content)
        .succeeds()
        .stdout_is(content);
    assert!(at.file_exists(file));
    assert_eq!(at.read(file), content.repeat(2));
}

#[test]
#[cfg(target_os = "linux")]
fn test_tee_no_more_writeable_1() {
    // equals to 'tee /dev/full out2 <multi_read' call
    let (at, mut ucmd) = at_and_ucmd!();
    let content = (1..=10).fold(String::new(), |mut output, x| {
        writeln!(output, "{x}").unwrap();
        output
    });
    let file_out = "tee_file_out";

    ucmd.arg("/dev/full")
        .arg(file_out)
        .pipe_in(&content[..])
        .fails()
        .stdout_contains(&content)
        .stderr_contains("No space left on device");

    assert_eq!(at.read(file_out), content);
}

#[test]
fn test_readonly() {
    let (at, mut ucmd) = at_and_ucmd!();
    let content_tee = "hello";
    let content_file = "world";
    let file_out = "tee_file_out";
    let writable_file = "tee_file_out2";
    at.write(file_out, content_file);
    at.set_readonly(file_out);
    ucmd.arg(file_out)
        .arg(writable_file)
        .pipe_in(content_tee)
        .ignore_stdin_write_error()
        .fails()
        .stdout_is(content_tee)
        // Windows says "Access is denied" for some reason.
        .stderr_matches(&Regex::new("(Permission|Access is) denied").unwrap());
    assert_eq!(at.read(file_out), content_file);
    assert_eq!(at.read(writable_file), content_tee);
}

#[test]
#[cfg(target_os = "linux")]
fn test_tee_no_more_writeable_2() {
    // should be equals to 'tee out1 out2 >/dev/full <multi_read' call
    // but currently there is no way to redirect stdout to /dev/full
    // so this test is disabled
    let (_at, mut ucmd) = at_and_ucmd!();
    let _content = (1..=10).fold(String::new(), |mut output, x| {
        let _ = writeln!(output, "{x}");
        output
    });
    let file_out_a = "tee_file_out_a";
    let file_out_b = "tee_file_out_b";

    let _result = ucmd
        .arg(file_out_a)
        .arg(file_out_b)
        .pipe_in("/dev/full")
        .succeeds(); // TODO: expected to succeed currently; change to fails() when required

    // TODO: comment in after https://github.com/uutils/coreutils/issues/1805 is fixed
    // assert_eq!(at.read(file_out_a), content);
    // assert_eq!(at.read(file_out_b), content);
    // assert!(result.stderr.contains("No space left on device"));
}

#[cfg(target_os = "linux")]
mod linux_only {
    use uutests::util::{AtPath, UCommand};

    use std::fmt::Write;
    use std::fs::File;
    use std::process::{Output, Stdio};
    use std::time::Duration;
    use uutests::at_and_ucmd;
    use uutests::new_ucmd;
    use uutests::util::TestScenario;
    use uutests::util_name;

    fn make_broken_pipe() -> File {
        use libc::c_int;
        use std::os::unix::io::FromRawFd;

        let mut fds: [c_int; 2] = [0, 0];
        assert!(
            (unsafe { libc::pipe(std::ptr::from_mut::<c_int>(&mut fds[0])) } == 0),
            "Failed to create pipe"
        );

        // Drop the read end of the pipe
        let _ = unsafe { File::from_raw_fd(fds[0]) };

        // Make the write end of the pipe into a Rust File
        unsafe { File::from_raw_fd(fds[1]) }
    }

    fn make_hanging_read() -> File {
        use libc::c_int;
        use std::os::unix::io::FromRawFd;

        let mut fds: [c_int; 2] = [0, 0];
        assert!(
            (unsafe { libc::pipe(std::ptr::from_mut::<c_int>(&mut fds[0])) } == 0),
            "Failed to create pipe"
        );

        // PURPOSELY leak the write end of the pipe, so the read end hangs.

        // Return the read end of the pipe
        unsafe { File::from_raw_fd(fds[0]) }
    }

    fn run_tee(proc: &mut UCommand) -> (String, Output) {
        let content = (1..=100_000).fold(String::new(), |mut output, x| {
            let _ = writeln!(output, "{x}");
            output
        });

        #[allow(deprecated)]
        let output = proc
            .ignore_stdin_write_error()
            .set_stdin(Stdio::piped())
            .run_no_wait()
            .pipe_in_and_wait_with_output(content.as_bytes());

        (content, output)
    }

    fn expect_success(output: &Output) {
        assert!(
            output.status.success(),
            "Command was expected to succeed.\nstdout = {}\n stderr = {}",
            std::str::from_utf8(&output.stdout).unwrap(),
            std::str::from_utf8(&output.stderr).unwrap(),
        );
        assert!(
            output.stderr.is_empty(),
            "Unexpected data on stderr.\n stderr = {}",
            std::str::from_utf8(&output.stderr).unwrap(),
        );
    }

    fn expect_failure(output: &Output, message: &str) {
        assert!(
            !output.status.success(),
            "Command was expected to fail.\nstdout = {}\n stderr = {}",
            std::str::from_utf8(&output.stdout).unwrap(),
            std::str::from_utf8(&output.stderr).unwrap(),
        );
        assert!(
            std::str::from_utf8(&output.stderr)
                .unwrap()
                .contains(message),
            "Expected to see error message fragment {} in stderr, but did not.\n stderr = {}",
            message,
            std::str::from_utf8(&output.stderr).unwrap(),
        );
    }

    fn expect_silent_failure(output: &Output) {
        assert!(
            !output.status.success(),
            "Command was expected to fail.\nstdout = {}\n stderr = {}",
            std::str::from_utf8(&output.stdout).unwrap(),
            std::str::from_utf8(&output.stderr).unwrap(),
        );
        assert!(
            output.stderr.is_empty(),
            "Unexpected data on stderr.\n stderr = {}",
            std::str::from_utf8(&output.stderr).unwrap(),
        );
    }

    fn expect_correct(name: &str, at: &AtPath, contents: &str) {
        assert!(at.file_exists(name));
        let compare = at.read(name);
        assert_eq!(compare, contents);
    }

    fn expect_short(name: &str, at: &AtPath, contents: &str) {
        assert!(at.file_exists(name));
        let compare = at.read(name);
        assert!(
            compare.len() < contents.len(),
            "Too many bytes ({}) written to {} (should be a short count from {})",
            compare.len(),
            name,
            contents.len()
        );
        assert!(contents.starts_with(&compare),
                "Expected truncated output to be a prefix of the correct output, but it isn't.\n Correct: {contents}\n Compare: {compare}");
    }

    #[test]
    fn test_pipe_error_default() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd.arg(file_out_a).set_stdout(make_broken_pipe());

        let (content, output) = run_tee(proc);

        expect_silent_failure(&output);
        expect_short(file_out_a, &at, content.as_str());
    }

    #[test]
    fn test_pipe_error_warn_nopipe_1() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd
            .arg("-p")
            .arg(file_out_a)
            .set_stdout(make_broken_pipe());

        let (content, output) = run_tee(proc);

        expect_success(&output);
        expect_correct(file_out_a, &at, content.as_str());
    }

    #[test]
    fn test_pipe_error_warn_nopipe_2() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd
            .arg("--output-error")
            .arg(file_out_a)
            .set_stdout(make_broken_pipe());

        let (content, output) = run_tee(proc);

        expect_success(&output);
        expect_correct(file_out_a, &at, content.as_str());
    }

    #[test]
    fn test_pipe_error_warn_nopipe_3() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd
            .arg("--output-error=warn-nopipe")
            .arg(file_out_a)
            .set_stdout(make_broken_pipe());

        let (content, output) = run_tee(proc);

        expect_success(&output);
        expect_correct(file_out_a, &at, content.as_str());
    }

    #[test]
    fn test_pipe_error_warn_nopipe_3_shortcut() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd
            .arg("--output-error=warn-")
            .arg(file_out_a)
            .set_stdout(make_broken_pipe());

        let (content, output) = run_tee(proc);

        expect_success(&output);
        expect_correct(file_out_a, &at, content.as_str());
    }

    #[test]
    fn test_pipe_error_warn() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd
            .arg("--output-error=warn")
            .arg(file_out_a)
            .set_stdout(make_broken_pipe());

        let (content, output) = run_tee(proc);

        expect_failure(&output, "Broken pipe");
        expect_correct(file_out_a, &at, content.as_str());
    }

    #[test]
    fn test_pipe_error_exit() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd
            .arg("--output-error=exit")
            .arg(file_out_a)
            .set_stdout(make_broken_pipe());

        let (content, output) = run_tee(proc);

        expect_failure(&output, "Broken pipe");
        expect_short(file_out_a, &at, content.as_str());
    }

    #[test]
    fn test_pipe_error_exit_nopipe() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd
            .arg("--output-error=exit-nopipe")
            .arg(file_out_a)
            .set_stdout(make_broken_pipe());

        let (content, output) = run_tee(proc);

        expect_success(&output);
        expect_correct(file_out_a, &at, content.as_str());
    }

    #[test]
    fn test_pipe_error_exit_nopipe_shortcut() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd
            .arg("--output-error=exit-nop")
            .arg(file_out_a)
            .set_stdout(make_broken_pipe());

        let (content, output) = run_tee(proc);

        expect_success(&output);
        expect_correct(file_out_a, &at, content.as_str());
    }

    #[test]
    fn test_space_error_default() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd.arg(file_out_a).arg("/dev/full");

        let (content, output) = run_tee(proc);

        expect_failure(&output, "No space left");
        expect_correct(file_out_a, &at, content.as_str());
    }

    #[test]
    fn test_space_error_warn_nopipe_1() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd
            .arg("-p")
            .arg(file_out_a)
            .arg("/dev/full")
            .set_stdout(make_broken_pipe());

        let (content, output) = run_tee(proc);

        expect_failure(&output, "No space left");
        expect_correct(file_out_a, &at, content.as_str());
    }

    #[test]
    fn test_space_error_warn_nopipe_2() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd
            .arg("--output-error")
            .arg(file_out_a)
            .arg("/dev/full")
            .set_stdout(make_broken_pipe());

        let (content, output) = run_tee(proc);

        expect_failure(&output, "No space left");
        expect_correct(file_out_a, &at, content.as_str());
    }

    #[test]
    fn test_space_error_warn_nopipe_3() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd
            .arg("--output-error=warn-nopipe")
            .arg(file_out_a)
            .arg("/dev/full")
            .set_stdout(make_broken_pipe());

        let (content, output) = run_tee(proc);

        expect_failure(&output, "No space left");
        expect_correct(file_out_a, &at, content.as_str());
    }

    #[test]
    fn test_space_error_warn() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd
            .arg("--output-error=warn")
            .arg(file_out_a)
            .arg("/dev/full")
            .set_stdout(make_broken_pipe());

        let (content, output) = run_tee(proc);

        expect_failure(&output, "No space left");
        expect_correct(file_out_a, &at, content.as_str());
    }

    #[test]
    fn test_space_error_exit() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd
            .arg("--output-error=exit")
            .arg(file_out_a)
            .arg("/dev/full")
            .set_stdout(make_broken_pipe());

        let (content, output) = run_tee(proc);

        expect_failure(&output, "No space left");
        expect_short(file_out_a, &at, content.as_str());
    }

    #[test]
    fn test_space_error_exit_nopipe() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd
            .arg("--output-error=exit-nopipe")
            .arg(file_out_a)
            .arg("/dev/full")
            .set_stdout(make_broken_pipe());

        let (content, output) = run_tee(proc);

        expect_failure(&output, "No space left");
        expect_short(file_out_a, &at, content.as_str());
    }

    #[test]
    fn test_pipe_mode_broken_pipe_only() {
        new_ucmd!()
            .timeout(Duration::from_secs(1))
            .arg("-p")
            .set_stdin(make_hanging_read())
            .set_stdout(make_broken_pipe())
            .succeeds();
    }

    #[test]
    fn test_pipe_mode_broken_pipe_file() {
        let (at, mut ucmd) = at_and_ucmd!();

        let file_out_a = "tee_file_out_a";

        let proc = ucmd
            .arg("-p")
            .arg(file_out_a)
            .set_stdout(make_broken_pipe());

        let (content, output) = run_tee(proc);

        expect_success(&output);
        expect_correct(file_out_a, &at, content.as_str());
    }
}
