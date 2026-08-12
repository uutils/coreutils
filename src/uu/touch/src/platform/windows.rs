// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) lpszfilepath

use std::io::stdout;
use std::os::windows::prelude::AsRawHandle;
use std::path::PathBuf;

use windows_sys::Win32::Foundation::{
    ERROR_INVALID_PARAMETER, ERROR_NOT_ENOUGH_MEMORY, ERROR_PATH_NOT_FOUND, GetLastError, HANDLE,
    MAX_PATH,
};
use windows_sys::Win32::Storage::FileSystem::{FILE_NAME_OPENED, GetFinalPathNameByHandleW};

use uucore::translate;

use crate::error::TouchError;

/// Returns a [`PathBuf`] to stdout, using `GetFinalPathNameByHandleW` to
/// attempt to get the path from the stdout handle.
pub fn pathbuf_from_stdout() -> Result<PathBuf, TouchError> {
    let handle = stdout().lock().as_raw_handle() as HANDLE;
    let mut file_path_buffer: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];

    // https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlea#examples
    // SAFETY: We transmute the handle to be able to cast *mut c_void into a
    // HANDLE (i32) so rustc will let us call GetFinalPathNameByHandleW. The
    // reference example code for GetFinalPathNameByHandleW implies that
    // it is safe for us to leave lpszfilepath uninitialized, so long as
    // the buffer size is correct. We know the buffer size (MAX_PATH) at
    // compile time. MAX_PATH is a small number (260) so we can cast it
    // to a u32.
    let ret = unsafe {
        GetFinalPathNameByHandleW(
            handle,
            file_path_buffer.as_mut_ptr(),
            file_path_buffer.len() as u32,
            FILE_NAME_OPENED,
        )
    };

    let buffer_size = match ret {
        ERROR_PATH_NOT_FOUND | ERROR_NOT_ENOUGH_MEMORY | ERROR_INVALID_PARAMETER => {
            return Err(TouchError::WindowsStdoutPathError(
                translate!("touch-error-windows-stdout-path-failed", "code" => ret),
            ));
        }
        0 => {
            return Err(TouchError::WindowsStdoutPathError(translate!(
            "touch-error-windows-stdout-path-failed",
                "code".to_string() =>
                format!(
                    "{}",
                    // SAFETY: GetLastError is thread-safe and has no documented memory unsafety.
                    unsafe { GetLastError() }
                ),
            )));
        }
        e => e as usize,
    };

    // Don't include the null terminator
    Ok(String::from_utf16(&file_path_buffer[0..buffer_size])
        .map_err(|e| TouchError::WindowsStdoutPathError(e.to_string()))?
        .into())
}
