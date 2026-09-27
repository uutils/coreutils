// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! I/O processing infrastructure for tr operations with SIMD optimizations

use crate::operation::ChunkProcessor;
use std::io::{BufRead, Write};
use uucore::error::{FromIo, UResult};
use uucore::translate;

/// Helper to detect single-character operations for optimization
pub fn find_single_change<T, F>(table: &[T; 256], check: F) -> Option<(u8, T)>
where
    F: Fn(usize, &T) -> bool,
    T: Copy,
{
    let matches: Vec<_> = table
        .iter()
        .enumerate()
        .filter_map(|(i, val)| check(i, val).then_some((i as u8, *val)))
        .take(2)
        .collect();

    (matches.len() == 1).then(|| matches[0])
}

/// SIMD-optimized single character replacement
#[inline]
pub fn process_single_char_replace(
    input: &[u8],
    output: &mut Vec<u8>,
    source_char: u8,
    target_char: u8,
) {
    let count = bytecount::count(input, source_char);
    if count == 0 {
        output.extend_from_slice(input);
    } else if count == input.len() {
        output.resize(output.len() + input.len(), target_char);
    } else {
        output.extend(
            input
                .iter()
                .map(|&b| if b == source_char { target_char } else { b }),
        );
    }
}

/// SIMD-optimized delete operation for single character
///
/// `keep` must be false for `delete_char` only.
pub fn process_single_delete(
    input: &[u8],
    output: &mut Vec<u8>,
    delete_char: u8,
    keep: &[bool; 256],
) {
    let count = bytecount::count(input, delete_char);
    if count == 0 {
        output.extend_from_slice(input);
    } else if count < input.len() / 128 {
        // Below one match per 128 bytes, copying the runs between matches
        // beats `process_delete`.
        let mut start = 0;
        for pos in memchr::memchr_iter(delete_char, input) {
            output.extend_from_slice(&input[start..pos]);
            start = pos + 1;
        }
        output.extend_from_slice(&input[start..]);
    } else if count < input.len() {
        process_delete(input, output, keep);
    }
    // If count == input.len(), all deleted, output nothing
}

/// Append to `output` the bytes of `input` whose `keep` entry is true.
pub fn process_delete(input: &[u8], output: &mut Vec<u8>, keep: &[bool; 256]) {
    // The index is always below `BLOCK` (a power of two), so the modulo is a
    // mask that only serves to drop the bounds check.
    const BLOCK: usize = 1024;
    // Below one kept byte in `FEW`, a branch per byte is well predicted and
    // stores less.
    const FEW: usize = 64;
    let mut block = [0; BLOCK];
    // Guess for the first block from its start, then go by the previous one.
    let start = &input[..input.len().min(256)];
    let mut few_kept = start.iter().filter(|&&b| keep[b as usize]).count() * FEW < start.len();
    for chunk in input.chunks(BLOCK) {
        let mut kept = 0;
        if few_kept {
            // Only store the kept bytes.
            for &b in chunk {
                if keep[b as usize] {
                    block[kept % BLOCK] = b;
                    kept += 1;
                }
            }
        } else {
            // Store every byte and only advance past kept ones: no branch.
            for &b in chunk {
                block[kept % BLOCK] = b;
                kept += usize::from(keep[b as usize]);
            }
        }
        output.extend_from_slice(&block[..kept]);
        few_kept = kept * FEW < chunk.len();
    }
}

/// Unified I/O processing for all operations
pub fn process_input<R, W, P>(input: &mut R, output: &mut W, processor: &P) -> UResult<()>
where
    R: BufRead,
    W: Write,
    P: ChunkProcessor + ?Sized,
{
    const BUFFER_SIZE: usize = 32768;
    let mut buf = [0; BUFFER_SIZE];
    let mut output_buf = Vec::with_capacity(BUFFER_SIZE);

    loop {
        let length = match input.read(&mut buf[..]) {
            Ok(0) => break,
            Ok(len) => len,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.map_err_context(|| translate!("tr-error-read-error"))),
        };

        output_buf.clear();
        processor.process_chunk(&buf[..length], &mut output_buf);

        if !output_buf.is_empty() {
            write_output(output, &output_buf)?;
        }
    }

    Ok(())
}

/// Helper function to handle platform-specific write operations
#[inline]
pub fn write_output<W: Write>(output: &mut W, buf: &[u8]) -> UResult<()> {
    #[cfg(not(windows))]
    return output
        .write_all(buf)
        .map_err_context(|| translate!("tr-error-write-error"));

    #[cfg(windows)]
    match output.write_all(buf) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => {
            std::process::exit(13);
        }
        Err(err) => Err(err.map_err_context(|| translate!("tr-error-write-error"))),
    }
}
