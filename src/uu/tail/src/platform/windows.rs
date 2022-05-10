/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alexander Batischev <eual.jp@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate winapi;

use self::winapi::shared::minwindef::DWORD;
use self::winapi::um::handleapi::CloseHandle;
use self::winapi::um::processthreadsapi::OpenProcess;
use self::winapi::um::synchapi::WaitForSingleObject;
use self::winapi::um::winbase::{WAIT_FAILED, WAIT_OBJECT_0};
use self::winapi::um::winnt::{HANDLE, SYNCHRONIZE};

pub type Pid = DWORD;

pub struct ProcessChecker {
    dead: bool,
    handle: HANDLE,
}

impl ProcessChecker {
    pub fn new(process_id: self::Pid) -> Self {
        #[allow(non_snake_case)]
        let FALSE = 0i32;
        let h = unsafe { OpenProcess(SYNCHRONIZE, FALSE, process_id as DWORD) };
        Self {
            dead: h.is_null(),
            handle: h,
        }
    }

    #[allow(clippy::wrong_self_convention)]
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
