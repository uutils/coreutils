// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Merge already sorted files.
//!
//! We achieve performance by splitting the tasks of sorting and writing, and reading and parsing between two threads.
//! The threads communicate over channels. There's one channel per file in the direction reader -> sorter, but only
//! one channel from the sorter back to the reader. The channels to the sorter are used to send the read chunks.
//! The sorter reads the next chunk from the channel whenever it needs the next chunk after running out of lines
//! from the previous read of the file. The channel back from the sorter to the reader has two purposes: To allow the reader
//! to reuse memory allocations and to tell the reader which file to read from next.

use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, BufWriter, Read, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    rc::Rc,
};
#[cfg(not(target_os = "wasi"))]
use std::{
    iter,
    sync::mpsc::{Receiver, Sender, SyncSender, TryRecvError, channel, sync_channel},
    thread::{self, JoinHandle},
};

use uucore::error::{FromIo, UResult};
use uucore::fs::FileInformation;

use crate::{
    GlobalSettings, Output, SortError,
    chunks::{self, Chunk, RecycledChunk},
    current_open_fd_count, fd_soft_limit, merge_compare, open,
    tmp_dir::{TmpDirWrapper, TmpPath},
};

/// If the output file occurs in the input files as well, copy the contents of the output file
/// and replace its occurrences in the inputs with that copy, which is returned.
fn replace_output_file_in_input_files(
    files: &mut [OsString],
    output: Option<&OsStr>,
    tmp_dir: &mut TmpDirWrapper,
) -> UResult<Option<TmpPath>> {
    let mut copy: Option<TmpPath> = None;
    if let Some(output_path) = output.map(Path::new)
        && let Ok(output_info) = FileInformation::from_path(output_path, true)
    {
        for file in files {
            // Different hard links have different canonical paths but still
            // share the file that will be truncated when output begins.
            if FileInformation::from_path(Path::new(file), true)
                .is_ok_and(|input_info| input_info == output_info)
            {
                if let Some(copy) = &copy {
                    *file = copy.path().into();
                } else {
                    // Write through the descriptor `next_file` just opened rather
                    // than `fs::copy`, which would put the source's permission bits
                    // on the temporary file and undo the 0600 it was created with.
                    let (mut copy_file, copy_path) = tmp_dir.next_file()?;
                    let mut source = File::open(output_path)
                        .map_err(|error| SortError::OpenTmpFileFailed { error })?;
                    io::copy(&mut source, &mut copy_file)
                        .map_err(|error| SortError::OpenTmpFileFailed { error })?;
                    *file = copy_path.path().into();
                    copy = Some(copy_path);
                }
            }
        }
    }
    Ok(copy)
}

/// Open the merge input `file`. `copy` is the copy of the output file returned by
/// [`replace_output_file_in_input_files`].
fn open_input(file: &OsStr, copy: Option<&TmpPath>) -> UResult<Box<dyn Read + Send>> {
    match copy {
        Some(copy) if Path::new(file) == copy.path() => Ok(Box::new(
            copy.open()
                .map_err(|error| SortError::OpenTmpFileFailed { error })?,
        )),
        _ => open(file),
    }
}

/// Determine the effective merge batch size, enforcing a minimum and respecting the
/// file-descriptor soft limit after reserving stdio/output and a safety margin.
fn effective_merge_batch_size(settings: &GlobalSettings) -> usize {
    const MIN_BATCH_SIZE: usize = 2;
    const RESERVED_TMP_OUTPUT: usize = 1;
    const RESERVED_CTRL_C: usize = 2;
    const RESERVED_RANDOM_SOURCE: usize = 1;
    const SAFETY_MARGIN: usize = 1;
    let mut batch_size = settings.merge_batch_size.max(MIN_BATCH_SIZE);

    if let Some(limit) = fd_soft_limit() {
        let open_fds = current_open_fd_count().unwrap_or(3);
        let mut reserved = RESERVED_TMP_OUTPUT + RESERVED_CTRL_C + SAFETY_MARGIN;
        if settings.salt.is_some() {
            reserved = reserved.saturating_add(RESERVED_RANDOM_SOURCE);
        }
        let available_inputs = limit.saturating_sub(open_fds.saturating_add(reserved));
        if available_inputs >= MIN_BATCH_SIZE {
            batch_size = batch_size.min(available_inputs);
        } else {
            batch_size = MIN_BATCH_SIZE;
        }
    }

    batch_size
}

