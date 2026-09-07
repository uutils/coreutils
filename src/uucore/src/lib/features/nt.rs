// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Windows NT API helpers with RAII wrappers.

use std::ffi::OsString;
use std::mem::MaybeUninit;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::{io, ptr};

use windows_sys::Wdk::Foundation::{NtClose, OBJECT_ATTRIBUTES};
pub use windows_sys::Wdk::Storage::FileSystem::{
    FILE_DIRECTORY_FILE, FILE_OPEN_FOR_FREE_SPACE_QUERY, FILE_SYNCHRONOUS_IO_NONALERT,
    FileFsDeviceInformation, FileFsFullSizeInformation,
};
use windows_sys::Wdk::Storage::FileSystem::{
    FS_INFORMATION_CLASS, FileFsAttributeInformation, NtOpenFile, NtQueryVolumeInformationFile,
    RtlDosPathNameToNtPathName_U_WithStatus,
};
pub use windows_sys::Wdk::System::SystemServices::{
    FILE_FS_DEVICE_INFORMATION, FILE_FS_FULL_SIZE_INFORMATION, FILE_REMOTE_DEVICE,
};
use windows_sys::Win32::Foundation::{
    HANDLE, MAX_PATH, NTSTATUS, OBJ_CASE_INSENSITIVE, RtlNtStatusToDosError, UNICODE_STRING,
};
pub use windows_sys::Win32::Storage::FileSystem::{
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, SYNCHRONIZE,
};
use windows_sys::Win32::Storage::FileSystem::{GetFinalPathNameByHandleW, VOLUME_NAME_NT};
use windows_sys::Win32::System::{IO::IO_STATUS_BLOCK, WindowsProgramming::RtlFreeUnicodeString};

use crate::wide::ToWide as _;

#[cold]
fn nt_status_to_io_error(status: NTSTATUS) -> io::Error {
    // SAFETY: This function accepts any NTSTATUS and has no pointer arguments.
    io::Error::from_raw_os_error(unsafe { RtlNtStatusToDosError(status) } as i32)
}

#[repr(transparent)]
pub struct NtHandle(HANDLE);

impl Drop for NtHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { NtClose(self.0) };
        }
    }
}

#[repr(transparent)]
struct UnicodeString(UNICODE_STRING);

impl UnicodeString {
    fn empty() -> Self {
        Self(UNICODE_STRING::default())
    }
}

impl Drop for UnicodeString {
    fn drop(&mut self) {
        if !self.0.Buffer.is_null() {
            unsafe { RtlFreeUnicodeString(&raw mut self.0) };
        }
    }
}

/// Opens a file or directory via `NtOpenFile`.
///
/// The file is opened with full share access (`READ | WRITE | DELETE`).
pub fn open_file(path: &Path, desired_access: u32, open_options: u32) -> io::Result<NtHandle> {
    let wide: Vec<u16> = path.to_wide_null();
    let mut nt_path = UnicodeString::empty();
    let status = unsafe {
        RtlDosPathNameToNtPathName_U_WithStatus(
            wide.as_ptr(),
            &raw mut nt_path.0,
            ptr::null_mut(),
            ptr::null(),
        )
    };
    if status < 0 {
        return Err(nt_status_to_io_error(status));
    }

    let attr = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: ptr::null_mut(),
        ObjectName: &raw const nt_path.0,
        Attributes: OBJ_CASE_INSENSITIVE,
        SecurityDescriptor: ptr::null_mut(),
        SecurityQualityOfService: ptr::null_mut(),
    };
    let mut handle = ptr::null_mut();
    let mut iosb = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
    let status = unsafe {
        NtOpenFile(
            &raw mut handle,
            desired_access,
            &raw const attr,
            iosb.as_mut_ptr(),
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            open_options,
        )
    };
    if status < 0 {
        return Err(nt_status_to_io_error(status));
    }
    Ok(NtHandle(handle))
}

/// Returns the NT path of the file associated with the given handle.
pub fn query_nt_path(handle: &NtHandle) -> io::Result<OsString> {
    let mut buffer = vec![0u16; MAX_PATH as usize];
    loop {
        let length = unsafe {
            GetFinalPathNameByHandleW(
                handle.0,
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                VOLUME_NAME_NT,
            ) as usize
        };
        if length == 0 {
            return Err(io::Error::last_os_error());
        }
        if length < buffer.len() {
            return Ok(OsString::from_wide(&buffer[..length]));
        }
        buffer.resize(length, 0);
    }
}

/// Queries volume information for the file associated with the given handle.
///
/// # Safety
///
/// `T` must be the correct struct for the given `information_class`.
pub unsafe fn query_volume_information<T>(
    handle: &NtHandle,
    information_class: FS_INFORMATION_CLASS,
) -> io::Result<T> {
    let mut info = MaybeUninit::<T>::uninit();
    let mut iosb = MaybeUninit::<IO_STATUS_BLOCK>::uninit();

    let status = unsafe {
        NtQueryVolumeInformationFile(
            handle.0,
            iosb.as_mut_ptr(),
            info.as_mut_ptr().cast(),
            size_of::<T>() as u32,
            information_class,
        )
    };
    if status < 0 {
        return Err(nt_status_to_io_error(status));
    }

    // SAFETY: The caller guarantees that a successful query produces a valid T,
    // with any potentially uninitialized fields represented by MaybeUninit.
    Ok(unsafe { info.assume_init() })
}

/// Returns the filesystem type name, such as "NTFS" or "ReFS".
pub fn query_filesystem_name(handle: &NtHandle) -> io::Result<String> {
    const BUF_LEN: usize = 122; // --> sizeof(FILE_FS_ATTRIBUTE_INFORMATION) == 256

    // The official FILE_FS_ATTRIBUTE_INFORMATION definition uses a
    // variable-length array for FileSystemName, but we allocate a
    // fixed-size array to make things easier.
    #[repr(C)]
    #[allow(dead_code, non_snake_case)]
    struct FILE_FS_ATTRIBUTE_INFORMATION {
        FileSystemAttributes: u32,
        MaximumComponentNameLength: i32,
        FileSystemNameLength: u32,
        FileSystemName: [MaybeUninit<u16>; BUF_LEN],
    }

    let info: FILE_FS_ATTRIBUTE_INFORMATION =
        unsafe { query_volume_information(handle, FileFsAttributeInformation)? };

    let length = info.FileSystemNameLength as usize;
    if !length.is_multiple_of(2) || length > BUF_LEN * 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid filesystem name length",
        ));
    }

    // SAFETY: The query initialized the reported name bytes, and the length
    // check ensures they form whole u16 elements within the buffer.
    Ok(String::from_utf16_lossy(unsafe {
        std::slice::from_raw_parts(
            info.FileSystemName.as_ptr().cast::<u16>(),
            length / size_of::<u16>(),
        )
    }))
}
