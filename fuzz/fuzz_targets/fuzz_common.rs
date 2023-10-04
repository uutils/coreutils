// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use libc::{dup, dup2, STDOUT_FILENO};
use std::ffi::OsString;
use std::io;
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

pub fn generate_and_run_uumain<F>(args: &[OsString], uumain_function: F) -> (String, i32)
where
    F: FnOnce(std::vec::IntoIter<OsString>) -> i32,
{
    let uumain_exit_status;

    let original_stdout_fd = unsafe { dup(STDOUT_FILENO) };
    println!("Running test {:?}", &args[1..]);
    let mut pipe_fds = [-1; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()) };

    {
        unsafe { dup2(pipe_fds[1], STDOUT_FILENO) };
        uumain_exit_status = uumain_function(args.to_owned().into_iter());
        unsafe { dup2(original_stdout_fd, STDOUT_FILENO) };
        unsafe { libc::close(original_stdout_fd) };
    }
    unsafe { libc::close(pipe_fds[1]) };

    let mut captured_output = Vec::new();
    let mut read_buffer = [0; 1024];
    loop {
        let bytes_read = unsafe {
            libc::read(
                pipe_fds[0],
                read_buffer.as_mut_ptr() as *mut libc::c_void,
                read_buffer.len(),
            )
        };
        if bytes_read <= 0 {
            break;
        }
        captured_output.extend_from_slice(&read_buffer[..bytes_read as usize]);
    }

    unsafe { libc::close(pipe_fds[0]) };

    let my_output = String::from_utf8_lossy(&captured_output)
        .to_string()
        .trim()
        .to_owned();

    (my_output, uumain_exit_status)
}

pub fn run_gnu_cmd(
    cmd_path: &str,
    args: &[OsString],
    check_gnu: bool,
) -> Result<(String, i32), io::Error> {
    if check_gnu {
        is_gnu_cmd(cmd_path)?; // Check if it's a GNU implementation
    }

    let mut command = Command::new(cmd_path);
    for arg in args {
        command.arg(arg);
    }

    let output = command.output()?;
    let exit_code = output.status.code().unwrap_or(-1);
    if output.status.success() || !check_gnu {
        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            exit_code,
        ))
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("GNU command execution failed with exit code {}", exit_code),
        ))
    }
}
