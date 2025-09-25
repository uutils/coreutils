// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Utilities for reading files as chunks.

// spell-checker:ignore memrchr

#![allow(dead_code)]
// Ignores non-used warning for `borrow_buffer` in `Chunk`

use std::{
    io::{ErrorKind, Read},
    sync::mpsc::SyncSender,
};

use memchr::memchr_iter;
use self_cell::self_cell;
use uucore::error::{UResult, USimpleError};

use crate::{GeneralBigDecimalParseResult, GlobalSettings, Line, numeric_str_cmp::NumInfo};

self_cell!(
    /// The chunk that is passed around between threads.
    pub struct Chunk {
        owner: Vec<u8>,

        #[covariant]
        dependent: ChunkContents,
    }

    impl {Debug}
);

#[derive(Debug)]
pub struct ChunkContents<'a> {
    pub lines: Vec<Line<'a>>,
    pub line_data: LineData<'a>,
}

#[derive(Debug)]
pub struct LineData<'a> {
    pub selections: Vec<&'a [u8]>,
    pub num_infos: Vec<NumInfo>,
    pub parsed_floats: Vec<GeneralBigDecimalParseResult>,
    pub line_num_floats: Vec<Option<f64>>,
}

impl Chunk {
    /// Destroy this chunk and return its components to be reused.
    pub fn recycle(mut self) -> RecycledChunk {
        let recycled_contents = self.with_dependent_mut(|_, contents| {
            contents.lines.clear();
            contents.line_data.selections.clear();
            contents.line_data.num_infos.clear();
            contents.line_data.parsed_floats.clear();
            contents.line_data.line_num_floats.clear();
            let lines = unsafe {
                // SAFETY: It is safe to (temporarily) transmute to a vector of lines with a longer lifetime,
                // because the vector is empty.
                // Transmuting is necessary to make recycling possible. See https://github.com/rust-lang/rfcs/pull/2802
                // for a rfc to make this unnecessary. Its example is similar to the code here.
                std::mem::transmute::<Vec<Line<'_>>, Vec<Line<'static>>>(std::mem::take(
                    &mut contents.lines,
                ))
            };
            let selections = unsafe {
                // SAFETY: (same as above) It is safe to (temporarily) transmute to a vector of &str with a longer lifetime,
                // because the vector is empty.
                std::mem::transmute::<Vec<&'_ [u8]>, Vec<&'static [u8]>>(std::mem::take(
                    &mut contents.line_data.selections,
                ))
            };
            (
                lines,
                selections,
                std::mem::take(&mut contents.line_data.num_infos),
                std::mem::take(&mut contents.line_data.parsed_floats),
                std::mem::take(&mut contents.line_data.line_num_floats),
            )
        });
        RecycledChunk {
            lines: recycled_contents.0,
            selections: recycled_contents.1,
            num_infos: recycled_contents.2,
            parsed_floats: recycled_contents.3,
            line_num_floats: recycled_contents.4,
            buffer: self.into_owner(),
        }
    }

    pub fn lines(&self) -> &Vec<Line<'_>> {
        &self.borrow_dependent().lines
    }

    pub fn line_data(&self) -> &LineData<'_> {
        &self.borrow_dependent().line_data
    }
}

pub struct RecycledChunk {
    lines: Vec<Line<'static>>,
    selections: Vec<&'static [u8]>,
    num_infos: Vec<NumInfo>,
    parsed_floats: Vec<GeneralBigDecimalParseResult>,
    line_num_floats: Vec<Option<f64>>,
    buffer: Vec<u8>,
}

