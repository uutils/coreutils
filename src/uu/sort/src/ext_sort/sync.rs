// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Synchronous external sort for targets without thread support
//! (e.g. `wasm32-wasip1`).
//!
//! Uses the same chunked sort-write-merge strategy as the threaded version,
//! but reads and sorts each chunk sequentially on the calling thread.

use std::cmp::Ordering;
use std::io::{Read, Write, stderr};

use itertools::Itertools;
use uucore::error::UResult;

use crate::Output;
use crate::chunks::{self, Chunk, RecycledChunk};
use crate::merge::{self, WriteablePlainTmpFile, WriteableTmpFile};
use crate::tmp_dir::TmpDirWrapper;
use crate::{GlobalSettings, compare_by, print_sorted, sort_by};

use super::{DEFAULT_BUF_SIZE, write};

pub fn ext_sort(
    files: &mut impl Iterator<Item = UResult<Box<dyn Read + Send>>>,
    settings: &GlobalSettings,
    output: Output,
    tmp_dir: &mut TmpDirWrapper,
) -> UResult<()> {
    let separator = settings.line_ending.into();
    let mut buffer_size = match settings.buffer_size {
        size if size <= 512 * 1024 * 1024 => size,
        size => size / 2,
    };
    if !settings.buffer_size_is_explicit {
        buffer_size = buffer_size.max(8 * 1024 * 1024);
    }

    if settings.compress_prog.is_some() {
        let _ = writeln!(
            stderr(),
            "sort: warning: --compress-program is ignored on this platform"
        );
    }

    let mut file = files.next().unwrap()?;
    let mut carry_over = vec![];

    // Read and sort first chunk.
    let (first, cont) = chunks::read_to_chunk(
        RecycledChunk::new(buffer_size.min(DEFAULT_BUF_SIZE)),
        Some(buffer_size),
        &mut carry_over,
        &mut file,
        files,
        separator,
        settings,
    )?;
    let Some(mut first) = first else {
        return Ok(()); // empty input
    };
    first.with_dependent_mut(|_, c| sort_by(&mut c.lines, settings, &c.line_data));

    if !cont {
        // All input fits in one chunk.
        return print_chunk(&first, settings, output);
    }

    // Read and sort second chunk.
    let (second, cont) = chunks::read_to_chunk(
        RecycledChunk::new(buffer_size.min(DEFAULT_BUF_SIZE)),
        Some(buffer_size),
        &mut carry_over,
        &mut file,
        files,
        separator,
        settings,
    )?;
    let Some(mut second) = second else {
        return print_chunk(&first, settings, output);
    };
    second.with_dependent_mut(|_, c| sort_by(&mut c.lines, settings, &c.line_data));

    if !cont {
        // All input fits in two chunks — merge in memory.
        return print_two_chunks(first, second, settings, output);
    }

    // More than two chunks: write sorted chunks to temp files, then merge.
    let mut tmp_files: Vec<<WriteablePlainTmpFile as WriteableTmpFile>::Closed> = vec![];

    tmp_files.push(write::<WriteablePlainTmpFile>(
        &first,
        tmp_dir.next_file()?,
        settings.compress_prog.as_deref(),
        separator,
    )?);
    drop(first);

    tmp_files.push(write::<WriteablePlainTmpFile>(
        &second,
        tmp_dir.next_file()?,
        settings.compress_prog.as_deref(),
        separator,
    )?);
    let mut recycled = second.recycle();

    loop {
        let (chunk, cont) = chunks::read_to_chunk(
            recycled,
            None,
            &mut carry_over,
            &mut file,
            files,
            separator,
            settings,
        )?;
        let Some(mut chunk) = chunk else { break };
        chunk.with_dependent_mut(|_, c| sort_by(&mut c.lines, settings, &c.line_data));
        tmp_files.push(write::<WriteablePlainTmpFile>(
            &chunk,
            tmp_dir.next_file()?,
            settings.compress_prog.as_deref(),
            separator,
        )?);
        recycled = chunk.recycle();
        if !cont {
            break;
        }
    }

    merge::merge_with_file_limit::<_, _, WriteablePlainTmpFile>(
        tmp_files.into_iter().map(merge::ClosedTmpFile::reopen),
        settings,
        output,
        tmp_dir,
    )
}

/// Print a single sorted chunk.
fn print_chunk(chunk: &Chunk, settings: &GlobalSettings, output: Output) -> UResult<()> {
    if settings.unique {
        print_sorted(
            chunk.lines().iter().dedup_by(|a, b| {
                compare_by(a, b, settings, chunk.line_data(), chunk.line_data()) == Ordering::Equal
            }),
            settings,
            output,
        )
    } else {
        print_sorted(chunk.lines().iter(), settings, output)
    }
}

/// Merge two in-memory chunks and print.
fn print_two_chunks(a: Chunk, b: Chunk, settings: &GlobalSettings, output: Output) -> UResult<()> {
    let merged_iter = a.lines().iter().map(|line| (line, &a)).merge_by(
        b.lines().iter().map(|line| (line, &b)),
        |(line_a, a), (line_b, b)| {
            compare_by(line_a, line_b, settings, a.line_data(), b.line_data()) != Ordering::Greater
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
        )
    } else {
        print_sorted(merged_iter.map(|(line, _)| line), settings, output)
    }
}