/// Merge pre-sorted `Box<dyn Read>`s.
///
/// If `settings.merge_batch_size` is greater than the length of `files`, intermediate files will be used.
/// If `settings.compress_prog` is `Some`, intermediate files will be compressed with it.
pub fn merge(
    files: &mut [OsString],
    settings: &GlobalSettings,
    output: Output,
    tmp_dir: &mut TmpDirWrapper,
) -> UResult<()> {
    let copy = replace_output_file_in_input_files(files, output.as_output_name(), tmp_dir)?;
    let files = files
        .iter()
        .map(|file| open_input(file, copy.as_ref()).map(|inner| PlainMergeInput { inner }));
    if settings.compress_prog.is_none() {
        merge_with_file_limit::<_, _, WriteablePlainTmpFile>(files, settings, output, tmp_dir)
    } else {
        merge_with_file_limit::<_, _, WriteableCompressedTmpFile>(files, settings, output, tmp_dir)
    }
}

// Merge already sorted `MergeInput`s.
pub fn merge_with_file_limit<
    M: MergeInput + 'static,
    F: ExactSizeIterator<Item = UResult<M>>,
    Tmp: WriteableTmpFile + 'static,
>(
    mut files: F,
    settings: &GlobalSettings,
    output: Output,
    tmp_dir: &mut TmpDirWrapper,
) -> UResult<()> {
    let batch_size = effective_merge_batch_size(settings);
    debug_assert!(batch_size >= 2);

    if files.len() <= batch_size {
        let merger = merge_without_limit(files, settings);
        merger?.write_all(settings, output)
    } else {
        let mut temporary_files = vec![];
        while files.len() > 0 {
            // Create the output before opening this batch's inputs, so that
            // creating it doesn't compete with them for file descriptors.
            let mut tmp_file =
                Tmp::create(tmp_dir.next_file()?, settings.compress_prog.as_deref())?;
            let merger = merge_without_limit(files.by_ref().take(batch_size), settings)?;
            merger.write_all_to(settings, tmp_file.as_write(), || "write failed".into())?;
            temporary_files.push(tmp_file.finished_writing()?);
        }
        merge_with_file_limit::<_, _, Tmp>(
            temporary_files
                .into_iter()
                .map(Box::new(|c: Tmp::Closed| c.reopen())
                    as Box<
                        dyn FnMut(Tmp::Closed) -> UResult<<Tmp::Closed as ClosedTmpFile>::Reopened>,
                    >),
            settings,
            output,
            tmp_dir,
        )
    }
}

/// Merge files without limiting how many files are concurrently open.
///
/// It is the responsibility of the caller to ensure that `files` yields only
/// as many files as we are allowed to open concurrently.
#[cfg(not(target_os = "wasi"))]
fn merge_without_limit<M: MergeInput + 'static, F: Iterator<Item = UResult<M>>>(
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
        // The merge comparator (`merge_compare`) compares whole-line locale keys lazily
        // with the ICU collator, so the reader does not need to precompute per-line sort
        // keys. Disabling `fast_locale_collation` here turns `Line::create` into a no-op
        // for that (whole-line, default) mode and avoids the dominant cost of merging
        // already-sorted input. Other modes are unaffected (the flag is already false).
        let mut settings = settings.clone();
        settings.precomputed.fast_locale_collation = false;
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
#[cfg(not(target_os = "wasi"))]
struct ReaderFile<M: MergeInput> {
    file: M,
    sender: SyncSender<Chunk>,
    carry_over: Vec<u8>,
}

