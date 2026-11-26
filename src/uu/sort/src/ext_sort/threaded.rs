// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Threaded external sort: read input in chunks, sort them in a background
//! thread, and spill to temporary files when memory is exceeded.

use std::cmp::Ordering;
use std::fs::File;
use std::io::{Read, Write, stderr};
use std::iter;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;

use itertools::Itertools;
use uucore::error::{UError, UResult, strip_errno};

use crate::Output;
use crate::chunks::RecycledChunk;
use crate::merge::WriteableCompressedTmpFile;
use crate::merge::WriteablePlainTmpFile;
use crate::merge::WriteableTmpFile;
use crate::tmp_dir::{TmpDirWrapper, TmpPath};
use crate::{
    GlobalSettings, Line, SortError,
    chunks::{self, Chunk},
    compare_by, merge, print_sorted, sort_by,
};

// Note: update `test_sort::test_start_buffer` if this size is changed
// Fixed to 8 KiB (equivalent to `std::sys::io::DEFAULT_BUF_SIZE` on most targets)
const DEFAULT_BUF_SIZE: usize = 8 * 1024;

/// Sort files by using auxiliary files for storing intermediate chunks (if needed), and output the result.
///
/// Two threads cooperate: one reads input and writes temporary chunk files,
/// while the other sorts each chunk in memory. Once all chunks are written,
/// they are merged back together for final output.
pub fn ext_sort(
    files: &mut impl Iterator<Item = UResult<Box<dyn Read + Send>>>,
    settings: &GlobalSettings,
    output: Output,
    tmp_dir: &mut TmpDirWrapper,
) -> UResult<()> {
    let (sorted_sender, sorted_receiver) = std::sync::mpsc::sync_channel(1);
    let (recycled_sender, recycled_receiver) = std::sync::mpsc::sync_channel(1);
    thread::spawn({
        let settings = settings.clone();
        move || sorter(&recycled_receiver, &sorted_sender, &settings)
    });

    // Test if compression program exists and works, disable if not
    let mut effective_settings = settings.clone();
    if let Some(ref prog) = settings.compress_prog {
        // Test the compression program by trying to spawn it
        match std::process::Command::new(prog)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                // Kill the test process immediately
                let _ = child.kill();
            }
            Err(err) => {
                // Print the error and disable compression
                let _ = writeln!(
                    stderr(),
                    "sort: could not run compress program '{prog}': {}",
                    strip_errno(&err)
                );
                effective_settings.compress_prog = None;
            }
        }
    }

    if effective_settings.compress_prog.is_some() {
        reader_writer::<_, WriteableCompressedTmpFile>(
            files,
            &effective_settings,
            &sorted_receiver,
            recycled_sender,
            output,
            tmp_dir,
        )
    } else {
        reader_writer::<_, WriteablePlainTmpFile>(
            files,
            &effective_settings,
            &sorted_receiver,
            recycled_sender,
            output,
            tmp_dir,
        )
    }
}

fn reader_writer<
    F: Iterator<Item = UResult<Box<dyn Read + Send>>>,
    Tmp: WriteableTmpFile + 'static,
