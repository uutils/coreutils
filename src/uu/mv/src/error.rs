// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
use std::error::Error;
use std::fmt::{Display, Formatter, Result};

use uucore::error::UError;

#[derive(Debug)]
pub enum MvError {
    NoSuchFile(String),
    SameFile(String, String),
    SelfSubdirectory(String),
    DirectoryToNonDirectory(String),
    NonDirectoryToDirectory(String, String),
    NotADirectory(String),
}

impl Error for MvError {}
impl UError for MvError {}
impl Display for MvError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Self::NoSuchFile(s) => write!(f, "cannot stat {}: No such file or directory", s),
            Self::SameFile(s, t) => write!(f, "{} and {} are the same file", s, t),
            Self::SelfSubdirectory(s) => write!(
                f,
                "cannot move '{s}' to a subdirectory of itself, '{s}/{s}'",
                s = s
            ),
            Self::DirectoryToNonDirectory(t) => {
                write!(f, "cannot overwrite directory {} with non-directory", t)
            }
            Self::NonDirectoryToDirectory(s, t) => write!(
                f,
                "cannot overwrite non-directory {} with directory {}",
                t, s
            ),
            Self::NotADirectory(t) => write!(f, "target {} is not a directory", t),
        }
    }
}
