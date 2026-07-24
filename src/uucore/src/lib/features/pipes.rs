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

/// A type allows to
/// - check that zero-copy succeed by ?.is_ok()
/// - check that zero-copy failed, but read/write fallback succeed by ?.is_err()
/// - catch the read/write fallback's error by ? or let Err(e)
///
/// use rustix::io::Result for functions without read/write fallback
type PipeRes = std::io::Result<Result<(), ()>>;

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
#[inline]
pub fn drain_pipe(pipe: &PipeReader, dest: &impl AsFd, len: usize) -> PipeRes {
    debug_assert!(len <= MAX_ROOTLESS_PIPE_SIZE, "unexpected RAM usage");
    let mut remaining = len;
    while remaining > 0 {
        if let Ok(s) = splice(pipe, dest, remaining) {
            remaining -= s;
        } else {
            // read/write fallback
            // use read_to_end to make pipe empty for the case write failed
            let mut drain = Vec::with_capacity(remaining);
            pipe.take(remaining as u64).read_to_end(&mut drain)?;
            RawWriter(&dest).write_all(&drain)?;
            return Ok(Err(()));
        }
    }
    Ok(Ok(()))
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
#[inline]
pub fn splice_unbounded_auto(source: &impl AsFd, dest: &mut impl AsFd) -> PipeRes {
    static PIPE_CACHE: OnceLock<Option<(PipeReader, PipeWriter)>> = OnceLock::new();
    let Some((pipe_rd, pipe_wr)) = PIPE_CACHE.get_or_init(|| pipe::<false>().ok()) else {
        return Ok(Err(()));
    };
    // fcntl for input would not improve throughput since
    // - sender with splice probably increased size already
    // - sender without splice is bottleneck
    let _ = fcntl_setpipe_size(&mut *dest, MAX_ROOTLESS_PIPE_SIZE);
    // pre-generate page caches for splice
    let _ = rustix::fs::fadvise(source, 0, None, rustix::fs::Advice::Sequential);
    // check support of splice and fallback
    match splice(&source, &pipe_wr, MAX_ROOTLESS_PIPE_SIZE) {
        Ok(0) => return Ok(Ok(())),
        Ok(n) => {
            if drain_pipe(pipe_rd, dest, n)?.is_err() {
                return Ok(Err(()));
            }
        }
        Err(_) => return Ok(Err(())),
    }
    // GNU catches all strace injections for except for 1st one
    while let mut n @ 1.. =
        splice(&source, &pipe_wr, MAX_ROOTLESS_PIPE_SIZE).map_err(std::io::Error::from)?
    {
        while n > 0 {
            n -= splice(&pipe_rd, dest, n)?;
        }
    }
    Ok(Ok(()))
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
    let succeed_or_fuse = if let Some((broker_r, broker_w)) = PIPE_CACHE
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
                    if drain_pipe(broker_r, &target, s)?.is_err() {
                        break false;
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