/// The function running on the reader thread.
#[cfg(not(target_os = "wasi"))]
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
/// The struct representing an input file.
///
/// WASI has no threads, so chunks are read synchronously, right when needed,
/// instead of by a background reader thread; there is no `receiver` to read
/// the next chunk from.
pub struct MergeableFile<'a> {
    current_chunk: Rc<Chunk>,
    line_idx: usize,
    #[cfg(not(target_os = "wasi"))]
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
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MergeableFile<'_> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        let mut cmp = merge_compare(
            &self.current_chunk.lines()[self.line_idx],
            &other.current_chunk.lines()[other.line_idx],
            self.settings,
            self.current_chunk.line_data(),
            other.current_chunk.line_data(),
        );
        if cmp == Ordering::Equal {
            // To make sorting stable, we need to consider the file number as well,
            // as lines from a file with a lower number are to be considered "earlier".
            cmp = self.file_number.cmp(&other.file_number);
        }
        // BinaryHeap is a max heap. We use it as a min heap, so we need to reverse the ordering.
        cmp.reverse()
    }
}

/// A struct to keep track of the previous line we encountered.
///
/// This is required for deduplication purposes.
struct PreviousLine {
    chunk: Rc<Chunk>,
    line_idx: usize,
    // Used to recycle the chunk back to the right file once nothing references it
    // anymore: to the reader thread's channel on non-WASI, directly onto the file's
    // `ReaderFile` on WASI (there is no reader thread to send it to).
    file_number: usize,
}

/// What went wrong writing the next line in [`write_next_line`].
enum WriteNextError {
    /// Writing the line to the output failed.
    Write(io::Error),
    /// Reading the next chunk for a file whose current one ran out failed.
    Chunk(Box<dyn uucore::error::UError>),
}

/// Drive `write_next` to exhaustion, the shared part of both [`FileMerger::write_all_to`] impls.
fn drain_write_next(
    mut write_next: impl FnMut() -> Result<bool, WriteNextError>,
    ctx: impl FnOnce() -> String,
) -> UResult<()> {
    loop {
        match write_next() {
            Ok(true) => (),
            Ok(false) => return Ok(()),
            Err(WriteNextError::Write(error)) => return Err(error).map_err_context(ctx),
            // A chunk read failure already has its own context; wrapping it as a write
            // failure here would be misleading.
            Err(WriteNextError::Chunk(error)) => return Err(error),
        }
    }
}

