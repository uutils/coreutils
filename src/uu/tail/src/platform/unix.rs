/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alexander Batischev <eual.jp@gmail.com>
 * (c) Thomas Queiroz <thomasqueirozb@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

// spell-checker:ignore (ToDO) stdlib, ISCHR, GETFD
// spell-checker:ignore (options) EPERM, ENOSYS

use libc::S_IFCHR;
use nix::sys::stat::fstat;
use std::io::Error;

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
#[inline]
pub fn stdin_is_pipe_or_fifo() -> bool {
    // IFCHR means the file (stdin) is a character input device, which is the case of a terminal.
    // We just need to check if stdin is not a character device here, because we are not interested
    // in the type of stdin itself.
    fstat(libc::STDIN_FILENO).map_or(false, |file| file.st_mode as libc::mode_t & S_IFCHR == 0)
}

//pub fn stdin_is_bad_fd() -> bool {
// FIXME: Detect a closed file descriptor, e.g.: `tail <&-`
// this is never `true`, even with `<&-` because Rust's stdlib is reopening fds as /dev/null
// see also: https://github.com/uutils/coreutils/issues/2873
// (gnu/tests/tail-2/follow-stdin.sh fails because of this)
// unsafe { libc::fcntl(fd, libc::F_GETFD) == -1 && get_errno() == libc::EBADF }
//false
//}
