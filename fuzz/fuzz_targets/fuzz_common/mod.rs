// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use console::Style;
use libc::STDIN_FILENO;
use libc::{STDERR_FILENO, STDOUT_FILENO, close, dup, dup2, pipe};
use pretty_print::{
    print_diff, print_end_with_status, print_or_empty, print_section, print_with_style,
};
use rand::Rng;
use rand::prelude::IndexedRandom;
use std::env::temp_dir;
use std::ffi::OsString;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::{Once, atomic::AtomicBool};
use std::{io, thread};

pub mod pretty_print;

/// Represents the result of running a command, including its standard output,
/// standard error, and exit code.
pub struct CommandResult {
    /// The standard output (stdout) of the command as a string.
    pub stdout: String,

    /// The standard error (stderr) of the command as a string.
    pub stderr: String,

    /// The exit code of the command.
    pub exit_code: i32,
}

static CHECK_GNU: Once = Once::new();
static IS_GNU: AtomicBool = AtomicBool::new(false);

pub fn is_gnu_cmd(cmd_path: &str) -> Result<(), std::io::Error> {
    CHECK_GNU.call_once(|| {
        let version_output = Command::new(cmd_path).arg("--version").output().unwrap();

        println!("version_output {version_output:#?}");

        let version_str = String::from_utf8_lossy(&version_output.stdout).to_string();
        if version_str.contains("GNU coreutils") {
            IS_GNU.store(true, Ordering::Relaxed);
        }
    });

    if IS_GNU.load(Ordering::Relaxed) {
        Ok(())
    } else {
        panic!("Not the GNU implementation");
    }
}

pub fn generate_and_run_uumain<F>(
    args: &[OsString],
    uumain_function: F,
    pipe_input: Option<&str>,
) -> CommandResult
where
    F: FnOnce(std::vec::IntoIter<OsString>) -> i32 + Send + 'static,
{
    // Duplicate the stdout and stderr file descriptors
    let original_stdout_fd = unsafe { dup(STDOUT_FILENO) };
    let original_stderr_fd = unsafe { dup(STDERR_FILENO) };
    if original_stdout_fd == -1 || original_stderr_fd == -1 {
        return CommandResult {
            stdout: "".to_string(),
            stderr: "Failed to duplicate STDOUT_FILENO or STDERR_FILENO".to_string(),
            exit_code: -1,
        };
    }

    println!("Running test {:?}", &args[0..]);
    let mut pipe_stdout_fds = [-1; 2];
    let mut pipe_stderr_fds = [-1; 2];

    // Create pipes for stdout and stderr
    if unsafe { pipe(pipe_stdout_fds.as_mut_ptr()) } == -1
        || unsafe { pipe(pipe_stderr_fds.as_mut_ptr()) } == -1
    {
        return CommandResult {
            stdout: "".to_string(),
            stderr: "Failed to create pipes".to_string(),
            exit_code: -1,
        };
    }

    // Redirect stdout and stderr to their respective pipes
    if unsafe { dup2(pipe_stdout_fds[1], STDOUT_FILENO) } == -1
        || unsafe { dup2(pipe_stderr_fds[1], STDERR_FILENO) } == -1
    {
        unsafe {
            close(pipe_stdout_fds[0]);
            close(pipe_stdout_fds[1]);
            close(pipe_stderr_fds[0]);
            close(pipe_stderr_fds[1]);
        }
        return CommandResult {
            stdout: "".to_string(),
            stderr: "Failed to redirect STDOUT_FILENO or STDERR_FILENO".to_string(),
            exit_code: -1,
        };
    }

    let original_stdin_fd = if let Some(input_str) = pipe_input {
        // we have pipe input
        let mut input_file = tempfile::tempfile().unwrap();
        write!(input_file, "{input_str}").unwrap();
        input_file.seek(SeekFrom::Start(0)).unwrap();

        // Redirect stdin to read from the in-memory file
        let original_stdin_fd = unsafe { dup(STDIN_FILENO) };
        if original_stdin_fd == -1 || unsafe { dup2(input_file.as_raw_fd(), STDIN_FILENO) } == -1 {
            return CommandResult {
                stdout: "".to_string(),
                stderr: "Failed to set up stdin redirection".to_string(),
                exit_code: -1,
            };
        }
        Some(original_stdin_fd)
    } else {
        None
    };

    let (uumain_exit_status, captured_stdout, captured_stderr) = thread::scope(|s| {
        let out = s.spawn(|| read_from_fd(pipe_stdout_fds[0]));
        let err = s.spawn(|| read_from_fd(pipe_stderr_fds[0]));
        #[allow(clippy::unnecessary_to_owned)]
        // TODO: clippy wants us to use args.iter().cloned() ?
        let status = uumain_function(args.to_owned().into_iter());
        // Reset the exit code global variable in case we run another test after this one
        // See https://github.com/uutils/coreutils/issues/5777
        uucore::error::set_exit_code(0);
        io::stdout().flush().unwrap();
        io::stderr().flush().unwrap();
        unsafe {
            close(pipe_stdout_fds[1]);
            close(pipe_stderr_fds[1]);
            close(STDOUT_FILENO);
            close(STDERR_FILENO);
        }
        (status, out.join().unwrap(), err.join().unwrap())
    });

    // Restore the original stdout and stderr
    if unsafe { dup2(original_stdout_fd, STDOUT_FILENO) } == -1
        || unsafe { dup2(original_stderr_fd, STDERR_FILENO) } == -1
    {
        return CommandResult {
            stdout: "".to_string(),
            stderr: "Failed to restore the original STDOUT_FILENO or STDERR_FILENO".to_string(),
            exit_code: -1,
        };
    }
    unsafe {
        close(original_stdout_fd);
        close(original_stderr_fd);
    }

    // Restore the original stdin if it was modified
    if let Some(fd) = original_stdin_fd {
        if unsafe { dup2(fd, STDIN_FILENO) } == -1 {
            return CommandResult {
                stdout: "".to_string(),
                stderr: "Failed to restore the original STDIN".to_string(),
                exit_code: -1,
            };
        }
        unsafe { close(fd) };
    }

    CommandResult {
        stdout: captured_stdout,
        stderr: captured_stderr
            .split_once(':')
            .map(|x| x.1)
            .unwrap_or("")
            .trim()
            .to_string(),
        exit_code: uumain_exit_status,
    }
}

