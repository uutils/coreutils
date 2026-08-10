// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Zero-copy-related functions.

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

/// Whether an error from the splice helpers means that splice is unusable
/// here, so the caller should fall back on read/write.
///
/// The helpers in this module use `Err(EINVAL)` as that marker:
///
/// - `drain_pipe` fell back to read/write (the data was still written)
/// - `splice_unbounded_auto` could not splice anything
///
/// and they fold `ENOSYS` (kernel without the syscall) into it.
#[inline]
pub fn splice_unusable(err: &std::io::Error) -> bool {
    err.raw_os_error() == Some(rustix::io::Errno::INVAL.raw_os_error())
}

#[inline]
fn splice_unusable_errno() -> std::io::Error {
    std::io::Error::from_raw_os_error(rustix::io::Errno::INVAL.raw_os_error())
}

/// return pipe and try to extend its size
/// SIZE_REQUIRED should be true if you want to fail when changing pipe size failed
/// e.g. writing size to pipe should not hang
/// SIZE_REQUIRED=false allows to continue unbuffered splice I/O with default pipe size even if fcntl failed
///
/// used for resolving the limitation for splice: one of a input or output should be pipe
#[inline]
pub fn pipe<const SIZE_REQUIRED: bool>() -> std::io::Result<(PipeReader, PipeWriter)> {
    let pair = std::io::pipe()?;
    // pipe size is not RAM consumed by pipe with zero-copy. So we never use other size
    let r = fcntl_setpipe_size(&pair.0, MAX_ROOTLESS_PIPE_SIZE);
    if SIZE_REQUIRED {
        r?;
    }

    Ok(pair)
}

/// Less noisy wrapper around splice syscall
///
/// Up to `len` bytes are moved from `source` to `target`. Returns the number
/// of successfully moved bytes.
///
/// splice fails if both of `source` and `target` are not pipe. Consider using
/// splice_unbounded_broker or splice_unbounded_auto in the case.
#[inline]
pub fn splice(source: &impl AsFd, target: &impl AsFd, len: usize) -> rustix::io::Result<usize> {
    rustix::pipe::splice(source, None, target, None, len, SpliceFlags::empty())
}

/// splice `len` bytes from `pipe` into `dest`.
///
/// Returns `Err(EINVAL)` if splice turned out to be unusable: the data was
/// delivered by the read/write fallback instead, see [`splice_unusable`].
#[inline]
pub fn drain_pipe(pipe: &PipeReader, dest: &impl AsFd, len: usize) -> std::io::Result<()> {
    debug_assert!(len <= MAX_ROOTLESS_PIPE_SIZE, "unexpected RAM usage");
    let mut remaining = len;
    while remaining > 0 {
        match splice(pipe, dest, remaining) {
            Ok(0) => {
                // no progress; drain by hand
                let mut drain = Vec::with_capacity(remaining);
                pipe.take(remaining as u64).read_to_end(&mut drain)?;
                RawWriter(&dest).write_all(&drain)?;
                return Err(splice_unusable_errno());
            }
            Ok(s) => remaining -= s,
            Err(_) => {
                // read/write fallback
                // use read_to_end to make pipe empty for the case write failed
                let mut drain = Vec::with_capacity(remaining);
                pipe.take(remaining as u64).read_to_end(&mut drain)?;
                RawWriter(&dest).write_all(&drain)?;
                return Err(splice_unusable_errno());
            }
        }
    }
    Ok(())
}

/// check that source is FUSE
/// we fallback to read() at FUSE <https://github.com/uutils/coreutils/issues/9609>
#[inline]
pub fn might_fuse(source: &impl AsFd) -> bool {
    rustix::fs::fstatfs(source).map_or(true, |stats| stats.f_type == 0x6573_5546) // FUSE magic number, too many platform specific clippy warning with const
}

