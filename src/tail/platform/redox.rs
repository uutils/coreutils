extern crate syscall;

use self::syscall::{Error, EPERM, ENOSYS};

pub type Pid = usize;

pub struct ProcessChecker {
    pid: self::Pid,
}

impl ProcessChecker {
    pub fn new(process_id: self::Pid) -> ProcessChecker {
        ProcessChecker { pid: process_id }
    }

    // Borrowing mutably to be aligned with Windows implementation
    pub fn is_dead(&mut self) -> bool {
        let res = syscall::kill(self.pid, 0);
        res != Ok(0) && res != Err(Error::new(EPERM))
    }
}

pub fn supports_pid_checks(pid: self::Pid) -> bool {
    true
}