fn read_from_fd(fd: RawFd) -> String {
    let mut captured_output = Vec::new();
    let mut read_buffer = [0; 1024];
    loop {
        let bytes_read = unsafe {
            libc::read(
                fd,
                read_buffer.as_mut_ptr() as *mut libc::c_void,
                read_buffer.len(),
            )
        };

        if bytes_read == -1 {
            eprintln!("Failed to read from the pipe");
            break;
        }
        if bytes_read == 0 {
            break;
        }
        captured_output.extend_from_slice(&read_buffer[..bytes_read as usize]);
    }

    unsafe { libc::close(fd) };

    String::from_utf8_lossy(&captured_output).into_owned()
}

pub fn run_gnu_cmd(
    cmd_path: &str,
    args: &[OsString],
    check_gnu: bool,
    pipe_input: Option<&str>,
) -> Result<CommandResult, CommandResult> {
    if check_gnu {
        match is_gnu_cmd(cmd_path) {
            Ok(_) => {} // if the check passes, do nothing
            Err(e) => {
                // Convert the io::Error into the function's error type
                return Err(CommandResult {
                    stdout: String::new(),
                    stderr: e.to_string(),
                    exit_code: -1,
                });
            }
        }
    }

    let mut command = Command::new(cmd_path);
    for arg in args {
        command.arg(arg);
    }

    // See https://github.com/uutils/coreutils/issues/6794
    // uutils' coreutils is not locale-aware, and aims to mirror/be compatible with GNU Core Utilities's LC_ALL=C behavior
    command.env("LC_ALL", "C");

    let output = if let Some(input_str) = pipe_input {
        // We have an pipe input
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().expect("Failed to execute command");
        let child_stdin = child.stdin.as_mut().unwrap();
        child_stdin
            .write_all(input_str.as_bytes())
            .expect("Failed to write to stdin");

        match child.wait_with_output() {
            Ok(output) => output,
            Err(e) => {
                return Err(CommandResult {
                    stdout: String::new(),
                    stderr: e.to_string(),
                    exit_code: -1,
                });
            }
        }
    } else {
        // Just run with args
        match command.output() {
            Ok(output) => output,
            Err(e) => {
                return Err(CommandResult {
                    stdout: String::new(),
                    stderr: e.to_string(),
                    exit_code: -1,
                });
            }
        }
    };
    let exit_code = output.status.code().unwrap_or(-1);
    // Here we get stdout and stderr as Strings
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stderr = stderr
        .split_once(':')
        .map(|x| x.1)
        .unwrap_or("")
        .trim()
        .to_string();

    if output.status.success() || !check_gnu {
        Ok(CommandResult {
            stdout,
            stderr,
            exit_code,
        })
    } else {
        Err(CommandResult {
            stdout,
            stderr,
            exit_code,
        })
    }
}

