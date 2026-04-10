// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Output traits and types for programmatic access to ls functionality.
//!
//! This module provides a visitor/sink pattern that separates file enumeration
//! logic from output formatting. This allows programmatic consumers (shells,
//! file managers, etc.) to receive structured data without parsing text output.
//!
//! # Example
//!
//! ```ignore
//! use std::path::Path;
//! use uu_ls::{Config, list_with_output, EntryInfo, LsOutput, StreamingOutput};
//!
//! struct MySink {
//!     pub count: usize,
//! }
//!
//! impl MySink {
//!     fn new() -> Self {
//!         Self { count: 0 }
//!     }
//! }
//!
//! impl LsOutput for MySink {
//!     fn stream_mode(&self) -> StreamMode {
//!         StreamMode::Streaming
//!     }
//!
//!     fn write_entry(&mut self, entry: &EntryInfo) -> uucore::error::UResult<()> {
//!         println!("{} -> {:?}", entry.path.display(), entry.file_type);
//!         self.count += 1;
//!         Ok(())
//!     }
//! }
//!
//! let config = Config::from(&matches)?;
//! let mut output = MySink::new();
//! list_with_output(vec![Path::new(".")], &config, &mut output)?;
//! println!("processed {} entries", output.count);
//! ```

//! Alternatively, use [`StreamingOutput`] when you want a reusable streaming sink
//! that collects `EntryInfo` objects as they arrive.

use crate::{Config, PathData};
use std::ffi::OsString;
use std::fs::{FileType, Metadata};
use std::path::PathBuf;
use uucore::error::UResult;

/// Information about a single file/directory entry.
///
/// This struct provides programmatic access to file metadata without
/// requiring text parsing. All fields are pre-computed and ready for use.
#[derive(Debug, Clone)]
pub struct EntryInfo {
    /// The full path to the file
    pub path: PathBuf,
    /// The display name (file name portion, may differ from path for . and ..)
    pub display_name: OsString,
    /// The file type (file, directory, symlink, etc.)
    pub file_type: Option<FileType>,
    /// File metadata (size, permissions, timestamps, etc.)
    pub metadata: Option<Metadata>,
    /// Security context (SELinux) if available
    pub security_context: String,
    /// Whether this entry was specified on the command line
    pub command_line: bool,
    /// Whether symlinks should be dereferenced for this entry
    pub must_dereference: bool,
}

impl EntryInfo {
    /// Returns true if this entry represents a directory
    pub fn is_dir(&self) -> bool {
        self.file_type.as_ref().is_some_and(FileType::is_dir)
    }

    /// Returns true if this entry represents a regular file
    pub fn is_file(&self) -> bool {
        self.file_type.as_ref().is_some_and(FileType::is_file)
    }

    /// Returns true if this entry represents a symbolic link
    pub fn is_symlink(&self) -> bool {
        self.file_type.as_ref().is_some_and(FileType::is_symlink)
    }

    /// Returns the file size in bytes, if metadata is available
    pub fn size(&self) -> Option<u64> {
        self.metadata.as_ref().map(Metadata::len)
    }

    /// Returns the file name as a string slice, if valid UTF-8
    pub fn file_name(&self) -> Option<&str> {
        self.display_name.to_str()
    }
}

/// Streaming mode for `LsOutput` sinks.
///
/// `Batch` sinks receive a directory's entries all at once via
/// [`write_entries`]. `Streaming` sinks receive one entry at a time via
/// [`write_entry`].
pub enum StreamMode {
    Batch,
    Streaming,
}

/// Trait for receiving ls output entries.
///
/// Implement this trait to receive structured data from the ls enumeration
/// process. The trait is designed to support both streaming (one entry at a time)
/// and batched (all entries at once) use cases.
///
/// For programmatic access, implement [`write_entry`](LsOutput::write_entry) to
/// receive each entry individually.
///
/// The internal `TextOutput` implementation uses [`write_entries`](LsOutput::write_entries)
/// to receive batches for proper column alignment and grid formatting.
pub trait LsOutput {
    /// Returns the preferred output mode for this sink.
    ///
    /// Default is `Batch` so text-formatting sinks can continue to receive full
    /// directory batches. Streaming sinks should override this to return
    /// `StreamMode::Streaming`.
    fn stream_mode(&self) -> StreamMode {
        StreamMode::Batch
    }

    /// Called for each file/directory entry (streaming mode).
    ///
    /// Default implementation does nothing. Override this for programmatic access
    /// where you want to process entries one at a time.
    fn write_entry(&mut self, _entry: &EntryInfo) -> UResult<()> {
        Ok(())
    }

    /// Called with a batch of entries for a directory.
    ///
    /// Default implementation calls `write_entry` for each entry.
    /// Override this for text output that needs all entries for formatting.
    fn write_entries(&mut self, entries: &[PathData], config: &Config) -> UResult<()> {
        for entry in entries {
            self.write_entry(&entry.to_entry_info(config))?;
        }
        Ok(())
    }

    /// Called when entering a directory (for recursive listings or multiple arguments).
    ///
    /// # Arguments
    /// * `path_data` - The directory being entered
    /// * `config` - The ls configuration
    /// * `is_first` - Whether this is the first directory (affects newline handling)
    fn write_dir_header(
        &mut self,
        _path_data: &PathData,
        _config: &Config,
        _is_first: bool,
    ) -> UResult<()> {
        Ok(())
    }

    /// Called to report the total blocks for a directory in long format.
    ///
    /// The `total_size` parameter is the total number of blocks used by
    /// files in the directory.
    fn write_total(&mut self, _total_size: u64, _config: &Config) -> UResult<()> {
        Ok(())
    }

