// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use quick_error::quick_error;

use uucore::error::UError;

quick_error! {
    #[derive(Debug)]
    pub enum MvError {
        NoSuchFile(s: String) {
            display("cannot stat {}: No such file or directory", s)
        }
        CannotStatNotADirectory(s: String) {
            display("cannot stat {}: Not a directory", s)
        }
        SameFile(s: String, t: String) {
            display("{} and {} are the same file", s, t)
        }
        SelfSubdirectory(s: String) {
            display("cannot move '{}' to a subdirectory of itself, '{}/{}'", s, s, s)
        }
        SelfTargetSubdirectory(s: String, t: String) {
            display("cannot move '{}' to a subdirectory of itself, '{}/{}'", s, t, s)
        }
        DirectoryToNonDirectory(t: String) {
            display("cannot overwrite directory {} with non-directory", t)
        }
        NonDirectoryToDirectory(s: String, t: String) {
            display("cannot overwrite non-directory {} with directory {}", t, s)
        }
        NotADirectory(t: String) {
            display("target {}: Not a directory", t)
        }
        TargetNotADirectory(t: String) {
            display("target directory {}: Not a directory", t)
        }
        FailedToAccessNotADirectory(t: String) {
            display("failed to access {}: Not a directory", t)
        }
    }
}

impl UError for MvError {}
