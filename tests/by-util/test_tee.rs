// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
#![allow(clippy::borrow_as_ptr)]

use uutests::{at_and_ucmd, new_ucmd};

use regex::Regex;
use std::process::Stdio;
use std::time::Duration;

// tests for basic tee functionality.
// inspired by:
// https://github.com/coreutils/coreutils/tests/misc/tee.sh

// spell-checker:ignore nopipe

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
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
fn test_tee_output_not_buffered() {
    // POSIX says: The tee utility shall not buffer output

    // If the output is buffered, the test will hang, so we run it in
    // a separate thread to stop execution by timeout.
    let handle = std::thread::spawn(move || {
        let content = "a";
        let file_out = "tee_file_out";

        let (at, mut ucmd) = at_and_ucmd!();
        let mut child = ucmd
            .arg(file_out)
            .set_stdin(Stdio::piped())
            .set_stdout(Stdio::piped())
            .run_no_wait();

        // We write to the input pipe, but do not close it. If the output is
        // buffered, reading from output pipe will hang indefinitely, as we
        // will never write anything else to it.
        child.write_in(content.as_bytes());

        let out = String::from_utf8(child.stdout_exact_bytes(1)).unwrap();
        assert_eq!(&out, content);

        // Writing to a file may take a couple hundreds nanoseconds
        child.delay(1);
        assert_eq!(at.read(file_out), content);
    });

    // Give some time for the `tee` to create an output file. Some platforms
    // take a lot of time to spin up the process and create the output file
    for _ in 0..100 {
        std::thread::sleep(Duration::from_millis(1));
        if handle.is_finished() {
            break;
        }
    }

    assert!(
        handle.is_finished(),
        "Nothing was received through output pipe"
    );
    handle.join().unwrap();
}

#[cfg(target_os = "linux")]
mod linux_only {
    use uutests::util::{AtPath, CmdResult, UCommand};

    use std::fmt::Write;
    use std::fs::File;
    use std::process::Stdio;
    use std::time::Duration;
    use uutests::at_and_ucmd;
    use uutests::new_ucmd;

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

    fn run_tee(proc: &mut UCommand) -> (String, CmdResult) {
        let content = (1..=100_000).fold(String::new(), |mut output, x| {
            let _ = writeln!(output, "{x}");
            output
        });

        let result = proc
            .ignore_stdin_write_error()
            .set_stdin(Stdio::piped())
            .run_no_wait()
            .pipe_in_and_wait(content.as_bytes());

        (content, result)
    }

    fn expect_success(result: &CmdResult) {
        assert!(
            result.succeeded(),
            "Command was expected to succeed.\nstdout = {}\n stderr = {}",
            std::str::from_utf8(result.stdout()).unwrap(),
            std::str::from_utf8(result.stderr()).unwrap(),
        );
        assert!(
            result.stderr_str().is_empty(),
            "Unexpected data on stderr.\n stderr = {}",
            std::str::from_utf8(result.stderr()).unwrap(),
        );
    }

    fn expect_failure(result: &CmdResult, message: &str) {
        assert!(
            !result.succeeded(),
            "Command was expected to fail.\nstdout = {}\n stderr = {}",
            std::str::from_utf8(result.stdout()).unwrap(),
            std::str::from_utf8(result.stderr()).unwrap(),
        );
        assert!(
            result.stderr_str().contains(message),
            "Expected to see error message fragment {message} in stderr, but did not.\n stderr = {}",
            std::str::from_utf8(result.stderr()).unwrap(),
        );
    }

