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
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::{
    io::Read,
    sync::mpsc::{Receiver, SyncSender},
    thread,
};

use itertools::Itertools;

use crate::merge::ClosedTmpFile;
use crate::merge::WriteableCompressedTmpFile;
use crate::merge::WriteablePlainTmpFile;
use crate::merge::WriteableTmpFile;
use crate::Line;
use crate::{
    chunks::{self, Chunk},
    compare_by, merge, output_sorted_lines, sort_by, GlobalSettings,
};
use tempfile::TempDir;

const START_BUFFER_SIZE: usize = 8_000;

/// Sort files by using auxiliary files for storing intermediate chunks (if needed), and output the result.
pub fn ext_sort(files: &mut impl Iterator<Item = Box<dyn Read + Send>>, settings: &GlobalSettings) {
    let (sorted_sender, sorted_receiver) = std::sync::mpsc::sync_channel(1);
    let (recycled_sender, recycled_receiver) = std::sync::mpsc::sync_channel(1);
    thread::spawn({
        let settings = settings.clone();
        move || sorter(recycled_receiver, sorted_sender, settings)
    });
    if settings.compress_prog.is_some() {
        reader_writer::<_, WriteableCompressedTmpFile>(
            files,
            settings,
            sorted_receiver,
            recycled_sender,
        );
    } else {
        reader_writer::<_, WriteablePlainTmpFile>(
            files,
            settings,
            sorted_receiver,
            recycled_sender,
        );
    }
}

fn reader_writer<F: Iterator<Item = Box<dyn Read + Send>>, Tmp: WriteableTmpFile + 'static>(
    files: F,
    settings: &GlobalSettings,
    receiver: Receiver<Chunk>,
    sender: SyncSender<Chunk>,
) {
    let separator = if settings.zero_terminated {
        b'\0'
    } else {
        b'\n'
    };

    // Heuristically chosen: Dividing by 10 seems to keep our memory usage roughly
    // around settings.buffer_size as a whole.
    let buffer_size = settings.buffer_size / 10;
    let read_result: ReadResult<Tmp> = read_write_loop(
        files,
        &settings.tmp_dir,
        separator,
        // Heuristically chosen: Dividing by 10 seems to keep our memory usage roughly
        // around settings.buffer_size as a whole.
        buffer_size,
        &settings,
        receiver,
        sender,
    );
    match read_result {
        ReadResult::WroteChunksToFile { tmp_files, tmp_dir } => {
            let tmp_dir_size = tmp_files.len();
            let mut merger = merge::merge_with_file_limit::<_, _, Tmp>(
                tmp_files.into_iter().map(|c| c.reopen()),
                &settings,
                Some((tmp_dir, tmp_dir_size)),
            );
            merger.write_all(&settings);
        }
        ReadResult::SortedSingleChunk(chunk) => {
            output_sorted_lines(chunk.borrow_lines().iter(), &settings);
        }
        ReadResult::SortedTwoChunks([a, b]) => {
            let merged_iter = a
                .borrow_lines()
                .iter()
                .merge_by(b.borrow_lines().iter(), |line_a, line_b| {
                    compare_by(line_a, line_b, &settings) != Ordering::Greater
                });
            output_sorted_lines(merged_iter, &settings);
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
enum ReadResult<I: WriteableTmpFile> {
    /// The input was empty. Nothing was read.
    EmptyInput,
    /// The input fits into a single Chunk, which was kept in memory.
    SortedSingleChunk(Chunk),
    /// The input fits into two chunks, which were kept in memory.
    SortedTwoChunks([Chunk; 2]),
    /// The input was read into multiple chunks, which were written to auxiliary files.
    WroteChunksToFile {
        tmp_files: Vec<I::Closed>,
        tmp_dir: TempDir,
    },
}
/// The function that is executed on the reader/writer thread.
fn read_write_loop<I: WriteableTmpFile>(
    mut files: impl Iterator<Item = Box<dyn Read + Send>>,
    tmp_dir_parent: &Path,
    separator: u8,
    buffer_size: usize,
    settings: &GlobalSettings,
    receiver: Receiver<Chunk>,
    sender: SyncSender<Chunk>,
) -> ReadResult<I> {
    let mut file = files.next().unwrap();

    let mut carry_over = vec![];
    // kick things off with two reads
    for _ in 0..2 {
        let should_continue = chunks::read(
            &sender,
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
            settings,
        );

        if !should_continue {
            drop(sender);
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

    let tmp_dir = crash_if_err!(
        1,
        tempfile::Builder::new()
            .prefix("uutils_sort")
            .tempdir_in(tmp_dir_parent)
    );

    let mut sender_option = Some(sender);
    let mut file_number = 0;
    let mut tmp_files = vec![];
    loop {
        let mut chunk = match receiver.recv() {
            Ok(it) => it,
            _ => {
                return ReadResult::WroteChunksToFile { tmp_files, tmp_dir };
            }
        };

        let tmp_file = write::<I>(
            &mut chunk,
            tmp_dir.path().join(file_number.to_string()),
            settings.compress_prog.as_deref(),
            separator,
        );
        tmp_files.push(tmp_file);

        file_number += 1;

        let (recycled_lines, recycled_buffer) = chunk.recycle();

        if let Some(sender) = &sender_option {
            let should_continue = chunks::read(
                &sender,
                recycled_buffer,
                None,
                &mut carry_over,
                &mut file,
                &mut files,
                separator,
                recycled_lines,
                settings,
            );
            if !should_continue {
                sender_option = None;
            }
        }
    }
}

/// Write the lines in `chunk` to `file`, separated by `separator`.
/// `compress_prog` is used to optionally compress file contents.
fn write<I: WriteableTmpFile>(
    chunk: &mut Chunk,
    file: PathBuf,
    compress_prog: Option<&str>,
    separator: u8,
) -> I::Closed {
    chunk.with_lines_mut(|lines| {
        // Write the lines to the file
        let mut tmp_file = I::create(file, compress_prog);
        write_lines(lines, tmp_file.as_write(), separator);
        tmp_file.finished_writing()
    })
}

fn write_lines<'a, T: Write>(lines: &[Line<'a>], writer: &mut T, separator: u8) {
    for s in lines {
        crash_if_err!(1, writer.write_all(s.line.as_bytes()));
        crash_if_err!(1, writer.write_all(&[separator]));
    }
}
