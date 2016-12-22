/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alexander Batischev <eual.jp@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate winapi;
extern crate kernel32;

use self::kernel32::{OpenProcess, CloseHandle, WaitForSingleObject};
use self::winapi::minwindef::DWORD;
use self::winapi::winbase::{WAIT_OBJECT_0, WAIT_FAILED};
use self::winapi::winnt::{HANDLE, SYNCHRONIZE};

pub type Pid = DWORD;

pub struct ProcessChecker {
    dead: bool,
    handle: HANDLE
}

impl ProcessChecker {
    pub fn new(process_id: self::Pid) -> ProcessChecker {
        #[allow(non_snake_case)]
        let FALSE = 0i32;
        let h = unsafe {
            OpenProcess(SYNCHRONIZE, FALSE, process_id as DWORD)
        };
        ProcessChecker { dead: h.is_null(), handle: h }
    }

    pub fn is_dead(&mut self) -> bool {
        if !self.dead {
            self.dead = unsafe {
                let status = WaitForSingleObject(self.handle, 0);
                status == WAIT_OBJECT_0 || status == WAIT_FAILED
            }
        }

        self.dead
    }
}

impl Drop for ProcessChecker {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

pub fn supports_pid_checks(_pid: self::Pid) -> bool {
    true
}
