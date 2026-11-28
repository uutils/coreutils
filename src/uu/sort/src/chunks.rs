// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore ELEMS

//! Utilities for reading files as chunks.

#![allow(dead_code)]
// Ignores non-used warning for `borrow_buffer` in `Chunk`

use std::{
    io::{ErrorKind, Read},
    ops::Range,
    sync::mpsc::SyncSender,
};

use memchr::{memchr, memchr_iter, memrchr};
use self_cell::self_cell;
use uucore::error::UResult;
use uucore::numeric_str_cmp::NumInfo;

use crate::{GeneralBigDecimalParseResult, GlobalSettings, Line, SortError};

const ALLOC_CHUNK_SIZE: usize = 64 * 1024;
// Keep enough address space available for temporary-file I/O and bookkeeping.
const SPILL_HEADROOM_BYTES: usize = 1024 * 1024;
const MAX_TOKEN_BUFFER_BYTES: usize = 4 * 1024 * 1024;
const MAX_TOKEN_BUFFER_ELEMS: usize = MAX_TOKEN_BUFFER_BYTES / size_of::<Range<usize>>();

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
    pub token_buffer: Vec<Range<usize>>,
    pub line_count_hint: usize,
}

#[derive(Debug, Default)]
pub struct LineData<'a> {
    pub selections: Vec<&'a [u8]>,
    pub num_infos: Vec<NumInfo>,
    pub parsed_floats: Vec<GeneralBigDecimalParseResult>,
    pub line_num_floats: Vec<Option<f64>>,
    /// Arena buffer holding all collation sort keys concatenated.
    pub collation_key_buffer: Vec<u8>,
    /// End offsets into `collation_key_buffer` for each line's sort key.
    pub collation_key_ends: Vec<usize>,
}

impl LineData<'_> {
    /// Get the collation sort key for a line at the given index.
    pub fn collation_key(&self, index: usize) -> &[u8] {
        let start = if index == 0 {
            0
        } else {
            self.collation_key_ends[index - 1]
        };
        let end = self.collation_key_ends[index];
        &self.collation_key_buffer[start..end]
    }
}

