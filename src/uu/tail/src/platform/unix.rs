// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) stdlib, ISCHR, GETFD
// spell-checker:ignore (options) EPERM, ENOSYS, NOSYS

use rustix::process::{Pid as RustixPid, test_kill_process};

pub type Pid = i32;

pub struct ProcessChecker {
    pid: Pid,
}

impl ProcessChecker {
    pub fn new(process_id: Pid) -> Self {
        Self { pid: process_id }
    }

    pub fn is_dead(&self) -> bool {
        let Some(pid) = RustixPid::from_raw(self.pid) else {
            return true;
        };
        match test_kill_process(pid) {
            Ok(()) => false,
            Err(rustix::io::Errno::PERM) => false,
            Err(_) => true,
        }
    }
}

impl Drop for ProcessChecker {
    fn drop(&mut self) {}
}

pub fn supports_pid_checks(pid: Pid) -> bool {
    let Some(pid) = RustixPid::from_raw(pid) else {
        return false;
    };
    match test_kill_process(pid) {
        Ok(()) => true,
        Err(rustix::io::Errno::NOSYS) => false,
        Err(_) => true,
    }
}

//pub fn stdin_is_bad_fd() -> bool {
// FIXME: Detect a closed file descriptor, e.g.: `tail <&-`
// this is never `true`, even with `<&-` because Rust's stdlib is reopening fds as /dev/null
// see also: https://github.com/uutils/coreutils/issues/2873
// (gnu/tests/tail-2/follow-stdin.sh fails because of this)
// unsafe { libc::fcntl(fd, libc::F_GETFD) == -1 && get_errno() == libc::EBADF }
//false
//}
