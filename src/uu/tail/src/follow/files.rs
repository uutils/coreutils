// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore tailable seekable stdlib (stdlib)

use crate::args::Settings;
use crate::chunks::BytesChunkBuffer;
use crate::paths::{HeaderPrinter, PathExtTail};
use crate::text;
use std::collections::HashMap;
use std::collections::hash_map::Keys;
use std::fs::{File, Metadata};
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write, stdout};
use std::path::{Path, PathBuf};
use std::time::Instant;
use uucore::error::UResult;

/// Combined trait for readers that support both buffered reading and seeking.
/// This allows us to detect file growth after renames in polling mode.
pub trait BufReadSeek: BufRead + Seek + Send {}

/// Blanket implementation for any type that implements BufRead, Seek, and Send
impl<T: BufRead + Seek + Send> BufReadSeek for T {}

/// Wrapper for non-seekable readers (like stdin) that implements Seek as a no-op.
/// This allows stdin to work with the BufReadSeek trait without actual seeking capability.
pub struct NonSeekableReader<R: BufRead + Send> {
    inner: R,
}

impl<R: BufRead + Send> NonSeekableReader<R> {
    pub fn new(inner: R) -> Self {
        Self { inner }
    }
}

impl<R: BufRead + Send> Read for NonSeekableReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<R: BufRead + Send> BufRead for NonSeekableReader<R> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt);
    }
}

impl<R: BufRead + Send> Seek for NonSeekableReader<R> {
    fn seek(&mut self, _pos: SeekFrom) -> std::io::Result<u64> {
        // No-op for non-seekable readers like stdin
        Ok(0)
    }
}

/// Identifies the source of a file system event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchSource {
    /// Event originated from watching the file directly
    File,
    /// Event originated from watching the parent directory
    /// (only used in Linux inotify + --follow=name mode)
    ParentDirectory,
}

/// Tracks watch metadata for a monitored file
#[derive(Debug, Clone)]
pub struct WatchedPath {
    /// The file being monitored
    #[allow(dead_code)]
    pub file_path: PathBuf,
    /// Parent directory watch (if enabled)
    #[allow(dead_code)]
    pub parent_path: Option<PathBuf>,
}

/// Data structure to keep a handle on files to follow.
/// `last` always holds the path/key of the last file that was printed from.
/// The keys of the [`HashMap`] can point to an existing file path (normal case),
/// or stdin ("-"), or to a non-existing path (--retry).
/// For existing files, all keys in the [`HashMap`] are absolute Paths.
pub struct FileHandling {
    map: HashMap<PathBuf, (PathData, Option<WatchedPath>)>,
    last: Option<PathBuf>,
    header_printer: HeaderPrinter,
}

impl FileHandling {
    pub fn from(settings: &Settings) -> Self {
        Self {
            map: HashMap::with_capacity(settings.inputs.len()),
            last: None,
            header_printer: HeaderPrinter::new(settings.verbose, false),
        }
    }

    /// Wrapper for [`HashMap::insert`] using [`Path::canonicalize`]
    pub fn insert(&mut self, k: &Path, v: PathData, update_last: bool) {
        self.insert_with_watch(k, v, None, update_last);
    }

    /// Insert a file with optional watch metadata
    pub fn insert_with_watch(
        &mut self,
        k: &Path,
        v: PathData,
        watch_info: Option<WatchedPath>,
        update_last: bool,
    ) {
        let k = Self::canonicalize_path(k);
        if update_last {
            self.last = Some(k.clone());
        }
        let _ = self.map.insert(k, (v, watch_info));
    }

    /// Wrapper for [`HashMap::remove`] using [`Path::canonicalize`]
    /// If the canonicalized path is not found, tries all keys in the map to find a match.
    /// This handles cases where a file was renamed and can no longer be canonicalized.
    pub fn remove(&mut self, k: &Path) -> PathData {
        let canonicalized = Self::canonicalize_path(k);

        // Try canonicalized path first (fast path for existing files)
        if let Some(entry) = self.map.remove(&canonicalized) {
            return entry.0;
        }

        // Fallback for renamed files: try the raw key directly
        if let Some(entry) = self.map.remove(k) {
            return entry.0;
        }

        // Last resort: search through all keys to find one that matches when canonicalized
        // This handles the case where the file was tracked under its canonical path
        // but the event refers to it by the pre-rename name
        let matching_key = self
            .map
            .keys()
            .find(|key| {
                // Check if this key, when made relative to the same directory as k, matches k
                if let (Some(k_file), Some(key_file)) = (k.file_name(), key.file_name()) {
                    if k_file == key_file {
                        // If the file names match, check if they refer to the same logical file
                        return true;
                    }
                }
                false
            })
            .cloned();

        if let Some(key) = matching_key {
            return self.map.remove(&key).unwrap().0;
        }

        panic!("No path was found. about [{}]", k.display())
    }