impl Chunk {
    /// Destroy this chunk and return its components to be reused.
    pub fn recycle(mut self) -> RecycledChunk {
        let mut recycled_contents = self.with_dependent_mut(|_, contents| {
            contents.lines.clear();
            contents.line_data.selections.clear();
            contents.line_data.num_infos.clear();
            contents.line_data.parsed_floats.clear();
            contents.line_data.line_num_floats.clear();
            contents.line_data.collation_key_buffer.clear();
            contents.line_data.collation_key_ends.clear();
            contents.token_buffer.clear();
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
            RecycledChunk {
                lines,
                selections,
                num_infos: std::mem::take(&mut contents.line_data.num_infos),
                parsed_floats: std::mem::take(&mut contents.line_data.parsed_floats),
                line_num_floats: std::mem::take(&mut contents.line_data.line_num_floats),
                collation_key_buffer: std::mem::take(&mut contents.line_data.collation_key_buffer),
                collation_key_ends: std::mem::take(&mut contents.line_data.collation_key_ends),
                token_buffer: std::mem::take(&mut contents.token_buffer),
                line_count_hint: contents.line_count_hint,
                // buffer is set below after we consume `self`
                buffer: Vec::new(),
            }
        });
        recycled_contents.buffer = self.into_owner();
        recycled_contents
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
    collation_key_buffer: Vec<u8>,
    collation_key_ends: Vec<usize>,
    token_buffer: Vec<Range<usize>>,
    line_count_hint: usize,
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
            collation_key_buffer: Vec::new(),
            collation_key_ends: Vec::new(),
            token_buffer: Vec::new(),
            line_count_hint: 0,
            buffer: vec![0; capacity],
        }
    }

    fn try_reserve_lines(&mut self, count: usize, settings: &GlobalSettings) -> bool {
        let precomputed = &settings.precomputed;
        if !try_reserve(&mut self.lines, count)
            || !try_reserve(
                &mut self.selections,
                count.saturating_mul(precomputed.selections_per_line),
            )
            || !try_reserve(
                &mut self.num_infos,
                count.saturating_mul(precomputed.num_infos_per_line),
            )
            || !try_reserve(
                &mut self.parsed_floats,
                count.saturating_mul(precomputed.floats_per_line),
            )
        {
            return false;
        }
        if precomputed.whole_line_numeric && !try_reserve(&mut self.line_num_floats, count) {
            return false;
        }
        #[cfg(feature = "i18n-collator")]
        if precomputed.fast_locale_collation {
            return try_reserve(&mut self.collation_key_ends, count);
        }
        true
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ReadStatus {
    End,
    More,
    Spill,
}

/// Read a chunk, parse lines and send them.
///
/// No empty chunk will be sent. If the input fits exactly into a buffer,
/// the next call can discover EOF. A spill status tells the caller to write
/// the in-memory runs before reading another chunk.
///
/// # Arguments
///
/// (see also `read_to_buffer` for a more detailed documentation)
///
/// * `sender`: The sender to send the lines to the sorter.
/// * `recycled_chunk`: The recycled chunk, as returned by `Chunk::recycle`.
/// * `max_buffer_size`: How big `buffer` can be.
/// * `carry_over`: The bytes that must be carried over in between invocations.
/// * `file`: The current file.
/// * `next_files`: What `file` should be updated to next.
/// * `separator`: The line separator.
/// * `settings`: The global settings.
#[allow(clippy::too_many_arguments)]
pub fn read<T: Read>(
    sender: &SyncSender<Chunk>,
    mut recycled_chunk: RecycledChunk,
    max_buffer_size: Option<usize>,
    carry_over: &mut Vec<u8>,
    file: &mut T,
    next_files: &mut impl Iterator<Item = UResult<T>>,
    separator: u8,
    settings: &GlobalSettings,
) -> UResult<ReadStatus> {
    let mut headroom = Vec::<u8>::new();
    let mut buffer = std::mem::take(&mut recycled_chunk.buffer);
    let limit = max_buffer_size.unwrap_or(buffer.len());
    if let Some(additional) = carry_over
        .len()
        .checked_sub(buffer.len())
        .filter(|&size| size > 0)
    {
        if !try_reserve(&mut buffer, additional) {
            return Err(out_of_memory().into());
        }
        // Separate carry_over and copy them to avoid cost of 0 fill buffer
        buffer.extend_from_slice(&carry_over[buffer.len()..]);
    }
    buffer[..carry_over.len()].copy_from_slice(carry_over);
    let start_offset = carry_over.len();
    let BufferRead {
        mut read,
        filled,
        mut status,
        mut line_count,
    } = read_to_buffer(
        file,
        next_files,
        &mut buffer,
        ReadBudget {
            limit,
            reserve: |count, bytes| {
                // Avoid reserving spill space for small in-memory sorts.
                if bytes >= SPILL_HEADROOM_BYTES && headroom.capacity() == 0 {
                    let read_ahead = ALLOC_CHUNK_SIZE.saturating_sub(carry_over.len());
                    if carry_over.try_reserve_exact(read_ahead).is_err()
                        || headroom.try_reserve_exact(SPILL_HEADROOM_BYTES).is_err()
                    {
                        return false;
                    }
                }
                recycled_chunk.try_reserve_lines(count, settings)
            },
        },
        start_offset,
        separator,
    )?;
    // Full read windows have already reserved their metadata. At EOF only
    // the final partial window may still need space.
    if read != 0
        && status == ReadStatus::End
        && !recycled_chunk.try_reserve_lines(line_count, settings)
    {
        let (fitted, fitted_line_count) = fit_line_capacity(&buffer[..read], separator, |count| {
            recycled_chunk.try_reserve_lines(count, settings)
        })?;
        if fitted < read {
            status = ReadStatus::Spill;
        }
        read = fitted;
        line_count = fitted_line_count;
    }
    recycled_chunk.line_count_hint = line_count;
    carry_over.clear();
    if status != ReadStatus::End {
        if !try_reserve(carry_over, filled - read) {
            return Err(out_of_memory().into());
        }
        carry_over.extend_from_slice(&buffer[read..filled]);
    }

    drop(headroom);

    if read != 0 {
        let RecycledChunk {
            lines,
            selections,
            num_infos,
            parsed_floats,
            line_num_floats,
            collation_key_buffer,
            collation_key_ends,
            mut token_buffer,
            line_count_hint,
            ..
        } = recycled_chunk;
        let mut line_count_hint = line_count_hint;
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
                collation_key_buffer,
                collation_key_ends,
            };
            parse_lines(
                read,
                &mut lines,
                &mut line_data,
                &mut token_buffer,
                &mut line_count_hint,
                separator,
                settings,
            );
            Ok(ChunkContents {
                lines,
                line_data,
                token_buffer,
                line_count_hint,
            })
        });
        sender.send(payload?).unwrap();
    }
    Ok(status)
}

