// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Synchronous ordered-file check for targets without thread support.

use std::cmp::Ordering;
use std::ffi::OsStr;
use std::io::Read;
use std::iter;

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
