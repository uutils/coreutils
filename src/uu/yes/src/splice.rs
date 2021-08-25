//! On Linux we can use vmsplice() to write data more efficiently.
//!
//! This does not always work. We're not allowed to splice to some targets,
//! and on some systems (notably WSL 1) it isn't supported at all.
//!
//! If we get an error code that suggests splicing isn't supported then we
//! tell that to the caller so it can fall back to a robust naïve method. If
//! we get another kind of error we bubble it up as normal.
//!
//! vmsplice() can only splice into a pipe, so if the output is not a pipe
//! we make our own and use splice() to bridge the gap from the pipe to the
//! output.
//!
//! We assume that an "unsupported" error will only ever happen before any
//! data was successfully written to the output. That way we don't have to
//! make any effort to rescue data from the pipe if splice() fails, we can
//! just fall back and start over from the beginning.

use std::{
    fs::File,
    io,
    os::unix::io::{AsRawFd, FromRawFd},
};

use nix::{
    errno::Errno,
    fcntl::SpliceFFlags,
    libc::S_IFIFO,
    sys::{stat::fstat, uio::IoVec},
};

pub(crate) fn splice_data(bytes: &[u8], out: &impl AsRawFd) -> Result<()> {
    let is_pipe = fstat(out.as_raw_fd())?.st_mode & S_IFIFO != 0;

    if is_pipe {
        loop {
            let mut bytes = bytes;
            while !bytes.is_empty() {
                let len = vmsplice(out, bytes)?;
                bytes = &bytes[len..];
            }
        }
    } else {
        let (read, write) = pipe()?;
        loop {
            let mut bytes = bytes;
            while !bytes.is_empty() {
                let len = vmsplice(&write, bytes)?;
                let mut remaining = len;
                while remaining > 0 {
                    match splice(&read, out, remaining)? {
                        0 => panic!("Unexpected end of pipe"),
                        n => remaining -= n,
                    };
                }
                bytes = &bytes[len..];
            }
        }
    }
}

pub(crate) enum Error {
    Unsupported,
    Io(io::Error),
}

type Result<T> = std::result::Result<T, Error>;

impl From<nix::Error> for Error {
    fn from(error: nix::Error) -> Self {
        match error {
            nix::Error::Sys(errno) => Error::Io(io::Error::from_raw_os_error(errno as i32)),
            _ => Error::Io(io::Error::last_os_error()),
        }
    }
}

fn maybe_unsupported(error: nix::Error) -> Error {
    match error.as_errno() {
        Some(Errno::EINVAL) | Some(Errno::ENOSYS) | Some(Errno::EBADF) => Error::Unsupported,
        _ => error.into(),
    }
}

fn splice(source: &impl AsRawFd, target: &impl AsRawFd, len: usize) -> Result<usize> {
    nix::fcntl::splice(
        source.as_raw_fd(),
        None,
        target.as_raw_fd(),
        None,
        len,
        SpliceFFlags::empty(),
    )
    .map_err(maybe_unsupported)
}

fn vmsplice(target: &impl AsRawFd, bytes: &[u8]) -> Result<usize> {
    nix::fcntl::vmsplice(
        target.as_raw_fd(),
        &[IoVec::from_slice(bytes)],
        SpliceFFlags::empty(),
    )
    .map_err(maybe_unsupported)
}

fn pipe() -> nix::Result<(File, File)> {
    let (read, write) = nix::unistd::pipe()?;
    // SAFETY: The file descriptors do not have other owners.
    unsafe { Ok((File::from_raw_fd(read), File::from_raw_fd(write))) }
}
