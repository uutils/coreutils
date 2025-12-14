// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use uucore::fs::FileInformation;
use winapi_util::AsHandleRef;
use windows_sys::Win32::Storage::FileSystem::{
    FILE_NAME_NORMALIZED, GetFinalPathNameByHandleW, VOLUME_NAME_NT,
};

/// An unsafe overwrite occurs when the same file is used as both stdin and stdout
/// and the stdout file is not empty.
pub fn is_unsafe_overwrite<I: AsHandleRef, O: AsHandleRef>(input: &I, output: &O) -> bool {
    if !is_same_file_by_path(input, output) {
        return false;
    }

    // Check if the output file is empty
    FileInformation::from_file(output)
        .map(|info| info.file_size() > 0)
        .unwrap_or(false)
}

/// Get the file path for a file handle
fn get_file_path_from_handle<F: AsHandleRef>(file: &F) -> Option<PathBuf> {
    let handle = file.as_raw();
    let mut path_buf = vec![0u16; 4096];

    // SAFETY: We should check how many bytes was written to `path_buf`
    // and only read that many bytes from it.
    let len = unsafe {
        GetFinalPathNameByHandleW(
            handle,
            path_buf.as_mut_ptr(),
            path_buf.len() as u32,
            FILE_NAME_NORMALIZED | VOLUME_NAME_NT,
        )
    };
    if len == 0 {
        return None;
    }
    let path = OsString::from_wide(&path_buf[..len as usize]);
    Some(PathBuf::from(path))
}

/// Compare two file handles if they correspond to the same file
fn is_same_file_by_path<A: AsHandleRef, B: AsHandleRef>(a: &A, b: &B) -> bool {
    match (get_file_path_from_handle(a), get_file_path_from_handle(b)) {
        (Some(path1), Some(path2)) => path1 == path2,
        _ => false,
    }
}
