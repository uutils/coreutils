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
    fs::{self, File},
    io::{BufWriter, Read, Write},
    iter,
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    rc::Rc,
    sync::mpsc::{channel, sync_channel, Receiver, Sender, SyncSender},
    thread,
};

use compare::Compare;
use itertools::Itertools;
use tempfile::TempDir;

use crate::{
    chunks::{self, Chunk},
    compare_by, GlobalSettings,
};

/// Merge pre-sorted `Box<dyn Read>`s.
///
/// If `settings.merge_batch_size` is greater than the length of `files`, intermediate files will be used.
/// If `settings.compress_prog` is `Some`, intermediate files will be compressed with it.
pub fn merge<Files: ExactSizeIterator<Item = Box<dyn Read + Send>>>(
    files: Files,
    settings: &GlobalSettings,
) -> FileMerger {
    if settings.compress_prog.is_none() {
        merge_with_file_limit::<_, _, WriteablePlainTmpFile>(
            files.map(|file| PlainMergeInput { inner: file }),
            settings,
            None,
        )
    } else {
        merge_with_file_limit::<_, _, WriteableCompressedTmpFile>(
            files.map(|file| PlainMergeInput { inner: file }),
            settings,
            None,
        )
    }
}

// Merge already sorted `MergeInput`s.
pub fn merge_with_file_limit<
    M: MergeInput + 'static,
    F: ExactSizeIterator<Item = M>,
    Tmp: WriteableTmpFile + 'static,
>(
    files: F,
    settings: &GlobalSettings,
    tmp_dir: Option<(TempDir, usize)>,
) -> FileMerger {
    if files.len() > settings.merge_batch_size {
        // If we did not get a tmp_dir, create one.
        let (tmp_dir, mut tmp_dir_size) = tmp_dir.unwrap_or_else(|| {
            (
                tempfile::Builder::new()
                    .prefix("uutils_sort")
                    .tempdir_in(&settings.tmp_dir)
                    .unwrap(),
                0,
            )
        });
        let mut remaining_files = files.len();
        let batches = files.chunks(settings.merge_batch_size);
        let mut batches = batches.into_iter();
        let mut temporary_files = vec![];
        while remaining_files != 0 {
            // Work around the fact that `Chunks` is not an `ExactSizeIterator`.
            remaining_files = remaining_files.saturating_sub(settings.merge_batch_size);
            let mut merger = merge_without_limit(batches.next().unwrap(), settings);
            let mut tmp_file = Tmp::create(
                tmp_dir.path().join(tmp_dir_size.to_string()),
                settings.compress_prog.as_deref(),
            );
            tmp_dir_size += 1;
            merger.write_all_to(settings, tmp_file.as_write());
            temporary_files.push(tmp_file.finished_writing());
        }
        assert!(batches.next().is_none());
        merge_with_file_limit::<_, _, Tmp>(
            temporary_files
                .into_iter()
                .map(Box::new(|c: Tmp::Closed| c.reopen())
                    as Box<
                        dyn FnMut(Tmp::Closed) -> <Tmp::Closed as ClosedTmpFile>::Reopened,
                    >),
            settings,
            Some((tmp_dir, tmp_dir_size)),
        )
    } else {
        merge_without_limit(files, settings)
    }
}

