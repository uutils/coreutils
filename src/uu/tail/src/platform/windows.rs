// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_FAILED, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{
    OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
};

pub type Pid = u32;

pub enum ProcessChecker {
    Alive(HANDLE),
    Dead,
}

impl ProcessChecker {
    pub fn new(process_id: self::Pid) -> Self {
        let h = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, process_id) };
        match h {
            Ok(h) => Self::Alive(h),
            Err(_) => Self::Dead,
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn is_dead(&mut self) -> bool {
        match self {
            Self::Alive(h) => {
                let status = unsafe {  WaitForSingleObject(*h, 0) };
                if status == WAIT_OBJECT_0 || status == WAIT_FAILED {
                    *self = Self::Dead;
                    true
                } else {
                    false
                }
            }
            Self::Dead => true,
        }
    }
}

impl Drop for ProcessChecker {
    fn drop(&mut self) {
        if let Self::Alive(h) = self {
            unsafe {
                let _ = CloseHandle(*h);
            }
        }
    }
}

pub fn supports_pid_checks(_pid: self::Pid) -> bool {
    true
}