    /// Called to flush any buffered output (e.g., before error messages).
    fn flush(&mut self) -> UResult<()> {
        Ok(())
    }

    /// Called when all entries have been written.
    ///
    /// Use this for final cleanup, printing dired output, etc.
    fn finalize(&mut self, _config: &Config) -> UResult<()> {
        Ok(())
    }

    /// Called at the start of listing, before any entries are processed.
    ///
    /// Use this for initialization that needs the config (e.g., color reset).
    fn initialize(&mut self, _config: &Config) -> UResult<()> {
        Ok(())
    }
}

/// A dedicated streaming output sink.
///
/// This sink is intended for programmatic consumers that want to receive
/// `EntryInfo` objects one at a time as they are emitted. It implements
/// [`LsOutput::stream_mode`] as `StreamMode::Streaming`.
///
/// # Example
///
/// ```ignore
/// use uu_ls::{Config, list_with_output, StreamingOutput};
/// use std::path::Path;
///
/// let mut output = StreamingOutput::new();
/// list_with_output(vec![Path::new(".")], &config, &mut output)?;
///
/// for entry in output.entries() {
///     println!("{}: {} bytes",
///         entry.display_name.to_string_lossy(),
///         entry.size().unwrap_or(0));
/// }
/// ```
#[derive(Debug, Default)]
pub struct StreamingOutput {
    entries: Vec<EntryInfo>,
    directories: Vec<PathBuf>,
    totals: Vec<u64>,
}

impl StreamingOutput {
    /// Create a new empty streaming sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all collected entries
    pub fn entries(&self) -> &[EntryInfo] {
        &self.entries
    }

    /// Consume the collector and return all entries
    pub fn into_entries(self) -> Vec<EntryInfo> {
        self.entries
    }

    /// Get all directory headers that were encountered
    pub fn directories(&self) -> &[PathBuf] {
        &self.directories
    }

    /// Get all totals that were written
    pub fn totals(&self) -> &[u64] {
        &self.totals
    }

    /// Clear all collected data
    pub fn clear(&mut self) {
        self.entries.clear();
        self.directories.clear();
        self.totals.clear();
    }
}

impl LsOutput for StreamingOutput {
    fn stream_mode(&self) -> StreamMode {
        StreamMode::Streaming
    }

    fn write_entry(&mut self, entry: &EntryInfo) -> UResult<()> {
        self.entries.push(entry.clone());
        Ok(())
    }

    fn write_dir_header(
        &mut self,
        path_data: &PathData,
        _config: &Config,
        _is_first: bool,
    ) -> UResult<()> {
        self.directories.push(path_data.path().to_path_buf());
        Ok(())
    }

    fn write_total(&mut self, total_size: u64, _config: &Config) -> UResult<()> {
        self.totals.push(total_size);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_info_is_dir() {
        let entry = EntryInfo {
            path: PathBuf::from("/test/dir"),
            display_name: OsString::from("dir"),
            file_type: None,
            metadata: None,
            security_context: String::new(),
            command_line: false,
            must_dereference: false,
        };
        assert!(!entry.is_dir());
    }

    #[test]
    fn test_entry_info_size() {
        let entry = EntryInfo {
            path: PathBuf::from("/test/file"),
            display_name: OsString::from("file"),
            file_type: None,
            metadata: None,
            security_context: String::new(),
            command_line: false,
            must_dereference: false,
        };
        assert_eq!(entry.size(), None);
    }

    #[test]
    fn test_entry_info_file_name() {
        let entry = EntryInfo {
            path: PathBuf::from("/test/file.txt"),
            display_name: OsString::from("file.txt"),
            file_type: None,
            metadata: None,
            security_context: String::new(),
            command_line: false,
            must_dereference: false,
        };
        assert_eq!(entry.file_name(), Some("file.txt"));
    }

    #[test]
    fn test_streaming_output_new() {
        let collector = StreamingOutput::new();
        assert!(collector.entries().is_empty());
        assert!(collector.directories().is_empty());
        assert!(collector.totals().is_empty());
    }

    #[test]
    fn test_streaming_output_write_entry() {
        let mut collector = StreamingOutput::new();
        let entry = EntryInfo {
            path: PathBuf::from("/test/file"),
            display_name: OsString::from("file"),
            file_type: None,
            metadata: None,
            security_context: String::new(),
            command_line: false,
            must_dereference: false,
        };
        collector.write_entry(&entry).unwrap();
        assert_eq!(collector.entries().len(), 1);
        assert_eq!(collector.entries()[0].display_name, OsString::from("file"));
    }

    #[test]
    fn test_streaming_output_clear() {
        let mut collector = StreamingOutput::new();
        let entry = EntryInfo {
            path: PathBuf::from("/test/file"),
            display_name: OsString::from("file"),
            file_type: None,
            metadata: None,
            security_context: String::new(),
            command_line: false,
            must_dereference: false,
        };
        collector.write_entry(&entry).unwrap();

        collector.clear();
        assert!(collector.entries().is_empty());
        assert!(collector.directories().is_empty());
        assert!(collector.totals().is_empty());
    }

    #[test]
    fn test_streaming_output_into_entries() {
        let mut collector = StreamingOutput::new();
        let entry = EntryInfo {
            path: PathBuf::from("/test/file"),
            display_name: OsString::from("file"),
            file_type: None,
            metadata: None,
            security_context: String::new(),
            command_line: false,
            must_dereference: false,
        };
        collector.write_entry(&entry).unwrap();

        let entries = collector.into_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].display_name, OsString::from("file"));
    }

    #[test]
    fn test_streaming_output_flush() {
        let mut collector = StreamingOutput::new();
        assert!(collector.flush().is_ok());
    }
}
