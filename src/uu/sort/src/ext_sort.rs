//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Michael Debertol <michael.debertol..AT..gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

//! Sort big files by using auxiliary files for storing intermediate chunks.
//!
//! Files are read into chunks of memory which are then sorted individually and
//! written to temporary files. There are two threads: One sorter, and one reader/writer.
//! The buffers for the individual chunks are recycled. There are two buffers.

use std::cmp::Ordering;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::{
    fs::OpenOptions,
    io::Read,
    sync::mpsc::{Receiver, SyncSender},
    thread,
};

use itertools::Itertools;

use tempfile::TempDir;

use crate::{
    chunks::{self, Chunk},
    compare_by, merge, output_sorted_lines, sort_by, GlobalSettings,
};

const START_BUFFER_SIZE: usize = 8_000;

/// Sort files by using auxiliary files for storing intermediate chunks (if needed), and output the result.
pub fn ext_sort(files: &mut impl Iterator<Item = Box<dyn Read + Send>>, settings: &GlobalSettings) {
    let tmp_dir = crash_if_err!(
        1,
        tempfile::Builder::new()
            .prefix("uutils_sort")
            .tempdir_in(&settings.tmp_dir)
    );
    let (sorted_sender, sorted_receiver) = std::sync::mpsc::sync_channel(1);
    let (recycled_sender, recycled_receiver) = std::sync::mpsc::sync_channel(1);
    thread::spawn({
        let settings = settings.clone();
        move || sorter(recycled_receiver, sorted_sender, settings)
    });
    let read_result = reader_writer(
        files,
        &tmp_dir,
        if settings.zero_terminated {
            b'\0'
        } else {
            b'\n'
        },
        // Heuristically chosen: Dividing by 10 seems to keep our memory usage roughly
        // around settings.buffer_size as a whole.
        settings.buffer_size / 10,
        settings.clone(),
        sorted_receiver,
        recycled_sender,
    );
    match read_result {
        ReadResult::WroteChunksToFile { chunks_written } => {
            let files = (0..chunks_written)
                .map(|chunk_num| tmp_dir.path().join(chunk_num.to_string()))
                .collect::<Vec<_>>();
            let mut merger = merge::merge(&files, settings);
            merger.write_all(settings);
        }
        ReadResult::SortedSingleChunk(chunk) => {
            output_sorted_lines(chunk.borrow_lines().iter(), settings);
        }
        ReadResult::SortedTwoChunks([a, b]) => {
            let merged_iter = a
                .borrow_lines()
                .iter()
                .merge_by(b.borrow_lines().iter(), |line_a, line_b| {
                    compare_by(line_a, line_b, settings) != Ordering::Greater
                });
            output_sorted_lines(merged_iter, settings);
        }
        ReadResult::EmptyInput => {
            // don't output anything
        }
    }
}

/// The function that is executed on the sorter thread.
fn sorter(receiver: Receiver<Chunk>, sender: SyncSender<Chunk>, settings: GlobalSettings) {
    while let Ok(mut payload) = receiver.recv() {
        payload.with_lines_mut(|lines| sort_by(lines, &settings));
        sender.send(payload).unwrap();
    }
}

/// Describes how we read the chunks from the input.
enum ReadResult {
    /// The input was empty. Nothing was read.
    EmptyInput,
    /// The input fits into a single Chunk, which was kept in memory.
    SortedSingleChunk(Chunk),
    /// The input fits into two chunks, which were kept in memory.
    SortedTwoChunks([Chunk; 2]),
    /// The input was read into multiple chunks, which were written to auxiliary files.
    WroteChunksToFile {
        /// The number of chunks written to auxiliary files.
        chunks_written: usize,
    },
}

/// The function that is executed on the reader/writer thread.
///
/// # Returns
/// * The number of chunks read.
fn reader_writer(
    mut files: impl Iterator<Item = Box<dyn Read + Send>>,
    tmp_dir: &TempDir,
    separator: u8,
    buffer_size: usize,
    settings: GlobalSettings,
    receiver: Receiver<Chunk>,
    sender: SyncSender<Chunk>,
) -> ReadResult {
    let mut sender_option = Some(sender);

    let mut file = files.next().unwrap();

    let mut carry_over = vec![];
    // kick things off with two reads
    for _ in 0..2 {
        chunks::read(
            &mut sender_option,
            vec![
                0;
                if START_BUFFER_SIZE < buffer_size {
                    START_BUFFER_SIZE
                } else {
                    buffer_size
                }
            ],
            Some(buffer_size),
            &mut carry_over,
            &mut file,
            &mut files,
            separator,
            Vec::new(),
            &settings,
        );
        if sender_option.is_none() {
            // We have already read the whole input. Since we are in our first two reads,
            // this means that we can fit the whole input into memory. Bypass writing below and
            // handle this case in a more straightforward way.
            return if let Ok(first_chunk) = receiver.recv() {
                if let Ok(second_chunk) = receiver.recv() {
                    ReadResult::SortedTwoChunks([first_chunk, second_chunk])
                } else {
                    ReadResult::SortedSingleChunk(first_chunk)
                }
            } else {
                ReadResult::EmptyInput
            };
        }
    }

    let mut file_number = 0;
    loop {
        let mut chunk = match receiver.recv() {
            Ok(it) => it,
            _ => {
                return ReadResult::WroteChunksToFile {
                    chunks_written: file_number,
                }
            }
        };

        write(
            &mut chunk,
            &tmp_dir.path().join(file_number.to_string()),
            separator,
        );

        file_number += 1;

        let (recycled_lines, recycled_buffer) = chunk.recycle();

        chunks::read(
            &mut sender_option,
            recycled_buffer,
            None,
            &mut carry_over,
            &mut file,
            &mut files,
            separator,
            recycled_lines,
            &settings,
        );
    }
}

/// Write the lines in `chunk` to `file`, separated by `separator`.
fn write(chunk: &mut Chunk, file: &Path, separator: u8) {
    chunk.with_lines_mut(|lines| {
        // Write the lines to the file
        let file = crash_if_err!(1, OpenOptions::new().create(true).write(true).open(file));
        let mut writer = BufWriter::new(file);
        for s in lines.iter() {
            crash_if_err!(1, writer.write_all(s.line.as_bytes()));
            crash_if_err!(1, writer.write_all(&[separator]));
        }
    });
}
