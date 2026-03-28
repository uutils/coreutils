// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::io;

use thiserror::Error;
use uucore::error::{UError, strip_errno};
use uucore::translate;

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("{}", translate!("date-error-stdout", "error" => .0))]
    Stdout(StdoutError),
}

impl From<StdoutError> for Error {
    fn from(error: StdoutError) -> Self {
        Self::Stdout(error)
    }
}

impl UError for Error {
    fn code(&self) -> i32 {
        EXIT_ERR
    }
}

#[derive(Debug, Error)]
pub(crate) enum StdoutError {
    #[cfg(unix)]
    #[error("{}", strip_errno(&io::Error::from_raw_os_error(libc::EBADF)))]
    BadFd,
    #[error("{}", strip_errno(.0))]
    Io(#[from] io::Error),
}

static EXIT_ERR: i32 = 1;
