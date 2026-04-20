// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Thin zero-copy-related wrappers around functions from the `rustix` crate.

#[cfg(any(target_os = "linux", target_os = "android"))]
use rustix::pipe::{SpliceFlags, fcntl_setpipe_size};
#[cfg(any(target_os = "linux", target_os = "android", test))]
use std::fs::File;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::{io::Read, os::fd::AsFd, sync::OnceLock};
#[cfg(any(target_os = "linux", target_os = "android"))]
pub const MAX_ROOTLESS_PIPE_SIZE: usize = 1024 * 1024;
#[cfg(any(target_os = "linux", target_os = "android"))]
const KERNEL_DEFAULT_PIPE_SIZE: usize = 64 * 1024;

/// A wrapper around [`rustix::pipe::pipe`] that ensures the pipe is cleaned up.
///
/// Returns two `File` objects: everything written to the second can be read
/// from the first.
/// used for resolving the limitation for splice: one of a input or output should be pipe
#[inline]
#[cfg(any(target_os = "linux", target_os = "android", test))]
pub fn pipe() -> std::io::Result<(File, File)> {
    let (read, write) = rustix::pipe::pipe()?;
    // improve performance for splice
    #[cfg(any(target_os = "linux", target_os = "android"))]
    let _ = fcntl_setpipe_size(&read, MAX_ROOTLESS_PIPE_SIZE);

    Ok((File::from(read), File::from(write)))
}

/// return pipe larger than given size and kernel's default size
///
/// useful to save RAM usage
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn pipe_with_size(s: usize) -> std::io::Result<(File, File)> {
    let (read, write) = rustix::pipe::pipe()?;
    if s > KERNEL_DEFAULT_PIPE_SIZE {
        let _ = fcntl_setpipe_size(&read, s);
    }

    Ok((File::from(read), File::from(write)))
}

/// Less noisy wrapper around [`rustix::pipe::splice`].
///
/// Up to `len` bytes are moved from `source` to `target`. Returns the number
/// of successfully moved bytes.
///
/// At least one of `source` and `target` must be some sort of pipe.
/// To get around this requirement, consider splicing from your source into
/// a [`pipe`] and then from the pipe into your target (with `splice_exact`):
/// this is still very efficient.
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn splice(source: &impl AsFd, target: &impl AsFd, len: usize) -> rustix::io::Result<usize> {
    rustix::pipe::splice(source, None, target, None, len, SpliceFlags::empty())
}

/// Splice wrapper which fully finishes the write.
///
/// Exactly `len` bytes are moved from `source` into `target`.
///
/// Panics if `source` runs out of data before `len` bytes have been moved.
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn splice_exact(source: &impl AsFd, target: &impl AsFd, len: usize) -> std::io::Result<()> {
    let mut left = len;
    while left > 0 {
        let written = splice(source, target, left)?;
        debug_assert_ne!(written, 0, "unexpected end of data");
        left -= written;
    }
    Ok(())
}

/// check that source is FUSE
/// we fallback to read() at FUSE <https://github.com/uutils/coreutils/issues/9609>
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn might_fuse(source: &impl AsFd) -> bool {
    rustix::fs::fstatfs(source).map_or(true, |stats| stats.f_type == 0x6573_5546) // FUSE magic number, too many platform specific clippy warning with const
}

/// splice `n` bytes with safe read/write fallback
/// return actually sent bytes
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn send_n_bytes(
    input: impl Read + AsFd,
    mut target: impl std::io::Write + AsFd,
    n: u64,
) -> std::io::Result<u64> {
    static PIPE_CACHE: OnceLock<Option<(File, File)>> = OnceLock::new();
    let pipe_size = MAX_ROOTLESS_PIPE_SIZE.min(n as usize);
    let mut n = n;
    let mut bytes_written: u64 = 0;
    // do not always fallback to write as it needs 2 Ctrl+D to exit process on tty
    let fallback = if let Ok(b) = splice(&input, &target, n as usize) {
        bytes_written = b as u64;
        n -= bytes_written;
        if n == 0 {
            // avoid unnecessary syscalls
            return Ok(bytes_written);
        }
        
        // improve throughput or save RAM usage
        // expected that input is already extended if it is coming from splice
        // we can use pipe_size * N with some case e.g. head -c N inputs, but we need N splice call anyway
        if pipe_size > KERNEL_DEFAULT_PIPE_SIZE {
            let _ = fcntl_setpipe_size(&target, pipe_size);
        }

        loop {
            match splice(&input, &target, n as usize) {
                Ok(0) => break might_fuse(&input),
                Ok(s @ 1..) => {
                    n -= s as u64;
                    bytes_written += s as u64;
                }
                _ => break true,
            }
        }
    } else if let Some((broker_r, broker_w)) = PIPE_CACHE
        .get_or_init(|| pipe_with_size(pipe_size).ok())
        .as_ref()
    {
        loop {
            match splice(&input, &broker_w, n as usize) {
                Ok(0) => break might_fuse(&input),
                Ok(s @ 1..) => {
                    if splice_exact(&broker_r, &target, s).is_ok() {
                        n -= s as u64;
                        bytes_written += s as u64;
                        if n == 0 {
                            // avoid unnecessary splice for small input
                            break false;
                        }
                    } else {
                        let mut drain = Vec::with_capacity(s); // bounded by pipe size
                        broker_r.take(s as u64).read_to_end(&mut drain)?;
                        target.write_all(&drain)?;
                        break true;
                    }
                }
                _ => break true,
            }
        }
    } else {
        true
    };

    if !fallback {
        return Ok(bytes_written);
    }
    let mut reader = input.take(n);
    let mut buf = vec![0u8; (32 * 1024).min(n as usize)]; //use heap to avoid early allocation
    loop {
        match reader.read(&mut buf)? {
            0 => return Ok(bytes_written),
            n => {
                target.write_all(&buf[..n])?;
                bytes_written += n as u64;
            }
        }
    }
}

/// Return verified /dev/null
///
/// `splice` to /dev/null is faster than `read` when we skip or count the non-seekable input
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn dev_null() -> Option<File> {
    let null = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .ok()?;
    let stat = rustix::fs::fstat(&null).ok()?;
    let dev = stat.st_rdev;
    if (rustix::fs::major(dev), rustix::fs::minor(dev)) == (1, 3) {
        Some(null)
    } else {
        None
    }
}
