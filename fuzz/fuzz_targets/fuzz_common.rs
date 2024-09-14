// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use libc::STDIN_FILENO;
use libc::{close, dup, dup2, pipe, STDERR_FILENO, STDOUT_FILENO};
use rand::prelude::SliceRandom;
use rand::Rng;
use similar::TextDiff;
use std::ffi::OsString;
use std::io::{Seek, SeekFrom, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicBool, Once};
use std::{io, thread};

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

        println!("version_output {:#?}", version_output);

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
        write!(input_file, "{}", input_str).unwrap();
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
    println!("Test Type: {}", test_type);
    println!("Input: {}", input);
    if let Some(pipe) = pipe_input {
        println!("Pipe: {}", pipe);
    }

    let mut discrepancies = Vec::new();
    let mut should_panic = false;

    if rust_result.stdout.trim() != gnu_result.stdout.trim() {
        discrepancies.push("stdout differs");
        println!("Rust stdout: {}", rust_result.stdout);
        println!("GNU stdout: {}", gnu_result.stdout);
        print_diff(&rust_result.stdout, &gnu_result.stdout);
        should_panic = true;
    }
    if rust_result.stderr.trim() != gnu_result.stderr.trim() {
        discrepancies.push("stderr differs");
        println!("Rust stderr: {}", rust_result.stderr);
        println!("GNU stderr: {}", gnu_result.stderr);
        print_diff(&rust_result.stderr, &gnu_result.stderr);
        if fail_on_stderr_diff {
            should_panic = true;
        }
    }
    if rust_result.exit_code != gnu_result.exit_code {
        discrepancies.push("exit code differs");
        println!("Rust exit code: {}", rust_result.exit_code);
        println!("GNU exit code: {}", gnu_result.exit_code);
        should_panic = true;
    }

    if discrepancies.is_empty() {
        println!("All outputs and exit codes matched.");
    } else {
        println!("Discrepancy detected: {}", discrepancies.join(", "));
        if should_panic {
            panic!("Test failed for {}: {}", test_type, input);
        } else {
            println!(
                "Test completed with discrepancies for {}: {}",
                test_type, input
            );
        }
    }
}

/// When we have different outputs, print the diff
fn print_diff(rust_output: &str, gnu_output: &str) {
    println!("Diff=");
    let diff = TextDiff::from_lines(rust_output, gnu_output);
    for change in diff.iter_all_changes() {
        print!("{}{}", change.tag(), change);
    }
    println!();
}

pub fn generate_random_string(max_length: usize) -> String {
    let mut rng = rand::thread_rng();
    let valid_utf8: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
        .chars()
        .collect();
    let invalid_utf8 = [0xC3, 0x28]; // Invalid UTF-8 sequence
    let mut result = String::new();

    for _ in 0..rng.gen_range(0..=max_length) {
        if rng.gen_bool(0.9) {
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