/// How a [`FileMerger`] gets a file's next chunk once its current one runs out, and recycles
/// a chunk once nothing references it anymore.
///
/// The threaded merger fetches chunks from its reader thread and recycles them back to it
/// over a channel; WASI, which has no threads, reads the next chunk directly and recycles
/// onto the file itself. [`write_next_line`] is the rest of merging, shared between them.
trait ChunkSource {
    fn advance(&mut self, file: &mut MergeableFile<'_>) -> UResult<Option<Chunk>>;
    fn recycle(&mut self, file_number: usize, chunk: Chunk);
}

/// Write one line from the file at the front of `heap` to `writer`, and advance that file,
/// using `source` to fetch its next chunk once the current one runs out.
fn write_next_line(
    heap: &mut BinaryHeap<MergeableFile<'_>>,
    prev: &mut Option<PreviousLine>,
    writer: &mut impl Write,
    settings: &GlobalSettings,
    source: &mut impl ChunkSource,
) -> Result<bool, WriteNextError> {
    let Some(mut file) = heap.peek_mut() else {
        return Ok(false);
    };

    let old_prev = prev.replace(PreviousLine {
        chunk: file.current_chunk.clone(),
        line_idx: file.line_idx,
        file_number: file.file_number,
    });

    file.current_chunk
        .with_dependent(|_, contents| {
            let current_line = &contents.lines[file.line_idx];
            if settings.unique
                && let Some(prev) = &old_prev
            {
                let cmp = merge_compare(
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
        })
        .map_err(WriteNextError::Write)?;

    let was_last_line_for_file = file.current_chunk.lines().len() == file.line_idx + 1;

    if was_last_line_for_file {
        let next_chunk = source.advance(&mut file).map_err(WriteNextError::Chunk)?;
        if let Some(next_chunk) = next_chunk {
            file.current_chunk = Rc::new(next_chunk);
            file.line_idx = 0;
        } else {
            std::collections::binary_heap::PeekMut::pop(file);
        }
    } else {
        // This will cause the comparison to use a different line and the heap to readjust.
        file.line_idx += 1;
    }

    if let Some(prev) = old_prev
        && let Ok(prev_chunk) = Rc::try_unwrap(prev.chunk)
    {
        // If nothing is referencing the previous chunk anymore, this means that the previous
        // line was the last line of the chunk. We can recycle the chunk.
        source.recycle(prev.file_number, prev_chunk);
    }

    Ok(true)
}

/// Merges files together. This is **not** an iterator because of lifetime problems.
#[cfg(not(target_os = "wasi"))]
struct FileMerger<'a> {
    heap: BinaryHeap<MergeableFile<'a>>,
    request_sender: Sender<(usize, RecycledChunk)>,
    prev: Option<PreviousLine>,
    reader_join_handle: JoinHandle<UResult<()>>,
}

#[cfg(not(target_os = "wasi"))]
impl FileMerger<'_> {
    /// Write the merged contents to the output file.
    fn write_all(self, settings: &GlobalSettings, output: Output) -> UResult<()> {
        let ctx = output.write_failed_context();
        let mut out = output.into_write()?;
        self.write_all_to(settings, &mut out, &ctx)?;
        out.flush().map_err_context(ctx)
    }

    /// Write the merged contents to `out`, reporting write errors with `ctx`.
    fn write_all_to(
        mut self,
        settings: &GlobalSettings,
        out: &mut impl Write,
        ctx: impl FnOnce() -> String,
    ) -> UResult<()> {
        // Don't return as soon as `drain_write_next` does: we still have to shut the
        // reader thread down in an orderly fashion below. Returning here would drop our
        // receivers while the reader is still sending, and `chunks::read` unwraps that send.
        let write_result = drain_write_next(|| self.write_next(out, settings), ctx);

        let Self {
            heap,
            request_sender,
            reader_join_handle,
            ..
        } = self;

        // Stop asking for chunks, so the reader runs out of work and returns.
        drop(request_sender);
        // Until it does, keep draining the files it might still be sending to. We have to
        // poll all of them in turn rather than draining one at a time: the reader can be
        // blocked on any one channel, and blocking on a different one would deadlock.
        let mut files = heap.into_vec();
        while !files.is_empty() {
            files.retain(|file| {
                !matches!(file.receiver.try_recv(), Err(TryRecvError::Disconnected))
            });
            thread::yield_now();
        }

        let reader_result = reader_join_handle.join().unwrap();
        // A write failure is what the user needs to hear about; the reader hitting an error
        // on the way down is secondary.
        write_result.and(reader_result)
    }

    fn write_next(
        &mut self,
        writer: &mut impl Write,
        settings: &GlobalSettings,
    ) -> Result<bool, WriteNextError> {
        let Self {
            heap,
            request_sender,
            prev,
            ..
        } = self;
        let mut source = ThreadedChunkSource { request_sender };
        write_next_line(heap, prev, writer, settings, &mut source)
    }
}

/// Fetches chunks from the reader thread and recycles them back to it over a channel.
#[cfg(not(target_os = "wasi"))]
struct ThreadedChunkSource<'a> {
    request_sender: &'a Sender<(usize, RecycledChunk)>,
}

#[cfg(not(target_os = "wasi"))]
impl ChunkSource for ThreadedChunkSource<'_> {
    fn advance(&mut self, file: &mut MergeableFile<'_>) -> UResult<Option<Chunk>> {
        Ok(file.receiver.recv().ok())
    }

    fn recycle(&mut self, file_number: usize, chunk: Chunk) {
        self.request_sender
            .send((file_number, chunk.recycle()))
            .ok();
    }
}

