// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore fiemap iowr

use rustc_hash::FxHashSet as HashSet;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, RawFd};

const POSIX_BLOCK_SIZE: u64 = 512;
const FIEMAP_EXTENT_COUNT: usize = 128;

const FIEMAP_EXTENT_LAST: u32 = 0x0000_0001;
const FIEMAP_EXTENT_UNKNOWN: u32 = 0x0000_0002;
const FIEMAP_EXTENT_DELALLOC: u32 = 0x0000_0004;
const FIEMAP_EXTENT_ENCODED: u32 = 0x0000_0008;
const FIEMAP_EXTENT_NOT_ALIGNED: u32 = 0x0000_0100;
const FIEMAP_EXTENT_DATA_INLINE: u32 = 0x0000_0200;
const FIEMAP_EXTENT_DATA_TAIL: u32 = 0x0000_0400;
const FIEMAP_EXTENT_MERGED: u32 = 0x0000_1000;
const FIEMAP_EXTENT_SHARED: u32 = 0x0000_2000;
const AMBIGUOUS_EXTENT_FLAGS: u32 = FIEMAP_EXTENT_UNKNOWN
    | FIEMAP_EXTENT_DELALLOC
    | FIEMAP_EXTENT_ENCODED
    | FIEMAP_EXTENT_NOT_ALIGNED
    | FIEMAP_EXTENT_DATA_INLINE
    | FIEMAP_EXTENT_DATA_TAIL
    | FIEMAP_EXTENT_MERGED;

// Field names mirror the C `struct fiemap` / `struct fiemap_extent` ABI.
#[allow(clippy::struct_field_names)]
#[derive(Default)]
#[repr(C)]
struct Fiemap {
    fm_start: u64,
    fm_length: u64,
    fm_flags: u32,
    fm_mapped_extents: u32,
    fm_extent_count: u32,
    fm_reserved: u32,
}

#[allow(clippy::struct_field_names)]
#[derive(Clone, Copy, Default)]
#[repr(C)]
struct FiemapExtent {
    fe_logical: u64,
    fe_physical: u64,
    fe_length: u64,
    fe_reserved64: [u64; 2],
    fe_flags: u32,
    fe_reserved: [u32; 3],
}

#[repr(C)]
struct FiemapBuffer {
    header: Fiemap,
    extents: [FiemapExtent; FIEMAP_EXTENT_COUNT],
}

impl FiemapBuffer {
    fn new(offset: u64) -> Self {
        Self {
            header: Fiemap {
                fm_start: offset,
                fm_length: u64::MAX,
                fm_extent_count: FIEMAP_EXTENT_COUNT as u32,
                ..Fiemap::default()
            },
            extents: [FiemapExtent::default(); FIEMAP_EXTENT_COUNT],
        }
    }
}

struct FiemapPage {
    extents: Vec<FiemapExtent>,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct SharedExtentKey {
    device: u64,
    physical: u64,
    length: u64,
}

/// Tracks shared physical extents already attributed to earlier files.
///
/// The first file encountered owns an extent. Only exact
/// `{device, physical, length}` matches are deduplicated; partially overlapping
/// extents from divergent CoW chains remain counted independently.
#[derive(Default)]
pub(crate) struct ReflinkDeduper {
    seen: HashSet<SharedExtentKey>,
}

impl ReflinkDeduper {
    pub(crate) fn adjust(&mut self, file: &File, device: u64, blocks: u64) -> io::Result<u64> {
        self.adjust_with_query(device, blocks, |offset| query_fiemap(file, offset))
    }

    fn adjust_with_query<F>(&mut self, device: u64, blocks: u64, query: F) -> io::Result<u64>
    where
        F: FnMut(u64) -> io::Result<FiemapPage>,
    {
        if blocks == 0 {
            return Ok(0);
        }

        // Collect the complete file first. A later page failure must not let
        // this file claim extents that callers were unable to account for.
        let extents = collect_shared_extents(device, query)?;
        let mut unique_in_file = HashSet::default();
        let duplicate_bytes = extents.iter().fold(0_u64, |total, extent| {
            let is_duplicate = self.seen.contains(extent) || !unique_in_file.insert(*extent);
            if is_duplicate {
                total.saturating_add(extent.length)
            } else {
                total
            }
        });

        self.seen.extend(unique_in_file);
        Ok(blocks.saturating_sub(duplicate_bytes / POSIX_BLOCK_SIZE))
    }
}

fn collect_shared_extents<F>(device: u64, mut query: F) -> io::Result<Vec<SharedExtentKey>>
where
    F: FnMut(u64) -> io::Result<FiemapPage>,
{
    let mut offset = 0;
    let mut shared = Vec::new();

    loop {
        let page = query(offset)?;
        if page.extents.is_empty() {
            break;
        }

        let mut last_end = offset;
        let mut is_last = false;
        for extent in page.extents {
            is_last |= extent.fe_flags & FIEMAP_EXTENT_LAST != 0;
            if extent.fe_length == 0 {
                continue;
            }

            last_end = last_end.max(extent.fe_logical.saturating_add(extent.fe_length));
            if extent.fe_flags & FIEMAP_EXTENT_SHARED != 0
                && extent.fe_flags & AMBIGUOUS_EXTENT_FLAGS == 0
                && extent.fe_physical != 0
            {
                shared.push(SharedExtentKey {
                    device,
                    physical: extent.fe_physical,
                    length: extent.fe_length,
                });
            }
        }

        if is_last {
            break;
        }
        if last_end <= offset {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "FIEMAP pagination made no progress",
            ));
        }
        offset = last_end;
    }

    Ok(shared)
}

