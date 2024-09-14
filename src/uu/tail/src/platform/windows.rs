// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HANDLE, WAIT_FAILED, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{
    OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
};

pub type Pid = u32;

pub struct ProcessChecker {
    dead: bool,
    handle: HANDLE,
}

impl ProcessChecker {
    pub fn new(process_id: self::Pid) -> Self {
        #[allow(non_snake_case)]
        let FALSE: BOOL = 0;
        let h = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, FALSE, process_id) };
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