/// Merge files without limiting how many files are concurrently open.
///
/// It is the responsibility of the caller to ensure that `files` yields only
/// as many files as we are allowed to open concurrently.
fn merge_without_limit<M: MergeInput + 'static, F: Iterator<Item = M>>(
    files: F,
    settings: &GlobalSettings,
) -> FileMerger {
    let (request_sender, request_receiver) = channel();
    let mut reader_files = Vec::with_capacity(files.size_hint().0);
    let mut loaded_receivers = Vec::with_capacity(files.size_hint().0);
    for (file_number, file) in files.enumerate() {
        let (sender, receiver) = sync_channel(2);
        loaded_receivers.push(receiver);
        reader_files.push(Some(ReaderFile {
            file,
            sender,
            carry_over: vec![],
        }));
        // Send the initial chunk to trigger a read for each file
        request_sender
            .send((file_number, Chunk::new(vec![0; 8 * 1024], |_| Vec::new())))
            .unwrap();
    }

    // Send the second chunk for each file
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
struct ReaderFile<M: MergeInput> {
    file: M,
    sender: SyncSender<Chunk>,
    carry_over: Vec<u8>,
}

/// The function running on the reader thread.
fn reader(
    recycled_receiver: Receiver<(usize, Chunk)>,
    files: &mut [Option<ReaderFile<impl MergeInput>>],
    settings: &GlobalSettings,
    separator: u8,
) {
    for (file_idx, chunk) in recycled_receiver.iter() {
        let (recycled_lines, recycled_buffer) = chunk.recycle();
        if let Some(ReaderFile {
            file,
            sender,
            carry_over,
        }) = &mut files[file_idx]
        {
            let should_continue = chunks::read(
                sender,
                recycled_buffer,
                None,
                carry_over,
                file.as_read(),
                &mut iter::empty(),
                separator,
                recycled_lines,
                settings,
            );
            if !should_continue {
                // Remove the file from the list by replacing it with `None`.
                let ReaderFile { file, .. } = files[file_idx].take().unwrap();
                // Depending on the kind of the `MergeInput`, this may delete the file:
                file.finished_reading();
            }
        }
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
        self.write_all_to(settings, &mut out);
    }

    pub fn write_all_to(&mut self, settings: &GlobalSettings, out: &mut impl Write) {
        while self.write_next(settings, out) {}
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
                // This will cause the comparison to use a different line and the heap to readjust.
                self.heap.peek_mut().unwrap().line_idx += 1;
            }

            if let Some(prev) = prev {
                if let Ok(prev_chunk) = Rc::try_unwrap(prev.chunk) {
                    // If nothing is referencing the previous chunk anymore, this means that the previous line
                    // was the last line of the chunk. We can recycle the chunk.
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
        // BinaryHeap is a max heap. We use it as a min heap, so we need to reverse the ordering.
        cmp.reverse()
    }
}

// Wait for the child to exit and check its exit code.
fn assert_child_success(mut child: Child, program: &str) {
    if !matches!(
        child.wait().map(|e| e.code()),
        Ok(Some(0)) | Ok(None) | Err(_)
    ) {
        crash!(2, "'{}' terminated abnormally", program)
    }
}

/// A temporary file that can be written to.
pub trait WriteableTmpFile {
    type Closed: ClosedTmpFile;
    type InnerWrite: Write;
    fn create(path: PathBuf, compress_prog: Option<&str>) -> Self;
    /// Closes the temporary file.
    fn finished_writing(self) -> Self::Closed;
    fn as_write(&mut self) -> &mut Self::InnerWrite;
}
/// A temporary file that is (temporarily) closed, but can be reopened.
pub trait ClosedTmpFile {
    type Reopened: MergeInput;
    /// Reopens the temporary file.
    fn reopen(self) -> Self::Reopened;
}
/// A pre-sorted input for merging.
pub trait MergeInput: Send {
    type InnerRead: Read;
    /// Cleans this `MergeInput` up.
    /// Implementations may delete the backing file.
    fn finished_reading(self);
    fn as_read(&mut self) -> &mut Self::InnerRead;
}

pub struct WriteablePlainTmpFile {
    path: PathBuf,
    file: BufWriter<File>,
}
pub struct ClosedPlainTmpFile {
    path: PathBuf,
}
pub struct PlainTmpMergeInput {
    path: PathBuf,
    file: File,
}
impl WriteableTmpFile for WriteablePlainTmpFile {
    type Closed = ClosedPlainTmpFile;
    type InnerWrite = BufWriter<File>;

    fn create(path: PathBuf, _: Option<&str>) -> Self {
        WriteablePlainTmpFile {
            file: BufWriter::new(File::create(&path).unwrap()),
            path,
        }
    }

    fn finished_writing(self) -> Self::Closed {
        ClosedPlainTmpFile { path: self.path }
    }

    fn as_write(&mut self) -> &mut Self::InnerWrite {
        &mut self.file
    }
}
impl ClosedTmpFile for ClosedPlainTmpFile {
    type Reopened = PlainTmpMergeInput;
    fn reopen(self) -> Self::Reopened {
        PlainTmpMergeInput {
            file: File::open(&self.path).unwrap(),
            path: self.path,
        }
    }
}
impl MergeInput for PlainTmpMergeInput {
    type InnerRead = File;

    fn finished_reading(self) {
        fs::remove_file(self.path).ok();
    }

    fn as_read(&mut self) -> &mut Self::InnerRead {
        &mut self.file
    }
}

pub struct WriteableCompressedTmpFile {
    path: PathBuf,
    compress_prog: String,
    child: Child,
    child_stdin: BufWriter<ChildStdin>,
}
pub struct ClosedCompressedTmpFile {
    path: PathBuf,
    compress_prog: String,
}
pub struct CompressedTmpMergeInput {
    path: PathBuf,
    compress_prog: String,
    child: Child,
    child_stdout: ChildStdout,
}
impl WriteableTmpFile for WriteableCompressedTmpFile {
    type Closed = ClosedCompressedTmpFile;
    type InnerWrite = BufWriter<ChildStdin>;

    fn create(path: PathBuf, compress_prog: Option<&str>) -> Self {
        let compress_prog = compress_prog.unwrap();
        let mut command = Command::new(compress_prog);
        command
            .stdin(Stdio::piped())
            .stdout(File::create(&path).unwrap());
        let mut child = crash_if_err!(
            2,
            command.spawn().map_err(|err| format!(
                "couldn't execute compress program: errno {}",
                err.raw_os_error().unwrap()
            ))
        );
        let child_stdin = child.stdin.take().unwrap();
        WriteableCompressedTmpFile {
            path,
            compress_prog: compress_prog.to_owned(),
            child,
            child_stdin: BufWriter::new(child_stdin),
        }
    }

    fn finished_writing(self) -> Self::Closed {
        drop(self.child_stdin);
        assert_child_success(self.child, &self.compress_prog);
        ClosedCompressedTmpFile {
            path: self.path,
            compress_prog: self.compress_prog,
        }
    }

    fn as_write(&mut self) -> &mut Self::InnerWrite {
        &mut self.child_stdin
    }
}
impl ClosedTmpFile for ClosedCompressedTmpFile {
    type Reopened = CompressedTmpMergeInput;

    fn reopen(self) -> Self::Reopened {
        let mut command = Command::new(&self.compress_prog);
        let file = File::open(&self.path).unwrap();
        command.stdin(file).stdout(Stdio::piped()).arg("-d");
        let mut child = crash_if_err!(
            2,
            command.spawn().map_err(|err| format!(
                "couldn't execute compress program: errno {}",
                err.raw_os_error().unwrap()
            ))
        );
        let child_stdout = child.stdout.take().unwrap();
        CompressedTmpMergeInput {
            path: self.path,
            compress_prog: self.compress_prog,
            child,
            child_stdout,
        }
    }
}
impl MergeInput for CompressedTmpMergeInput {
    type InnerRead = ChildStdout;

    fn finished_reading(self) {
        drop(self.child_stdout);
        assert_child_success(self.child, &self.compress_prog);
        fs::remove_file(self.path).ok();
    }

    fn as_read(&mut self) -> &mut Self::InnerRead {
        &mut self.child_stdout
    }
}

pub struct PlainMergeInput<R: Read + Send> {
    inner: R,
}
impl<R: Read + Send> MergeInput for PlainMergeInput<R> {
    type InnerRead = R;
    fn finished_reading(self) {}
    fn as_read(&mut self) -> &mut Self::InnerRead {
        &mut self.inner
    }
}
