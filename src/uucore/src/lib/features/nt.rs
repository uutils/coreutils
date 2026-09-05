// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Windows NT API helpers with RAII wrappers.

use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use std::{io::Error, mem::MaybeUninit};

use crate::error::{UResult, USimpleError};

use windows_sys::Win32::Foundation::NTSTATUS;

pub const SYNCHRONIZE: u32 = 0x00100000;
pub const FILE_SHARE_READ: u32 = 0x00000001;
pub const FILE_SHARE_WRITE: u32 = 0x00000002;
pub const FILE_SHARE_DELETE: u32 = 0x00000004;
pub const FILE_DIRECTORY_FILE: u32 = 0x00000001;
pub const FILE_SYNCHRONOUS_IO_NONALERT: u32 = 0x00000020;
pub const FILE_OPEN_FOR_FREE_SPACE_QUERY: u32 = 0x00800000;

const OBJ_CASE_INSENSITIVE: u32 = 0x00000040;

pub const FILE_REMOTE_DEVICE: u32 = 0x00000010;

#[allow(non_upper_case_globals)]
pub const FileFsDeviceInformation: u32 = 4;
#[allow(non_upper_case_globals)]
pub const FileFsAttributeInformation: u32 = 5;
#[allow(non_upper_case_globals)]
pub const FileFsFullSizeInformation: u32 = 7;

#[repr(C)]
pub struct FILE_FS_DEVICE_INFORMATION {
    pub device_type: u32,
    pub characteristics: u32,
}

#[repr(C)]
pub struct FILE_FS_ATTRIBUTE_INFORMATION {
    pub file_system_attributes: u32,
    pub maximum_component_name_length: i32,
    pub file_system_name_length: u32,
    pub file_system_name: [u16; 128],
}

#[repr(C)]
pub struct FILE_FS_FULL_SIZE_INFORMATION {
    pub total_allocation_units: i64,
    pub caller_available_allocation_units: i64,
    pub actual_available_allocation_units: i64,
    pub sectors_per_allocation_unit: u32,
    pub bytes_per_sector: u32,
}

#[repr(C)]
struct UNICODE_STRING {
    length: u16,
    maximum_length: u16,
    buffer: *mut u16,
}

#[repr(C)]
struct OBJECT_ATTRIBUTES {
    length: u32,
    root_directory: *mut std::ffi::c_void,
    object_name: *const UNICODE_STRING,
    attributes: u32,
    security_descriptor: *mut std::ffi::c_void,
    security_quality_of_service: *mut std::ffi::c_void,
}

#[repr(C)]
struct IO_STATUS_BLOCK {
    status: NTSTATUS,
    information: usize,
}

unsafe extern "system" {
    fn NtOpenFile(
        file_handle: *mut *mut std::ffi::c_void,
        desired_access: u32,
        object_attributes: *const OBJECT_ATTRIBUTES,
        io_status_block: *mut IO_STATUS_BLOCK,
        share_access: u32,
        open_options: u32,
    ) -> NTSTATUS;

    fn NtClose(handle: *mut std::ffi::c_void) -> NTSTATUS;

    fn NtQueryVolumeInformationFile(
        file_handle: *mut std::ffi::c_void,
        io_status_block: *mut IO_STATUS_BLOCK,
        fs_information: *mut std::ffi::c_void,
        length: u32,
        fs_information_class: u32,
    ) -> NTSTATUS;

    fn RtlDosPathNameToNtPathName_U(
        dos_file_name: *const u16,
        nt_file_name: *mut UNICODE_STRING,
        file_part: *mut *mut u16,
        reserved: *mut std::ffi::c_void,
    ) -> u8;

    fn RtlFreeUnicodeString(unicode_string: *mut UNICODE_STRING);
}

#[repr(transparent)]
pub struct NtHandle(*mut std::ffi::c_void);

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
        Self(UNICODE_STRING {
            length: 0,
            maximum_length: 0,
            buffer: ptr::null_mut(),
        })
    }
}

impl Drop for UnicodeString {
    fn drop(&mut self) {
        if !self.0.buffer.is_null() {
            unsafe { RtlFreeUnicodeString(&raw mut self.0) };
        }
    }
}

/// Opens a file or directory via `NtOpenFile`.
///
/// The file is opened with full share access (`READ | WRITE | DELETE`).
pub fn open_file(path: &Path, desired_access: u32, open_options: u32) -> UResult<NtHandle> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut nt_path = UnicodeString::empty();
    if unsafe {
        RtlDosPathNameToNtPathName_U(
            wide.as_ptr(),
            &raw mut nt_path.0,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(USimpleError::new(
            1,
            format!(
                "RtlDosPathNameToNtPathName_U failed: {}",
                Error::last_os_error()
            ),
        ));
    }

    let attr = OBJECT_ATTRIBUTES {
        length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        root_directory: ptr::null_mut(),
        object_name: &nt_path.0,
        attributes: OBJ_CASE_INSENSITIVE,
        security_descriptor: ptr::null_mut(),
        security_quality_of_service: ptr::null_mut(),
    };
    let mut handle = ptr::null_mut();
    let mut iosb = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
    let status = unsafe {
        NtOpenFile(
            &raw mut handle,
            desired_access,
            &attr,
            iosb.as_mut_ptr(),
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            open_options,
        )
    };
    if status < 0 {
        return Err(USimpleError::new(
            1,
            format!("NtOpenFile failed: 0x{:08X}", status as u32),
        ));
    }
    Ok(NtHandle(handle))
}

/// Queries volume information for the file associated with the given handle.
///
/// # Safety
///
/// `T` must be the correct struct for the given `information_class`.
pub unsafe fn query_volume_information<T>(handle: &NtHandle, information_class: u32) -> UResult<T> {
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
        return Err(USimpleError::new(
            1,
            format!(
                "NtQueryVolumeInformationFile failed: 0x{:08X}",
                status as u32
            ),
        ));
    }
    Ok(unsafe { info.assume_init() })
}