/// Merge files without limiting how many files are concurrently open.
///
/// WASI has no threads, so chunks are read synchronously, right when the
/// merger needs them, instead of by a background reader thread.
///
/// It is the responsibility of the caller to ensure that `files` yields only
/// as many files as we are allowed to open concurrently.
#[cfg(target_os = "wasi")]
fn merge_without_limit<M: MergeInput + 'static, F: Iterator<Item = UResult<M>>>(
    files: F,
    settings: &GlobalSettings,
) -> UResult<FileMerger<'_, M>> {
    let separator = settings.line_ending.into();
    let mut reader_files = Vec::with_capacity(files.size_hint().0);
    for file in files {
        reader_files.push(Some(ReaderFile {
            file: file?,
            carry_over: vec![],
            finished: false,
            recycled: None,
        }));
    }

    // The merge comparator (`merge_compare`) compares whole-line locale keys lazily
    // with the ICU collator, so chunks don't need precomputed per-line sort keys.
    // Disabling `fast_locale_collation` here turns `Line::create` into a no-op for
    // that (whole-line, default) mode when reading, same as the threaded merger.
    let mut read_settings = settings.clone();
    read_settings.precomputed.fast_locale_collation = false;

    let mut mergeable_files = vec![];
    for (file_number, reader_file) in reader_files.iter_mut().enumerate() {
        let chunk = read_next_chunk(reader_file.as_mut().unwrap(), &read_settings, separator)?;
        if let Some(chunk) = chunk {
            mergeable_files.push(MergeableFile {
                current_chunk: Rc::new(chunk),
                file_number,
                line_idx: 0,
                settings,
            });
        } else {
            let ReaderFile { file, .. } = reader_file.take().unwrap();
            file.finished_reading()?;
        }
    }

    Ok(FileMerger {
        heap: BinaryHeap::from(mergeable_files),
        reader_files,
        prev: None,
        separator,
        read_settings,
    })
}

/// The struct representing an input file, read synchronously on demand.
#[cfg(target_os = "wasi")]
struct ReaderFile<M: MergeInput> {
    file: M,
    carry_over: Vec<u8>,
    /// Set once there is nothing left to read; stops a stale `carry_over` tail from being
    /// misparsed as a spurious extra chunk.
    finished: bool,
    /// A finished chunk's buffer, recycled here instead of to a reader thread, for reuse
    /// on this file's next read.
    recycled: Option<RecycledChunk>,
}

/// Reads the next chunk of `reader_file`, or `None` at end of file.
#[cfg(target_os = "wasi")]
fn read_next_chunk<M: MergeInput>(
    reader_file: &mut ReaderFile<M>,
    settings: &GlobalSettings,
    separator: u8,
) -> UResult<Option<Chunk>> {
    if reader_file.finished {
        return Ok(None);
    }
    let recycled_chunk = reader_file
        .recycled
        .take()
        .unwrap_or_else(|| RecycledChunk::new(8 * 1024));
    let (chunk, should_continue) = chunks::read_once(
        recycled_chunk,
        &mut reader_file.carry_over,
        reader_file.file.as_read(),
        separator,
        settings,
    )?;
    reader_file.finished = !should_continue;
    Ok(chunk)
}

/// Merges files together. This is **not** an iterator because of lifetime problems.
///
/// WASI has no threads, so there is no reader thread to request chunks from;
/// `reader_files` holds each input directly, read from on demand.
#[cfg(target_os = "wasi")]
struct FileMerger<'a, M: MergeInput> {
    heap: BinaryHeap<MergeableFile<'a>>,
    reader_files: Vec<Option<ReaderFile<M>>>,
    prev: Option<PreviousLine>,
    separator: u8,
    /// A clone of the real settings with `fast_locale_collation` forced off, used only for
    /// reading chunks. See the comment in `merge_without_limit`.
    read_settings: GlobalSettings,
}

