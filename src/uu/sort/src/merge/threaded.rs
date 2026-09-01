// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Multi-threaded merge: a reader thread feeds chunks into the per-file
//! channels while the main thread merges the next-line heap.

use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    io::Write,
    iter,
    rc::Rc,
    sync::mpsc::{Receiver, Sender, SyncSender, TryRecvError, channel, sync_channel},
    thread::{self, JoinHandle},
};

use uucore::error::{FromIo, UResult};

use crate::{
    GlobalSettings,
    chunks::{self, Chunk, RecycledChunk},
    compare_by,
};

use super::{MergeInput, PreviousLine};

pub(super) const SUPPORTS_COMPRESSION: bool = true;

/// Merge files without limiting how many files are concurrently open.
///
/// It is the responsibility of the caller to ensure that `files` yields only
/// as many files as we are allowed to open concurrently.
pub(super) fn merge_without_limit<M: MergeInput + 'static, F: Iterator<Item = UResult<M>>>(
    files: F,
    settings: &GlobalSettings,
) -> UResult<FileMerger<'_>> {
    let (request_sender, request_receiver) = channel();
    let mut reader_files = Vec::with_capacity(files.size_hint().0);
    let mut loaded_receivers = Vec::with_capacity(files.size_hint().0);
    for (file_number, file) in files.enumerate() {
        let (sender, receiver) = sync_channel(2);
        loaded_receivers.push(receiver);
        reader_files.push(Some(ReaderFile {
            file: file?,
            sender,
            carry_over: vec![],
        }));
        // Send the initial chunk to trigger a read for each file
        request_sender
            .send((file_number, RecycledChunk::new(8 * 1024)))
            .unwrap();
    }

    // Send the second chunk for each file
    for file_number in 0..reader_files.len() {
        request_sender
            .send((file_number, RecycledChunk::new(8 * 1024)))
            .unwrap();
    }

    let reader_join_handle = thread::spawn({
        let settings = settings.clone();
        move || {
            reader(
                &request_receiver,
                &mut reader_files,
                &settings,
                settings.line_ending.into(),
            )
        }
    });

    let mut mergeable_files = vec![];

    for (file_number, receiver) in loaded_receivers.into_iter().enumerate() {
        if let Ok(chunk) = receiver.recv() {
            mergeable_files.push(MergeableFile {
                current_chunk: Rc::new(chunk),
                file_number,
                line_idx: 0,
                receiver,
                settings,
            });
        }
    }

    Ok(FileMerger {
        heap: BinaryHeap::from(mergeable_files),
        request_sender,
        prev: None,
        reader_join_handle,
    })
}

/// The struct on the reader thread representing an input file
struct ReaderFile<M: MergeInput> {
    file: M,
    sender: SyncSender<Chunk>,
    carry_over: Vec<u8>,
}

/// The function running on the reader thread.
fn reader(
    recycled_receiver: &Receiver<(usize, RecycledChunk)>,
    files: &mut [Option<ReaderFile<impl MergeInput>>],
    settings: &GlobalSettings,
    separator: u8,
) -> UResult<()> {
    for (file_idx, recycled_chunk) in recycled_receiver {
        if let Some(ReaderFile {
            file,
            sender,
            carry_over,
        }) = &mut files[file_idx]
        {
            let should_continue = chunks::read(
                sender,
                recycled_chunk,
                None,
                carry_over,
                file.as_read(),
                &mut iter::empty(),
                separator,
                settings,
            )?;
            if !should_continue {
                // Remove the file from the list by replacing it with `None`.
                let ReaderFile { file, .. } = files[file_idx].take().unwrap();
                // Depending on the kind of the `MergeInput`, this may delete the file:
                file.finished_reading()?;
            }
        }
    }
    Ok(())
}

/// The struct on the main thread representing an input file
pub(super) struct MergeableFile<'a> {
    current_chunk: Rc<Chunk>,
    line_idx: usize,
    receiver: Receiver<Chunk>,
    file_number: usize,
    settings: &'a GlobalSettings,
}

impl PartialEq for MergeableFile<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for MergeableFile<'_> {}

impl PartialOrd for MergeableFile<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MergeableFile<'_> {
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

/// Merges files together. This is **not** an iterator because of lifetime problems.
pub(super) struct FileMerger<'a> {
    heap: BinaryHeap<MergeableFile<'a>>,
    request_sender: Sender<(usize, RecycledChunk)>,
    prev: Option<PreviousLine>,
    reader_join_handle: JoinHandle<UResult<()>>,
}

impl FileMerger<'_> {
    pub(super) fn write_all_to(
        mut self,
        settings: &GlobalSettings,
        out: &mut impl Write,
    ) -> UResult<()> {
        let write_result = loop {
            match self
                .write_next(out, settings)
                .map_err_context(|| "write failed".into())
            {
                Ok(true) => (),
                Ok(false) => break Ok(()),
                Err(error) => break Err(error),
            }
        };

        let Self {
            heap,
            request_sender,
            reader_join_handle,
            ..
        } = self;

        drop(request_sender);
        let mut files = heap.into_vec();
        while !files.is_empty() {
            files.retain(|file| {
                !matches!(file.receiver.try_recv(), Err(TryRecvError::Disconnected))
            });
            thread::yield_now();
        }

        let reader_result = reader_join_handle.join().unwrap();
        write_result.and(reader_result)
    }

    fn write_next(
        &mut self,
        writer: &mut impl Write,
        settings: &GlobalSettings,
    ) -> std::io::Result<bool> {
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

            let was_last_line_for_file = file.current_chunk.lines().len() == file.line_idx + 1;

            if was_last_line_for_file {
                if let Ok(next_chunk) = file.receiver.recv() {
                    let mut file = self.heap.peek_mut().unwrap();
                    file.current_chunk = Rc::new(next_chunk);
                    file.line_idx = 0;
                } else {
                    self.heap.pop();
                }
            } else {
                // This will cause the comparison to use a different line and the heap to readjust.
                self.heap.peek_mut().unwrap().line_idx += 1;
            }

            if let Some(prev) = prev
                && let Ok(prev_chunk) = Rc::try_unwrap(prev.chunk)
            {
                // If nothing is referencing the previous chunk anymore, this means that the previous line
                // was the last line of the chunk. We can recycle the chunk.
                self.request_sender
                    .send((prev.file_number, prev_chunk.recycle()))
                    .ok();
            }
        }
        Ok(!self.heap.is_empty())
    }
}
