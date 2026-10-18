// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::{wc_simd_allowed, word_count::WordCount};
use uucore::hardware::SimdPolicy;

use super::WordCountable;

use std::io::{self, ErrorKind, Read};

#[cfg(unix)]
use rustix::fs::SeekFrom;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
#[cfg(windows)]
const FILE_ATTRIBUTE_ARCHIVE: u32 = 32;
#[cfg(windows)]
const FILE_ATTRIBUTE_NORMAL: u32 = 128;

const BUF_SIZE: usize = 64 * 1024;

/// This is a Linux-specific function to count the number of bytes using the
/// `splice` system call, which is faster than using `read`.
///
/// On error it returns the number of bytes it did manage to read, since the
/// caller will fall back to a simpler method.
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn count_bytes_using_splice(fd: &impl rustix::fd::AsFd) -> Result<usize, usize> {
    use uucore::pipes::{MAX_ROOTLESS_PIPE_SIZE, pipe, splice};
    let null_file = uucore::pipes::dev_null().ok_or(0_usize)?;
    let mut byte_count = 0;
    // no need to increase pipe size of input fd since
    // - sender with splice probably increased size already
    // - sender without splice is bottleneck of our wc -c
    loop {
        match splice(fd, &null_file, MAX_ROOTLESS_PIPE_SIZE) {
            Ok(0) => return Ok(byte_count),
            Ok(res) => byte_count += res,
            Err(_) => break, // input is not pipe. needs additional pipe...
        }
    }
    let (pipe_rd, pipe_wr) = pipe::<false>().map_err(|_| byte_count)?;
    while let s @ 1.. = splice(fd, &pipe_wr, MAX_ROOTLESS_PIPE_SIZE).map_err(|_| byte_count)? {
        byte_count += s;
        // pipe to null is not blocked. So this returns the same length at most cases
        // next splice does not hang if we discarded 1+ pages
        splice(&pipe_rd, &null_file, s).map_err(|_| byte_count)?;
    }
    Ok(byte_count)
}

