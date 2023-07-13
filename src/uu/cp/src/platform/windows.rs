//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

use std::{
    cmp::min,
    fs::File,
    mem::{size_of, size_of_val, zeroed},
    os::windows::prelude::AsRawHandle,
    os::windows::{fs::MetadataExt, prelude::RawHandle},
    path::Path,
    ptr::{null, null_mut},
};

use crate::{CopyDebug, CopyResult, OffloadReflinkDebug, ReflinkMode, SparseDebug, SparseMode};

use quick_error::ResultExt;
use windows_sys::Win32::{
    Storage::FileSystem::FILE_ATTRIBUTE_SPARSE_FILE,
    System::{
        Ioctl::{
            DUPLICATE_EXTENTS_DATA, FILE_SET_SPARSE_BUFFER, FSCTL_DUPLICATE_EXTENTS_TO_FILE,
            FSCTL_GET_INTEGRITY_INFORMATION, FSCTL_GET_INTEGRITY_INFORMATION_BUFFER,
            FSCTL_SET_SPARSE,
        },
        IO::DeviceIoControl,
    },
};

fn set_sparse(handle: RawHandle, sparse: bool) -> std::io::Result<()> {
    unsafe {
        if 0 == DeviceIoControl(
            handle as isize,
            FSCTL_SET_SPARSE,
            &FILE_SET_SPARSE_BUFFER {
                SetSparse: sparse.into(),
            } as *const FILE_SET_SPARSE_BUFFER as _,
            size_of::<FILE_SET_SPARSE_BUFFER>() as u32,
            null_mut(),
            0,
            null_mut(),
            null_mut(),
        ) {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}
// Use DeviceIoControl(FSCTL_DUPLICATE_EXTENTS_TO_FILE) to do a copy-on-write clone
fn duplicate_extents(source: &Path, dest: &Path) -> std::io::Result<()> {
    let src_file = File::open(source)?;
    let dest_file = File::create(dest)?;
    let size: i64 = src_file.metadata()?.len().try_into().unwrap();
    let src_handle = src_file.as_raw_handle();
    let dest_handle = dest_file.as_raw_handle();

    // we need to set the destination as sparse, to avoid writing a bunch of zeros to disk
    set_sparse(dest_handle, true)?;
    // if the source was not sparse we should unset sparse on the destination
    // when we're done or if there's an error
    let should_be_sparse =
        dest_file.metadata()?.file_attributes() & FILE_ATTRIBUTE_SPARSE_FILE != 0;

    dest_file.set_len(size as u64)?;

    let mut dup_extents_data = DUPLICATE_EXTENTS_DATA {
        FileHandle: src_handle as isize,
        ByteCount: 0,
        SourceFileOffset: 0,
        TargetFileOffset: 0,
    };

    let mut integrety_info: FSCTL_GET_INTEGRITY_INFORMATION_BUFFER;
    unsafe {
        integrety_info = zeroed();
        // this will fail on non-ReFS filesystems, which is fine
        // since ReFS is the only major filesystem on windows that supports reflinking
        if 0 == DeviceIoControl(
            src_handle as isize,
            FSCTL_GET_INTEGRITY_INFORMATION,
            null(),
            0,
            &mut integrety_info as *mut _ as _,
            size_of_val(&integrety_info).try_into().unwrap(),
            null_mut(),
            null_mut(),
        ) {
            set_sparse(dest_handle, should_be_sparse)?;
            return Err(std::io::Error::last_os_error());
        }
    }
    let blksize: i64 = integrety_info.ClusterSizeInBytes.into();
    while dup_extents_data.SourceFileOffset < size {
        let mut chunk_size = min(
            4_294_967_296 - blksize,
            size - dup_extents_data.SourceFileOffset,
        );
        if chunk_size % blksize != 0 {
            chunk_size += blksize - (chunk_size % blksize);
        }
        dup_extents_data.ByteCount = chunk_size;
        unsafe {
            if 0 == DeviceIoControl(
                dest_handle as isize,
                FSCTL_DUPLICATE_EXTENTS_TO_FILE,
                &dup_extents_data as *const _ as _,
                size_of_val(&dup_extents_data).try_into().unwrap(),
                null_mut(),
                0,
                null_mut(),
                null_mut(),
            ) {
                set_sparse(dest_handle, should_be_sparse)?;
                return Err(std::io::Error::last_os_error());
            }
        }
        dup_extents_data.SourceFileOffset += chunk_size;
        dup_extents_data.TargetFileOffset += chunk_size;
    }
    set_sparse(dest_handle, should_be_sparse)?;

    Ok(())
}
pub(crate) fn copy_on_write(
    source: &Path,
    dest: &Path,
    reflink_mode: ReflinkMode,
    sparse_mode: SparseMode,
    context: &str,
) -> CopyResult<CopyDebug> {
    if sparse_mode != SparseMode::Auto {
        return Err("--sparse is only supported on linux".to_string().into());
    }
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unsupported,
        reflink: OffloadReflinkDebug::Unknown,
        sparse_detection: SparseDebug::Unsupported,
    };
    let result = match reflink_mode {
        ReflinkMode::Never => {
            copy_debug.reflink = OffloadReflinkDebug::No;
            std::fs::copy(source, dest).map(|_| ())
        }
        ReflinkMode::Always => {
            copy_debug.reflink = OffloadReflinkDebug::Yes;
            duplicate_extents(source, dest)
        }
        ReflinkMode::Auto => match duplicate_extents(source, dest) {
            Err(_) => {
                copy_debug.reflink = OffloadReflinkDebug::No;
                std::fs::copy(source, dest).map(|_| ())
            }
            Ok(_) => {
                copy_debug.reflink = OffloadReflinkDebug::Yes;
                Ok(())
            }
        },
    };
    result.context(context)?;
    Ok(copy_debug)
}
