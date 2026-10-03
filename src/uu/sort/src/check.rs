// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Check if a file is ordered

use crate::{
    GlobalSettings, SortError,
    chunks::{self, Chunk, RecycledChunk},
    compare_by, open,
};
use itertools::Itertools;
use std::{cmp::Ordering, ffi::OsStr};
#[cfg(not(target_os = "wasi"))]
use std::{
    io::Read,
    iter,
    sync::mpsc::{Receiver, SyncSender, sync_channel},
    thread,
};
use uucore::error::UResult;

fn buffer_size(settings: &GlobalSettings) -> usize {
    if settings.buffer_size < 100 * 1024 {
        // when the buffer size is smaller than 100KiB we choose it instead of the default.
        // this improves testability.
        settings.buffer_size
    } else {
        100 * 1024
    }
}

/// Given the chunks of a file (in order), find the first pair of adjacent
/// lines that violates the requested ordering and report it as a
/// [`SortError::Disorder`].
#[cfg(target_os = "wasi")]
fn check_chunks(
    path: &OsStr,
    settings: &GlobalSettings,
    max_allowed_cmp: Ordering,
    chunks: impl Iterator<Item = Chunk>,
) -> UResult<()> {
    let mut prev_chunk: Option<Chunk> = None;
    let mut line_idx = 0;
    for chunk in chunks {
        line_idx += 1;
        if let Some(prev_chunk) = &prev_chunk {
            // Check if the first element of the new chunk is greater than the last
            // element from the previous chunk
            let prev_last = prev_chunk.lines().last().unwrap();
            let new_first = chunk.lines().first().unwrap();

            if compare_by(
                prev_last,
                new_first,
                settings,
                prev_chunk.line_data(),
                chunk.line_data(),
            ) > max_allowed_cmp
            {
                return Err(SortError::Disorder {
                    file: path.to_owned(),
                    line_number: line_idx,
                    line: String::from_utf8_lossy(new_first.line).into_owned(),
                    silent: settings.check_silent,
                }
                .into());
            }
        }

        for (a, b) in chunk.lines().iter().tuple_windows() {
            line_idx += 1;
            if compare_by(a, b, settings, chunk.line_data(), chunk.line_data()) > max_allowed_cmp {
                return Err(SortError::Disorder {
                    file: path.to_owned(),
                    line_number: line_idx,
                    line: String::from_utf8_lossy(b.line).into_owned(),
                    silent: settings.check_silent,
                }
                .into());
            }
        }

        prev_chunk = Some(chunk);
    }
    Ok(())
}

