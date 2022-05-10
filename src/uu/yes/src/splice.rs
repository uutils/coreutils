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

use std::{io, os::unix::io::AsRawFd};

use nix::{errno::Errno, libc::S_IFIFO, sys::stat::fstat};

use uucore::pipes::{pipe, splice_exact, vmsplice};

pub(crate) fn splice_data(bytes: &[u8], out: &impl AsRawFd) -> Result<()> {
    let is_pipe = fstat(out.as_raw_fd())?.st_mode as nix::libc::mode_t & S_IFIFO != 0;

    if is_pipe {
        loop {
            let mut bytes = bytes;
            while !bytes.is_empty() {
                let len = vmsplice(out, bytes).map_err(maybe_unsupported)?;
                bytes = &bytes[len..];
            }
        }
    } else {
        let (read, write) = pipe()?;
        loop {
            let mut bytes = bytes;
            while !bytes.is_empty() {
                let len = vmsplice(&write, bytes).map_err(maybe_unsupported)?;
                splice_exact(&read, out, len).map_err(maybe_unsupported)?;
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
        Self::Io(io::Error::from_raw_os_error(error as i32))
    }
}

fn maybe_unsupported(error: nix::Error) -> Error {
    match error {
        Errno::EINVAL | Errno::ENOSYS | Errno::EBADF => Error::Unsupported,
        _ => error.into(),
    }
}