/// Find a prefix whose line descriptors fit, leaving whole records for the next read.
fn fit_line_capacity(
    input: &[u8],
    separator: u8,
    mut reserve: impl FnMut(usize) -> bool,
) -> UResult<(usize, usize)> {
    let mut end = input.len();
    loop {
        let chunk = &input[..end];
        let contents = chunk.strip_suffix(&[separator]).unwrap_or(chunk);
        let count = memchr_iter(separator, contents).count() + 1;
        if reserve(count) {
            return Ok((end, count));
        }

        // The last byte is excluded so each retry makes progress and the
        // suffix can always begin at a record boundary.
        let (before, after) = chunk[..end - 1].split_at((end - 1) / 2);
        let split = memrchr(separator, before)
            .or_else(|| memchr(separator, after).map(|index| before.len() + index));
        end = split.ok_or_else(out_of_memory)? + 1;
    }
}

fn try_reserve<T>(buffer: &mut Vec<T>, additional: usize) -> bool {
    buffer.try_reserve(additional).is_ok() || buffer.try_reserve_exact(additional).is_ok()
}

fn spillable_prefix(read: usize, filled: usize, line_count: usize) -> UResult<BufferRead> {
    if read == 0 {
        return Err(out_of_memory().into());
    }
    Ok(BufferRead {
        read,
        filled,
        status: ReadStatus::Spill,
        line_count,
    })
}

fn out_of_memory() -> SortError {
    SortError::ChunkReadFailed {
        error: ErrorKind::OutOfMemory.into(),
    }
}

/// Split `read` into `Line`s, and add them to `lines`.
fn parse_lines<'a>(
    read: &'a [u8],
    lines: &mut Vec<Line<'a>>,
    line_data: &mut LineData<'a>,
    token_buffer: &mut Vec<Range<usize>>,
    line_count_hint: &mut usize,
    separator: u8,
    settings: &GlobalSettings,
) {
    let read = read.strip_suffix(&[separator]).unwrap_or(read);

    assert!(lines.is_empty());
    assert!(line_data.selections.is_empty());
    assert!(line_data.num_infos.is_empty());
    assert!(line_data.parsed_floats.is_empty());
    assert!(line_data.line_num_floats.is_empty());
    assert!(line_data.collation_key_buffer.is_empty());
    assert!(line_data.collation_key_ends.is_empty());
    token_buffer.clear();
    let estimated = if *line_count_hint == 0 {
        memchr_iter(separator, read).count() + 1
    } else {
        *line_count_hint
    };
    lines.reserve(estimated);
    if settings.precomputed.selections_per_line > 0 {
        line_data
            .selections
            .reserve(estimated.saturating_mul(settings.precomputed.selections_per_line));
    }
    if settings.precomputed.num_infos_per_line > 0 {
        line_data
            .num_infos
            .reserve(estimated.saturating_mul(settings.precomputed.num_infos_per_line));
    }
    if settings.precomputed.floats_per_line > 0 {
        line_data
            .parsed_floats
            .reserve(estimated.saturating_mul(settings.precomputed.floats_per_line));
    }
    if settings.precomputed.whole_line_numeric {
        line_data.line_num_floats.reserve(estimated);
    }
    let mut start = 0usize;
    let mut index = 0usize;
    for sep_idx in memchr_iter(separator, read) {
        let line = &read[start..sep_idx];
        lines.push(Line::create(line, index, line_data, token_buffer, settings));
        index += 1;
        start = sep_idx + 1;
    }
    let line = &read[start..];
    lines.push(Line::create(line, index, line_data, token_buffer, settings));
    *line_count_hint = index + 1;
}

