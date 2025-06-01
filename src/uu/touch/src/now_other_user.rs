// spell-checker:ignore (terms) FDCWD libcs timeval utimensat
extern crate libc;

use std::ffi::CString;
use std::io::{self, Result};
use std::os::unix::prelude::*;
use std::path::Path;
use std::ptr;

pub fn set_symlink_file_times_now(path: &Path) -> Result<()> {
    set_file_times_for(path, false)
}

pub fn set_file_times_now(path: &Path) -> Result<()> {
    set_file_times_for(path, true)
}

fn set_file_times_for(p: &Path, follow_symlink: bool) -> Result<()> {
    let flags = if !follow_symlink {
        libc::AT_SYMLINK_NOFOLLOW
    } else {
        0
    };

    let p = CString::new(p.as_os_str().as_bytes())?;
    let rc = unsafe {
        libc::utimensat(
            libc::AT_FDCWD,
            p.as_ptr(),
            // FIXME: Might need to pass a nullptr to libc::timeval for some libcs here?
            ptr::null::<libc::timespec>(),
            flags,
        )
    };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
