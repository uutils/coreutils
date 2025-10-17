// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Sort big files by using auxiliary files for storing intermediate chunks.
//!
//! Files are read into chunks of memory which are then sorted individually and
//! written to temporary files. There are two threads: One sorter, and one reader/writer.
//! The buffers for the individual chunks are recycled. There are two buffers.

use std::cmp::Ordering;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::{
    sync::mpsc::{Receiver, SyncSender, TryRecvError},
    thread,
};

use itertools::Itertools;
use memchr::memchr;
use uucore::error::{UResult, USimpleError};

use crate::Output;
use crate::chunks::{ReadProgress, RecycledChunk};
use crate::merge::ClosedTmpFile;
use crate::merge::MergeInput;
use crate::merge::WriteableCompressedTmpFile;
use crate::merge::WriteablePlainTmpFile;
use crate::merge::WriteableTmpFile;
use crate::tmp_dir::TmpDirWrapper;
use crate::{
    GlobalSettings,
    chunks::{self, Chunk},
    compare_by, merge, sort_by,
};
use crate::{Line, print_sorted};

// Note: update `test_sort::test_start_buffer` if this size is changed
const START_BUFFER_SIZE: usize = 8_000;

/// Sort files by using auxiliary files for storing intermediate chunks (if needed), and output the result.
pub fn ext_sort(
    files: &mut impl Iterator<Item = UResult<Box<dyn Read + Send>>>,
    settings: &GlobalSettings,
    output: Output,
    tmp_dir: &mut TmpDirWrapper,
) -> UResult<()> {
    // Allow up to two in-flight chunks in each direction to avoid deadlock
    // when pre-filling reads while the sorter is ready to send back.
    let (sorted_sender, sorted_receiver) = std::sync::mpsc::sync_channel(2);
    let (recycled_sender, recycled_receiver) = std::sync::mpsc::sync_channel(2);
    thread::spawn({
        let settings = settings.clone();
        move || sorter(&recycled_receiver, &sorted_sender, &settings)
    });
    if settings.compress_prog.is_some() {
        reader_writer::<_, WriteableCompressedTmpFile>(
            files,
            settings,
            &sorted_receiver,
            recycled_sender,
            output,
            tmp_dir,
        )
    } else {
        reader_writer::<_, WriteablePlainTmpFile>(
            files,
            settings,
            &sorted_receiver,
            recycled_sender,
            output,
            tmp_dir,
        )
    }
}

fn reader_writer<
    F: Iterator<Item = UResult<Box<dyn Read + Send>>>,
    Tmp: WriteableTmpFile + 'static,
>(
    files: F,
    settings: &GlobalSettings,
    receiver: &Receiver<Chunk>,
    sender: SyncSender<Chunk>,
    output: Output,
    tmp_dir: &mut TmpDirWrapper,
) -> UResult<()> {
    let separator = settings.line_ending.into();

    // Heuristically chosen: Dividing by 10 seems to keep our memory usage roughly
    // around settings.buffer_size as a whole.
    let buffer_size = settings.buffer_size / 10;
    let read_result: ReadResult<Tmp> = read_write_loop(
        files,
        tmp_dir,
        separator,
        buffer_size,
        settings,
        receiver,
        sender,
    )?;
    match read_result {
        ReadResult::WroteChunksToFile { tmp_files } => {
            // Optimization: if there is only one temporary run (plain file) and no dedup needed,
            // stream it directly to output to avoid re-reading huge records into memory.
            if tmp_files.len() == 1 && settings.compress_prog.is_none() && !settings.unique {
                let mut reopened = tmp_files.into_iter().next().unwrap().reopen()?;
                let mut out = output.into_write();
                std::io::copy(reopened.as_read(), &mut out)
                    .map_err(|e| USimpleError::new(2, e.to_string()))?;
                out.flush()
                    .map_err(|e| USimpleError::new(2, e.to_string()))?;
            } else {
                merge::merge_with_file_limit::<_, _, Tmp>(
                    tmp_files.into_iter().map(|c| c.reopen()),
                    settings,
                    output,
                    tmp_dir,
                )?;
            }
        }
        ReadResult::SortedSingleChunk(chunk) => {
            if settings.unique {
                print_sorted(
                    chunk.lines().iter().dedup_by(|a, b| {
                        compare_by(a, b, settings, chunk.line_data(), chunk.line_data())
                            == Ordering::Equal
                    }),
                    settings,
                    output,
                )?;
            } else {
                print_sorted(chunk.lines().iter(), settings, output)?;
            }
        }
        ReadResult::SortedTwoChunks([a, b]) => {
            let merged_iter = a.lines().iter().map(|line| (line, &a)).merge_by(
                b.lines().iter().map(|line| (line, &b)),
                |(line_a, a), (line_b, b)| {
                    compare_by(line_a, line_b, settings, a.line_data(), b.line_data())
                        != Ordering::Greater
                },
            );
            if settings.unique {
                print_sorted(
                    merged_iter
                        .dedup_by(|(line_a, a), (line_b, b)| {
                            compare_by(line_a, line_b, settings, a.line_data(), b.line_data())
                                == Ordering::Equal
                        })
                        .map(|(line, _)| line),
                    settings,
                    output,
                )?;
            } else {
                print_sorted(merged_iter.map(|(line, _)| line), settings, output)?;
            }
        }
        ReadResult::EmptyInput => {
            // don't output anything
        }
    }
    Ok(())
}

