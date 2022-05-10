/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alexander Batischev <eual.jp@gmail.com>
 * (c) Thomas Queiroz <thomasqueirozb@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

// spell-checker:ignore (ToDO) errno EPERM ENOSYS

use std::io::{stdin, Error};

use std::os::unix::prelude::AsRawFd;

use nix::sys::stat::fstat;

use libc::{S_IFIFO, S_IFSOCK};

pub type Pid = libc::pid_t;

pub struct ProcessChecker {
    pid: self::Pid,
}

impl ProcessChecker {
    pub fn new(process_id: self::Pid) -> Self {
        Self { pid: process_id }
    }

    // Borrowing mutably to be aligned with Windows implementation
    #[allow(clippy::wrong_self_convention)]
    pub fn is_dead(&mut self) -> bool {
        unsafe { libc::kill(self.pid, 0) != 0 && get_errno() != libc::EPERM }
    }
}

impl Drop for ProcessChecker {
    fn drop(&mut self) {}
}

pub fn supports_pid_checks(pid: self::Pid) -> bool {
    unsafe { !(libc::kill(pid, 0) != 0 && get_errno() == libc::ENOSYS) }
}

#[inline]
fn get_errno() -> i32 {
    Error::last_os_error().raw_os_error().unwrap()
}

pub fn stdin_is_pipe_or_fifo() -> bool {
    let fd = stdin().lock().as_raw_fd();
    fd >= 0 // GNU tail checks fd >= 0
                            && match fstat(fd) {
                                Ok(stat) => {
                                    let mode = stat.st_mode as libc::mode_t;
                                    // NOTE: This is probably not the most correct way to check this
                                    (mode & S_IFIFO != 0) || (mode & S_IFSOCK != 0)
                                }
                                Err(err) => panic!("{}", err),
                            }
}