#[cfg(target_os = "wasi")]
impl<M: MergeInput> FileMerger<'_, M> {
    /// Write the merged contents to the output file.
    fn write_all(self, settings: &GlobalSettings, output: Output) -> UResult<()> {
        let ctx = output.write_failed_context();
        let mut out = output.into_write()?;
        self.write_all_to(settings, &mut out, &ctx)?;
        out.flush().map_err_context(ctx)
    }

    /// Write the merged contents to `out`, reporting write errors with `ctx`.
    fn write_all_to(
        mut self,
        settings: &GlobalSettings,
        out: &mut impl Write,
        ctx: impl FnOnce() -> String,
    ) -> UResult<()> {
        drain_write_next(|| self.write_next(out, settings), ctx)
    }

    fn write_next(
        &mut self,
        writer: &mut impl Write,
        settings: &GlobalSettings,
    ) -> Result<bool, WriteNextError> {
        let Self {
            heap,
            reader_files,
            prev,
            separator,
            read_settings,
        } = self;
        let mut source = WasiChunkSource {
            reader_files,
            settings: read_settings,
            separator: *separator,
        };
        write_next_line(heap, prev, writer, settings, &mut source)
    }
}

/// Reads chunks synchronously, right when needed, and recycles them onto the file itself:
/// there is no reader thread to fetch from or send a recycled chunk back to.
#[cfg(target_os = "wasi")]
struct WasiChunkSource<'a, M: MergeInput> {
    reader_files: &'a mut Vec<Option<ReaderFile<M>>>,
    settings: &'a GlobalSettings,
    separator: u8,
}

#[cfg(target_os = "wasi")]
impl<M: MergeInput> ChunkSource for WasiChunkSource<'_, M> {
    fn advance(&mut self, file: &mut MergeableFile<'_>) -> UResult<Option<Chunk>> {
        let file_number = file.file_number;
        let reader_file = self.reader_files[file_number].as_mut().unwrap();
        let next_chunk = read_next_chunk(reader_file, self.settings, self.separator)?;
        if next_chunk.is_none() {
            let ReaderFile { file, .. } = self.reader_files[file_number].take().unwrap();
            file.finished_reading()?;
        }
        Ok(next_chunk)
    }

    fn recycle(&mut self, file_number: usize, chunk: Chunk) {
        if let Some(reader_file) = self.reader_files[file_number].as_mut() {
            // If nothing is referencing the previous chunk anymore, this means that the
            // previous line was the last line of the chunk. We can recycle the chunk for
            // this file's next read.
            reader_file.recycled = Some(chunk.recycle());
        }
    }
}

/// Wait for the child to exit and check its exit code.
fn check_child_success(mut child: Child, program: &str) -> UResult<()> {
    if matches!(child.wait().map(|e| e.code()), Ok(Some(0) | None) | Err(_)) {
        Ok(())
    } else {
        Err(SortError::CompressProgTerminatedAbnormally {
            prog: program.to_owned(),
        }
        .into())
    }
}

/// A temporary file that can be written to.
pub trait WriteableTmpFile: Sized {
    type Closed: ClosedTmpFile;
    type InnerWrite: Write;
    fn create(file: (File, TmpPath), compress_prog: Option<&str>) -> UResult<Self>;
    /// Closes the temporary file.
    fn finished_writing(self) -> UResult<Self::Closed>;
    fn as_write(&mut self) -> &mut Self::InnerWrite;
}
/// A temporary file that is (temporarily) closed, but can be reopened.
pub trait ClosedTmpFile {
    type Reopened: MergeInput;
    /// Reopens the temporary file.
    fn reopen(self) -> UResult<Self::Reopened>;
}
/// A pre-sorted input for merging.
pub trait MergeInput: Send {
    type InnerRead: Read;
    /// Cleans this `MergeInput` up.
    /// Implementations may delete the backing file.
    fn finished_reading(self) -> UResult<()>;
    fn as_read(&mut self) -> &mut Self::InnerRead;
}