/// The function that is executed on the sorter thread.
fn sorter(receiver: &Receiver<Chunk>, sender: &SyncSender<Chunk>, settings: &GlobalSettings) {
    while let Ok(mut payload) = receiver.recv() {
        payload.with_dependent_mut(|_, contents| {
            sort_by(&mut contents.lines, settings, &contents.line_data);
        });
        if sender.send(payload).is_err() {
            // The receiver has gone away, likely because the other thread hit an error.
            // We stop silently because the actual error is printed by the other thread.
            return;
        }
    }
}

/// Describes how we read the chunks from the input.
enum ReadResult<I: WriteableTmpFile> {
    /// The input was empty. Nothing was read.
    EmptyInput,
    /// The input fits into a single Chunk, which was kept in memory.
    SortedSingleChunk(Chunk),
    /// The input fits into two chunks, which were kept in memory.
    SortedTwoChunks([Chunk; 2]),
    /// The input was read into multiple chunks, which were written to auxiliary files.
    WroteChunksToFile { tmp_files: Vec<I::Closed> },
}
/// The function that is executed on the reader/writer thread.
fn read_write_loop<I: WriteableTmpFile>(
    mut files: impl Iterator<Item = UResult<Box<dyn Read + Send>>>,
    tmp_dir: &mut TmpDirWrapper,
    separator: u8,
    buffer_size: usize,
    settings: &GlobalSettings,
    receiver: &Receiver<Chunk>,
    sender: SyncSender<Chunk>,
) -> UResult<ReadResult<I>> {
    let mut file = files.next().unwrap()?;
    let mut carry_over = vec![];

    // Maintain up to two in-flight reads to keep the sorter busy
    let mut recycled_pool: Vec<RecycledChunk> =
        std::iter::repeat_with(|| RecycledChunk::new(START_BUFFER_SIZE.min(buffer_size)))
            .take(2)
            .collect();
    let mut in_flight = 0usize;
    let mut sender_option = Some(sender);
    let mut tmp_files: Vec<I::Closed> = vec![];
    let mut mem_chunks: Vec<Chunk> = vec![];

    // Helper to try reading and sending more chunks or spilling long records
    let try_read_more = |recycled_pool: &mut Vec<RecycledChunk>,
                         in_flight: &mut usize,
                         sender_option: &mut Option<SyncSender<Chunk>>,
                         tmp_files: &mut Vec<I::Closed>,
                         carry_over: &mut Vec<u8>,
                         file: &mut Box<dyn Read + Send>,
                         files: &mut dyn Iterator<Item = UResult<Box<dyn Read + Send>>>,
                         tmp_dir: &mut TmpDirWrapper|
     -> UResult<()> {
        while sender_option.is_some() && *in_flight < 2 {
            let recycled = if let Some(rc) = recycled_pool.pop() {
                rc
            } else {
                RecycledChunk::new(if START_BUFFER_SIZE < buffer_size {
                    START_BUFFER_SIZE
                } else {
                    buffer_size
                })
            };
            match chunks::read(
                sender_option.as_ref().unwrap(),
                recycled,
                Some(buffer_size),
                carry_over,
                file,
                files,
                separator,
                settings,
            )? {
                ReadProgress::SentChunk => {
                    *in_flight += 1;
                }
                ReadProgress::NeedSpill => {
                    // Spill this oversized record into its own run file
                    let tmp = spill_long_record::<I>(
                        tmp_dir,
                        carry_over,
                        file.as_mut(),
                        separator,
                        settings.compress_prog.as_deref(),
                    )?;
                    tmp_files.push(tmp);
                    // Try to read again (do not change in_flight)
                }
                ReadProgress::NoChunk => {
                    // Nothing to send yet; try reading again (continue loop)
                }
                ReadProgress::Finished => {
                    *sender_option = None;
                    break;
                }
            }
        }
        Ok(())
    };

    // Initial fill
    try_read_more(
        &mut recycled_pool,
        &mut in_flight,
        &mut sender_option,
        &mut tmp_files,
        &mut carry_over,
        &mut file,
        &mut files,
        tmp_dir,
    )?;

    loop {
        if in_flight > 0 {
            let Ok(chunk) = receiver.recv() else {
                // Sender dropped; finish by merging whatever we have
                break;
            };

            in_flight -= 1;
            if tmp_files.is_empty() && sender_option.is_none() && mem_chunks.len() < 2 {
                // Potential small input: keep in memory for fast path
                mem_chunks.push(chunk);
            } else {
                // General path: write to tmp file
                let tmp_file = write::<I>(
                    &chunk,
                    tmp_dir.next_file()?,
                    settings.compress_prog.as_deref(),
                    separator,
                )?;
                tmp_files.push(tmp_file);
                // Recycle buffer for next reads
                recycled_pool.push(chunk.recycle());
            }

            // Attempt to fill again
            try_read_more(
                &mut recycled_pool,
                &mut in_flight,
                &mut sender_option,
                &mut tmp_files,
                &mut carry_over,
                &mut file,
                &mut files,
                tmp_dir,
            )?;
        } else {
            if sender_option.is_none() {
                // No more reads possible and no in-flight chunks
                if tmp_files.is_empty() {
                    return Ok(match mem_chunks.len() {
                        0 => ReadResult::EmptyInput,
                        1 => ReadResult::SortedSingleChunk(mem_chunks.pop().unwrap()),
                        2 => ReadResult::SortedTwoChunks([
                            mem_chunks.remove(0),
                            mem_chunks.remove(0),
                        ]),
                        _ => unreachable!(),
                    });
                }
                // Flush any in-memory chunks to tmp and finish with merge
                for ch in mem_chunks.drain(..) {
                    let tmp_file = write::<I>(
                        &ch,
                        tmp_dir.next_file()?,
                        settings.compress_prog.as_deref(),
                        separator,
                    )?;
                    tmp_files.push(tmp_file);
                }
                return Ok(ReadResult::WroteChunksToFile { tmp_files });
            }

            // Try reading more if possible
            try_read_more(
                &mut recycled_pool,
                &mut in_flight,
                &mut sender_option,
                &mut tmp_files,
                &mut carry_over,
                &mut file,
                &mut files,
                tmp_dir,
            )?;

            if in_flight == 0 {
                // No chunk to receive yet; loop continues until either we can read or finish
                // To avoid busy-looping, try a non-blocking receive in case a chunk just arrived
                match receiver.try_recv() {
                    Ok(chunk) => {
                        // Process as above
                        if tmp_files.is_empty() && sender_option.is_none() && mem_chunks.len() < 2 {
                            mem_chunks.push(chunk);
                        } else {
                            let tmp_file = write::<I>(
                                &chunk,
                                tmp_dir.next_file()?,
                                settings.compress_prog.as_deref(),
                                separator,
                            )?;
                            tmp_files.push(tmp_file);
                            recycled_pool.push(chunk.recycle());
                        }
                    }
                    Err(TryRecvError::Empty) => {
                        // nothing to do right now
                    }
                    Err(TryRecvError::Disconnected) => break,
                }
            }
        }
    }

    Ok(ReadResult::WroteChunksToFile { tmp_files })
}

