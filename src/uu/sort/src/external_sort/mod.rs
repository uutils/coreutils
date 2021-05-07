use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::SeekFrom;
use std::io::{BufRead, BufReader, BufWriter, Seek, Write};
use std::path::Path;

use tempdir::TempDir;

use super::{GlobalSettings, Line};

/// Iterator that provides sorted `T`s
pub struct ExtSortedIterator {
    buffers: Vec<VecDeque<Line>>,
    chunk_offsets: Vec<u64>,
    max_per_chunk: usize,
    chunks: usize,
    tmp_dir: TempDir,
    settings: GlobalSettings,
    failed: bool,
}

impl Iterator for ExtSortedIterator {
    type Item = Line;

    /// # Errors
    ///
    /// This method can fail due to issues reading intermediate sorted chunks
    /// from disk
    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        // fill up any empty buffers
        let mut empty = true;
        for chunk_num in 0..self.chunks {
            if self.buffers[chunk_num as usize].is_empty() {
                let mut f = crash_if_err!(
                    1,
                    File::open(self.tmp_dir.path().join(chunk_num.to_string()))
                );
                crash_if_err!(1, f.seek(SeekFrom::Start(self.chunk_offsets[chunk_num])));
                let bytes_read = fill_buff(
                    &mut self.buffers[chunk_num as usize],
                    f,
                    self.max_per_chunk,
                    &self.settings,
                );
                self.chunk_offsets[chunk_num as usize] += bytes_read as u64;
                if !self.buffers[chunk_num as usize].is_empty() {
                    empty = false;
                }
            } else {
                empty = false;
            }
        }
        if empty {
            return None;
        }

        // find the next record to write
        // check is_empty() before unwrap()ing
        let mut idx = 0;
        for chunk_num in 0..self.chunks as usize {
            if !self.buffers[chunk_num].is_empty()
                && (self.buffers[idx].is_empty()
                    || super::compare_by(
                        self.buffers[chunk_num].front().unwrap(),
                        self.buffers[idx].front().unwrap(),
                        &self.settings,
                    ) == Ordering::Less)
            {
                idx = chunk_num;
            }
        }

        // unwrap due to checks above
        let r = self.buffers[idx].pop_front().unwrap();
        Some(r)
    }
}

/// Sort (based on `compare`) the `T`s provided by `unsorted` and return an
/// iterator
///
/// # Errors
///
/// This method can fail due to issues writing intermediate sorted chunks
/// to disk.
pub fn ext_sort(
    unsorted: impl Iterator<Item = Line>,
    settings: &GlobalSettings,
) -> ExtSortedIterator {
    let tmp_dir = crash_if_err!(1, TempDir::new_in(&settings.tmp_dir, "uutils_sort"));

    let mut iter = ExtSortedIterator {
        buffers: Vec::new(),
        chunk_offsets: Vec::new(),
        max_per_chunk: 0,
        chunks: 0,
        tmp_dir,
        settings: settings.clone(),
        failed: false,
    };

    let mut total_read = 0;
    let mut chunk = Vec::new();

    // make the initial chunks on disk
    for seq in unsorted {
        let seq_size = seq.estimate_size();
        total_read += seq_size;

        chunk.push(seq);

        if total_read + chunk.len() * std::mem::size_of::<Line>() >= settings.buffer_size {
            super::sort_by(&mut chunk, &settings);
            write_chunk(
                settings,
                &iter.tmp_dir.path().join(iter.chunks.to_string()),
                &mut chunk,
            );
            chunk.clear();
            total_read = 0;
            iter.chunks += 1;
        }
    }
    // write the last chunk
    if !chunk.is_empty() {
        super::sort_by(&mut chunk, &settings);
        write_chunk(
            settings,
            &iter.tmp_dir.path().join(iter.chunks.to_string()),
            &mut chunk,
        );
        iter.chunks += 1;
    }

    // We manually drop here to not go over our memory limit when we allocate below.
    drop(chunk);

    // initialize buffers for each chunk
    //
    // Having a right sized buffer for each chunk for smallish values seems silly to me?
    //
    // We will have to have the entire iter in memory sometime right?
    // Set minimum to the size of the writer buffer, ~8K

    const MINIMUM_READBACK_BUFFER: usize = 8200;
    let right_sized_buffer = settings
        .buffer_size
        .checked_div(iter.chunks)
        .unwrap_or(settings.buffer_size);
    iter.max_per_chunk = if right_sized_buffer > MINIMUM_READBACK_BUFFER {
        right_sized_buffer
    } else {
        MINIMUM_READBACK_BUFFER
    };
    iter.buffers = vec![VecDeque::new(); iter.chunks];
    iter.chunk_offsets = vec![0; iter.chunks];
    for chunk_num in 0..iter.chunks {
        let offset = fill_buff(
            &mut iter.buffers[chunk_num],
            crash_if_err!(
                1,
                File::open(iter.tmp_dir.path().join(chunk_num.to_string()))
            ),
            iter.max_per_chunk,
            &settings,
        );
        iter.chunk_offsets[chunk_num] = offset as u64;
    }

    iter
}

fn write_chunk(settings: &GlobalSettings, file: &Path, chunk: &mut Vec<Line>) {
    let new_file = crash_if_err!(1, OpenOptions::new().create(true).append(true).open(file));
    let mut buf_write = BufWriter::new(new_file);
    for s in chunk {
        crash_if_err!(1, buf_write.write_all(s.line.as_bytes()));
        crash_if_err!(
            1,
            buf_write.write_all(if settings.zero_terminated { "\0" } else { "\n" }.as_bytes(),)
        );
    }
    crash_if_err!(1, buf_write.flush());
}

fn fill_buff(
    vec: &mut VecDeque<Line>,
    file: File,
    max_bytes: usize,
    settings: &GlobalSettings,
) -> usize {
    let mut total_read = 0;
    let mut bytes_read = 0;
    for line in BufReader::new(file).split(if settings.zero_terminated {
        b'\0'
    } else {
        b'\n'
    }) {
        let line_s = String::from_utf8(crash_if_err!(1, line)).unwrap();
        bytes_read += line_s.len() + 1;
        let deserialized = Line::new(line_s, settings);
        total_read += deserialized.estimate_size();
        vec.push_back(deserialized);
        if total_read > max_bytes {
            break;
        }
    }

    bytes_read
}
