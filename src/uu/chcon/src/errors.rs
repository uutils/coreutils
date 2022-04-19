use std::ffi::OsString;
use std::fmt::Write;
use std::io;

use uucore::display::Quotable;

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("No context is specified")]
    MissingContext,

    #[error("No files are specified")]
    MissingFiles,

    #[error("{0}")]
    ArgumentsMismatch(String),

    #[error(transparent)]
    CommandLine(#[from] clap::Error),

    #[error("{operation} failed")]
    SELinux {
        operation: &'static str,
        source: selinux::errors::Error,
    },

    #[error("{operation} failed")]
    Io {
        operation: &'static str,
        source: io::Error,
    },

    #[error("{operation} failed on {}", .operand1.quote())]
    Io1 {
        operation: &'static str,
        operand1: OsString,
        source: io::Error,
    },
}

impl Error {
    pub(crate) fn from_io(operation: &'static str, source: io::Error) -> Self {
        Self::Io { operation, source }
    }

    pub(crate) fn from_io1(
        operation: &'static str,
        operand1: impl Into<OsString>,
        source: io::Error,
    ) -> Self {
        Self::Io1 {
            operation,
            operand1: operand1.into(),
            source,
        }
    }

    pub(crate) fn from_selinux(operation: &'static str, source: selinux::errors::Error) -> Self {
        Self::SELinux { operation, source }
    }
}

pub(crate) fn report_full_error(mut err: &dyn std::error::Error) -> String {
    let mut desc = String::with_capacity(256);
    write!(desc, "{}", err).unwrap();
    while let Some(source) = err.source() {
        err = source;
        write!(desc, ". {}", err).unwrap();
    }
    desc
}
