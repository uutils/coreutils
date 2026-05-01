// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! I/O processing infrastructure for tr operations with SIMD optimizations

use crate::operation::{AsciiRangeTranslate, ChunkProcessor};
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
pub fn process_single_delete(input: &[u8], output: &mut Vec<u8>, delete_char: u8) {
    let count = bytecount::count(input, delete_char);
    if count == 0 {
        output.extend_from_slice(input);
    } else if count < input.len() {
        output.extend(input.iter().filter(|&&b| b != delete_char).copied());
    }
    // If count == input.len(), all deleted, output nothing
}

/// Translate a contiguous ASCII byte range by a constant wrapping delta.
#[inline]
pub(crate) fn translate_ascii_range(
    input: &[u8],
    output: &mut Vec<u8>,
    range: AsciiRangeTranslate,
) {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if input.len() >= 32 && std::is_x86_feature_detected!("avx2") {
        unsafe {
            translate_ascii_range_avx2(input, output, range);
        }
        return;
    }

    translate_ascii_range_scalar(input, output, range);
}

#[inline]
fn translate_ascii_range_scalar(input: &[u8], output: &mut Vec<u8>, range: AsciiRangeTranslate) {
    output.extend(input.iter().map(|&byte| {
        if (range.start..=range.end).contains(&byte) {
            byte.wrapping_add(range.delta)
        } else {
            byte
        }
    }));
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
/// # Safety
///
/// Callers must only call this function when AVX2 is available on the current CPU.
unsafe fn translate_ascii_range_avx2(
    input: &[u8],
    output: &mut Vec<u8>,
    range: AsciiRangeTranslate,
) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{
        _mm256_add_epi8, _mm256_and_si256, _mm256_blendv_epi8, _mm256_cmpgt_epi8,
        _mm256_loadu_si256, _mm256_set1_epi8, _mm256_storeu_si256, _mm256_xor_si256,
    };
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{
        _mm256_add_epi8, _mm256_and_si256, _mm256_blendv_epi8, _mm256_cmpgt_epi8,
        _mm256_loadu_si256, _mm256_set1_epi8, _mm256_storeu_si256, _mm256_xor_si256,
    };

    let start_len = output.len();
    output.resize(start_len + input.len(), 0);

    let start_minus_one = (range.start.wrapping_sub(1)) as i8;
    let start = _mm256_set1_epi8(start_minus_one);
    let end = _mm256_set1_epi8(range.end as i8);
    let delta = _mm256_set1_epi8(range.delta as i8);
    let all_bits = _mm256_set1_epi8(-1);

    let mut offset = 0usize;
    while offset + 32 <= input.len() {
        let bytes = unsafe { _mm256_loadu_si256(input.as_ptr().add(offset).cast()) };
        let greater_equal_start = _mm256_cmpgt_epi8(bytes, start);
        let greater_than_end = _mm256_cmpgt_epi8(bytes, end);
        let less_equal_end = _mm256_xor_si256(greater_than_end, all_bits);
        let in_range = _mm256_and_si256(greater_equal_start, less_equal_end);
        let translated = _mm256_add_epi8(bytes, delta);
        let blended = _mm256_blendv_epi8(bytes, translated, in_range);
        unsafe {
            _mm256_storeu_si256(output.as_mut_ptr().add(start_len + offset).cast(), blended);
        }
        offset += 32;
    }

    for (index, &byte) in input[offset..].iter().enumerate() {
        output[start_len + offset + index] = if (range.start..=range.end).contains(&byte) {
            byte.wrapping_add(range.delta)
        } else {
            byte
        };
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
    #[cfg(not(target_os = "windows"))]
    return output
        .write_all(buf)
        .map_err_context(|| translate!("tr-error-write-error"));

    #[cfg(target_os = "windows")]
    match output.write_all(buf) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => {
            std::process::exit(13);
        }
        Err(err) => Err(err.map_err_context(|| translate!("tr-error-write-error"))),
    }
}