impl RecycledChunk {
    pub fn new(capacity: usize) -> Self {
        Self {
            lines: Vec::new(),
            selections: Vec::new(),
            num_infos: Vec::new(),
            parsed_floats: Vec::new(),
            line_num_floats: Vec::new(),
            buffer: vec![0; capacity],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadProgress {
    /// At least one full line was read and sent as a chunk to the sorter.
    SentChunk,
    /// Buffer cap reached without a separator; caller should spill the current record.
    NeedSpill,
    /// No more input remains; nothing was sent.
    Finished,
    /// No complete line available yet, but more input may remain.
    NoChunk,
}

/// Read a chunk, parse complete records, and send them to the sorter.
///
/// This function attempts to read at least one complete record (delimited by `separator`).
/// Data after the last complete record is left in `carry_over` to be prefixed on the
/// next invocation. Memory growth is bounded by `max_buffer_size` when provided.
///
/// # Returns
/// - `SentChunk`: At least one full record was read; a `Chunk` was sent to `sender`.
/// - `NoChunk`: No full record yet, but more input may remain; call again.
/// - `NeedSpill`: The buffer hit the cap with no separator found; caller should spill the
///   current oversized record to a run file and then continue.
/// - `Finished`: No more input remains; nothing was sent.
///
/// # Arguments
///
/// * `sender`: Channel to send the populated `Chunk` to the sorter thread.
/// * `recycled_chunk`: Result of `Chunk::recycle`, providing reusable vectors and buffer.
///   `buffer.len()` equals its current capacity and will be reused for reading.
/// * `max_buffer_size`: Maximum buffer size in bytes; if `Some`, reading respects this cap.
/// * `carry_over`: Bytes from the previous call after the last separator; they are copied
///   to the beginning of the buffer before reading.
/// * `file`: Current reader.
/// * `next_files`: Iterator to advance to the next file once `file` reaches EOF.
/// * `separator`: Record delimiter (e.g., `b'\n'` or `b'\0'`).
/// * `settings`: Global sort settings (used for tokenization decisions when building `Chunk`).
#[allow(clippy::too_many_arguments)]
pub fn read<T: Read>(
    sender: &SyncSender<Chunk>,
    recycled_chunk: RecycledChunk,
    max_buffer_size: Option<usize>,
    carry_over: &mut Vec<u8>,
    file: &mut T,
    next_files: &mut dyn Iterator<Item = UResult<T>>,
    separator: u8,
    settings: &GlobalSettings,
) -> UResult<ReadProgress> {
    let RecycledChunk {
        lines,
        selections,
        num_infos,
        parsed_floats,
        line_num_floats,
        mut buffer,
    } = recycled_chunk;
    if buffer.len() < carry_over.len() {
        buffer.resize(carry_over.len() + 10 * 1024, 0);
    }
    buffer[..carry_over.len()].copy_from_slice(carry_over);
    let (read, should_continue, need_spill) = read_to_buffer(
        file,
        next_files,
        &mut buffer,
        max_buffer_size,
        carry_over.len(),
        separator,
    )?;
    carry_over.clear();
    carry_over.extend_from_slice(&buffer[read..]);

    if need_spill {
        return Ok(ReadProgress::NeedSpill);
    }

    if read != 0 {
        let payload: UResult<Chunk> = Chunk::try_new(buffer, |buffer| {
            let selections = unsafe {
                // SAFETY: It is safe to transmute to an empty vector of selections with shorter lifetime.
                // It was only temporarily transmuted to a Vec<Line<'static>> to make recycling possible.
                std::mem::transmute::<Vec<&'static [u8]>, Vec<&'_ [u8]>>(selections)
            };
            let mut lines = unsafe {
                // SAFETY: (same as above) It is safe to transmute to a vector of lines with shorter lifetime,
                // because it was only temporarily transmuted to a Vec<Line<'static>> to make recycling possible.
                std::mem::transmute::<Vec<Line<'static>>, Vec<Line<'_>>>(lines)
            };
            let read = &buffer[..read];
            let mut line_data = LineData {
                selections,
                num_infos,
                parsed_floats,
                line_num_floats,
            };
            parse_lines(read, &mut lines, &mut line_data, separator, settings);
            Ok(ChunkContents { lines, line_data })
        });
        sender.send(payload?).unwrap();
        return Ok(ReadProgress::SentChunk);
    }
    Ok(if should_continue {
        // No full line could be sent now, but there might still be input.
        // This case happens when the input exactly fits the buffer without a separator at the end.
        // The next call will continue reading and eventually emit a chunk or finish.
        ReadProgress::NoChunk
    } else {
        ReadProgress::Finished
    })
}

/// Split `read` into `Line`s, and add them to `lines`.
fn parse_lines<'a>(
    read: &'a [u8],
    lines: &mut Vec<Line<'a>>,
    line_data: &mut LineData<'a>,
    separator: u8,
    settings: &GlobalSettings,
) {
    let read = read.strip_suffix(&[separator]).unwrap_or(read);

    assert!(lines.is_empty());
    assert!(line_data.selections.is_empty());
    assert!(line_data.num_infos.is_empty());
    assert!(line_data.parsed_floats.is_empty());
    assert!(line_data.line_num_floats.is_empty());
    let mut token_buffer = vec![];
    lines.extend(
        read.split(|&c| c == separator)
            .enumerate()
            .map(|(index, line)| Line::create(line, index, line_data, &mut token_buffer, settings)),
    );
}

/// Read from `file` into `buffer` until at least one complete record is present or EOF.
///
/// This function makes sure that at least one complete record (terminated by `separator`) is
/// available in `buffer` (unless we reach EOF and there is no next file). The buffer is grown
/// if necessary, respecting `max_buffer_size` when provided. The bytes after the last complete
/// record remain in `buffer` and should be carried over to the next invocation.
///
/// Arguments:
/// - `file`: The file to read from initially.
/// - `next_files`: Iterator used to advance to the next file when `file` reaches EOF; reading continues seamlessly.
/// - `buffer`: The destination buffer. Contents from `start_offset` onward will be overwritten by new data.
/// - `max_buffer_size`: Optional cap for `buffer` growth in bytes.
/// - `start_offset`: Number of bytes at the start of `buffer` containing carry-over data that must be preserved.
/// - `separator`: Record delimiter byte.
///
/// Returns `(read_len, should_continue, need_spill)`:
/// - `read_len`: The number of bytes in `buffer` that form complete records ready for parsing.
/// - `should_continue`: `true` if more input may remain and another call could read additional data.
/// - `need_spill`: `true` if the buffer reached `max_buffer_size` without encountering a separator,
///   indicating the caller should spill the current oversized record to disk.
fn read_to_buffer<T: Read>(
    file: &mut T,
    next_files: &mut dyn Iterator<Item = UResult<T>>,
    buffer: &mut Vec<u8>,
    max_buffer_size: Option<usize>,
    start_offset: usize,
    separator: u8,
) -> UResult<(usize, bool, bool)> {
    let mut read_target = &mut buffer[start_offset..];
    let mut last_file_empty = true;
    // Only search for newlines in regions we haven't scanned before to avoid quadratic behavior.
    let mut newline_search_offset = 0;
    let mut found_newline = false;
    loop {
        match file.read(read_target) {
            Ok(0) => {
                if read_target.is_empty() {
                    // Buffer full
                    if let Some(max) = max_buffer_size {
                        if max > buffer.len() {
                            // We can grow the buffer
                            let prev_len = buffer.len();
                            if buffer.len() < max / 2 {
                                buffer.resize(buffer.len() * 2, 0);
                            } else {
                                buffer.resize(max, 0);
                            }
                            read_target = &mut buffer[prev_len..];
                            continue;
                        }
                    }

                    // Buffer cannot grow further or exactly filled: find the last newline seen so far
                    let mut sep_iter =
                        memchr_iter(separator, &buffer[newline_search_offset..buffer.len()]).rev();
                    newline_search_offset = buffer.len();
                    if let Some(last_line_end) = sep_iter.next() {
                        if found_newline || sep_iter.next().is_some() {
                            // We read enough lines. Include the separator so it isn't carried over.
                            return Ok((last_line_end + 1, true, false));
                        }
                        found_newline = true;
                    }

                    // Need more data for a full line
                    if let Some(max) = max_buffer_size {
                        if buffer.len() >= max {
                            // Hard cap hit and no newline yet: signal spill
                            return Ok((0, true, true));
                        }
                    }
                    let len = buffer.len();
                    buffer.resize(len + 1024 * 10, 0);
                    read_target = &mut buffer[len..];
                } else {
                    // This file has been fully read.
                    let mut leftover_len = read_target.len();
                    if !last_file_empty {
                        // The file was not empty: ensure a trailing separator
                        let read_len = buffer.len() - leftover_len;
                        if buffer[read_len - 1] != separator {
                            buffer[read_len] = separator;
                            leftover_len -= 1;
                        }
                        let read_len = buffer.len() - leftover_len;
                        read_target = &mut buffer[read_len..];
                    }
                    if let Some(next_file) = next_files.next() {
                        // There is another file.
                        last_file_empty = true;
                        *file = next_file?;
                    } else {
                        // This was the last file.
                        let read_len = buffer.len() - leftover_len;
                        return Ok((read_len, false, false));
                    }
                }
            }
            Ok(n) => {
                read_target = &mut read_target[n..];
                last_file_empty = false;
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => {
                // retry
            }
            Err(e) => return Err(USimpleError::new(2, e.to_string())),
        }
    }
}

/// Grow `buffer` by at least a minimal increment, up to an optional cap.
///
/// If `max_buffer_size` is `Some`, the new length will not exceed it. Once the buffer
/// size reaches the cap, no further growth occurs.
/// Otherwise, the buffer grows approximately by doubling, with a minimum increment of 10 KiB.
///
/// Ensures monotonic growth: the resulting length is always greater than the current length.
fn grow_buffer(buffer: &mut Vec<u8>, max_buffer_size: Option<usize>) {
    const MIN_GROW: usize = 10 * 1024;
    let current_len = buffer.len();
    let mut next_len = if current_len == 0 {
        MIN_GROW
    } else if let Some(max_buffer_size) = max_buffer_size {
        if current_len < max_buffer_size {
            std::cmp::min(current_len.saturating_mul(2), max_buffer_size)
                .max(current_len.saturating_add(MIN_GROW))
        } else {
            // Respect the cap: do not grow further.
            current_len
        }
    } else {
        current_len
            .saturating_mul(2)
            .max(current_len.saturating_add(MIN_GROW))
    };

    if next_len <= current_len {
        next_len = current_len.saturating_add(MIN_GROW.max(1));
    }

    buffer.resize(next_len, 0);
}
