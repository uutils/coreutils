//! Merge already sorted files.
//!
//! We achieve performance by splitting the tasks of sorting and writing, and reading and parsing between two threads.
//! The threads communicate over channels. There's one channel per file in the direction reader -> sorter, but only
//! one channel from the sorter back to the reader. The channels to the sorter are used to send the read chunks.
//! The sorter reads the next chunk from the channel whenever it needs the next chunk after running out of lines
//! from the previous read of the file. The channel back from the sorter to the reader has two purposes: To allow the reader
//! to reuse memory allocations and to tell the reader which file to read from next.

use std::{
    cmp::Ordering,
    io::{Read, Write},
    iter,
    rc::Rc,
    sync::mpsc::{channel, sync_channel, Receiver, Sender, SyncSender},
    thread,
};

use compare::Compare;

use crate::{
    chunks::{self, Chunk},
    compare_by, GlobalSettings,
};

// Merge already sorted files.
pub fn merge<F: ExactSizeIterator<Item = Box<dyn Read + Send>>>(
    files: F,
    settings: &GlobalSettings,
) -> FileMerger {
    let (request_sender, request_receiver) = channel();
    let mut reader_files = Vec::with_capacity(files.len());
    let mut loaded_receivers = Vec::with_capacity(files.len());
    for (file_number, file) in files.enumerate() {
        let (sender, receiver) = sync_channel(2);
        loaded_receivers.push(receiver);
        reader_files.push(ReaderFile {
            file,
            sender: Some(sender),
            carry_over: vec![],
        });
        request_sender
            .send((file_number, Chunk::new(vec![0; 8 * 1024], |_| Vec::new())))
            .unwrap();
    }

    for file_number in 0..reader_files.len() {
        request_sender
            .send((file_number, Chunk::new(vec![0; 8 * 1024], |_| Vec::new())))
            .unwrap();
    }

    thread::spawn({
        let settings = settings.clone();
        move || {
            reader(
                request_receiver,
                &mut reader_files,
                &settings,
                if settings.zero_terminated {
                    b'\0'
                } else {
                    b'\n'
                },
            )
        }
    });

    let mut mergeable_files = vec![];

    for (file_number, receiver) in loaded_receivers.into_iter().enumerate() {
        mergeable_files.push(MergeableFile {
            current_chunk: Rc::new(receiver.recv().unwrap()),
            file_number,
            line_idx: 0,
            receiver,
        })
    }

    FileMerger {
        heap: binary_heap_plus::BinaryHeap::from_vec_cmp(
            mergeable_files,
            FileComparator { settings },
        ),
        request_sender,
        prev: None,
    }
}
/// The struct on the reader thread representing an input file
struct ReaderFile {
    file: Box<dyn Read + Send>,
    sender: Option<SyncSender<Chunk>>,
    carry_over: Vec<u8>,
}

/// The function running on the reader thread.
fn reader(
    recycled_receiver: Receiver<(usize, Chunk)>,
    files: &mut [ReaderFile],
    settings: &GlobalSettings,
    separator: u8,
) {
    for (file_idx, chunk) in recycled_receiver.iter() {
        let (recycled_lines, recycled_buffer) = chunk.recycle();
        let ReaderFile {
            file,
            sender,
            carry_over,
        } = &mut files[file_idx];
        chunks::read(
            sender,
            recycled_buffer,
            None,
            carry_over,
            file,
            &mut iter::empty(),
            separator,
            recycled_lines,
            settings,
        );
    }
}
/// The struct on the main thread representing an input file
pub struct MergeableFile {
    current_chunk: Rc<Chunk>,
    line_idx: usize,
    receiver: Receiver<Chunk>,
    file_number: usize,
}

/// A struct to keep track of the previous line we encountered.
///
/// This is required for deduplication purposes.
struct PreviousLine {
    chunk: Rc<Chunk>,
    line_idx: usize,
    file_number: usize,
}

/// Merges files together. This is **not** an iterator because of lifetime problems.
pub struct FileMerger<'a> {
    heap: binary_heap_plus::BinaryHeap<MergeableFile, FileComparator<'a>>,
    request_sender: Sender<(usize, Chunk)>,
    prev: Option<PreviousLine>,
}

impl<'a> FileMerger<'a> {
    /// Write the merged contents to the output file.
    pub fn write_all(&mut self, settings: &GlobalSettings) {
        let mut out = settings.out_writer();
        while self.write_next(settings, &mut out) {}
    }

    fn write_next(&mut self, settings: &GlobalSettings, out: &mut impl Write) -> bool {
        if let Some(file) = self.heap.peek() {
            let prev = self.prev.replace(PreviousLine {
                chunk: file.current_chunk.clone(),
                line_idx: file.line_idx,
                file_number: file.file_number,
            });

            file.current_chunk.with_lines(|lines| {
                let current_line = &lines[file.line_idx];
                if settings.unique {
                    if let Some(prev) = &prev {
                        let cmp = compare_by(
                            &prev.chunk.borrow_lines()[prev.line_idx],
                            current_line,
                            settings,
                        );
                        if cmp == Ordering::Equal {
                            return;
                        }
                    }
                }
                current_line.print(out, settings);
            });

            let was_last_line_for_file =
                file.current_chunk.borrow_lines().len() == file.line_idx + 1;

            if was_last_line_for_file {
                if let Ok(next_chunk) = file.receiver.recv() {
                    let mut file = self.heap.peek_mut().unwrap();
                    file.current_chunk = Rc::new(next_chunk);
                    file.line_idx = 0;
                } else {
                    self.heap.pop();
                }
            } else {
                self.heap.peek_mut().unwrap().line_idx += 1;
            }

            if let Some(prev) = prev {
                if let Ok(prev_chunk) = Rc::try_unwrap(prev.chunk) {
                    self.request_sender
                        .send((prev.file_number, prev_chunk))
                        .ok();
                }
            }
        }
        !self.heap.is_empty()
    }
}

/// Compares files by their current line.
struct FileComparator<'a> {
    settings: &'a GlobalSettings,
}

impl<'a> Compare<MergeableFile> for FileComparator<'a> {
    fn compare(&self, a: &MergeableFile, b: &MergeableFile) -> Ordering {
        let mut cmp = compare_by(
            &a.current_chunk.borrow_lines()[a.line_idx],
            &b.current_chunk.borrow_lines()[b.line_idx],
            self.settings,
        );
        if cmp == Ordering::Equal {
            // To make sorting stable, we need to consider the file number as well,
            // as lines from a file with a lower number are to be considered "earlier".
            cmp = a.file_number.cmp(&b.file_number);
        }
        // Our BinaryHeap is a max heap. We use it as a min heap, so we need to reverse the ordering.
        cmp.reverse()
    }
}
