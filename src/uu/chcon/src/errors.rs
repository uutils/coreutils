// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
#![cfg(target_os = "linux")]

use std::ffi::OsString;
use std::fmt::Write;
use std::io;

use thiserror::Error;
use uucore::display::Quotable;
use uucore::translate;

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub(crate) enum Error {
    #[error("{}", translate!("chcon-error-no-context-specified"))]
    MissingContext,

    #[error("{}", translate!("chcon-error-no-files-specified"))]
    MissingFiles,

    #[error("{}", translate!("chcon-error-data-out-of-range"))]
    OutOfRange,

    #[error("{0}")]
    ArgumentsMismatch(String),

    #[error(transparent)]
    CommandLine(#[from] clap::Error),

    #[error("{}", translate!("chcon-error-operation-failed", "operation" => operation.clone()))]
    SELinux {
        operation: String,
        #[source]
        source: selinux::errors::Error,
    },

    #[error("{}", translate!("chcon-error-operation-failed", "operation" => operation.clone()))]
    Io {
        operation: String,
        #[source]
        source: io::Error,
    },

    #[error("{}", translate!("chcon-error-operation-failed-on", "operation" => operation.clone(), "operand" => operand1.quote()))]
    Io1 {
        operation: String,
        operand1: OsString,
        #[source]
        source: io::Error,
    },
}

impl Error {
    pub(crate) fn from_io(operation: impl Into<String>, source: io::Error) -> Self {
        Self::Io {
            operation: operation.into(),
            source,
        }
    }

    pub(crate) fn from_io1(
        operation: impl Into<String>,
        operand1: impl Into<OsString>,
        source: io::Error,
    ) -> Self {
        Self::Io1 {
            operation: operation.into(),
            operand1: operand1.into(),
            source,
        }
    }

    pub(crate) fn from_selinux(
        operation: impl Into<String>,
        source: selinux::errors::Error,
    ) -> Self {
        Self::SELinux {
            operation: operation.into(),
            source,
        }
    }
}

pub(crate) fn report_full_error(mut err: &dyn std::error::Error) -> String {
    let mut desc = String::with_capacity(256);
    write!(desc, "{err}").unwrap();
    while let Some(source) = err.source() {
        err = source;
        write!(desc, ". {err}").unwrap();
    }
    desc
}