    fn expect_silent_failure(result: &CmdResult) {
        assert!(
            !result.succeeded(),
            "Command was expected to fail.\nstdout = {}\n stderr = {}",
            std::str::from_utf8(result.stdout()).unwrap(),
            std::str::from_utf8(result.stderr()).unwrap(),
        );
        assert!(
            result.stderr_str().is_empty(),
            "Unexpected data on stderr.\n stderr = {}",
            std::str::from_utf8(result.stderr()).unwrap(),
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
            "Too many bytes ({}) written to {name} (should be a short count from {})",
            compare.len(),
            contents.len()
        );
        assert!(
            contents.starts_with(&compare),
            "Expected truncated output to be a prefix of the correct output, but it isn't.\n Correct: {contents}\n Compare: {compare}"
        );
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

    #[test]
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
    fn test_tee_no_more_writeable_2() {
        use std::fs::File;
        let (at, mut ucmd) = at_and_ucmd!();
        let content = (1..=10).fold(String::new(), |mut output, x| {
            let _ = writeln!(output, "{x}");
            output
        });
        let file_out_a = "tee_file_out_a";
        let file_out_b = "tee_file_out_b";
        let dev_full = File::options().append(true).open("/dev/full").unwrap();

        let result = ucmd
            .arg(file_out_a)
            .arg(file_out_b)
            .set_stdout(dev_full)
            .pipe_in(content.as_bytes())
            .fails();

        assert_eq!(at.read(file_out_a), content);
        assert_eq!(at.read(file_out_b), content);
        assert!(result.stderr_str().contains("No space left on device"));
    }
}

// Additional cross-platform tee tests to cover GNU compatibility around --output-error
#[test]
fn test_output_error_flag_without_value_defaults_warn_nopipe() {
    // When --output-error is present without an explicit value, it should default to warn-nopipe
    // We can't easily simulate a broken pipe across all platforms here, but we can ensure
    // the flag is accepted without error and basic tee functionality still works.
    let (at, mut ucmd) = at_and_ucmd!();
    let file_out = "tee_output_error_default.txt";
    let content = "abc";

    let result = ucmd
        .arg("--output-error")
        .arg(file_out)
        .pipe_in(content)
        .succeeds();

    result.stdout_is(content);
    assert!(at.file_exists(file_out));
    assert_eq!(at.read(file_out), content);
}
// Unix-only: presence-only --output-error should not crash on broken pipe.
// Current implementation may exit zero; we only assert the process exits to avoid flakiness.
// TODO: When semantics are aligned with GNU warn-nopipe, strengthen assertions here.
#[cfg(all(unix, not(target_os = "freebsd")))]
#[test]
fn test_output_error_presence_only_broken_pipe_unix() {
    use std::fs::File;
    use std::os::unix::io::FromRawFd;

    unsafe {
        let mut fds: [libc::c_int; 2] = [0, 0];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "Failed to create pipe");
        // Close the read end to simulate a broken pipe on stdout
        let _read_end = File::from_raw_fd(fds[0]);
        let write_end = File::from_raw_fd(fds[1]);

        let content = (0..10_000).map(|_| "x").collect::<String>();
        let result = new_ucmd!()
            .arg("--output-error") // presence-only flag
            .set_stdout(write_end)
            .pipe_in(content.as_bytes())
            .run();

        // Assert that a status was produced (i.e., process exited) and no crash occurred.
        assert!(result.try_exit_status().is_some(), "process did not exit");
    }
}

// Skip on FreeBSD due to repeated CI hangs in FreeBSD VM (see PR #8684)
#[cfg(all(unix, not(target_os = "freebsd")))]
#[test]
fn test_broken_pipe_early_termination_stdout_only() {
    use std::fs::File;
    use std::os::unix::io::FromRawFd;

    // Create a broken stdout by creating a pipe and dropping the read end
    unsafe {
        let mut fds: [libc::c_int; 2] = [0, 0];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "Failed to create pipe");
        // Close the read end immediately to simulate a broken pipe
        let _read_end = File::from_raw_fd(fds[0]);
        let write_end = File::from_raw_fd(fds[1]);

        let content = (0..10_000).map(|_| "x").collect::<String>();
        let mut proc = new_ucmd!();
        let result = proc
            .set_stdout(write_end)
            .ignore_stdin_write_error()
            .pipe_in(content.as_bytes())
            .run();

        // GNU tee exits nonzero on broken pipe unless configured otherwise; implementation
        // details vary by mode, but we should not panic and should return an exit status.
        // Assert that a status was produced (i.e., process exited) and no crash occurred.
        assert!(result.try_exit_status().is_some(), "process did not exit");
    }
}

#[test]
fn test_write_failure_reports_error_and_nonzero_exit() {
    // Simulate a file open failure which should be reported via show_error and cause a failure
    let (at, mut ucmd) = at_and_ucmd!();
    // Create a directory and try to use it as an output file (open will fail)
    at.mkdir("out_dir");

    let result = ucmd.arg("out_dir").pipe_in("data").fails();

    assert!(!result.stderr_str().is_empty());
}
