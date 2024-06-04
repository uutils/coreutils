// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use quick_error::quick_error;
use std::fmt::Debug;
use uucore::error::UError;

quick_error! {
    #[derive(Debug)]
    pub enum NumfmtError {
        IoError(s: String) {
            display("{}", s)
        }
        IllegalArgument(s: String) {
            display("{}", s)
        }
        FormattingError(s: String) {
            display("{}", s)
        }
    }
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