struct ReadBudget<F> {
    limit: usize,
    reserve: F,
}

struct BufferRead {
    // Bytes forming complete records and ready to sort.
    read: usize,
    // End of the input actually read. Bytes between read and filled
    // must be carried over, while bytes after filled may be stale.
    filled: usize,
    status: ReadStatus,
    line_count: usize,
}

/// Fill the initialized buffer in stages so each stage can check its memory
/// budget. The read window can grow past the budget to complete long records.
fn read_to_buffer<T: Read>(
    file: &mut T,
    next_files: &mut impl Iterator<Item = UResult<T>>,
    buffer: &mut Vec<u8>,
    budget: ReadBudget<impl FnMut(usize, usize) -> bool>,
    start_offset: usize,
    separator: u8,
) -> UResult<BufferRead> {
    let ReadBudget { limit, mut reserve } = budget;
    // Keep the initialized bytes of recycled buffers. Advance a separate
    // read window so that reusing them does not zero-fill the same bytes again.
    let mut end = buffer.len().min(ALLOC_CHUNK_SIZE.max(start_offset));
    let mut read_target = &mut buffer[start_offset..end];
    let mut newline_search_offset = 0;
    let mut found_newline = false;
    let mut scanned = 0;
    let mut line_count = 0;
    let mut reserved_end = 0;
    let mut reserved_line_count = 0;
    loop {
        match file.read(read_target) {
            Ok(0) => {
                if read_target.is_empty() {
                    // Reserve metadata before growing again so the last fitting
                    // prefix can be spilled without allocating a large read-ahead.
                    let scanned_bytes = &buffer[scanned..end];
                    line_count += memchr_iter(separator, scanned_bytes).count();
                    if !reserve(line_count, end) {
                        return spillable_prefix(reserved_end, end, reserved_line_count);
                    }
                    if let Some(pos) = memrchr(separator, scanned_bytes) {
                        reserved_end = scanned + pos + 1;
                        reserved_line_count = line_count;
                    }
                    scanned = end;

                    if end >= limit {
                        let mut separators =
                            memchr_iter(separator, &buffer[newline_search_offset..end]).rev();
                        if let Some(last_line_end) = separators.next() {
                            if found_newline || separators.next().is_some() {
                                return Ok(BufferRead {
                                    read: newline_search_offset + last_line_end + 1,
                                    filled: end,
                                    status: ReadStatus::More,
                                    line_count,
                                });
                            }
                            found_newline = true;
                        }
                        newline_search_offset = end;
                    }

                    let next_end = end + ALLOC_CHUNK_SIZE;
                    if next_end > buffer.len() {
                        if !try_reserve(buffer, next_end - buffer.len()) {
                            return spillable_prefix(reserved_end, end, reserved_line_count);
                        }
                        buffer.resize(next_end, 0);
                    }
                    read_target = &mut buffer[end..next_end];
                    end = next_end;
                } else {
                    // This file has been fully read.
                    let mut leftover_len = read_target.len();
                    // Decide from buffered bytes: a file may span calls, so a 0-byte read can
                    // still leave an unterminated tail that must be separated from the next file.
                    let read_len = end - leftover_len;
                    if read_len > 0 && buffer[read_len - 1] != separator {
                        // The data so far does not end with a separator. We have to insert one.
                        // `read_target` is non-empty here, so `buffer[read_len]` is in bounds.
                        buffer[read_len] = separator;
                        leftover_len -= 1;
                    }
                    let read_len = end - leftover_len;
                    read_target = &mut buffer[read_len..end];
                    if let Some(next_file) = next_files.next() {
                        // There is another file.
                        *file = next_file?;
                    } else {
                        // This was the last file.
                        let read_len = end - leftover_len;
                        line_count += memchr_iter(separator, &buffer[scanned..read_len]).count();
                        return Ok(BufferRead {
                            read: read_len,
                            filled: read_len,
                            status: ReadStatus::End,
                            line_count,
                        });
                    }
                }
            }
            Ok(n) => {
                read_target = &mut read_target[n..];
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => {
                // retry
            }
            Err(error) => {
                return Err(SortError::ChunkReadFailed { error }.into());
            }
        }
    }
}

/// Parse a buffer into a `ChunkContents` suitable for `Chunk::try_new`.
/// Used by the WASI single-threaded sort path.
#[cfg(target_os = "wasi")]
pub fn parse_into_chunk<'a>(
    buffer: &'a [u8],
    separator: u8,
    settings: &GlobalSettings,
) -> ChunkContents<'a> {
    let mut lines = Vec::new();
    let mut line_data = LineData::default();
    let mut token_buffer = Vec::new();
    let mut line_count_hint = 0;
    parse_lines(
        buffer,
        &mut lines,
        &mut line_data,
        &mut token_buffer,
        &mut line_count_hint,
        separator,
        settings,
    );
    ChunkContents {
        lines,
        line_data,
        token_buffer,
        line_count_hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recycled_buffer_leaves_unused_initialized_bytes_untouched() {
        let mut buffer = vec![0xff; 2 * ALLOC_CHUNK_SIZE];
        let input = b"x\n".repeat(3 * ALLOC_CHUNK_SIZE / 4);
        let result = read_to_buffer(
            &mut input.as_slice(),
            &mut std::iter::empty(),
            &mut buffer,
            ReadBudget {
                limit: 2 * ALLOC_CHUNK_SIZE,
                reserve: |_, _| true,
            },
            0,
            b'\n',
        )
        .unwrap();

        assert!(result.status == ReadStatus::End);
        assert_eq!(result.line_count, input.len() / 2);
        assert_eq!(result.read, input.len());
        assert_eq!(result.filled, input.len());
        assert_eq!(&buffer[..result.read], input.as_slice());
        assert!(buffer[result.filled..].iter().all(|&byte| byte == 0xff));
    }

    #[test]
    fn complete_prefix_reports_only_its_reserved_records() {
        let mut input = b"x\n".repeat(ALLOC_CHUNK_SIZE);
        input[ALLOC_CHUNK_SIZE - 1] = b'x';
        let line_count = ALLOC_CHUNK_SIZE / 2 - 1;

        for (limit, status) in [
            (ALLOC_CHUNK_SIZE, ReadStatus::More),
            (2 * ALLOC_CHUNK_SIZE, ReadStatus::Spill),
        ] {
            let mut buffer = vec![0; 2 * ALLOC_CHUNK_SIZE];
            let result = read_to_buffer(
                &mut input.as_slice(),
                &mut std::iter::empty(),
                &mut buffer,
                ReadBudget {
                    limit,
                    reserve: |count, _| count <= line_count,
                },
                0,
                b'\n',
            )
            .unwrap();

            assert!(result.status == status);
            assert_eq!(result.read, ALLOC_CHUNK_SIZE - 2);
            assert_eq!(result.line_count, line_count);
        }
    }

    #[test]
    fn eof_counts_an_unterminated_record_once() {
        let mut buffer = vec![0; ALLOC_CHUNK_SIZE];
        let result = read_to_buffer(
            &mut b"b\na".as_slice(),
            &mut std::iter::empty(),
            &mut buffer,
            ReadBudget {
                limit: ALLOC_CHUNK_SIZE,
                reserve: |_, _| true,
            },
            0,
            b'\n',
        )
        .unwrap();

        assert!(result.status == ReadStatus::End);
        assert_eq!(result.line_count, 2);
        assert_eq!(&buffer[..result.read], b"b\na\n");
    }

    #[test]
    fn retry_uses_fewer_complete_records() {
        for &separator in b"\n\0" {
            let input = [b'b', separator, b'a', separator, b'c', separator, b'd'];
            let mut attempts = Vec::new();
            let fitted = fit_line_capacity(&input, separator, |count| {
                attempts.push(count);
                count <= 2
            })
            .unwrap();

            assert_eq!(attempts, [4, 1]);
            assert_eq!(fitted, (2, 1));
            assert_eq!(
                &input[fitted.0..],
                &[b'a', separator, b'c', separator, b'd']
            );
        }
    }

    #[test]
    fn retry_keeps_an_indivisible_record_intact() {
        assert!(fit_line_capacity(b"one record\n", b'\n', |_| false).is_err());
    }
}
