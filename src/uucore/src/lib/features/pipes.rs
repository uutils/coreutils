// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Thin zero-copy-related wrappers around functions.

#![cfg(any(target_os = "linux", target_os = "android"))]

use crate::io::{RawReader, RawWriter};
use rustix::pipe::{SpliceFlags, fcntl_setpipe_size};
use std::{
    io::{PipeReader, PipeWriter, Read, Write},
    os::fd::AsFd,
    sync::OnceLock,
};
pub const MAX_ROOTLESS_PIPE_SIZE: usize = 1024 * 1024;
const KERNEL_DEFAULT_PIPE_SIZE: usize = 64 * 1024;

/// return pipe larger than given size
/// SIZE_REQUIRED should be true if you want to fail when changing pipe size failed
/// e.g. writing size to pipe should not hang
///
/// used for resolving the limitation for splice: one of a input or output should be pipe
#[inline]
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
pub fn splice(source: &impl AsFd, target: &impl AsFd, len: usize) -> rustix::io::Result<usize> {
    rustix::pipe::splice(source, None, target, None, len, SpliceFlags::empty())
}

/// Try to splice `len` bytes from `source` into `target`.
///
/// Note that this splice_exact does not provide bytes sent when it failed.
/// In the case failed relaying splice via pipe, all content of the pipe
/// should be drained by `read` to keep bytes sent accurate.
#[inline]
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
pub fn might_fuse(source: &impl AsFd) -> bool {
    rustix::fs::fstatfs(source).map_or(true, |stats| stats.f_type == 0x6573_5546) // FUSE magic number, too many platform specific clippy warning with const
}

/// splice all of source to dest
/// returns Ok(()) at end of file
#[inline]
pub fn splice_unbounded(source: &impl AsFd, dest: &mut impl AsFd) -> rustix::io::Result<()> {
    // avoid fcntl overhead for small input. splice twice to catch end of file.
    if splice(&source, &dest, MAX_ROOTLESS_PIPE_SIZE)? == 0
        || splice(&source, &dest, MAX_ROOTLESS_PIPE_SIZE)? == 0
    {
        return Ok(());
    }
    // fcntl for input would not improve throughput since
    // - sender with splice probably increased size already
    // - sender without splice is bottleneck
    let _ = fcntl_setpipe_size(&mut *dest, MAX_ROOTLESS_PIPE_SIZE);
    while splice(&source, &dest, MAX_ROOTLESS_PIPE_SIZE)? > 0 {}
    Ok(())
}

/// force-splice source to dest even both of them are not pipe via broker pipe
/// returns Ok(Ok(())) if splice succeeds
/// returns Ok(Err()) if splice failed, but you can fallback to read/write
/// returns std::io::Result if splice from broker failed and read/write fallback from broker failed
///
/// Thus, ?.is_err() returns serious error at early stage and checks that you can fallback
/// This should not be used if one of them are pipe to save resources
#[inline]
pub fn splice_unbounded_broker(
    source: &impl AsFd,
    dest: &mut impl AsFd,
) -> std::io::Result<Result<(), ()>> {
    static PIPE_CACHE: OnceLock<Option<(PipeReader, PipeWriter)>> = OnceLock::new();
    let Some((pipe_rd, pipe_wr)) =
        PIPE_CACHE.get_or_init(|| pipe::<false>(MAX_ROOTLESS_PIPE_SIZE).ok())
    else {
        return Ok(Err(()));
    };
    // improve throughput
    // no need to increase pipe size of input fd since
    // - sender with splice probably increased size already
    // - sender without splice is bottleneck
    let _ = fcntl_setpipe_size(&mut *dest, MAX_ROOTLESS_PIPE_SIZE);

    loop {
        match splice(&source, &pipe_wr, MAX_ROOTLESS_PIPE_SIZE) {
            Ok(0) => return Ok(Ok(())),
            Ok(n) => {
                if splice_exact(&pipe_rd, dest, n).is_err() {
                    // If the first splice manages to copy to the intermediate
                    // pipe, but the second splice to stdout fails for some reason
                    // we can recover by copying the data that we have from the
                    // intermediate pipe to stdout using unbuffered read/write. Then
                    // we tell the caller to fall back.
                    // use read_to_end to drain pipe for the case write failed
                    debug_assert!(n <= MAX_ROOTLESS_PIPE_SIZE, "unexpected RAM usage");
                    let mut drain = Vec::with_capacity(n);
                    pipe_rd.take(n as u64).read_to_end(&mut drain)?;
                    RawWriter(&dest).write_all(&drain)?;
                    return Ok(Err(()));
                }
            }
            Err(_) => return Ok(Err(())),
        }
    }
}

/// try splice_unbounded 1st and splice_unbounded_broker if both of in/output are not pipe
///
/// return true if write fallback is needed
/// (the fallback will be embedded to this function in the future)
#[inline]
pub fn splice_unbounded_auto(source: &impl AsFd, dest: &mut impl AsFd) -> std::io::Result<bool> {
    // use splice to check that input or output is pipe which is efficient
    let fallback = match splice(&source, dest, MAX_ROOTLESS_PIPE_SIZE) {
        Ok(_) => splice_unbounded(source, dest).is_err(),
        _ => splice_unbounded_broker(source, dest)?.is_err(),
    };
    Ok(fallback)
}

/// splice `n` bytes with safe read/write fallback
/// return actually sent bytes
#[inline]
pub fn send_n_bytes(input: impl AsFd, target: impl AsFd, n: u64) -> std::io::Result<u64> {
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
                    n -= s as u64;
                    bytes_written += s as u64;
                    if splice_exact(&broker_r, &target, s).is_ok() {
                        if n == 0 {
                            // avoid unnecessary splice for small input
                            break false;
                        }
                    } else {
                        debug_assert!(s <= MAX_ROOTLESS_PIPE_SIZE, "unexpected RAM usage");
                        // use read_to_end to drain pipe at this fallback for the case write failed
                        let mut drain = Vec::with_capacity(s);
                        broker_r.take(s as u64).read_to_end(&mut drain)?;
                        RawWriter(&target).write_all(&drain)?;
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
    // do not buffer at this fallback, or order of output would be wrong with multiple input
    bytes_written += std::io::copy(&mut RawReader(input).take(n), &mut RawWriter(target))?;
    Ok(bytes_written)
}

/// Return verified /dev/null
///
/// `splice` to /dev/null is faster than `read` when we skip or count the non-seekable input
#[inline]
pub fn dev_null() -> Option<std::fs::File> {
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
pub fn tee(source: &impl AsFd, target: &impl AsFd, len: usize) -> rustix::io::Result<usize> {
    rustix::pipe::tee(source, target, len, SpliceFlags::empty())
}
