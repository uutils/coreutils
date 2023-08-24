// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
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
    TargetNotADirectory(String),
}

impl Error for MvError {}
impl UError for MvError {}
impl Display for MvError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Self::NoSuchFile(s) => write!(f, "cannot stat {s}: No such file or directory"),
            Self::SameFile(s, t) => write!(f, "{s} and {t} are the same file"),
            Self::SelfSubdirectory(s) => write!(
                f,
                "cannot move '{s}' to a subdirectory of itself, '{s}/{s}'"
            ),
            Self::DirectoryToNonDirectory(t) => {
                write!(f, "cannot overwrite directory {t} with non-directory")
            }
            Self::NonDirectoryToDirectory(s, t) => {
                write!(f, "cannot overwrite non-directory {t} with directory {s}")
            }
            Self::NotADirectory(t) => write!(f, "target {t}: Not a directory"),
            Self::TargetNotADirectory(t) => write!(f, "target directory {t}: Not a directory"),
        }
    }
}
