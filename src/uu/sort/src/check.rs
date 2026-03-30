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
use std::{
    cmp::Ordering,
    ffi::OsStr,
    io::Read,
    iter,
};
#[cfg(not(all(target_os = "wasi", not(target_feature = "atomics"))))]
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
#[cfg(not(all(target_os = "wasi", not(target_feature = "atomics"))))]
use std::thread;
use uucore::error::UResult;

/// Check if the file at `path` is ordered.
///
/// # Returns
///
/// The code we should exit with.
pub fn check(path: &OsStr, settings: &GlobalSettings) -> UResult<()> {
    let max_allowed_cmp = if settings.unique {
        Ordering::Less
    } else {
        Ordering::Equal
    };
    let file = open(path)?;
    let chunk_size = if settings.buffer_size < 100 * 1024 {
        settings.buffer_size
    } else {
        100 * 1024
    };

    #[cfg(not(all(target_os = "wasi", not(target_feature = "atomics"))))]
    {
        check_threaded(path, settings, max_allowed_cmp, file, chunk_size)
    }
    #[cfg(all(target_os = "wasi", not(target_feature = "atomics")))]
    {
        check_sync(path, settings, max_allowed_cmp, file, chunk_size)
    }
}

#[cfg(not(all(target_os = "wasi", not(target_feature = "atomics"))))]
fn check_threaded(
    path: &OsStr,
    settings: &GlobalSettings,
    max_allowed_cmp: Ordering,
    file: Box<dyn Read + Send>,
    chunk_size: usize,
) -> UResult<()> {
    let (recycled_sender, recycled_receiver) = sync_channel(2);
    let (loaded_sender, loaded_receiver) = sync_channel(2);
    thread::spawn({
        let settings = settings.clone();
        move || reader(file, &recycled_receiver, &loaded_sender, &settings)
    });
    for _ in 0..2 {
        let _ = recycled_sender.send(RecycledChunk::new(chunk_size));
    }

    let mut prev_chunk: Option<Chunk> = None;
    let mut line_idx = 0;
    for chunk in loaded_receiver {
        line_idx += 1;
        if let Some(prev_chunk) = prev_chunk.take() {
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
            let _ = recycled_sender.send(prev_chunk.recycle());
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

/// The function running on the reader thread.
#[cfg(not(all(target_os = "wasi", not(target_feature = "atomics"))))]
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

/// Synchronous check for targets without thread support.
#[cfg(all(target_os = "wasi", not(target_feature = "atomics")))]
fn check_sync(
    path: &OsStr,
    settings: &GlobalSettings,
    max_allowed_cmp: Ordering,
    mut file: Box<dyn Read + Send>,
    chunk_size: usize,
) -> UResult<()> {
    let separator = settings.line_ending.into();
    let mut carry_over = vec![];
    let mut prev_chunk: Option<Chunk> = None;
    let mut spare_recycled: Option<RecycledChunk> = None;
    let mut line_idx = 0;

    loop {
        let recycled = spare_recycled
            .take()
            .unwrap_or_else(|| RecycledChunk::new(chunk_size));

        let (chunk, should_continue) = chunks::read_to_chunk(
            recycled,
            None,
            &mut carry_over,
            &mut file,
            &mut iter::empty(),
            separator,
            settings,
        )?;

        let Some(chunk) = chunk else {
            break;
        };

        line_idx += 1;
        if let Some(prev) = prev_chunk.take() {
            let prev_last = prev.lines().last().unwrap();
            let new_first = chunk.lines().first().unwrap();

            if compare_by(
                prev_last,
                new_first,
                settings,
                prev.line_data(),
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
            spare_recycled = Some(prev.recycle());
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

        if !should_continue {
            break;
        }
    }
    Ok(())
}
