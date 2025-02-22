// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Utilities for reading files as chunks.

#![allow(dead_code)]
// Ignores non-used warning for `borrow_buffer` in `Chunk`

use std::{
    io::{ErrorKind, Read},
    sync::mpsc::SyncSender,
};

use memchr::memchr_iter;
use self_cell::self_cell;
use uucore::error::{UResult, USimpleError};

use crate::{numeric_str_cmp::NumInfo, GeneralF64ParseResult, GlobalSettings, Line, SortError};

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
    pub selections: Vec<&'a str>,
    pub num_infos: Vec<NumInfo>,
    pub parsed_floats: Vec<GeneralF64ParseResult>,
}

impl Chunk {
    /// Destroy this chunk and return its components to be reused.
    pub fn recycle(mut self) -> RecycledChunk {
        let recycled_contents = self.with_dependent_mut(|_, contents| {
            contents.lines.clear();
            contents.line_data.selections.clear();
            contents.line_data.num_infos.clear();
            contents.line_data.parsed_floats.clear();
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
                std::mem::transmute::<Vec<&'_ str>, Vec<&'static str>>(std::mem::take(
                    &mut contents.line_data.selections,
                ))
            };
            (
                lines,
                selections,
                std::mem::take(&mut contents.line_data.num_infos),
                std::mem::take(&mut contents.line_data.parsed_floats),
            )
        });
        RecycledChunk {
            lines: recycled_contents.0,
            selections: recycled_contents.1,
            num_infos: recycled_contents.2,
            parsed_floats: recycled_contents.3,
            buffer: self.into_owner(),
        }
    }

    pub fn lines(&self) -> &Vec<Line> {
        &self.borrow_dependent().lines
    }
    pub fn line_data(&self) -> &LineData {
        &self.borrow_dependent().line_data
    }
}

pub struct RecycledChunk {
    lines: Vec<Line<'static>>,
    selections: Vec<&'static str>,
    num_infos: Vec<NumInfo>,
    parsed_floats: Vec<GeneralF64ParseResult>,
    buffer: Vec<u8>,
}

impl RecycledChunk {
    pub fn new(capacity: usize) -> Self {
        Self {
            lines: Vec::new(),
            selections: Vec::new(),
            num_infos: Vec::new(),
            parsed_floats: Vec::new(),
            buffer: vec![0; capacity],
        }
    }
}

/// Read a chunk, parse lines and send them.
///
/// No empty chunk will be sent. If we reach the end of the input, `false` is returned.
/// However, if this function returns `true`, it is not guaranteed that there is still
/// input left: If the input fits _exactly_ into a buffer, we will only notice that there's
/// nothing more to read at the next invocation. In case there is no input left, nothing will
/// be sent.
///
/// # Arguments
///
/// (see also `read_to_chunk` for a more detailed documentation)
///
/// * `sender`: The sender to send the lines to the sorter.
/// * `recycled_chunk`: The recycled chunk, as returned by `Chunk::recycle`.
///   (i.e. `buffer.len()` should be equal to `buffer.capacity()`)
/// * `max_buffer_size`: How big `buffer` can be.
/// * `carry_over`: The bytes that must be carried over in between invocations.
/// * `file`: The current file.
/// * `next_files`: What `file` should be updated to next.
/// * `separator`: The line separator.
/// * `settings`: The global settings.
#[allow(clippy::too_many_arguments)]
pub fn read<T: Read>(
    sender: &SyncSender<Chunk>,
    recycled_chunk: RecycledChunk,
    max_buffer_size: Option<usize>,
    carry_over: &mut Vec<u8>,
    file: &mut T,
    next_files: &mut impl Iterator<Item = UResult<T>>,
    separator: u8,
    settings: &GlobalSettings,
) -> UResult<bool> {
    let RecycledChunk {
        lines,
        selections,
        num_infos,
        parsed_floats,
        mut buffer,
    } = recycled_chunk;
    if buffer.len() < carry_over.len() {
        buffer.resize(carry_over.len() + 10 * 1024, 0);
    }
    buffer[..carry_over.len()].copy_from_slice(carry_over);
    let (read, should_continue) = read_to_buffer(
        file,
        next_files,
        &mut buffer,
        max_buffer_size,
        carry_over.len(),
        separator,
    )?;
    carry_over.clear();
    carry_over.extend_from_slice(&buffer[read..]);

    if read != 0 {
        let payload: UResult<Chunk> = Chunk::try_new(buffer, |buffer| {
            let selections = unsafe {
                // SAFETY: It is safe to transmute to an empty vector of selections with shorter lifetime.
                // It was only temporarily transmuted to a Vec<Line<'static>> to make recycling possible.
                std::mem::transmute::<Vec<&'static str>, Vec<&'_ str>>(selections)
            };
            let mut lines = unsafe {
                // SAFETY: (same as above) It is safe to transmute to a vector of lines with shorter lifetime,
                // because it was only temporarily transmuted to a Vec<Line<'static>> to make recycling possible.
                std::mem::transmute::<Vec<Line<'static>>, Vec<Line<'_>>>(lines)
            };
            let read = std::str::from_utf8(&buffer[..read])
                .map_err(|error| SortError::Uft8Error { error })?;
            let mut line_data = LineData {
                selections,
                num_infos,
                parsed_floats,
            };
            parse_lines(read, &mut lines, &mut line_data, separator, settings);
            Ok(ChunkContents { lines, line_data })
        });
        sender.send(payload?).unwrap();
    }
    Ok(should_continue)
}

