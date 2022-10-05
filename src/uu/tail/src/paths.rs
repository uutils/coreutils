//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore tailable seekable stdlib (stdlib)

#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};

use std::collections::VecDeque;
use std::fs::{File, Metadata};
use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};

use uucore::error::UResult;

use crate::args::Settings;
use crate::text;

//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[derive(Debug, Clone)]
pub enum InputKind {
    File(PathBuf),
    Stdin,
}

#[derive(Debug, Clone)]
pub struct Input {
    kind: InputKind,
    pub display_name: String,
}

impl Input {
    pub fn from(string: String) -> Self {
        let kind = if string == text::DASH {
            InputKind::Stdin
        } else {
            InputKind::File(PathBuf::from(&string))
        };

        let display_name = match kind {
            InputKind::File(_) => string,
            InputKind::Stdin => {
                if cfg!(unix) {
                    text::STDIN_HEADER.to_string()
                } else {
                    string
                }
            }
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
                if cfg!(unix) {
                    match PathBuf::from(text::DEV_STDIN).canonicalize().ok() {
                        Some(path) if path != PathBuf::from(text::FD0) => Some(path),
                        Some(_) | None => None,
                    }
                } else {
                    None
                }
            }
        }
    }

    pub fn is_tailable(&self) -> bool {
        match &self.kind {
            InputKind::File(path) => path_is_tailable(path),
            InputKind::Stdin => self.resolve().map_or(false, |path| path_is_tailable(&path)),
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

#[derive(Debug, Clone)]
pub struct InputService {
    pub inputs: VecDeque<Input>,
    pub presume_input_pipe: bool,
    pub header_printer: HeaderPrinter,
}

impl InputService {
    pub fn new(verbose: bool, presume_input_pipe: bool, inputs: VecDeque<Input>) -> Self {
        Self {
            inputs,
            presume_input_pipe,
            header_printer: HeaderPrinter::new(verbose, true),
        }
    }

    pub fn from(settings: &Settings) -> Self {
        Self::new(
            settings.verbose,
            settings.presume_input_pipe,
            settings.inputs.clone(),
        )
    }

    pub fn has_stdin(&mut self) -> bool {
        self.inputs.iter().any(|input| input.is_stdin())
    }

    pub fn has_only_stdin(&self) -> bool {
        self.inputs.iter().all(|input| input.is_stdin())
    }

    pub fn print_header(&mut self, input: &Input) {
        self.header_printer.print_input(input);
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
        self.seek(SeekFrom::Current(0)).is_ok()
            && self.seek(SeekFrom::End(0)).is_ok()
            && self.seek(SeekFrom::Start(current_offset)).is_ok()
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
            // use std::os::windows::prelude::*;
            // if let Some(self_id) = self.file_index() {
            //     if let Some(other_id) = other.file_index() {
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

#[inline]
pub fn stdin_is_bad_fd() -> bool {
    // FIXME : Rust's stdlib is reopening fds as /dev/null
    // see also: https://github.com/uutils/coreutils/issues/2873
    // (gnu/tests/tail-2/follow-stdin.sh fails because of this)
    //#[cfg(unix)]
    {
        //platform::stdin_is_bad_fd()
    }
    //#[cfg(not(unix))]
    false
}