>(
    files: F,
    settings: &GlobalSettings,
    receiver: &Receiver<Chunk>,
    sender: SyncSender<Chunk>,
    output: Output,
    tmp_dir: &mut TmpDirWrapper,
) -> UResult<()> {
    let separator = settings.line_ending.into();

    // Two chunks are kept in memory, so split an explicit limit between them.
    // Keep the automatic sizing heuristic's cap and minimum growth allowance.
    let mut buffer_size = settings.buffer_size;
    if settings.buffer_size_is_explicit || buffer_size > 512 * 1024 * 1024 {
        buffer_size /= 2;
    }
    if !settings.buffer_size_is_explicit {
        buffer_size = buffer_size.max(8 * 1024 * 1024);
    }
    let read_result: ReadResult<Tmp> = read_write_loop(
        files,
        tmp_dir,
        separator,
        buffer_size,
        settings,
        receiver,
        sender,
    )?;
    match read_result {
        ReadResult::WroteChunksToFile { tmp_files } => {
            merge::merge_with_file_limit::<_, _, Tmp>(
                tmp_files.into_iter().map(merge::ClosedTmpFile::reopen),
                settings,
                output,
                tmp_dir,
            )?;
        }
        ReadResult::SortedSingleChunk(chunk) => {
            if settings.unique {
                print_sorted(
                    chunk.lines().iter().dedup_by(|a, b| {
                        compare_by(a, b, settings, chunk.line_data(), chunk.line_data())
                            == Ordering::Equal
                    }),
                    settings,
                    output,
                )?;
            } else {
                print_sorted(chunk.lines().iter(), settings, output)?;
            }
        }
        ReadResult::SortedTwoChunks([a, b]) => {
            let merged_iter = a.lines().iter().map(|line| (line, &a)).merge_by(
                b.lines().iter().map(|line| (line, &b)),
                |(line_a, a), (line_b, b)| {
                    compare_by(line_a, line_b, settings, a.line_data(), b.line_data())
                        != Ordering::Greater
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
                )?;
            } else {
                print_sorted(merged_iter.map(|(line, _)| line), settings, output)?;
            }
        }
        ReadResult::EmptyInput => {
            // output empty too, as coreutils does
            print_sorted(iter::empty::<&Line<'_>>(), settings, output)?;
        }
    }
    Ok(())
}

/// The function that is executed on the sorter thread.
fn sorter(receiver: &Receiver<Chunk>, sender: &SyncSender<Chunk>, settings: &GlobalSettings) {
    while let Ok(mut payload) = receiver.recv() {
        payload.with_dependent_mut(|_, contents| {
            sort_by(&mut contents.lines, settings, &contents.line_data);
        });
        if sender.send(payload).is_err() {
            // The receiver has gone away, likely because the other thread hit an error.
            // We stop silently because the actual error is printed by the other thread.
            return;
        }
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
    WroteChunksToFile { tmp_files: Vec<I::Closed> },
}
/// The function that is executed on the reader/writer thread.
fn read_write_loop<I: WriteableTmpFile>(
    mut files: impl Iterator<Item = UResult<Box<dyn Read + Send>>>,
    tmp_dir: &mut TmpDirWrapper,
    separator: u8,
    buffer_size: usize,
    settings: &GlobalSettings,
    receiver: &Receiver<Chunk>,
    sender: SyncSender<Chunk>,
) -> UResult<ReadResult<I>> {
    let mut file = files.next().unwrap()?;

    let mut carry_over = vec![];
    // kick things off with two reads
    for _ in 0..2 {
        let should_continue = chunks::read(
            &sender,
            RecycledChunk::new(buffer_size.min(DEFAULT_BUF_SIZE)),
            Some(buffer_size),
            &mut carry_over,
            &mut file,
            &mut files,
            separator,
            settings,
        )?;

        if !should_continue {
            drop(sender);
            // We have already read the whole input. Since we are in our first two reads,
            // this means that we can fit the whole input into memory. Bypass writing below and
            // handle this case in a more straightforward way.
            return Ok(if let Ok(first_chunk) = receiver.recv() {
                if let Ok(second_chunk) = receiver.recv() {
                    ReadResult::SortedTwoChunks([first_chunk, second_chunk])
                } else {
                    ReadResult::SortedSingleChunk(first_chunk)
                }
            } else {
                ReadResult::EmptyInput
            });
        }
    }

    let mut sender_option = Some(sender);
    let mut tmp_files = vec![];
    loop {
        let Ok(chunk) = receiver.recv() else {
            return Ok(ReadResult::WroteChunksToFile { tmp_files });
        };

        let tmp_file = write::<I>(
            &chunk,
            tmp_dir.next_file()?,
            settings.compress_prog.as_deref(),
            separator,
        )?;
        tmp_files.push(tmp_file);

        let recycled_chunk = chunk.recycle();

        if let Some(sender) = &sender_option {
            let should_continue = chunks::read(
                sender,
                recycled_chunk,
                None,
                &mut carry_over,
                &mut file,
                &mut files,
                separator,
                settings,
            )?;
            if !should_continue {
                sender_option = None;
            }
        }
    }
}

/// Write the lines in `chunk` to `file`, separated by `separator`.
/// `compress_prog` is used to optionally compress file contents.
fn write<I: WriteableTmpFile>(
    chunk: &Chunk,
    file: (File, TmpPath),
    compress_prog: Option<&str>,
    separator: u8,
) -> UResult<I::Closed> {
    let path = file.1.path().to_owned();
    let mut tmp_file = I::create(file, compress_prog)?;
    // GNU reports a full temporary file when it closes it (`close failed`),
    // including when the write that filled the buffer is what failed.
    write_lines(chunk.lines(), tmp_file.as_write(), separator).map_err(|error| {
        Box::new(SortError::TmpFileCloseFailed { path, error }) as Box<dyn UError>
    })?;
    tmp_file.finished_writing()
}

fn write_lines<T: Write>(lines: &[Line], writer: &mut T, separator: u8) -> std::io::Result<()> {
    for s in lines {
        writer.write_all(s.line)?;
        writer.write_all(&[separator])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::io;
    use std::path::Path;

    use super::*;
    use crate::chunks::{ChunkContents, LineData};

    struct FailingWriter(io::ErrorKind);

    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(self.0))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn write_lines_propagates_write_errors() {
        let lines = [Line {
            line: b"line",
            index: 0,
        }];

        let result = write_lines(
            &lines,
            &mut FailingWriter(io::ErrorKind::StorageFull),
            b'\n',
        );

        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::StorageFull);
    }

    fn one_line_chunk(payload: &[u8]) -> Chunk {
        let mut data = payload.to_vec();
        data.push(b'\n');
        Chunk::try_new(data, |buffer| {
            let line = &buffer[..buffer.len() - 1];
            Ok::<_, std::convert::Infallible>(ChunkContents {
                lines: vec![Line { line, index: 0 }],
                line_data: LineData::default(),
                token_buffer: Vec::new(),
                line_count_hint: 1,
            })
        })
        .unwrap()
    }

    fn assert_close_failed(err: &(dyn UError + 'static), expected: &Path) {
        assert_eq!(err.code(), 2, "{err}");
        match (err as &(dyn Error + 'static))
            .downcast_ref::<SortError>()
            .unwrap_or_else(|| panic!("expected SortError, got {err}"))
        {
            SortError::TmpFileCloseFailed { path, .. } => assert_eq!(path, expected),
            other => panic!("expected a temporary-file close failure, got {other:?}"),
        }
    }

    /// A read-only temporary file fails when the buffered chunk is flushed.
    #[test]
    fn buffered_temp_write_reports_close_failure_with_the_file_name() {
        let dir = tempfile::tempdir().unwrap();
        let mut tmp_dir = TmpDirWrapper::new(dir.path().to_owned());
        let (file, tmp_path) = tmp_dir.next_file().unwrap();
        let expected = tmp_path.path().to_owned();
        drop(file);
        let ro = File::open(&expected).unwrap();
        let chunk = one_line_chunk(b"line");
        let Err(err) = write::<WriteablePlainTmpFile>(&chunk, (ro, tmp_path), None, b'\n') else {
            panic!("expected the temporary file write to fail");
        };
        assert_close_failed(err.as_ref(), &expected);
    }

    /// A payload larger than the buffer fails during the write itself.
    #[test]
    fn large_temp_write_reports_close_failure_with_the_file_name() {
        let dir = tempfile::tempdir().unwrap();
        let mut tmp_dir = TmpDirWrapper::new(dir.path().to_owned());
        let (file, tmp_path) = tmp_dir.next_file().unwrap();
        let expected = tmp_path.path().to_owned();
        drop(file);
        let ro = File::open(&expected).unwrap();
        let payload = vec![b'x'; 20 * 1024];
        let chunk = one_line_chunk(&payload);
        let Err(err) = write::<WriteablePlainTmpFile>(&chunk, (ro, tmp_path), None, b'\n') else {
            panic!("expected the temporary file write to fail");
        };
        assert_close_failed(err.as_ref(), &expected);
    }
}
