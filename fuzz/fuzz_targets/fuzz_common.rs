// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use libc::{close, dup, dup2, pipe, STDERR_FILENO, STDOUT_FILENO};
use std::ffi::OsString;
use std::io;
use std::io::Write;
use std::os::fd::RawFd;
use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicBool, Once};

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

pub fn generate_and_run_uumain<F>(args: &[OsString], uumain_function: F) -> (String, String, i32)
where
    F: FnOnce(std::vec::IntoIter<OsString>) -> i32,
{
    let uumain_exit_status;

    // Duplicate the stdout and stderr file descriptors
    let original_stdout_fd = unsafe { dup(STDOUT_FILENO) };
    let original_stderr_fd = unsafe { dup(STDERR_FILENO) };
    if original_stdout_fd == -1 || original_stderr_fd == -1 {
        return (
            "Failed to duplicate STDOUT_FILENO or STDERR_FILENO".to_string(),
            "".to_string(),
            -1,
        );
    }
    println!("Running test {:?}", &args[0..]);
    let mut pipe_stdout_fds = [-1; 2];
    let mut pipe_stderr_fds = [-1; 2];

    // Create pipes for stdout and stderr
    if unsafe { pipe(pipe_stdout_fds.as_mut_ptr()) } == -1
        || unsafe { pipe(pipe_stderr_fds.as_mut_ptr()) } == -1
    {
        return ("Failed to create pipes".to_string(), "".to_string(), -1);
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
        return (
            "Failed to redirect STDOUT_FILENO or STDERR_FILENO".to_string(),
            "".to_string(),
            -1,
        );
    }

    uumain_exit_status = uumain_function(args.to_owned().into_iter());
    io::stdout().flush().unwrap();
    io::stderr().flush().unwrap();

    // Restore the original stdout and stderr
    if unsafe { dup2(original_stdout_fd, STDOUT_FILENO) } == -1
        || unsafe { dup2(original_stderr_fd, STDERR_FILENO) } == -1
    {
        return (
            "Failed to restore the original STDOUT_FILENO or STDERR_FILENO".to_string(),
            "".to_string(),
            -1,
        );
    }
    unsafe {
        close(original_stdout_fd);
        close(original_stderr_fd);
    }
    unsafe { close(pipe_stdout_fds[1]) };
    unsafe { close(pipe_stderr_fds[1]) };

    let captured_stdout = read_from_fd(pipe_stdout_fds[0]).trim().to_string();
    let captured_stderr = read_from_fd(pipe_stderr_fds[0]).to_string();
    let captured_stderr = captured_stderr
        .splitn(2, ':')
        .nth(1)
        .unwrap_or("")
        .trim()
        .to_string();

    (captured_stdout, captured_stderr, uumain_exit_status)
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
) -> Result<(String, String, i32), (String, String, i32)> {
    if check_gnu {
        match is_gnu_cmd(cmd_path) {
            Ok(_) => {} // if the check passes, do nothing
            Err(e) => {
                // Convert the io::Error into the function's error type
                return Err((String::new(), e.to_string(), -1));
            }
        }
    }

    let mut command = Command::new(cmd_path);
    for arg in args {
        command.arg(arg);
    }

    let output = match command.output() {
        Ok(output) => output,
        Err(e) => return Err((String::new(), e.to_string(), -1)),
    };
    let exit_code = output.status.code().unwrap_or(-1);

    // Here we get stdout and stderr as Strings
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stderr = stderr
        .splitn(2, ':')
        .nth(1)
        .unwrap_or("")
        .trim()
        .to_string();

    if output.status.success() || !check_gnu {
        Ok((stdout, stderr, exit_code))
    } else {
        Err((stdout, stderr, exit_code))
    }
}
