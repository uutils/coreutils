// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore tailable seekable stdlib (stdlib)

use crate::text;
use std::ffi::OsStr;
use std::fs::{File, Metadata};
use std::io::{Seek, SeekFrom};
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};
use uucore::error::UResult;
use uucore::translate;

#[derive(Debug, Clone)]
pub enum InputKind {
    File(PathBuf),
    Stdin,
}

#[cfg(unix)]
impl From<&OsStr> for InputKind {
    fn from(value: &OsStr) -> Self {
        if value == OsStr::new("-") {
            Self::Stdin
        } else {
            Self::File(PathBuf::from(value))
        }
    }
}

#[cfg(not(unix))]
impl From<&OsStr> for InputKind {
    fn from(value: &OsStr) -> Self {
        if value == OsStr::new(text::DASH) {
            Self::Stdin
        } else {
            Self::File(PathBuf::from(value))
        }
    }
}

#[derive(Debug, Clone)]
pub struct Input {
    kind: InputKind,
    pub display_name: String,
}

impl Input {
    pub fn from<T: AsRef<OsStr>>(string: T) -> Self {
        let string = string.as_ref();

        let kind = string.into();
        let display_name = match kind {
            InputKind::File(_) => string.to_string_lossy().to_string(),
            InputKind::Stdin => translate!("tail-stdin-header"),
        };

        Self { kind, display_name }
    }

    pub fn kind(&self) -> &InputKind {
        &self.kind
    }

    pub fn is_stdin(&self) -> bool {
        match self.kind {
            InputKind::File(_) => false,
            InputKind::Stdin => true,
        }
    }

    pub fn resolve(&self) -> Option<PathBuf> {
        match &self.kind {
            InputKind::File(path) if path != &PathBuf::from(text::DEV_STDIN) => {
                path.canonicalize().ok()
            }
            InputKind::File(_) | InputKind::Stdin => {
                // Don't try to canonicalize stdin when it's a pipe or special fd.
                // Attempting to canonicalize stdin (/dev/fd/0) when it's a pipe
                // causes issues with pseudo (used by build systems) which
                // intercepts realpath() syscalls and cannot handle pipe descriptors.
                // This results in errors like:
                //   "couldn't allocate absolute path for 'null'"
                // See: https://github.com/uutils/coreutils/issues/9292
                //
                // On macOS, /dev/fd isn't backed by /proc and canonicalize()
                // on dev/fd/0 (or /dev/stdin) will fail (NotFound),
                // so we treat stdin as a pipe here
                // https://github.com/rust-lang/rust/issues/95239
                #[cfg(target_os = "macos")]
                {
                    None
                }
                #[cfg(not(target_os = "macos"))]
                {
                    // Try to check if stdin is a regular file before canonicalizing
                    // Only canonicalize if it's a regular file (e.g., redirected file)
                    // For pipes, fifos, or other special files, return None
                    if let Ok(metadata) = std::fs::metadata(text::FD0) {
                        let file_type = metadata.file_type();
                        if file_type.is_file() {
                            // It's a regular file (like a redirected file), safe to canonicalize
                            return PathBuf::from(text::FD0).canonicalize().ok();
                        }
                    }
                    // For pipes, fifos, or if metadata fails, don't canonicalize
                    None
                }
            }
        }
    }

    pub fn is_tailable(&self) -> bool {
        match &self.kind {
            InputKind::File(path) => path_is_tailable(path),
            InputKind::Stdin => self.resolve().is_some_and(|path| path_is_tailable(&path)),
        }
    }
}

