use crate::word_count::WordCount;

use super::WordCountable;

#[cfg(any(target_os = "linux", target_os = "android"))]
use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind, Read};

#[cfg(unix)]
use libc::S_IFREG;
#[cfg(unix)]
use nix::sys::stat;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};

#[cfg(any(target_os = "linux", target_os = "android"))]
use libc::S_IFIFO;
#[cfg(any(target_os = "linux", target_os = "android"))]
use nix::fcntl::{splice, SpliceFFlags};
#[cfg(any(target_os = "linux", target_os = "android"))]
use nix::unistd::pipe;

const BUF_SIZE: usize = 16 * 1024;
#[cfg(any(target_os = "linux", target_os = "android"))]
const SPLICE_SIZE: usize = 128 * 1024;

/// Splice wrapper which handles short writes
#[cfg(any(target_os = "linux", target_os = "android"))]
#[inline]
fn splice_exact(read_fd: RawFd, write_fd: RawFd, num_bytes: usize) -> nix::Result<()> {
    let mut left = num_bytes;
    loop {
        let written = splice(read_fd, None, write_fd, None, left, SpliceFFlags::empty())?;
        left -= written;
        if left == 0 {
            break;
        }
    }
    Ok(())
}

/// This is a Linux-specific function to count the number of bytes using the
/// `splice` system call, which is faster than using `read`.
///
/// On error it returns the number of bytes it did manage to read, since the
/// caller will fall back to a simpler method.
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn count_bytes_using_splice(fd: RawFd) -> Result<usize, usize> {
    let null_file = OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .map_err(|_| 0_usize)?;
    let null = null_file.as_raw_fd();
    let null_rdev = stat::fstat(null).map_err(|_| 0_usize)?.st_rdev;
    if (stat::major(null_rdev), stat::minor(null_rdev)) != (1, 3) {
        // This is not a proper /dev/null, writing to it is probably bad
        // Bit of an edge case, but it has been known to happen
        return Err(0);
    }
    let (pipe_rd, pipe_wr) = pipe().map_err(|_| 0_usize)?;

    // Ensure the pipe is closed when the function returns.
    // SAFETY: The file descriptors do not have other owners.
    let _handles = unsafe { (File::from_raw_fd(pipe_rd), File::from_raw_fd(pipe_wr)) };

    let mut byte_count = 0;
    loop {
        match splice(fd, None, pipe_wr, None, SPLICE_SIZE, SpliceFFlags::empty()) {
            Ok(0) => break,
            Ok(res) => {
                byte_count += res;
                if splice_exact(pipe_rd, null, res).is_err() {
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
            if (stat.st_mode & S_IFREG) != 0 {
                return (stat.st_size as usize, None);
            }
            #[cfg(any(target_os = "linux", target_os = "android"))]
            {
                // Else, if we're on Linux and our file is a FIFO pipe
                // (or stdin), we use splice to count the number of bytes.
                if (stat.st_mode & S_IFIFO) != 0 {
                    match count_bytes_using_splice(fd) {
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
