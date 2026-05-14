// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Output traits and types for programmatic access to df functionality.
//!
//! This module separates filesystem discovery from output formatting so
//! consumers can use `df` without parsing text written to stdout.

use crate::{Filesystem, Options};
use uucore::error::UResult;

/// Streaming mode for `DfOutput` sinks.
///
/// `Batch` sinks receive all filesystems at once via
/// [`DfOutput::write_filesystems`]. `Streaming` sinks receive one filesystem at
/// a time via [`DfOutput::write_filesystem`].
pub enum StreamMode {
    Batch,
    Streaming,
}

/// Trait for receiving df filesystem entries.
///
/// Implement this trait to receive structured data from `df`. Text-formatting
/// sinks can use [`DfOutput::write_filesystems`] to format aligned tables, while
/// programmatic consumers can use [`DfOutput::write_filesystem`] to receive
/// entries one at a time.
pub trait DfOutput {
    /// Returns the preferred output mode for this sink.
    fn stream_mode(&self) -> StreamMode {
        StreamMode::Batch
    }

    /// Called for each filesystem entry in streaming mode.
    fn write_filesystem(&mut self, _filesystem: &Filesystem, _options: &Options) -> UResult<()> {
        Ok(())
    }

    /// Called with all filesystem entries in batch mode.
    fn write_filesystems(&mut self, filesystems: &[Filesystem], options: &Options) -> UResult<()> {
        for filesystem in filesystems {
            self.write_filesystem(filesystem, options)?;
        }
        Ok(())
    }

    /// Called to flush buffered output before diagnostics.
    fn flush(&mut self) -> UResult<()> {
        Ok(())
    }

    /// Called when all filesystems have been written.
    fn finalize(&mut self, _options: &Options) -> UResult<()> {
        Ok(())
    }

    /// Called before any filesystems are processed.
    fn initialize(&mut self, _options: &Options) -> UResult<()> {
        Ok(())
    }
}

/// A streaming sink that collects filesystem entries as they are emitted.
#[derive(Debug, Default)]
pub struct StreamingOutput {
    filesystems: Vec<Filesystem>,
}

impl StreamingOutput {
    /// Create a new empty streaming sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all collected filesystem entries.
    pub fn filesystems(&self) -> &[Filesystem] {
        &self.filesystems
    }

    /// Consume the collector and return all filesystem entries.
    pub fn into_filesystems(self) -> Vec<Filesystem> {
        self.filesystems
    }

    /// Clear all collected data.
    pub fn clear(&mut self) {
        self.filesystems.clear();
    }
}

impl DfOutput for StreamingOutput {
    fn stream_mode(&self) -> StreamMode {
        StreamMode::Streaming
    }

    fn write_filesystem(&mut self, filesystem: &Filesystem, _options: &Options) -> UResult<()> {
        self.filesystems.push(filesystem.clone());
        Ok(())
    }
}