    /// Wrapper for [`HashMap::get`] using [`Path::canonicalize`]
    pub fn get(&self, k: &Path) -> &PathData {
        &self.map.get(&Self::canonicalize_path(k)).unwrap().0
    }

    /// Wrapper for [`HashMap::get_mut`] using [`Path::canonicalize`]
    pub fn get_mut(&mut self, k: &Path) -> &mut PathData {
        &mut self.map.get_mut(&Self::canonicalize_path(k)).unwrap().0
    }

    /// Get watch metadata for a path
    #[allow(dead_code)]
    pub fn get_watch_info(&self, k: &Path) -> Option<&WatchedPath> {
        self.map
            .get(&Self::canonicalize_path(k))
            .and_then(|(_, watch)| watch.as_ref())
    }

    /// Canonicalize `path` if it is not already an absolute path
    fn canonicalize_path(path: &Path) -> PathBuf {
        if path.is_relative() && !path.is_stdin() {
            if let Ok(p) = path.canonicalize() {
                return p;
            }
        }
        path.to_owned()
    }

    pub fn get_mut_metadata(&mut self, path: &Path) -> Option<&Metadata> {
        self.get_mut(path).metadata.as_ref()
    }

    pub fn keys(&self) -> Keys<'_, PathBuf, (PathData, Option<WatchedPath>)> {
        self.map.keys()
    }

    pub fn contains_key(&self, k: &Path) -> bool {
        self.map.contains_key(&Self::canonicalize_path(k))
    }

    /// Return true if there is only stdin remaining
    pub fn only_stdin_remaining(&self) -> bool {
        self.map.len() == 1 && (self.map.contains_key(Path::new(text::DASH)))
    }

    /// Return true if there is at least one "tailable" path (or stdin) remaining
    pub fn files_remaining(&self) -> bool {
        for path in self.map.keys() {
            if path.is_tailable() || path.is_stdin() {
                return true;
            }
        }
        false
    }

    /// Returns true if there are no files remaining
    pub fn no_files_remaining(&self, settings: &Settings) -> bool {
        self.map.is_empty() || !self.files_remaining() && !settings.retry
    }

    /// Set `reader` to None to indicate that `path` is not an existing file anymore.
    pub fn reset_reader(&mut self, path: &Path) {
        self.get_mut(path).reader = None;
    }

    /// Reopen the file at the monitored `path`, or reset reader state if already open
    pub fn update_reader(&mut self, path: &Path) -> UResult<()> {
        let path_data = self.get_mut(path);

        if let Some(reader) = path_data.reader.as_mut() {
            // File is already open. In descriptor mode after rename, the path may not exist
            // but the FD is still valid. Seek to current position to clear any internal EOF state.
            if let Ok(pos) = reader.stream_position() {
                if reader.seek(SeekFrom::Start(pos)).is_ok() {
                    // Successfully cleared EOF state without changing position
                    return Ok(());
                }
            }
            // If seek failed, fall through to reopen
        }

        // No reader or seek failed, try to reopen file
        if let Ok(file) = File::open(path) {
            self.get_mut(path)
                .reader
                .replace(Box::new(BufReader::new(file)));
            Ok(())
        } else {
            // File doesn't exist (e.g., after rename in descriptor mode)
            // Keep the existing reader - it may still be valid
            Ok(())
        }
    }

    /// Reopen file and position at the last N lines/bytes (for truncate events)
    pub fn update_reader_with_positioning(&mut self, path: &Path, settings: &Settings) -> UResult<()> {
        // Close existing reader
        self.get_mut(path).reader = None;

        // Reopen file and position at end
        if let Ok(mut file) = File::open(path) {
            // Apply bounded_tail logic to position at last N lines/bytes
            super::super::bounded_tail(&mut file, settings);

            // Create buffered reader from positioned file
            self.get_mut(path)
                .reader
                .replace(Box::new(BufReader::new(file)));
            Ok(())
        } else {
            // File doesn't exist
            Ok(())
        }
    }

    /// Reload metadata from `path`, or `metadata`
    pub fn update_metadata(&mut self, path: &Path, metadata: Option<Metadata>) {
        self.get_mut(path).metadata = if metadata.is_some() {
            metadata
        } else {
            path.metadata().ok()
        };
    }

    /// Read new data from `path` and print it to stdout
    pub fn tail_file(&mut self, path: &Path, verbose: bool) -> UResult<bool> {
        let mut chunks = BytesChunkBuffer::new(u64::MAX);
        if let Some(reader) = self.get_mut(path).reader.as_mut() {
            chunks.fill(reader)?;
        }
        if chunks.has_data() {
            if self.needs_header(path, verbose) {
                let display_name = self.get(path).display_name.clone();
                self.header_printer.print(display_name.as_str());
            }

            let mut writer = BufWriter::new(stdout().lock());
            chunks.print(&mut writer)?;
            writer.flush()?;

            self.last.replace(path.to_owned());
            self.update_metadata(path, None);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Decide if printing `path` needs a header based on when it was last printed
    pub fn needs_header(&self, path: &Path, verbose: bool) -> bool {
        if verbose {
            if let Some(ref last) = self.last {
                !last.eq(&path)
            } else {
                true
            }
        } else {
            false
        }
    }

    /// Poll a single file descriptor for new data.
    /// Returns Ok(true) if new data was read and output.
    pub fn poll_fd(&mut self, path: &Path, verbose: bool) -> UResult<bool> {
        let path_data = self.get_mut(path);

        // Only poll if marked for fallback and is a regular file
        if !path_data.fallback_to_polling || !path_data.is_regular_file {
            return Ok(false);
        }

        // Throttle polling: minimum 50ms between polls
        let now = Instant::now();
        if let Some(last_polled) = path_data.last_polled {
            if now.duration_since(last_polled).as_millis() < 50 {
                return Ok(false);
            }
        }
        path_data.last_polled = Some(now);

        // After a rename, the path no longer exists on disk, but the file descriptor
        // is still valid. We can't use metadata to check file size, so we'll just
        // try to read from the FD. If there's data, we'll output it.

        // Check if we have a reader (file descriptor)
        if self.get(path).reader.is_none() {
            return Ok(false);
        }

        // Read and output new data (similar to tail_file)
        let mut chunks = BytesChunkBuffer::new(u64::MAX);

        if let Some(reader) = self.get_mut(path).reader.as_mut() {
            chunks.fill(reader)?;
        }

        if chunks.has_data() {
            if self.needs_header(path, verbose) {
                let display_name = self.get(path).display_name.clone();
                self.header_printer.print(display_name.as_str());
            }

            let mut writer = BufWriter::new(stdout().lock());
            chunks.print(&mut writer)?;
            writer.flush()?;

            self.last.replace(path.to_owned());
            self.update_metadata(path, None);

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Poll all file descriptors marked for polling fallback.
    /// Returns Ok(true) if any file made progress.
    pub fn poll_all_fds(&mut self, verbose: bool) -> UResult<bool> {
        let paths_to_poll: Vec<PathBuf> = self
            .map
            .iter()
            .filter(|(_, (data, _))| data.fallback_to_polling && data.is_regular_file)
            .map(|(path, _)| path.clone())
            .collect();

        let mut any_progress = false;
        for path in paths_to_poll {
            if self.poll_fd(&path, verbose)? {
                any_progress = true;
            }
        }

        Ok(any_progress)
    }

    /// Check if any files are marked for polling fallback.
    pub fn has_polling_fallback(&self) -> bool {
        self.map.values().any(|(data, _)| data.fallback_to_polling)
    }
}

/// Data structure to keep a handle on the [`BufReader`], [`Metadata`]
/// and the `display_name` (`header_name`) of files that are being followed.
pub struct PathData {
    pub reader: Option<Box<dyn BufReadSeek>>,
    pub metadata: Option<Metadata>,
    pub display_name: String,
    /// After a rename event in descriptor mode, switch to periodic FD polling
    pub fallback_to_polling: bool,
    /// Track when we last polled this FD to throttle polling frequency
    pub last_polled: Option<Instant>,
    /// Whether this is a regular file (skip polling for pipes/sockets)
    pub is_regular_file: bool,
}

impl PathData {
    pub fn new(
        reader: Option<Box<dyn BufReadSeek>>,
        metadata: Option<Metadata>,
        display_name: &str,
    ) -> Self {
        let is_regular_file = metadata.as_ref().map(|m| m.is_file()).unwrap_or(false);

        Self {
            reader,
            metadata,
            display_name: display_name.to_owned(),
            fallback_to_polling: false,
            last_polled: None,
            is_regular_file,
        }
    }
}
