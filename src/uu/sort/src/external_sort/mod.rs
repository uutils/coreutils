use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::Path;

use tempdir::TempDir;

use crate::{file_to_lines_iter, FileMerger};

use super::{GlobalSettings, Line};

/// Iterator that provides sorted `T`s
pub struct ExtSortedIterator<'a> {
    file_merger: FileMerger<'a>,
    // Keep tmp_dir around, it is deleted when dropped.
    _tmp_dir: TempDir,
}

impl<'a> Iterator for ExtSortedIterator<'a> {
    type Item = Line;
    fn next(&mut self) -> Option<Self::Item> {
        self.file_merger.next()
    }
}

/// Sort (based on `compare`) the `T`s provided by `unsorted` and return an
/// iterator
///
/// # Panics
///
/// This method can panic due to issues writing intermediate sorted chunks
/// to disk.
pub fn ext_sort(
    unsorted: impl Iterator<Item = Line>,
    settings: &GlobalSettings,
) -> ExtSortedIterator {
    let tmp_dir = crash_if_err!(1, TempDir::new_in(&settings.tmp_dir, "uutils_sort"));

    let mut total_read = 0;
    let mut chunk = Vec::new();

    let mut chunks_read = 0;
    let mut file_merger = FileMerger::new(settings);

    // make the initial chunks on disk
    for seq in unsorted {
        let seq_size = seq.estimate_size();
        total_read += seq_size;

        chunk.push(seq);

        if total_read >= settings.buffer_size && chunk.len() >= 2 {
            super::sort_by(&mut chunk, &settings);

            let file_path = tmp_dir.path().join(chunks_read.to_string());
            write_chunk(settings, &file_path, &mut chunk);
            chunk.clear();
            total_read = 0;
            chunks_read += 1;

            file_merger.push_file(Box::new(file_to_lines_iter(file_path, settings).unwrap()))
        }
    }
    // write the last chunk
    if !chunk.is_empty() {
        super::sort_by(&mut chunk, &settings);

        let file_path = tmp_dir.path().join(chunks_read.to_string());
        write_chunk(
            settings,
            &tmp_dir.path().join(chunks_read.to_string()),
            &mut chunk,
        );

        file_merger.push_file(Box::new(file_to_lines_iter(file_path, settings).unwrap()));
    }
    ExtSortedIterator {
        file_merger,
        _tmp_dir: tmp_dir,
    }
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
