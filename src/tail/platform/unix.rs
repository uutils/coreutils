/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alexander Batischev <eual.jp@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate libc;

use std::io::Error;

pub type Pid = libc::pid_t;

pub struct ProcessChecker { pid: self::Pid }

impl ProcessChecker {
    pub fn new(process_id: self::Pid) -> ProcessChecker {
        ProcessChecker { pid: process_id }
    }

    // Borrowing mutably to be aligned with Windows implementation
    pub fn is_dead(&mut self) -> bool {
        unsafe {
            libc::kill(self.pid, 0) != 0 && get_errno() != libc::EPERM
        }
    }
}

impl Drop for ProcessChecker {
    fn drop(&mut self) {
    }
}

pub fn supports_pid_checks(pid: self::Pid) -> bool {
    unsafe {
        !(libc::kill(pid, 0) != 0 && get_errno() == libc::ENOSYS)
    }
}

#[inline]
fn get_errno() -> i32 {
    Error::last_os_error().raw_os_error().unwrap()
}