/// Compare results from two different implementations of a command.
///
/// # Arguments
/// * `test_type` - The command.
/// * `input` - The input provided to the command.
/// * `rust_result` - The result of running the command with the Rust implementation.
/// * `gnu_result` - The result of running the command with the GNU implementation.
/// * `fail_on_stderr_diff` - Whether to fail the test if there is a difference in stderr output.
pub fn compare_result(
    test_type: &str,
    input: &str,
    pipe_input: Option<&str>,
    rust_result: &CommandResult,
    gnu_result: &CommandResult,
    fail_on_stderr_diff: bool,
) {
    print_section(format!("Compare result for: {test_type} {input}"));

    if let Some(pipe) = pipe_input {
        println!("Pipe: {pipe}");
    }

    let mut discrepancies = Vec::new();
    let mut should_panic = false;

    if rust_result.stdout.trim() != gnu_result.stdout.trim() {
        discrepancies.push("stdout differs");
        println!("Rust stdout:");
        print_or_empty(rust_result.stdout.as_str());
        println!("GNU stdout:");
        print_or_empty(gnu_result.stdout.as_ref());
        print_diff(&rust_result.stdout, &gnu_result.stdout);
        should_panic = true;
    }

    if rust_result.stderr.trim() != gnu_result.stderr.trim() {
        discrepancies.push("stderr differs");
        println!("Rust stderr:");
        print_or_empty(rust_result.stderr.as_str());
        println!("GNU stderr:");
        print_or_empty(gnu_result.stderr.as_str());
        print_diff(&rust_result.stderr, &gnu_result.stderr);
        if fail_on_stderr_diff {
            should_panic = true;
        }
    }

    if rust_result.exit_code != gnu_result.exit_code {
        discrepancies.push("exit code differs");
        println!(
            "Different exit code: (Rust: {}, GNU: {})",
            rust_result.exit_code, gnu_result.exit_code
        );
        should_panic = true;
    }

    if discrepancies.is_empty() {
        print_end_with_status("Same behavior", true);
    } else {
        print_with_style(
            format!("Discrepancies detected: {}", discrepancies.join(", ")),
            Style::new().red(),
        );
        if should_panic {
            print_end_with_status(
                format!("Test failed and will panic for: {test_type} {input}"),
                false,
            );
            panic!("Test failed for: {test_type} {input}");
        } else {
            print_end_with_status(
                format!("Test completed with discrepancies for: {test_type} {input}"),
                false,
            );
        }
    }
    println!();
}

pub fn generate_random_string(max_length: usize) -> String {
    let mut rng = rand::rng();
    let valid_utf8: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
        .chars()
        .collect();
    let invalid_utf8 = [0xC3, 0x28]; // Invalid UTF-8 sequence
    let mut result = String::new();

    for _ in 0..rng.random_range(0..=max_length) {
        if rng.random_bool(0.9) {
            let ch = valid_utf8.choose(&mut rng).unwrap();
            result.push(*ch);
        } else {
            let ch = invalid_utf8.choose(&mut rng).unwrap();
            if let Some(c) = char::from_u32(*ch as u32) {
                result.push(c);
            }
        }
    }

    result
}

#[allow(dead_code)]
pub fn generate_random_file() -> Result<String, std::io::Error> {
    let mut rng = rand::rng();
    let file_name: String = (0..10)
        .map(|_| rng.random_range(b'a'..=b'z') as char)
        .collect();
    let mut file_path = temp_dir();
    file_path.push(file_name);

    let mut file = File::create(&file_path)?;

    let content_length = rng.random_range(10..1000);
    let content: String = (0..content_length)
        .map(|_| (rng.random_range(b' '..=b'~') as char))
        .collect();

    file.write_all(content.as_bytes())?;

    Ok(file_path.to_str().unwrap().to_string())
}

#[allow(dead_code)]
pub fn replace_fuzz_binary_name(cmd: &str, result: &mut CommandResult) {
    let fuzz_bin_name = format!("fuzz/target/x86_64-unknown-linux-gnu/release/fuzz_{cmd}");

    result.stdout = result.stdout.replace(&fuzz_bin_name, cmd);
    result.stderr = result.stderr.replace(&fuzz_bin_name, cmd);
}
