//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Michael Debertol <michael.debertol..AT..gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

//! Utilities for reading files as chunks.

use std::{
    io::{ErrorKind, Read},
    sync::mpsc::SyncSender,
};

use memchr::memchr_iter;
use ouroboros::self_referencing;

use crate::{GlobalSettings, Line};

/// The chunk that is passed around between threads.
/// `lines` consist of slices into `buffer`.
#[self_referencing(pub_extras)]
#[derive(Debug)]
pub struct Chunk {
    pub buffer: Vec<u8>,
    #[borrows(buffer)]
    #[covariant]
    pub lines: Vec<Line<'this>>,
}

impl Chunk {
    /// Destroy this chunk and return its components to be reused.
    ///
    /// # Returns
    ///
    /// * The `lines` vector, emptied
    /// * The `buffer` vector, **not** emptied
    pub fn recycle(mut self) -> (Vec<Line<'static>>, Vec<u8>) {
        let recycled_lines = self.with_lines_mut(|lines| {
            lines.clear();
            unsafe {
                // SAFETY: It is safe to (temporarily) transmute to a vector of lines with a longer lifetime,
                // because the vector is empty.
                // Transmuting is necessary to make recycling possible. See https://github.com/rust-lang/rfcs/pull/2802
                // for a rfc to make this unnecessary. Its example is similar to the code here.
                std::mem::transmute::<Vec<Line<'_>>, Vec<Line<'static>>>(std::mem::take(lines))
            }
        });
        (recycled_lines, self.into_heads().buffer)
    }
}

/// Read a chunk, parse lines and send them.
///
/// No empty chunk will be sent.
///
/// # Arguments
///
/// * `sender_option`: The sender to send the lines to the sorter. If `None`, does nothing.
/// * `buffer`: The recycled buffer. All contents will be overwritten, but it must already be filled.
///   (i.e. `buffer.len()` should be equal to `buffer.capacity()`)
/// * `carry_over`: The bytes that must be carried over in between invocations.
/// * `file`: The current file.
/// * `next_files`: What `file` should be updated to next.
/// * `separator`: The line separator.
/// * `lines`: The recycled vector to fill with lines. Must be empty.
/// * `settings`: The global settings.
#[allow(clippy::too_many_arguments)]
pub fn read(
    sender_option: &mut Option<SyncSender<Chunk>>,
    mut buffer: Vec<u8>,
    carry_over: &mut Vec<u8>,
    file: &mut Box<dyn Read + Send>,
    next_files: &mut impl Iterator<Item = Box<dyn Read + Send>>,
    separator: u8,
    lines: Vec<Line<'static>>,
    settings: &GlobalSettings,
) {
    assert!(lines.is_empty());
    if let Some(sender) = sender_option {
        if buffer.len() < carry_over.len() {
            buffer.resize(carry_over.len() + 10 * 1024, 0);
        }
        buffer[..carry_over.len()].copy_from_slice(&carry_over);
        let (read, should_continue) =
            read_to_buffer(file, next_files, &mut buffer, carry_over.len(), separator);
        carry_over.clear();
        carry_over.extend_from_slice(&buffer[read..]);

        let payload = Chunk::new(buffer, |buf| {
            let mut lines = unsafe {
                // SAFETY: It is safe to transmute to a vector of lines with shorter lifetime,
                // because it was only temporarily transmuted to a Vec<Line<'static>> to make recycling possible.
                std::mem::transmute::<Vec<Line<'static>>, Vec<Line<'_>>>(lines)
            };
            let read = crash_if_err!(1, std::str::from_utf8(&buf[..read]));
            parse_lines(read, &mut lines, separator, &settings);
            lines
        });
        if !payload.borrow_lines().is_empty() {
            sender.send(payload).unwrap();
        }
        if !should_continue {
            *sender_option = None;
        }
    }
}

/// Split `read` into `Line`s, and add them to `lines`.
fn parse_lines<'a>(
    mut read: &'a str,
    lines: &mut Vec<Line<'a>>,
    separator: u8,
    settings: &GlobalSettings,
) {
    // Strip a trailing separator. TODO: Once our MinRustV is 1.45 or above, use strip_suffix() instead.
    if read.ends_with(separator as char) {
        read = &read[..read.len() - 1];
    }

    lines.extend(
        read.split(separator as char)
            .map(|line| Line::create(line, settings)),
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
///    and this function continues reading.
/// * `buffer`: The buffer that is filled with bytes. Its contents will mostly be overwritten (see `start_offset`
///   as well). It will not be grown by default, unless that is necessary to read at least two lines.
/// * `start_offset`: The amount of bytes at the start of `buffer` that were carried over
///    from the previous read and should not be overwritten.
/// * `separator`: The byte that separates lines.
///
/// # Returns
///
/// * The amount of bytes in `buffer` that can now be interpreted as lines.
///   The remaining bytes must be copied to the start of the buffer for the next invocation,
///   if another invocation is necessary, which is determined by the other return value.
/// * Whether this function should be called again.
fn read_to_buffer(
    file: &mut Box<dyn Read + Send>,
    next_files: &mut impl Iterator<Item = Box<dyn Read + Send>>,
    buffer: &mut Vec<u8>,
    start_offset: usize,
    separator: u8,
) -> (usize, bool) {
    let mut read_target = &mut buffer[start_offset..];
    loop {
        match file.read(read_target) {
            Ok(0) => {
                if read_target.is_empty() {
                    // chunk is full
                    let mut sep_iter = memchr_iter(separator, &buffer).rev();
                    let last_line_end = sep_iter.next();
                    if sep_iter.next().is_some() {
                        // We read enough lines.
                        let end = last_line_end.unwrap();
                        // We want to include the separator here, because it shouldn't be carried over.
                        return (end + 1, true);
                    } else {
                        // We need to read more lines
                        let len = buffer.len();
                        // resize the vector to 10 KB more
                        buffer.resize(len + 1024 * 10, 0);
                        read_target = &mut buffer[len..];
                    }
                } else {
                    // This file is empty.
                    if let Some(next_file) = next_files.next() {
                        // There is another file.
                        *file = next_file;
                    } else {
                        // This was the last file.
                        let leftover_len = read_target.len();
                        return (buffer.len() - leftover_len, false);
                    }
                }
            }
            Ok(n) => {
                read_target = &mut read_target[n..];
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => {
                // retry
            }
            Err(e) => {
                crash!(1, "{}", e)
            }
        }
    }
}
