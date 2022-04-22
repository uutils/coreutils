use crate::word_count::WordCount;

use super::WordCountable;

#[cfg(any(target_os = "linux", target_os = "android"))]
use std::fs::OpenOptions;
use std::io::{self, ErrorKind, Read};

#[cfg(unix)]
use libc::S_IFREG;
#[cfg(unix)]
use nix::sys::stat;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::os::unix::io::AsRawFd;

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
fn count_bytes_using_splice(fd: &impl AsRawFd) -> Result<usize, usize> {
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
            // The second case happens for files in pseudo-filesystems. For
            // example with /proc/version and /sys/kernel/profiling. So,
            // if it is 0 we don't report that and instead do a full read.
            if (stat.st_mode as libc::mode_t & S_IFREG) != 0 && stat.st_size > 0 {
                return (stat.st_size as usize, None);
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

pub(crate) fn count_bytes_and_lines_fast<R: Read>(
    handle: &mut R,
) -> (WordCount, Option<io::Error>) {
    let mut total = WordCount::default();
    let mut buf = [0; BUF_SIZE];
    loop {
        match handle.read(&mut buf) {
            Ok(0) => return (total, None),
            Ok(n) => {
                total.bytes += n;
                total.lines += bytecount::count(&buf[..n], b'\n');
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return (total, Some(e)),
        }
    }
}