/// Check if the file at `path` is ordered.
///
/// # Returns
///
/// The code we should exit with.
#[cfg(not(target_os = "wasi"))]
pub fn check(path: &OsStr, settings: &GlobalSettings) -> UResult<()> {
    let max_allowed_cmp = if settings.unique {
        // If `unique` is enabled, the previous line must compare _less_ to the next one.
        Ordering::Less
    } else {
        // Otherwise, the line previous line must compare _less or equal_ to the next one.
        Ordering::Equal
    };
    let file = open(path)?;
    let (recycled_sender, recycled_receiver) = sync_channel(2);
    let (loaded_sender, loaded_receiver) = sync_channel(2);
    thread::spawn({
        let settings = settings.clone();
        move || reader(file, &recycled_receiver, &loaded_sender, &settings)
    });
    for _ in 0..2 {
        let _ = recycled_sender.send(RecycledChunk::new(buffer_size(settings)));
    }

    let mut prev_chunk: Option<Chunk> = None;
    let mut line_idx = 0;
    let mut result: UResult<()> = Ok(());
    // Note that we iterate over a reference, so that `loaded_receiver` is still alive
    // once we stop: `chunks::read` unwraps its `send`, so dropping our end while the
    // reader thread is still going would panic it. Since we stop at the *first*
    // disorder, the reader is usually still working at that point, so we shut it down
    // in an orderly fashion below instead of just dropping our end.
    'outer: for chunk in &loaded_receiver {
        line_idx += 1;
        if let Some(prev_chunk) = prev_chunk.take() {
            // Check if the first element of the new chunk is greater than the last
            // element from the previous chunk
            let prev_last = prev_chunk.lines().last().unwrap();
            let new_first = chunk.lines().first().unwrap();

            if compare_by(
                prev_last,
                new_first,
                settings,
                prev_chunk.line_data(),
                chunk.line_data(),
            ) > max_allowed_cmp
            {
                result = Err(SortError::Disorder {
                    file: path.to_owned(),
                    line_number: line_idx,
                    line: String::from_utf8_lossy(new_first.line).into_owned(),
                    silent: settings.check_silent,
                }
                .into());
                break 'outer;
            }
            let _ = recycled_sender.send(prev_chunk.recycle());
        }

        for (a, b) in chunk.lines().iter().tuple_windows() {
            line_idx += 1;
            if compare_by(a, b, settings, chunk.line_data(), chunk.line_data()) > max_allowed_cmp {
                result = Err(SortError::Disorder {
                    file: path.to_owned(),
                    line_number: line_idx,
                    line: String::from_utf8_lossy(b.line).into_owned(),
                    silent: settings.check_silent,
                }
                .into());
                break 'outer;
            }
        }

        prev_chunk = Some(chunk);
    }

    // Stop handing out buffers, so the reader runs out of work, then drain anything it
    // has already produced. This lets its in-flight `send` complete instead of failing,
    // and terminates because the reader can only own the (at most two) recycled chunks
    // that are still outstanding.
    drop(recycled_sender);
    while loaded_receiver.recv().is_ok() {}

    result
}

/// Check if the file at `path` is ordered.
///
/// WASI has no thread support, so this reads every chunk up front on the
/// current thread instead of streaming them from a background reader thread.
///
/// # Returns
///
/// The code we should exit with.
#[cfg(target_os = "wasi")]
pub fn check(path: &OsStr, settings: &GlobalSettings) -> UResult<()> {
    let max_allowed_cmp = if settings.unique {
        Ordering::Less
    } else {
        Ordering::Equal
    };
    let chunks = read_all_chunks(path, settings)?;
    check_chunks(path, settings, max_allowed_cmp, chunks.into_iter())
}

/// The function running on the reader thread.
#[cfg(not(target_os = "wasi"))]
fn reader(
    mut file: Box<dyn Read + Send>,
    receiver: &Receiver<RecycledChunk>,
    sender: &SyncSender<Chunk>,
    settings: &GlobalSettings,
) -> UResult<()> {
    let mut carry_over = vec![];
    for recycled_chunk in receiver {
        let should_continue = chunks::read(
            sender,
            recycled_chunk,
            None,
            &mut carry_over,
            &mut file,
            &mut iter::empty(),
            settings.line_ending.into(),
            settings,
        )?;
        if !should_continue {
            break;
        }
    }
    Ok(())
}

/// Read every chunk of `path` up front, without any recycling or background
/// thread. Used on WASI, which has no thread support.
#[cfg(target_os = "wasi")]
fn read_all_chunks(path: &OsStr, settings: &GlobalSettings) -> UResult<Vec<Chunk>> {
    let mut file = open(path)?;
    let mut carry_over = vec![];
    let mut chunks = Vec::new();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    loop {
        let recycled = RecycledChunk::new(buffer_size(settings));
        let should_continue = chunks::read(
            &sender,
            recycled,
            None,
            &mut carry_over,
            &mut file,
            &mut std::iter::empty(),
            settings.line_ending.into(),
            settings,
        )?;
        while let Ok(chunk) = receiver.try_recv() {
            chunks.push(chunk);
        }
        if !should_continue {
            break;
        }
    }
    Ok(chunks)
}
