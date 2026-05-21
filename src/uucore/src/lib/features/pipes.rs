// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Thin zero-copy-related wrappers around functions.

#[cfg(any(target_os = "linux", target_os = "android"))]
use rustix::pipe::{SpliceFlags, fcntl_setpipe_size};
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::{
    fs::File,
    io::{PipeReader, PipeWriter, Read, Write},
    os::fd::AsFd,
    sync::OnceLock,
};
#[cfg(any(target_os = "linux", target_os = "android"))]
pub const MAX_ROOTLESS_PIPE_SIZE: usize = 1024 * 1024;
#[cfg(any(target_os = "linux", target_os = "android"))]
const KERNEL_DEFAULT_PIPE_SIZE: usize = 64 * 1024;

/// return pipe larger than given size
/// SIZE_REQUIRED should be true if you want to fail when changing pipe size failed
/// e.g. writing size to pipe should not hang
///
/// used for resolving the limitation for splice: one of a input or output should be pipe
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn pipe<const SIZE_REQUIRED: bool>(s: usize) -> std::io::Result<(PipeReader, PipeWriter)> {
    let pair = std::io::pipe()?;
    // guard unnecessary syscall
    if s > KERNEL_DEFAULT_PIPE_SIZE {
        let r = fcntl_setpipe_size(&pair.0, s);
        if SIZE_REQUIRED {
            r?;
        }
    }

    Ok(pair)
}

/// Less noisy wrapper around splice syscall
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

/// Try to splice `len` bytes from `source` into `target`.
///
/// Note that this splice_exact does not provide bytes sent when it failed.
/// In the case failed relaying splice via pipe, all content of the pipe
/// should be drained by `read` to keep bytes sent accurate.
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn splice_exact(source: &impl AsFd, target: &impl AsFd, len: usize) -> rustix::io::Result<()> {
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

/// splice all of source to dest
/// return true if we need read/write fallback
/// fails if one of in/output should be pipe
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn splice_unbounded(source: &impl AsFd, dest: &mut impl AsFd) -> std::io::Result<bool> {
    // improve throughput
    // todo: avoid fcntl overhead for small input, but don't fcntl inside of the loop
    // no need to increase pipe size of input fd since
    // - sender with splice probably increased size already
    // - sender without splice is bottleneck
    let _ = fcntl_setpipe_size(&mut *dest, MAX_ROOTLESS_PIPE_SIZE);
    loop {
        match splice(&source, &dest, MAX_ROOTLESS_PIPE_SIZE) {
            Ok(1..) => {}
            Ok(0) => return Ok(false),
            Err(_) => return Ok(true),
        }
    }
}

/// force-splice source to dest even both of them are not pipe
/// return true if we need read/write fallback
///
/// This should not be used if one of them are pipe to save resources
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn splice_unbounded_broker<R, S>(source: &R, dest: &mut S) -> std::io::Result<bool>
where
    R: Read + AsFd,
    S: AsFd,
{
    static PIPE_CACHE: OnceLock<Option<(PipeReader, PipeWriter)>> = OnceLock::new();
    let Some((pipe_rd, pipe_wr)) = PIPE_CACHE
        .get_or_init(|| pipe::<false>(MAX_ROOTLESS_PIPE_SIZE).ok())
        .as_ref()
    else {
        return Ok(true);
    };
    // improve throughput
    // no need to increase pipe size of input fd since
    // - sender with splice probably increased size already
    // - sender without splice is bottleneck
    let _ = fcntl_setpipe_size(&mut *dest, MAX_ROOTLESS_PIPE_SIZE);

    loop {
        match splice(&source, &pipe_wr, MAX_ROOTLESS_PIPE_SIZE) {
            Ok(0) => return Ok(false),
            Ok(n) => {
                if splice_exact(&pipe_rd, dest, n).is_err() {
                    // If the first splice manages to copy to the intermediate
                    // pipe, but the second splice to stdout fails for some reason
                    // we can recover by copying the data that we have from the
                    // intermediate pipe to stdout using unbuffered read/write. Then
                    // we tell the caller to fall back.
                    debug_assert!(n <= MAX_ROOTLESS_PIPE_SIZE, "unexpected RAM usage");
                    let mut drain = Vec::with_capacity(n);
                    pipe_rd.take(n as u64).read_to_end(&mut drain)?;
                    crate::io::RawWriter(&dest).write_all(&drain)?;
                    return Ok(true);
                }
            }
            Err(_) => return Ok(true),
        }
    }
}

/// try splice_unbounded 1st and splice_unbounded_broker if both of in/output are not pipe
///
/// return true if write fallback is needed
/// (the fallback will be embedded to this function in the future)
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn splice_unbounded_auto<R, S>(source: &R, dest: &mut S) -> std::io::Result<bool>
where
    R: Read + AsFd,
    S: AsFd,
{
    // use splice to check that input or output is pipe which is efficient
    let fallback = match splice(&source, dest, MAX_ROOTLESS_PIPE_SIZE) {
        Ok(_) => splice_unbounded(source, dest)?,
        _ => splice_unbounded_broker(source, dest)?,
    };
    Ok(fallback)
}

/// splice `n` bytes with safe read/write fallback
/// return actually sent bytes
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn send_n_bytes(
    input: impl Read + AsFd,
    mut target: impl Write + AsFd,
    n: u64,
) -> std::io::Result<u64> {
    static PIPE_CACHE: OnceLock<Option<(PipeReader, PipeWriter)>> = OnceLock::new();
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
                Ok(s) => {
                    n -= s as u64;
                    bytes_written += s as u64;
                }
                _ => break true,
            }
        }
    } else if let Some((broker_r, broker_w)) = PIPE_CACHE
        .get_or_init(|| pipe::<false>(pipe_size).ok())
        .as_ref()
    {
        // todo: create fn splice_bounded_broker
        loop {
            match splice(&input, &broker_w, n as usize) {
                Ok(0) => break might_fuse(&input),
                Ok(s) => {
                    if splice_exact(&broker_r, &target, s).is_ok() {
                        n -= s as u64;
                        bytes_written += s as u64;
                        if n == 0 {
                            // avoid unnecessary splice for small input
                            break false;
                        }
                    } else {
                        debug_assert!(s <= MAX_ROOTLESS_PIPE_SIZE, "unexpected RAM usage");
                        // drain pipe before fallback to raw write
                        let mut drain = Vec::with_capacity(s);
                        broker_r.take(s as u64).read_to_end(&mut drain)?;
                        crate::io::RawWriter(&target).write_all(&drain)?;
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
    ((rustix::fs::major(dev), rustix::fs::minor(dev)) == (1, 3)).then_some(null)
}

// Less noisy wrapper around tee syscall
#[inline]
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn tee(source: &impl AsFd, target: &impl AsFd, len: usize) -> rustix::io::Result<usize> {
    rustix::pipe::tee(source, target, len, SpliceFlags::empty())
}
