// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Synchronous merge for targets without thread support (e.g. wasm32-wasip1).
//!
//! Reads chunks on demand from each input on the calling thread instead of
//! using a dedicated reader thread.

use std::{cmp::Ordering, collections::BinaryHeap, io::Write, iter, rc::Rc};

use uucore::error::UResult;

use crate::{
    GlobalSettings,
    chunks::{self, Chunk, RecycledChunk},
    compare_by,
};

use super::{MergeInput, PreviousLine};

pub(super) const SUPPORTS_COMPRESSION: bool = false;

struct SyncReaderFile<M: MergeInput> {
    file: M,
    carry_over: Vec<u8>,
}

struct SyncMergeableFile<'a> {
    current_chunk: Rc<Chunk>,
    line_idx: usize,
    file_number: usize,
    settings: &'a GlobalSettings,
}

impl PartialEq for SyncMergeableFile<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for SyncMergeableFile<'_> {}

impl PartialOrd for SyncMergeableFile<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SyncMergeableFile<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        let mut cmp = compare_by(
            &self.current_chunk.lines()[self.line_idx],
            &other.current_chunk.lines()[other.line_idx],
            self.settings,
            self.current_chunk.line_data(),
            other.current_chunk.line_data(),
        );
        if cmp == Ordering::Equal {
            cmp = self.file_number.cmp(&other.file_number);
        }
        cmp.reverse()
    }
}

pub(super) struct SyncFileMerger<'a, M: MergeInput> {
    heap: BinaryHeap<SyncMergeableFile<'a>>,
    readers: Vec<Option<SyncReaderFile<M>>>,
    prev: Option<PreviousLine>,
    recycled: Option<RecycledChunk>,
    settings: &'a GlobalSettings,
}

impl<M: MergeInput> SyncFileMerger<'_, M> {
    pub(super) fn write_all_to(
        mut self,
        settings: &GlobalSettings,
        out: &mut impl Write,
    ) -> UResult<()> {
        let write_result = loop {
            match self.write_next(out, settings) {
                Ok(true) => {}
                Ok(false) => break Ok(()),
                Err(error) => break Err(error),
            }
        };

        // Readers finished during the merge are replaced with `None`; clean up
        // any that remain after an early write failure.
        let cleanup_result = self
            .readers
            .into_iter()
            .flatten()
            .try_for_each(|reader| reader.file.finished_reading());
        write_result.and(cleanup_result)
    }

    fn write_next(&mut self, writer: &mut impl Write, settings: &GlobalSettings) -> UResult<bool> {
        if let Some(file) = self.heap.peek() {
            let prev = self.prev.replace(PreviousLine {
                chunk: file.current_chunk.clone(),
                line_idx: file.line_idx,
                file_number: file.file_number,
            });

            file.current_chunk.with_dependent(|_, contents| {
                let current_line = &contents.lines[file.line_idx];
                if settings.unique
                    && let Some(prev) = &prev
                {
                    let cmp = compare_by(
                        &prev.chunk.lines()[prev.line_idx],
                        current_line,
                        settings,
                        prev.chunk.line_data(),
                        file.current_chunk.line_data(),
                    );
                    if cmp == Ordering::Equal {
                        return Ok(());
                    }
                }
                current_line.write(writer, settings)
            })?;

            let was_last = file.current_chunk.lines().len() == file.line_idx + 1;
            let file_number = file.file_number;

            if was_last {
                let separator = self.settings.line_ending.into();
                let recycled = self
                    .recycled
                    .take()
                    .unwrap_or_else(|| RecycledChunk::new(8 * 1024));
                let next_chunk = if let Some(reader) = self.readers[file_number].as_mut() {
                    let (chunk, should_continue) = chunks::read_to_chunk(
                        recycled,
                        None,
                        &mut reader.carry_over,
                        reader.file.as_read(),
                        &mut iter::empty(),
                        separator,
                        self.settings,
                    )?;
                    if !should_continue && let Some(reader) = self.readers[file_number].take() {
                        reader.file.finished_reading()?;
                    }
                    chunk
                } else {
                    None
                };

                if let Some(next_chunk) = next_chunk {
                    let mut file = self.heap.peek_mut().unwrap();
                    file.current_chunk = Rc::new(next_chunk);
                    file.line_idx = 0;
                } else {
                    self.heap.pop();
                }
            } else {
                self.heap.peek_mut().unwrap().line_idx += 1;
            }

            // Recycle the previous chunk if no other reference holds it.
            if let Some(prev) = prev
                && let Ok(chunk) = Rc::try_unwrap(prev.chunk)
            {
                self.recycled = Some(chunk.recycle());
            }
        }
        Ok(!self.heap.is_empty())
    }
}

pub(super) fn merge_without_limit<M: MergeInput + 'static, F: Iterator<Item = UResult<M>>>(
    files: F,
    settings: &GlobalSettings,
) -> UResult<SyncFileMerger<'_, M>> {
    let separator = settings.line_ending.into();
    let mut readers: Vec<Option<SyncReaderFile<M>>> = Vec::new();
    let mut mergeable_files = Vec::new();

    for (file_number, file) in files.enumerate() {
        let mut reader = SyncReaderFile {
            file: file?,
            carry_over: vec![],
        };
        let recycled = RecycledChunk::new(8 * 1024);
        let (chunk, should_continue) = chunks::read_to_chunk(
            recycled,
            None,
            &mut reader.carry_over,
            reader.file.as_read(),
            &mut iter::empty(),
            separator,
            settings,
        )?;

        if let Some(chunk) = chunk {
            mergeable_files.push(SyncMergeableFile {
                current_chunk: Rc::new(chunk),
                line_idx: 0,
                file_number,
                settings,
            });
            if should_continue {
                readers.push(Some(reader));
            } else {
                reader.file.finished_reading()?;
                readers.push(None);
            }
        } else {
            reader.file.finished_reading()?;
            readers.push(None);
        }
    }

    Ok(SyncFileMerger {
        heap: BinaryHeap::from(mergeable_files),
        readers,
        prev: None,
        recycled: None,
        settings,
    })
}