/// Split `read` into `Line`s, and add them to `lines`.
fn parse_lines<'a>(
    read: &'a str,
    lines: &mut Vec<Line<'a>>,
    line_data: &mut LineData<'a>,
    separator: u8,
    settings: &GlobalSettings,
) {
    let read = read.strip_suffix(separator as char).unwrap_or(read);

    assert!(lines.is_empty());
    assert!(line_data.selections.is_empty());
    assert!(line_data.num_infos.is_empty());
    assert!(line_data.parsed_floats.is_empty());
    let mut token_buffer = vec![];
    lines.extend(
        read.split(separator as char)
            .enumerate()
            .map(|(index, line)| Line::create(line, index, line_data, &mut token_buffer, settings)),
    );
}

/// Read from `file` into `buffer`.
///
/// This function makes sure that at least two lines are read (unless we reach EOF and there's no next file),
/// growing the buffer if necessary.
/// The last line is likely to not have been fully read into the buffer. Its bytes must be copied to
/// the front of the buffer for the next invocation so that it can be continued to be read
/// (see the return values and `start_offset`).
///
/// # Arguments
///
/// * `file`: The file to start reading from.
/// * `next_files`: When `file` reaches EOF, it is updated to `next_files.next()` if that is `Some`,
///   and this function continues reading.
/// * `buffer`: The buffer that is filled with bytes. Its contents will mostly be overwritten (see `start_offset`
///   as well). It will be grown up to `max_buffer_size` if necessary, but it will always grow to read at least two lines.
/// * `max_buffer_size`: Grow the buffer to at most this length. If None, the buffer will not grow, unless needed to read at least two lines.
/// * `start_offset`: The amount of bytes at the start of `buffer` that were carried over
///   from the previous read and should not be overwritten.
/// * `separator`: The byte that separates lines.
///
/// # Returns
///
/// * The amount of bytes in `buffer` that can now be interpreted as lines.
///   The remaining bytes must be copied to the start of the buffer for the next invocation,
///   if another invocation is necessary, which is determined by the other return value.
/// * Whether this function should be called again.
fn read_to_buffer<T: Read>(
    file: &mut T,
    next_files: &mut impl Iterator<Item = UResult<T>>,
    buffer: &mut Vec<u8>,
    max_buffer_size: Option<usize>,
    start_offset: usize,
    separator: u8,
) -> UResult<(usize, bool)> {
    let mut read_target = &mut buffer[start_offset..];
    let mut last_file_target_size = read_target.len();
    loop {
        match file.read(read_target) {
            Ok(0) => {
                if read_target.is_empty() {
                    // chunk is full
                    if let Some(max_buffer_size) = max_buffer_size {
                        if max_buffer_size > buffer.len() {
                            // we can grow the buffer
                            let prev_len = buffer.len();
                            if buffer.len() < max_buffer_size / 2 {
                                buffer.resize(buffer.len() * 2, 0);
                            } else {
                                buffer.resize(max_buffer_size, 0);
                            }
                            read_target = &mut buffer[prev_len..];
                            continue;
                        }
                    }
                    let mut sep_iter = memchr_iter(separator, buffer).rev();
                    let last_line_end = sep_iter.next();
                    if sep_iter.next().is_some() {
                        // We read enough lines.
                        let end = last_line_end.unwrap();
                        // We want to include the separator here, because it shouldn't be carried over.
                        return Ok((end + 1, true));
                    } else {
                        // We need to read more lines
                        let len = buffer.len();
                        // resize the vector to 10 KB more
                        buffer.resize(len + 1024 * 10, 0);
                        read_target = &mut buffer[len..];
                    }
                } else {
                    // This file has been fully read.
                    let mut leftover_len = read_target.len();
                    if last_file_target_size != leftover_len {
                        // The file was not empty.
                        let read_len = buffer.len() - leftover_len;
                        if buffer[read_len - 1] != separator {
                            // The file did not end with a separator. We have to insert one.
                            buffer[read_len] = separator;
                            leftover_len -= 1;
                        }
                        let read_len = buffer.len() - leftover_len;
                        read_target = &mut buffer[read_len..];
                    }
                    if let Some(next_file) = next_files.next() {
                        // There is another file.
                        last_file_target_size = leftover_len;
                        *file = next_file?;
                    } else {
                        // This was the last file.
                        let read_len = buffer.len() - leftover_len;
                        return Ok((read_len, false));
                    }
                }
            }
            Ok(n) => {
                read_target = &mut read_target[n..];
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => {
                // retry
            }
            Err(e) => return Err(USimpleError::new(2, e.to_string())),
        }
    }
}