pub struct WriteablePlainTmpFile {
    path: TmpPath,
    file: BufWriter<File>,
}
pub struct ClosedPlainTmpFile {
    path: TmpPath,
}
pub struct PlainTmpMergeInput {
    path: TmpPath,
    file: File,
}
impl WriteableTmpFile for WriteablePlainTmpFile {
    type Closed = ClosedPlainTmpFile;
    type InnerWrite = BufWriter<File>;

    fn create((file, path): (File, TmpPath), _: Option<&str>) -> UResult<Self> {
        Ok(Self {
            file: BufWriter::new(file),
            path,
        })
    }

    fn finished_writing(self) -> UResult<Self::Closed> {
        Ok(ClosedPlainTmpFile { path: self.path })
    }

    fn as_write(&mut self) -> &mut Self::InnerWrite {
        &mut self.file
    }
}
impl ClosedTmpFile for ClosedPlainTmpFile {
    type Reopened = PlainTmpMergeInput;
    fn reopen(self) -> UResult<Self::Reopened> {
        Ok(PlainTmpMergeInput {
            file: self
                .path
                .open()
                .map_err(|error| SortError::OpenTmpFileFailed { error })?,
            path: self.path,
        })
    }
}
impl MergeInput for PlainTmpMergeInput {
    type InnerRead = File;

    fn finished_reading(self) -> UResult<()> {
        // we ignore failures to delete the temporary file,
        // because there is a race at the end of the execution and the whole
        // temporary directory might already be gone.
        // Close the file first: removing it briefly takes another descriptor.
        drop(self.file);
        self.path.remove();
        Ok(())
    }

    fn as_read(&mut self) -> &mut Self::InnerRead {
        &mut self.file
    }
}

pub struct WriteableCompressedTmpFile {
    path: TmpPath,
    compress_prog: String,
    child: Child,
    child_stdin: BufWriter<ChildStdin>,
}
pub struct ClosedCompressedTmpFile {
    path: TmpPath,
    compress_prog: String,
}
pub struct CompressedTmpMergeInput {
    path: TmpPath,
    compress_prog: String,
    child: Child,
    child_stdout: ChildStdout,
}
impl WriteableTmpFile for WriteableCompressedTmpFile {
    type Closed = ClosedCompressedTmpFile;
    type InnerWrite = BufWriter<ChildStdin>;

    fn create((file, path): (File, TmpPath), compress_prog: Option<&str>) -> UResult<Self> {
        let compress_prog = compress_prog.unwrap();
        let mut command = Command::new(compress_prog);
        command.stdin(Stdio::piped()).stdout(file);
        let mut child = command
            .spawn()
            .map_err(|err| SortError::CompressProgExecutionFailed {
                prog: compress_prog.to_owned(),
                error: err,
            })?;
        let child_stdin = child.stdin.take().unwrap();
        Ok(Self {
            path,
            compress_prog: compress_prog.to_owned(),
            child,
            child_stdin: BufWriter::new(child_stdin),
        })
    }

    fn finished_writing(self) -> UResult<Self::Closed> {
        drop(self.child_stdin);
        check_child_success(self.child, &self.compress_prog)?;
        Ok(ClosedCompressedTmpFile {
            path: self.path,
            compress_prog: self.compress_prog,
        })
    }

    fn as_write(&mut self) -> &mut Self::InnerWrite {
        &mut self.child_stdin
    }
}
impl ClosedTmpFile for ClosedCompressedTmpFile {
    type Reopened = CompressedTmpMergeInput;

    fn reopen(self) -> UResult<Self::Reopened> {
        let mut command = Command::new(&self.compress_prog);
        // mirroring what is done for ClosedPlainTmpFile
        let file = self
            .path
            .open()
            .map_err(|error| SortError::OpenTmpFileFailed { error })?;
        command.stdin(file).stdout(Stdio::piped()).arg("-d");
        let mut child = command
            .spawn()
            .map_err(|err| SortError::CompressProgExecutionFailed {
                prog: self.compress_prog.clone(),
                error: err,
            })?;
        let child_stdout = child.stdout.take().unwrap();
        Ok(CompressedTmpMergeInput {
            path: self.path,
            compress_prog: self.compress_prog,
            child,
            child_stdout,
        })
    }
}
impl MergeInput for CompressedTmpMergeInput {
    type InnerRead = ChildStdout;