/// force-splice source to dest even both of them are not pipe via broker pipe
///
/// throughput is better than direct splice for the case one of in/output is pipe by unknown reason
/// This includes read ahead and optimization for stdout's pipe size
///
/// Returns `Err(EINVAL)` when nothing could be spliced, see [`splice_unusable`].
/// Errors while draining the intermediate pipe are real output errors.
#[inline]
pub fn splice_unbounded_auto(source: &impl AsFd, dest: &mut impl AsFd) -> std::io::Result<()> {
    static PIPE_CACHE: OnceLock<Option<(PipeReader, PipeWriter)>> = OnceLock::new();
    let Some((pipe_rd, pipe_wr)) = PIPE_CACHE.get_or_init(|| pipe::<false>().ok()) else {
        return Err(splice_unusable_errno());
    };

    // fcntl for input would not improve throughput since
    // - sender with splice probably increased size already
    // - sender without splice is bottleneck
    let _ = fcntl_setpipe_size(&mut *dest, MAX_ROOTLESS_PIPE_SIZE);
    // pre-generate page caches for splice
    let _ = rustix::fs::fadvise(source, 0, None, rustix::fs::Advice::Sequential);
    loop {
        match splice(&source, &pipe_wr, MAX_ROOTLESS_PIPE_SIZE) {
            Ok(0) => return Ok(()),
            Ok(n) => {
                drain_pipe(pipe_rd, dest, n)?;
            }
            Err(_) => return Err(splice_unusable_errno()),
        }
    }
}

/// splice `n` bytes with read/write fallback
/// return actually sent bytes
#[inline]
pub fn send_n_bytes(input: impl AsFd, target: impl AsFd, n: u64) -> std::io::Result<u64> {
    static PIPE_CACHE: OnceLock<Option<(PipeReader, PipeWriter)>> = OnceLock::new();
    let pipe_size = MAX_ROOTLESS_PIPE_SIZE.min(n as usize);
    // improve throughput if output is pipe
    // expected that input is already extended if it is coming from splice
    if pipe_size > KERNEL_DEFAULT_PIPE_SIZE {
        let _ = fcntl_setpipe_size(&target, pipe_size);
    }
    let mut n = n;
    let mut bytes_written: u64 = 0;
    let succeed_or_fuse = loop {
        if n == 0 {
            // avoid unnecessary syscall
            return Ok(bytes_written);
        }
        match splice(&input, &target, n as usize) {
            Ok(0) => break true,
            Ok(s) => {
                n -= s as u64;
                bytes_written += s as u64;
            }
            _ => break false, // input or output is not pipe
        }
    };
    let succeed_or_fuse = succeed_or_fuse
        || if let Some((broker_r, broker_w)) = PIPE_CACHE
            .get_or_init(|| {
                // use std::io::pipe to avoid unnecessary fcntl
                let pair = std::io::pipe().ok()?;
                if pipe_size > KERNEL_DEFAULT_PIPE_SIZE {
                    let _ = fcntl_setpipe_size(&pair.0, pipe_size);
                }
                Some(pair)
            })
            .as_ref()
        {
            // todo: create fn splice_bounded_broker
            loop {
                if n == 0 {
                    return Ok(bytes_written);
                }
                match splice(&input, &broker_w, n as usize) {
                    Ok(0) => break true,
                    Ok(s) => {
                        n -= s as u64;
                        bytes_written += s as u64;
                        if let Err(e) = drain_pipe(broker_r, &target, s) {
                            if splice_unusable(&e) {
                                break false;
                            }
                            return Err(e);
                        }
                    }
                    _ => break false,
                }
            }
        } else {
            false
        };
    // do not always fallback to write for fuse, or 2 Ctrl+D is required to exit on tty
    // todo: move fuse patch to callers
    if !succeed_or_fuse || might_fuse(&input) {
        // remove buffering from this fallback by RawReader, or order of output would be wrong with multiple input
        bytes_written += std::io::copy(&mut RawReader(input).take(n), &mut RawWriter(target))?;
    }

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
