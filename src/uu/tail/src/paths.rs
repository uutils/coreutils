//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore tailable seekable stdlib (stdlib)

use crate::text;
use same_file::Handle;
use std::ffi::OsStr;
use std::fs::{File, Metadata};
use std::io::{self, Seek};
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};
use uucore::error::UResult;

pub enum Opened {
    File(File),
    Fifo(File),
    Pipe(File),
}

/// The kind of input, either a `File` or `Stdin` defining an [`Input`]  .
#[derive(Debug, Clone)]
pub enum InputKind {
    File(PathBuf),
    Stdin,
}

/// Represent an input from the command line arguments.
///
/// Is composed of an [`InputKind`] and a display name.
#[derive(Debug, Clone)]
pub struct Input {
    kind: InputKind,
    pub display_name: String,
}

impl Input {
    pub fn from<T: AsRef<OsStr>>(string: T) -> Self {
        let string = string.as_ref();
        let kind = if string == OsStr::new(text::DASH) {
            InputKind::Stdin
        } else {
            InputKind::File(PathBuf::from(string))
        };

        let display_name = match kind {
            InputKind::File(_) => string.to_string_lossy().to_string(),
            InputKind::Stdin => text::STDIN_HEADER.to_string(),
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

    pub fn open(&self) -> io::Result<Opened> {
        match &self.kind {
            InputKind::File(path) => Ok(Opened::File(File::open(path)?)),
            InputKind::Stdin => {
                let mut handle = Handle::stdin()?;
                let file = handle.as_file_mut();
                if file.is_seekable() {
                    Ok(Opened::Fifo(file.try_clone()?))
                } else {
                    Ok(Opened::Pipe(file.try_clone()?))
                }
            }
        }
    }

    #[cfg(unix)]
    pub fn path(&self) -> Option<PathBuf> {
        match &self.kind {
            InputKind::File(path) => Some(path.to_owned()),
            InputKind::Stdin => Some(PathBuf::from(text::DEV_STDIN)),
        }
    }

    #[cfg(windows)]
    pub fn path(&self) -> Option<PathBuf> {
        match &self.kind {
            InputKind::File(path) => Some(path.to_owned()),
            InputKind::Stdin => None,
        }
    }
}

impl Default for Input {
    fn default() -> Self {
        Self {
            kind: InputKind::Stdin,
            display_name: String::from(text::STDIN_HEADER),
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
                string,
            );
            self.first_header = false;
        }
    }
}
pub trait FileExtTail {
    #[allow(clippy::wrong_self_convention)]
    fn is_seekable(&mut self) -> bool;
}

impl FileExtTail for File {
    /// Test if File is seekable.
    fn is_seekable(&mut self) -> bool {
        self.stream_position().is_ok()
    }
}

pub trait MetadataExtTail {
    fn is_tailable(&self) -> bool;
    fn got_truncated(&self, other: &Metadata) -> UResult<bool>;
    fn get_block_size(&self) -> u64;
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

    fn get_block_size(&self) -> u64 {
        #[cfg(unix)]
        {
            self.blocks()
        }
        #[cfg(not(unix))]
        {
            self.len()
        }
    }

    fn file_id_eq(&self, _other: &Metadata) -> bool {
        #[cfg(unix)]
        {
            self.ino().eq(&_other.ino())
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
            || self.eq(Self::new(text::STDIN_HEADER))
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
    path.is_file() || path.exists() && path.metadata().map_or(false, |meta| meta.is_tailable())
}