/// Spill a single oversized record into its own temporary run file.
fn spill_long_record<I: WriteableTmpFile>(
    tmp_dir: &mut TmpDirWrapper,
    carry_over: &mut Vec<u8>,
    file: &mut dyn Read,
    separator: u8,
    compress_prog: Option<&str>,
) -> UResult<I::Closed> {
    let mut tmp_file = I::create(tmp_dir.next_file()?, compress_prog)?;
    if !carry_over.is_empty() {
        tmp_file.as_write().write_all(carry_over).unwrap();
        carry_over.clear();
    }
    let mut buf = vec![0u8; 128 * 1024];
    let mut _current_file_had_data = false;
    let mut _last_byte: Option<u8> = None;
    loop {
        match file.read(&mut buf) {
            Ok(0) => {
                // EOF: end current record here
                break;
            }
            Ok(n) => {
                _current_file_had_data = true;
                if let Some(pos) = memchr(separator, &buf[..n]) {
                    // End of record found within this chunk
                    tmp_file.as_write().write_all(&buf[..pos]).unwrap();
                    // Save remainder after separator for next reads
                    carry_over.extend_from_slice(&buf[pos + 1..n]);
                    _last_byte = Some(buf[n - 1]);
                    break;
                }
                tmp_file.as_write().write_all(&buf[..n]).unwrap();
                _last_byte = Some(buf[n - 1]);
            }
            Err(e) => return Err(e.into()),
        }
    }
    // Append a separator to the run to match write_lines semantics
    tmp_file.as_write().write_all(&[separator]).unwrap();
    tmp_file.finished_writing()
}

/// Write the lines in `chunk` to `file`, separated by `separator`.
/// `compress_prog` is used to optionally compress file contents.
fn write<I: WriteableTmpFile>(
    chunk: &Chunk,
    file: (File, PathBuf),
    compress_prog: Option<&str>,
    separator: u8,
) -> UResult<I::Closed> {
    let mut tmp_file = I::create(file, compress_prog)?;
    write_lines(chunk.lines(), tmp_file.as_write(), separator);
    tmp_file.finished_writing()
}

fn write_lines<T: Write>(lines: &[Line], writer: &mut T, separator: u8) {
    for s in lines {
        writer.write_all(s.line).unwrap();
        writer.write_all(&[separator]).unwrap();
    }
}
