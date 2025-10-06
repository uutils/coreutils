// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::fmt::Debug;
use thiserror::Error;
use uucore::error::UError;

#[derive(Debug, Error)]
#[error("{0}")]
pub enum NumfmtError {
    IoError(String),
    IllegalArgument(String),
    FormattingError(String),
}

impl UError for NumfmtError {
    fn code(&self) -> i32 {
        match self {
            Self::IoError(_) => 1,
            Self::IllegalArgument(_) => 1,
            Self::FormattingError(_) => 2,
        }
    }
}
