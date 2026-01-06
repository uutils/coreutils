// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore fiemap iowr

#![cfg(target_os = "linux")]

use std::fs::File;
use std::io;
use std::os::unix::io::AsRawFd;

#[derive(Default)]
#[repr(C)]
pub struct Fiemap {
    pub fm_start: u64,
    pub fm_length: u64,
    pub fm_flags: u32,
    pub fm_mapped_extents: u32,
    pub fm_extent_count: u32,
    fm_reserved: u32,
}

#[derive(Default)]
#[repr(C)]
pub struct FiemapExtent {
    pub fe_logical: u64,
    pub fe_physical: u64,
    pub fe_length: u64,
    fe_reserved64: [u64; 2],
    pub fe_flags: u32,
    fe_reserved: [u32; 3],
}

pub const FIEMAP_EXTENT_COUNT: usize = 128;

#[repr(C)]
pub struct FiemapBuffer {
    pub header: Fiemap,
    pub extents: [FiemapExtent; FIEMAP_EXTENT_COUNT],
}

impl Default for FiemapBuffer {
    fn default() -> Self {
        Self {
            header: Fiemap::default(),
            extents: std::array::from_fn(|_| FiemapExtent::default()),
        }
    }
}

impl FiemapBuffer {
    pub fn new(offset: u64) -> Self {
        let mut buffer = Self::default();
        buffer.header.fm_start = offset;
        buffer.header.fm_length = u64::MAX;
        buffer.header.fm_extent_count = FIEMAP_EXTENT_COUNT as u32;
        buffer
    }
}

pub const FIEMAP_EXTENT_LAST: u32 = 0x00000001;
pub const FIEMAP_EXTENT_ENCODED: u32 = 0x00000008;
pub const FIEMAP_EXTENT_SHARED: u32 = 0x00002000;
pub const FS_IOC_FIEMAP: libc::Ioctl = libc::_IOWR::<Fiemap>(b'f' as u32, 11);

pub fn walk_fiemap_extents<F>(file: &File, start_offset: u64, mut visit: F) -> io::Result<()>
where
    F: FnMut(&FiemapExtent) -> bool,
{
    let mut offset = start_offset;

    loop {
        let mut buffer = FiemapBuffer::new(offset);

        let result = unsafe { libc::ioctl(file.as_raw_fd(), FS_IOC_FIEMAP, &mut buffer.header) };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }

        let mapped = buffer.header.fm_mapped_extents as usize;
        if mapped == 0 {
            break;
        }

        let mut last_end = offset;
        let mut is_last = false;
        for extent in &buffer.extents[..mapped] {
            if extent.fe_length == 0 {
                continue;
            }

            last_end = extent.fe_logical.saturating_add(extent.fe_length);
            if (extent.fe_flags & FIEMAP_EXTENT_LAST) != 0 {
                is_last = true;
            }

            if !visit(extent) {
                return Ok(());
            }
        }

        if is_last || last_end <= offset {
            break;
        }

        offset = last_end;
    }

    Ok(())
}
