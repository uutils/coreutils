//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Michael Debertol <michael.debertol..AT..gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

//! Sort big files by using files for storing intermediate chunks.
//!
//! Files are read into chunks of memory which are then sorted individually and
//! written to temporary files. There are two threads: One sorter, and one reader/writer.
//! The buffers for the individual chunks are recycled. There are two buffers.

use std::io::{BufWriter, Write};
use std::path::Path;
use std::{
    fs::OpenOptions,
    io::Read,
    sync::mpsc::{Receiver, SyncSender},
    thread,
};

use tempdir::TempDir;

use crate::{
    chunks::{self, Chunk},
    merge::{self, FileMerger},
    sort_by, GlobalSettings,
};

/// Iterator that wraps the
pub struct ExtSortedMerger<'a> {
    pub file_merger: FileMerger<'a>,
    // Keep _tmp_dir around, as it is deleted when dropped.
    _tmp_dir: TempDir,
}

/// Sort big files by using files for storing intermediate chunks.
///
/// # Returns
///
/// An iterator that merges intermediate files back together.
pub fn ext_sort<'a>(
    files: &mut impl Iterator<Item = Box<dyn Read + Send>>,
    settings: &'a GlobalSettings,
) -> ExtSortedMerger<'a> {
    let tmp_dir = crash_if_err!(1, TempDir::new_in(&settings.tmp_dir, "uutils_sort"));
    let (sorted_sender, sorted_receiver) = std::sync::mpsc::sync_channel(1);
    let (recycled_sender, recycled_receiver) = std::sync::mpsc::sync_channel(1);
    thread::spawn({
        let settings = settings.clone();
        move || sorter(recycled_receiver, sorted_sender, settings)
    });
    let chunks_read = reader_writer(
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
    let files = (0..chunks_read)
        .map(|chunk_num| tmp_dir.path().join(chunk_num.to_string()))
        .collect::<Vec<_>>();

    ExtSortedMerger {
        file_merger: merge::merge(&files, settings),
        _tmp_dir: tmp_dir,
    }
}

/// The function that is executed on the sorter thread.
fn sorter(receiver: Receiver<Chunk>, sender: SyncSender<Chunk>, settings: GlobalSettings) {
    while let Ok(mut payload) = receiver.recv() {
        payload.with_lines_mut(|lines| sort_by(lines, &settings));
        sender.send(payload).unwrap();
    }
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
) -> usize {
    let mut sender_option = Some(sender);

    let mut file = files.next().unwrap();

    let mut carry_over = vec![];
    // kick things off with two reads
    for _ in 0..2 {
        chunks::read(
            &mut sender_option,
            vec![0; buffer_size],
            &mut carry_over,
            &mut file,
            &mut files,
            separator,
            Vec::new(),
            &settings,
        )
    }

    let mut file_number = 0;
    loop {
        let mut chunk = match receiver.recv() {
            Ok(it) => it,
            _ => return file_number,
        };

        write(
            &mut chunk,
            &tmp_dir.path().join(file_number.to_string()),
            separator,
        );

        let (recycled_lines, recycled_buffer) = chunk.recycle();

        file_number += 1;

        chunks::read(
            &mut sender_option,
            recycled_buffer,
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
