//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

use std::{
    error::Error,
    fmt::{Debug, Display},
};
use uucore::error::UError;

#[derive(Debug)]
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

impl Error for NumfmtError {}

impl Display for NumfmtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(s) | Self::IllegalArgument(s) | Self::FormattingError(s) => {
                write!(f, "{s}")
            }
        }
    }
}