impl Default for Input {
    fn default() -> Self {
        Self {
            kind: InputKind::Stdin,
            display_name: translate!("tail-stdin-header"),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct HeaderPrinter {
    verbose: bool,
    first_header: bool,
}

impl HeaderPrinter {
    pub fn new(verbose: bool, first_header: bool) -> Self {
        Self {
            verbose,
            first_header,
        }
    }

    pub fn print_input(&mut self, input: &Input) {
        self.print(input.display_name.as_str());
    }

    pub fn print(&mut self, string: &str) {
        if self.verbose {
            println!(
                "{}==> {string} <==",
                if self.first_header { "" } else { "\n" },
            );
            self.first_header = false;
        }
    }
}
pub trait FileExtTail {
    #[allow(clippy::wrong_self_convention)]
    fn is_seekable(&mut self, current_offset: u64) -> bool;
}

impl FileExtTail for File {
    /// Test if File is seekable.
    /// Set the current position offset to `current_offset`.
    fn is_seekable(&mut self, current_offset: u64) -> bool {
        self.stream_position().is_ok()
            && self.seek(SeekFrom::End(0)).is_ok()
            && self.seek(SeekFrom::Start(current_offset)).is_ok()
    }
}

pub trait MetadataExtTail {
    fn is_tailable(&self) -> bool;
    fn got_truncated(&self, other: &Metadata) -> UResult<bool>;
    fn file_id_eq(&self, other: &Metadata) -> bool;
}

impl MetadataExtTail for Metadata {
    fn is_tailable(&self) -> bool {
        let ft = self.file_type();
        #[cfg(unix)]
        {
            ft.is_file() || ft.is_char_device() || ft.is_fifo()
        }
        #[cfg(not(unix))]
        {
            ft.is_file()
        }
    }

    /// Return true if the file was modified and is now shorter
    fn got_truncated(&self, other: &Metadata) -> UResult<bool> {
        Ok(other.len() < self.len() && other.modified()? != self.modified()?)
    }

    fn file_id_eq(&self, #[cfg(unix)] other: &Metadata, #[cfg(not(unix))] _: &Metadata) -> bool {
        #[cfg(unix)]
        {
            self.ino().eq(&other.ino())
        }
        #[cfg(windows)]
        {
            // TODO: `file_index` requires unstable library feature `windows_by_handle`
            // use std::os::windows::prelude::*;
            // if let Some(self_id) = self.file_index() {
            //     if let Some(other_id) = other.file_index() {
            //     // TODO: not sure this is the equivalent of comparing inode numbers
            //
            //         return self_id.eq(&other_id);
            //     }
            // }
            false
        }
    }
}

pub trait PathExtTail {
    fn is_stdin(&self) -> bool;
    fn is_orphan(&self) -> bool;
    fn is_tailable(&self) -> bool;
}

impl PathExtTail for Path {
    fn is_stdin(&self) -> bool {
        self.eq(Self::new(text::DASH))
            || self.eq(Self::new(text::DEV_STDIN))
            || self.eq(Self::new(&translate!("tail-stdin-header")))
    }

    /// Return true if `path` does not have an existing parent directory
    fn is_orphan(&self) -> bool {
        !matches!(self.parent(), Some(parent) if parent.is_dir())
    }

    /// Return true if `path` is is a file type that can be tailed
    fn is_tailable(&self) -> bool {
        path_is_tailable(self)
    }
}

pub fn path_is_tailable(path: &Path) -> bool {
    path.is_file() || path.exists() && path.metadata().is_ok_and(|meta| meta.is_tailable())
}

#[inline]
#[cfg(unix)]
pub fn stdin_is_bad_fd() -> bool {
    uucore::signals::stdin_was_closed()
}

#[inline]
#[cfg(not(unix))]
pub fn stdin_is_bad_fd() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_input_from_dash_creates_stdin() {
        let input = Input::from("-");
        assert!(input.is_stdin());
        assert!(matches!(input.kind(), InputKind::Stdin));
    }

    #[test]
    fn test_input_from_regular_path_creates_file() {
        let input = Input::from("test.txt");
        assert!(!input.is_stdin());
        assert!(matches!(input.kind(), InputKind::File(_)));
    }

    #[test]
    fn test_input_resolve_does_not_panic_on_stdin() {
        // This test ensures that calling resolve() on stdin doesn't panic
        // even when stdin is not available or is a pipe
        let input = Input::default(); // Creates stdin input
        let result = input.resolve();
        // On macOS, this should always be None
        // On Linux, this should be None for pipes/special files
        // We just verify it doesn't panic
        #[cfg(target_os = "macos")]
        assert_eq!(result, None);
        // On non-macOS, the result depends on what stdin actually is
        // but it should not panic regardless
        #[cfg(not(target_os = "macos"))]
        let _ = result;
    }

    #[test]
    fn test_input_resolve_with_regular_file() {
        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("tail_test_resolve.txt");
        fs::write(&test_file, "test content").unwrap();

        let input = Input::from(test_file.as_os_str());
        let resolved = input.resolve();

        // Regular files should resolve to their canonical path
        assert!(resolved.is_some());
        let resolved_path = resolved.unwrap();
        assert!(resolved_path.is_absolute());

        // Cleanup
        fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_input_resolve_does_not_canonicalize_dev_stdin() {
        // This test verifies the fix for issue #9292
        // Ensure that /dev/stdin is not canonicalized when it might be a pipe
        #[cfg(unix)]
        {
            let input = Input::from(text::DEV_STDIN);
            let result = input.resolve();

            // On macOS, should always return None
            #[cfg(target_os = "macos")]
            assert_eq!(
                result, None,
                "On macOS, /dev/stdin should not be canonicalized"
            );

            // On other Unix systems, should return None for pipes
            // If stdin happens to be a regular file, it might return Some
            // but the important thing is it doesn't panic or cause errors
            #[cfg(not(target_os = "macos"))]
            {
                // Just verify it doesn't panic - result depends on actual stdin state
                let _ = result;
            }
        }
    }

    #[test]
    fn test_path_is_tailable_with_regular_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("tail_test_tailable.txt");
        fs::write(&test_file, "test").unwrap();

        assert!(path_is_tailable(&test_file));

        fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_path_is_stdin() {
        use crate::text;

        let dash_path = Path::new(text::DASH);
        assert!(dash_path.is_stdin());

        let dev_stdin_path = Path::new(text::DEV_STDIN);
        assert!(dev_stdin_path.is_stdin());

        let regular_path = Path::new("test.txt");
        assert!(!regular_path.is_stdin());
    }
}
