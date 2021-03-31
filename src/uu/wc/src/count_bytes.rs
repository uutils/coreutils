use super::{WcResult, WordCountable};

#[cfg(any(target_os = "linux", target_os = "android"))]
use std::fs::OpenOptions;
use std::io::ErrorKind;

#[cfg(unix)]
use libc::S_IFREG;
#[cfg(unix)]
use nix::sys::stat::fstat;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::os::unix::io::{AsRawFd, RawFd};

#[cfg(any(target_os = "linux", target_os = "android"))]
use libc::S_IFIFO;
#[cfg(any(target_os = "linux", target_os = "android"))]
use nix::fcntl::{splice, SpliceFFlags};
#[cfg(any(target_os = "linux", target_os = "android"))]
use nix::unistd::pipe;

const BUF_SIZE: usize = 16384;

/// This is a Linux-specific function to count the number of bytes using the
/// `splice` system call, which is faster than using `read`.
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn count_bytes_using_splice(fd: RawFd) -> nix::Result<usize> {
    let null_file = OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .map_err(|_| nix::Error::last())?;
    let null = null_file.as_raw_fd();
    let (pipe_rd, pipe_wr) = pipe()?;

    let mut byte_count = 0;
    loop {
        let res = splice(fd, None, pipe_wr, None, BUF_SIZE, SpliceFFlags::empty())?;
        if res == 0 {
            break;
        }
        byte_count += res;
        splice(pipe_rd, None, null, None, res, SpliceFFlags::empty())?;
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
pub(crate) fn count_bytes_fast<T: WordCountable>(handle: &mut T) -> WcResult<usize> {
    #[cfg(unix)]
    {
        let fd = handle.as_raw_fd();
        match fstat(fd) {
            Ok(stat) => {
                // If the file is regular, then the `st_size` should hold
                // the file's size in bytes.
                if (stat.st_mode & S_IFREG) != 0 {
                    return Ok(stat.st_size as usize);
                }
                #[cfg(any(target_os = "linux", target_os = "android"))]
                {
                    // Else, if we're on Linux and our file is a FIFO pipe
                    // (or stdin), we use splice to count the number of bytes.
                    if (stat.st_mode & S_IFIFO) != 0 {
                        if let Ok(n) = count_bytes_using_splice(fd) {
                            return Ok(n);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Fall back on `read`, but without the overhead of counting words and lines.
    let mut buf = [0 as u8; BUF_SIZE];
    let mut byte_count = 0;
    loop {
        match handle.read(&mut buf) {
            Ok(0) => return Ok(byte_count),
            Ok(n) => {
                byte_count += n;
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }
}
