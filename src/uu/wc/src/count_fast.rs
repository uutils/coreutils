// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// cSpell:ignore sysconf
use crate::word_count::WordCount;

use super::WordCountable;

#[cfg(any(target_os = "linux", target_os = "android"))]
use std::fs::OpenOptions;
use std::io::{self, ErrorKind, Read};

#[cfg(unix)]
use libc::{sysconf, S_IFREG, _SC_PAGESIZE};
#[cfg(unix)]
use nix::sys::stat;
#[cfg(unix)]
use std::io::{Seek, SeekFrom};
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::os::fd::{AsFd, AsRawFd};
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
#[cfg(windows)]
const FILE_ATTRIBUTE_ARCHIVE: u32 = 32;
#[cfg(windows)]
const FILE_ATTRIBUTE_NORMAL: u32 = 128;

#[cfg(any(target_os = "linux", target_os = "android"))]
use libc::S_IFIFO;
#[cfg(any(target_os = "linux", target_os = "android"))]
use uucore::pipes::{pipe, splice, splice_exact};

const BUF_SIZE: usize = 16 * 1024;
#[cfg(any(target_os = "linux", target_os = "android"))]
const SPLICE_SIZE: usize = 128 * 1024;

/// This is a Linux-specific function to count the number of bytes using the
/// `splice` system call, which is faster than using `read`.
///
/// On error it returns the number of bytes it did manage to read, since the
/// caller will fall back to a simpler method.
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn count_bytes_using_splice(fd: &impl AsFd) -> Result<usize, usize> {
    let null_file = OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .map_err(|_| 0_usize)?;
    let null_rdev = stat::fstat(null_file.as_raw_fd())
        .map_err(|_| 0_usize)?
        .st_rdev as libc::dev_t;
    if unsafe { (libc::major(null_rdev), libc::minor(null_rdev)) } != (1, 3) {
        // This is not a proper /dev/null, writing to it is probably bad
        // Bit of an edge case, but it has been known to happen
        return Err(0);
    }
    let (pipe_rd, pipe_wr) = pipe().map_err(|_| 0_usize)?;

    let mut byte_count = 0;
    loop {
        match splice(fd, &pipe_wr, SPLICE_SIZE) {
            Ok(0) => break,
            Ok(res) => {
                byte_count += res;
                // Silent the warning as we want to the error message
                #[allow(clippy::question_mark)]
                if splice_exact(&pipe_rd, &null_file, res).is_err() {
                    return Err(byte_count);
                }
            }
            Err(_) => return Err(byte_count),
        };
    }

    Ok(byte_count)
}

/// In the special case where we only need to count the number of bytes. There
/// are several optimizations we can do:
///   1. On Unix,  we can simply `stat` the file if it is regular.
///   2. On Linux -- if the above did not work -- we can use splice to count
///      the number of bytes if the file is a FIFO.
///   3. On Windows we can use `std::os::windows::fs::MetadataExt` to get file size
///      for regular files
///   3. Otherwise, we just read normally, but without the overhead of counting
///      other things such as lines and words.
#[inline]
pub(crate) fn count_bytes_fast<T: WordCountable>(handle: &mut T) -> (usize, Option<io::Error>) {
    let mut byte_count = 0;

    #[cfg(unix)]
    {
        let fd = handle.as_raw_fd();
        if let Ok(stat) = stat::fstat(fd) {
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
            // And finally a special case of input redirection in *nix shell:
            // `( wc -c ; wc -c ) < file` should return
            // ```
            // size_of_file
            // 0
            // ```
            // Similarly
            // `( head -c1 ; wc -c ) < file` should return
            // ```
            // first_byte_of_file
            // size_of_file - 1
            // ```
            // Since the input stream from file is treated as continuous across both commands inside ().
            // In cases like this, due to `<` redirect, the `stat.st_mode` would report input as a regular file
            // and `stat.st_size` would report the size of file on disk
            // and NOT the remaining number of bytes in the input stream.
            // However, the raw file descriptor in this situation would be equal to `0`
            // for STDIN in both invocations.
            // Therefore we cannot rely of `st_size` here and should fall back on full read.
            if fd > 0 && (stat.st_mode as libc::mode_t & S_IFREG) != 0 && stat.st_size > 0 {
                let sys_page_size = unsafe { sysconf(_SC_PAGESIZE) as usize };
                if stat.st_size as usize % sys_page_size > 0 {
                    // regular file or file from /proc, /sys and similar pseudo-filesystems
                    // with size that is NOT a multiple of system page size
                    return (stat.st_size as usize, None);
                } else if let Some(file) = handle.inner_file() {
                    // On some platforms `stat.st_blksize` and `stat.st_size`
                    // are of different types: i64 vs i32
                    // i.e. MacOS on Apple Silicon (aarch64-apple-darwin),
                    // Debian Linux on ARM (aarch64-unknown-linux-gnu),
                    // 32bit i686 targets, etc.
                    // While on the others they are of the same type.
                    #[allow(clippy::unnecessary_cast)]
                    let offset =
                        stat.st_size as i64 - stat.st_size as i64 % (stat.st_blksize as i64 + 1);

                    if let Ok(n) = file.seek(SeekFrom::Start(offset as u64)) {
                        byte_count = n as usize;
                    }
                }
            }
            #[cfg(any(target_os = "linux", target_os = "android"))]
            {
                // Else, if we're on Linux and our file is a FIFO pipe
                // (or stdin), we use splice to count the number of bytes.
                if (stat.st_mode as libc::mode_t & S_IFIFO) != 0 {
                    match count_bytes_using_splice(handle) {
                        Ok(n) => return (n, None),
                        Err(n) => byte_count = n,
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        if let Some(file) = handle.inner_file() {
            if let Ok(metadata) = file.metadata() {
                let attributes = metadata.file_attributes();

                if (attributes & FILE_ATTRIBUTE_ARCHIVE) != 0
                    || (attributes & FILE_ATTRIBUTE_NORMAL) != 0
                {
                    return (metadata.file_size() as usize, None);
                }
            }
        }
    }

    // Fall back on `read`, but without the overhead of counting words and lines.
    let mut buf = [0_u8; BUF_SIZE];
    loop {
        match handle.read(&mut buf) {
            Ok(0) => return (byte_count, None),
            Ok(n) => {
                byte_count += n;
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return (byte_count, Some(e)),
        }
    }
}

/// Returns a WordCount that counts the number of bytes, lines, and/or the number of Unicode characters encoded in UTF-8 read via a Reader.
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
    /// Mask of the value bits of a continuation byte
    const CONT_MASK: u8 = 0b0011_1111u8;
    /// Value of the tag bits (tag mask is !CONT_MASK) of a continuation byte
    const TAG_CONT_U8: u8 = 0b1000_0000u8;

    let mut total = WordCount::default();
    let mut buf = [0; BUF_SIZE];
    loop {
        match handle.read(&mut buf) {
            Ok(0) => return (total, None),
            Ok(n) => {
                if COUNT_BYTES {
                    total.bytes += n;
                }
                if COUNT_CHARS {
                    total.chars += buf[..n]
                        .iter()
                        .filter(|&&byte| (byte & !CONT_MASK) != TAG_CONT_U8)
                        .count();
                }
                if COUNT_LINES {
                    total.lines += bytecount::count(&buf[..n], b'\n');
                }
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return (total, Some(e)),
        }
    }
}
