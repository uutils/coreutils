// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Multi-threaded ordered-file check: a reader thread streams chunks while
//! the main thread compares the boundary between consecutive chunks.

use std::cmp::Ordering;
use std::ffi::OsStr;
use std::io::Read;
use std::iter;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread;

use itertools::Itertools;
use uucore::error::UResult;

use crate::{
    GlobalSettings, SortError,
    chunks::{self, Chunk, RecycledChunk},
    compare_by,
};

pub(super) fn check(
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
