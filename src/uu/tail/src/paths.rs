// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore tailable stdlib (stdlib)

use crate::text;
use std::ffi::OsStr;
use std::fs::{File, Metadata};
use std::io::{Seek, SeekFrom};
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};
#[cfg(not(target_os = "wasi"))]
use std::path::Path;
use std::path::PathBuf;
#[cfg(not(target_os = "wasi"))]
use uucore::error::UResult;
use uucore::quoting_style::locale_aware_shell_escape;
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
                #[cfg(target_vendor = "apple")]
                {
                    use std::os::unix::ffi::OsStrExt;

                    // /dev/fd/0 cannot be canonicalized on macOS; query the descriptor instead.
                    let path = rustix::fs::getpath(std::io::stdin()).ok()?;
                    Path::new(OsStr::from_bytes(path.to_bytes()))
                        .canonicalize()
                        .ok()
                }
                #[cfg(windows)]
                {
                    resolve_stdin_path()
                }
                #[cfg(not(any(target_vendor = "apple", windows)))]
                {
                    PathBuf::from(text::FD0).canonicalize().ok()
                }
            }
        }
    }
}

#[cfg(windows)]
fn resolve_stdin_path() -> Option<PathBuf> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::MAX_PATH;
    use windows_sys::Win32::Storage::FileSystem::{FILE_NAME_OPENED, GetFinalPathNameByHandleW};

    let handle = std::io::stdin().lock().as_raw_handle();
    if handle.is_null() {
        return None;
    }

    let mut buffer = [0u16; MAX_PATH as usize];
    // SAFETY: the handle is borrowed from stdin and the buffer is valid for the given length.
    let len = unsafe {
        GetFinalPathNameByHandleW(
            handle,
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            FILE_NAME_OPENED,
        )
    } as usize;

    if len == 0 || len >= buffer.len() {
        return None;
    }

    String::from_utf16(&buffer[..len]).ok().map(PathBuf::from)
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
                "{}==> {} <==",
                if self.first_header { "" } else { "\n" },
                // GNU quotes the name shown in the header when it needs it.
                locale_aware_shell_escape(string),
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
    #[cfg(not(target_os = "wasi"))]
    fn got_truncated(&self, other: &Metadata) -> UResult<bool>;
    #[cfg(not(target_os = "wasi"))]
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
    #[cfg(not(target_os = "wasi"))]
    fn got_truncated(&self, other: &Metadata) -> UResult<bool> {
        Ok(other.len() < self.len() && other.modified()? != self.modified()?)
    }

    #[cfg(not(target_os = "wasi"))]
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

#[cfg(not(target_os = "wasi"))]
pub trait PathExtTail {
    fn is_stdin(&self) -> bool;
    fn has_active_parent(&self) -> bool;
    fn is_tailable(&self) -> bool;
}

#[cfg(not(target_os = "wasi"))]
impl PathExtTail for Path {
    fn is_stdin(&self) -> bool {
        self.eq(Self::new(text::DASH))
            || self.eq(Self::new(text::DEV_STDIN))
            || self.eq(Self::new(&translate!("tail-stdin-header")))
    }

    /// Return true if `path` has an existing parent directory
    fn has_active_parent(&self) -> bool {
        self.parent().is_some_and(Self::is_dir)
    }

    /// Return true if `path` is a file type that can be tailed
    fn is_tailable(&self) -> bool {
        path_is_tailable(self)
    }
}

#[cfg(not(target_os = "wasi"))]
pub fn path_is_tailable(path: &Path) -> bool {
    path.is_file() || path.exists() && path.metadata().is_ok_and(|meta| meta.is_tailable())
}

#[inline]
#[cfg(all(unix, not(target_os = "fuchsia")))]
pub fn stdin_is_bad_fd() -> bool {
    uucore::signals::stdin_was_closed()
}

#[inline]
#[cfg(not(all(unix, not(target_os = "fuchsia"))))]
pub fn stdin_is_bad_fd() -> bool {
    false
}