    fn finished_reading(self) -> UResult<()> {
        // Explicitly close stdout before waiting on the child process.
        #[allow(clippy::drop_non_drop)]
        drop(self.child_stdout);
        check_child_success(self.child, &self.compress_prog)?;
        self.path.remove();
        Ok(())
    }

    fn as_read(&mut self) -> &mut Self::InnerRead {
        &mut self.child_stdout
    }
}

pub struct PlainMergeInput<R: Read + Send> {
    inner: R,
}
impl<R: Read + Send> MergeInput for PlainMergeInput<R> {
    type InnerRead = R;
    fn finished_reading(self) -> UResult<()> {
        Ok(())
    }
    fn as_read(&mut self) -> &mut Self::InnerRead {
        &mut self.inner
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    /// When the output file is also an input it is copied to a temporary file
    /// first. That copy must stay private to the owner: `fs::copy` would carry the
    /// source's 0644 over and undo what `next_file` created the file with.
    #[test]
    fn output_copy_keeps_the_tmp_file_private() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out");
        fs::write(&out, b"a\n").unwrap();
        fs::set_permissions(&out, fs::Permissions::from_mode(0o644)).unwrap();

        let mut tmp_dir = TmpDirWrapper::new(dir.path().to_owned());
        let mut files = vec![out.clone().into_os_string()];
        replace_output_file_in_input_files(&mut files, Some(out.as_os_str()), &mut tmp_dir)
            .unwrap();

        let copy = Path::new(&files[0]);
        assert_ne!(copy, out, "the input was not replaced by a copy");
        assert_eq!(
            fs::metadata(copy).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(fs::read(copy).unwrap(), b"a\n");
    }

    /// The copy of the output file is only read while it is the file sort
    /// created: once the temporary directory's path leads elsewhere, the file
    /// found at its name there is not read.
    #[test]
    #[cfg(not(target_os = "redox"))]
    fn output_copy_is_not_read_through_a_replaced_tmp_dir() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        fs::write(other.path().join("0"), b"other\n").unwrap();
        let out = dir.path().join("out");
        fs::write(&out, b"a\n").unwrap();

        let mut tmp_dir = TmpDirWrapper::new(dir.path().to_owned());
        let mut files = vec![out.clone().into_os_string()];
        let copy =
            replace_output_file_in_input_files(&mut files, Some(out.as_os_str()), &mut tmp_dir)
                .unwrap();
        let tmp = copy.as_ref().unwrap().path().parent().unwrap();
        fs::rename(tmp, dir.path().join("moved")).unwrap();
        std::os::unix::fs::symlink(other.path(), tmp).unwrap();

        assert!(open_input(&files[0], copy.as_ref()).is_err());
    }

    /// A temporary file that is being merged when the directory's path is made
    /// to lead elsewhere: removing it afterwards leaves the file of the same
    /// name there alone.
    #[test]
    #[cfg(not(target_os = "redox"))]
    fn finished_input_is_not_removed_through_a_replaced_tmp_dir() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        fs::write(other.path().join("0"), b"other\n").unwrap();

        let mut tmp_dir = TmpDirWrapper::new(dir.path().to_owned());
        let input = WriteablePlainTmpFile::create(tmp_dir.next_file().unwrap(), None)
            .unwrap()
            .finished_writing()
            .unwrap()
            .reopen()
            .unwrap();
        let tmp = input.path.path().parent().unwrap().to_owned();
        fs::rename(&tmp, dir.path().join("moved")).unwrap();
        std::os::unix::fs::symlink(other.path(), &tmp).unwrap();
        input.finished_reading().unwrap();

        assert_eq!(fs::read(other.path().join("0")).unwrap(), b"other\n");
    }
}