/// In the special case where we only need to count the number of bytes. There
/// are several optimizations we can do:
///   1. On Unix,  we can simply `stat` the file if it is regular.
///   2. On Linux -- if the above did not work -- we can use splice with broker.
///   3. On Windows we can use `std::os::windows::fs::MetadataExt` to get file size
///      for regular files
///   3. Otherwise, we just read normally, but without the overhead of counting
///      other things such as lines and words.
#[inline]
pub(crate) fn count_bytes_fast<T: WordCountable>(handle: &mut T) -> (usize, Option<io::Error>) {
    let mut byte_count = 0;

    #[cfg(unix)]
    if let Ok(stat) = rustix::fs::fstat(&handle) {
        // If the file is regular, then the `st_size` should hold
        // the file's size in bytes.
        // If stat.st_size = 0 then
        //  - either the size is 0
        //  - or the size is unknown.
        // The second case happens for files in pseudo-filesystems.
        // For example with /proc/version.
        // So, if it is 0 we don't report that and instead do a full read.
        //
        // Another thing to consider for files in pseudo-filesystems like /proc, /sys
        // and similar is that they could report `st_size` greater than actual content.
        // For example /sys/kernel/profiling could report `st_size` equal to
        // system page size (typically 4096 on 64bit system), while it's file content
        // would count up only to a couple of bytes.
        // This condition usually occurs for files in pseudo-filesystems like /proc, /sys
        // that report `st_size` in the multiples of system page size.
        // In such cases - attempt `seek()` almost to the end of the file
        // and then fall back on read to count the rest.
        //
        // The descriptor may already be partway through a redirected file.
        // Count only the unread suffix and advance it so a later `-` operand or
        // another process sharing stdin sees EOF.
        if (stat.st_mode as libc::mode_t & libc::S_IFREG) != 0
            && stat.st_size > 0
            && let Ok(position) = rustix::fs::seek(&handle, SeekFrom::Current(0))
        {
            let size = stat.st_size as u64;
            if position >= size {
                return (0, None);
            }
            let known_size = !(size as usize).is_multiple_of(rustix::param::page_size());
            let offset = if known_size {
                size
            } else {
                // Probe the final block of page-aligned files: pseudo-files can
                // advertise a full page even when their contents are shorter.
                // st_blksize has different integer types across supported targets.
                #[allow(clippy::unnecessary_cast)]
                let block_size = stat.st_blksize as u64;
                (size - size % (block_size + 1)).max(position)
            };
            if let Ok(offset) = rustix::fs::seek(&handle, SeekFrom::Start(offset)) {
                byte_count = (offset - position) as usize;
                if known_size {
                    return (byte_count, None);
                }
            }
        }
        // Else, if we're on Linux, we use splice with broker (mostly for stream)
        #[cfg(any(target_os = "linux", target_os = "android"))]
        match count_bytes_using_splice(handle) {
            Ok(n) => return (byte_count + n, None),
            Err(n) => byte_count = n,
        }
    }

    #[cfg(windows)]
    if let Some(file) = handle.inner_file()
        && let Ok(metadata) = file.metadata()
    {
        let attributes = metadata.file_attributes();
        if (attributes & FILE_ATTRIBUTE_ARCHIVE) != 0 || (attributes & FILE_ATTRIBUTE_NORMAL) != 0 {
            return (metadata.file_size() as usize, None);
        }
    }

    // Fall back on `read`, but without the overhead of counting words and lines.

    let mut buf = [0_u8; BUF_SIZE];
    loop {
        match handle.read(&mut buf) {
            Ok(0) => return (byte_count, None),
            Ok(n) => byte_count += n,
            Err(e) if e.kind() == ErrorKind::Interrupted => (),
            Err(e) => return (byte_count, Some(e)),
        }
    }
}

/// A simple structure used to align a [`BUF_SIZE`] buffer to 32-byte boundary.
///
/// This is useful as bytecount uses 256-bit wide vector operations that run much
/// faster on aligned data (at least on x86 with AVX2 support).
#[repr(align(32))]
struct AlignedBuffer {
    data: [u8; BUF_SIZE],
}

impl Default for AlignedBuffer {
    fn default() -> Self {
        Self {
            data: [0; BUF_SIZE],
        }
    }
}

/// Returns a [`WordCount`] that counts the number of bytes, lines, and/or the number of Unicode characters encoded in UTF-8 read via a Reader.
///
/// This corresponds to the `-c`, `-l` and `-m` command line flags to wc.
///
/// # Arguments
///
/// * `R` - A Reader from which the UTF-8 stream will be read.
pub(crate) fn count_bytes_chars_and_lines_fast<
    R: Read,
    const COUNT_BYTES: bool,
    const COUNT_CHARS: bool,
    const COUNT_LINES: bool,
>(
    handle: &mut R,
) -> (WordCount, Option<io::Error>) {
    let mut total = WordCount::default();
    let buf: &mut [u8] = &mut AlignedBuffer::default().data;
    let policy = SimdPolicy::detect();
    let simd_allowed = wc_simd_allowed(policy);
    loop {
        match handle.read(buf) {
            Ok(0) => return (total, None),
            Ok(n) => {
                if COUNT_BYTES {
                    total.bytes += n;
                }
                if COUNT_CHARS {
                    total.chars += if simd_allowed {
                        bytecount::num_chars(&buf[..n])
                    } else {
                        bytecount::naive_num_chars(&buf[..n])
                    };
                }
                if COUNT_LINES {
                    total.lines += if simd_allowed {
                        bytecount::count(&buf[..n], b'\n')
                    } else {
                        bytecount::naive_count(&buf[..n], b'\n')
                    };
                }
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => (),
            Err(e) => return (total, Some(e)),
        }
    }
}
