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
            Ok(()) | Err(rustix::io::Errno::PERM) => false,
            Err(_) => true,
        }
    }
}

impl Drop for ProcessChecker {
    fn drop(&mut self) {}
}

pub fn supports_pid_checks(pid: Pid) -> bool {
    RustixPid::from_raw(pid).is_some_and(|p| test_kill_process(p) != Err(rustix::io::Errno::NOSYS))
}