const FS_IOC_FIEMAP: libc::Ioctl = libc::_IOWR::<Fiemap>(b'f' as u32, 11);

fn query_fiemap(file: &File, offset: u64) -> io::Result<FiemapPage> {
    query_fiemap_with(file.as_raw_fd(), offset, |fd, buffer| {
        // SAFETY: `buffer` points to a live, writable FiemapBuffer with a
        // correctly initialized ABI header and space for fm_extent_count items.
        let result = unsafe { libc::ioctl(fd, FS_IOC_FIEMAP, buffer) };
        if result == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    })
}

fn query_fiemap_with<F>(fd: RawFd, offset: u64, mut ioctl: F) -> io::Result<FiemapPage>
where
    F: FnMut(RawFd, *mut FiemapBuffer) -> io::Result<()>,
{
    loop {
        let mut buffer = FiemapBuffer::new(offset);
        match ioctl(fd, &raw mut buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
            Ok(()) => {
                let mapped = buffer.header.fm_mapped_extents as usize;
                if mapped > buffer.extents.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "FIEMAP returned too many extents",
                    ));
                }
                return Ok(FiemapPage {
                    extents: buffer.extents[..mapped].to_vec(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn extent(logical: u64, physical: u64, length: u64, flags: u32) -> FiemapExtent {
        FiemapExtent {
            fe_logical: logical,
            fe_physical: physical,
            fe_length: length,
            fe_flags: flags,
            ..FiemapExtent::default()
        }
    }

    fn page(extents: Vec<FiemapExtent>) -> FiemapPage {
        FiemapPage { extents }
    }

    #[test]
    fn exact_shared_extents_are_counted_once() {
        let mut deduper = ReflinkDeduper::default();
        let query = |_| {
            Ok(page(vec![extent(
                0,
                4096,
                4096,
                FIEMAP_EXTENT_SHARED | FIEMAP_EXTENT_LAST,
            )]))
        };

        assert_eq!(deduper.adjust_with_query(7, 8, query).unwrap(), 8);
        assert_eq!(deduper.adjust_with_query(7, 8, query).unwrap(), 0);
    }

    #[test]
    fn ambiguous_extents_are_not_deduplicated() {
        const AMBIGUOUS_FLAGS: [u32; 7] = [
            FIEMAP_EXTENT_UNKNOWN,
            FIEMAP_EXTENT_DELALLOC,
            FIEMAP_EXTENT_ENCODED,
            FIEMAP_EXTENT_NOT_ALIGNED,
            FIEMAP_EXTENT_DATA_INLINE,
            FIEMAP_EXTENT_DATA_TAIL,
            FIEMAP_EXTENT_MERGED,
        ];

        for flag in AMBIGUOUS_FLAGS {
            let mut deduper = ReflinkDeduper::default();
            let query = |_| {
                Ok(page(vec![extent(
                    0,
                    4096,
                    4096,
                    FIEMAP_EXTENT_SHARED | FIEMAP_EXTENT_LAST | flag,
                )]))
            };

            assert_eq!(deduper.adjust_with_query(7, 8, query).unwrap(), 8);
            assert_eq!(deduper.adjust_with_query(7, 8, query).unwrap(), 8);
        }
    }

    #[test]
    fn zero_physical_extents_are_not_deduplicated() {
        let mut deduper = ReflinkDeduper::default();
        let query = |_| {
            Ok(page(vec![extent(
                0,
                0,
                4096,
                FIEMAP_EXTENT_SHARED | FIEMAP_EXTENT_LAST,
            )]))
        };

        assert_eq!(deduper.adjust_with_query(7, 8, query).unwrap(), 8);
        assert_eq!(deduper.adjust_with_query(7, 8, query).unwrap(), 8);
    }

    #[test]
    fn partial_extent_matches_are_not_deduplicated() {
        let mut deduper = ReflinkDeduper::default();
        let full_extent = |_| {
            Ok(page(vec![extent(
                0,
                4096,
                4096,
                FIEMAP_EXTENT_SHARED | FIEMAP_EXTENT_LAST,
            )]))
        };
        let partial_extent = |_| {
            Ok(page(vec![extent(
                0,
                4096,
                2048,
                FIEMAP_EXTENT_SHARED | FIEMAP_EXTENT_LAST,
            )]))
        };

        assert_eq!(deduper.adjust_with_query(7, 8, full_extent).unwrap(), 8);
        assert_eq!(deduper.adjust_with_query(7, 4, partial_extent).unwrap(), 4);
    }

    #[test]
    fn repeated_mapping_within_one_file_is_counted_once() {
        let mut deduper = ReflinkDeduper::default();
        let query = |_| {
            Ok(page(vec![
                extent(0, 4096, 4096, FIEMAP_EXTENT_SHARED),
                extent(4096, 4096, 4096, FIEMAP_EXTENT_SHARED | FIEMAP_EXTENT_LAST),
            ]))
        };

        assert_eq!(deduper.adjust_with_query(7, 16, query).unwrap(), 8);
        assert_eq!(deduper.adjust_with_query(7, 16, query).unwrap(), 0);
    }

    #[test]
    fn paginates_from_end_of_last_extent() {
        let mut deduper = ReflinkDeduper::default();
        let offsets = std::cell::RefCell::new(Vec::new());
        let query = |offset| {
            offsets.borrow_mut().push(offset);
            match offset {
                0 => Ok(page(vec![extent(0, 4096, 512, FIEMAP_EXTENT_SHARED)])),
                512 => Ok(page(vec![extent(
                    512,
                    8192,
                    512,
                    FIEMAP_EXTENT_SHARED | FIEMAP_EXTENT_LAST,
                )])),
                _ => unreachable!(),
            }
        };

        assert_eq!(deduper.adjust_with_query(7, 2, query).unwrap(), 2);
        assert_eq!(*offsets.borrow(), vec![0, 512]);
    }

    #[test]
    fn failed_query_does_not_commit_partial_state() {
        let mut deduper = ReflinkDeduper::default();
        let calls = Cell::new(0);
        let result = deduper.adjust_with_query(7, 8, |_| {
            let call = calls.get();
            calls.set(call + 1);
            if call == 0 {
                Ok(page(vec![extent(0, 4096, 4096, FIEMAP_EXTENT_SHARED)]))
            } else {
                Err(io::Error::other("query failed"))
            }
        });
        assert!(result.is_err());

        let query = |_| {
            Ok(page(vec![extent(
                0,
                4096,
                4096,
                FIEMAP_EXTENT_SHARED | FIEMAP_EXTENT_LAST,
            )]))
        };
        assert_eq!(deduper.adjust_with_query(7, 8, query).unwrap(), 8);
    }

    #[test]
    fn no_progress_page_is_an_error() {
        let mut deduper = ReflinkDeduper::default();
        let result = deduper.adjust_with_query(7, 8, |_| {
            Ok(page(vec![extent(0, 4096, 0, FIEMAP_EXTENT_SHARED)]))
        });

        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn retries_interrupted_ioctl_with_full_buffer_pointer() {
        let calls = Cell::new(0);
        let page = query_fiemap_with(-1, 123, |_, buffer| {
            let call = calls.get();
            calls.set(call + 1);

            // The ioctl argument points at the complete allocation, whose first
            // field is the ABI header.
            let header = unsafe { std::ptr::addr_of_mut!((*buffer).header) };
            assert_eq!(header.cast::<FiemapBuffer>(), buffer);

            if call == 0 {
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            }

            unsafe {
                (*buffer).header.fm_mapped_extents = 1;
                (*buffer).extents[0] =
                    extent(123, 4096, 512, FIEMAP_EXTENT_SHARED | FIEMAP_EXTENT_LAST);
            }
            Ok(())
        })
        .unwrap();

        assert_eq!(calls.get(), 2);
        assert_eq!(page.extents.len(), 1);
        assert_eq!(page.extents[0].fe_logical, 123);
    }

    #[test]
    fn rejects_kernel_extent_count_larger_than_buffer() {
        let error = query_fiemap_with(-1, 0, |_, buffer| {
            unsafe {
                (*buffer).header.fm_mapped_extents = FIEMAP_EXTENT_COUNT as u32 + 1;
            }
            Ok(())
        })
        .err()
        .unwrap();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
