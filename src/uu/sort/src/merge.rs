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
    ffi::OsString,
    fs::{self, File},
    io::{BufWriter, Read, Write},
    iter,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    rc::Rc,
    sync::mpsc::{channel, sync_channel, Receiver, Sender, SyncSender},
    thread::{self, JoinHandle},
};

use compare::Compare;
use itertools::Itertools;
use uucore::error::UResult;

use crate::{
    chunks::{self, Chunk, RecycledChunk},
    compare_by, open,
    tmp_dir::TmpDirWrapper,
    GlobalSettings, Output, SortError,
};

/// If the output file occurs in the input files as well, copy the contents of the output file
/// and replace its occurrences in the inputs with that copy.
fn replace_output_file_in_input_files(
    files: &mut [OsString],
    output: Option<&str>,
    tmp_dir: &mut TmpDirWrapper,
) -> UResult<()> {
    let mut copy: Option<PathBuf> = None;
    if let Some(Ok(output_path)) = output.map(|path| Path::new(path).canonicalize()) {
        for file in files {
            if let Ok(file_path) = Path::new(file).canonicalize() {
                if file_path == output_path {
                    if let Some(copy) = &copy {
                        *file = copy.clone().into_os_string();
                    } else {
                        let (_file, copy_path) = tmp_dir.next_file()?;
                        std::fs::copy(file_path, &copy_path)
                            .map_err(|error| SortError::OpenTmpFileFailed { error })?;
                        *file = copy_path.clone().into_os_string();
                        copy = Some(copy_path);
                    }
                }
            }
        }
    }
    Ok(())
}

/// Merge pre-sorted `Box<dyn Read>`s.
///
/// If `settings.merge_batch_size` is greater than the length of `files`, intermediate files will be used.
/// If `settings.compress_prog` is `Some`, intermediate files will be compressed with it.
pub fn merge<'a>(
    files: &mut [OsString],
    settings: &'a GlobalSettings,
    output: Option<&str>,
    tmp_dir: &mut TmpDirWrapper,
) -> UResult<FileMerger<'a>> {
    replace_output_file_in_input_files(files, output, tmp_dir)?;
    if settings.compress_prog.is_none() {
        merge_with_file_limit::<_, _, WriteablePlainTmpFile>(
            files
                .iter()
                .map(|file| open(file).map(|file| PlainMergeInput { inner: file })),
            settings,
            tmp_dir,
        )
    } else {
        merge_with_file_limit::<_, _, WriteableCompressedTmpFile>(
            files
                .iter()
                .map(|file| open(file).map(|file| PlainMergeInput { inner: file })),
            settings,
            tmp_dir,
        )
    }
}

// Merge already sorted `MergeInput`s.
pub fn merge_with_file_limit<
    'a,
    M: MergeInput + 'static,
    F: ExactSizeIterator<Item = UResult<M>>,
    Tmp: WriteableTmpFile + 'static,
>(
    files: F,
    settings: &'a GlobalSettings,
    tmp_dir: &mut TmpDirWrapper,
) -> UResult<FileMerger<'a>> {
    if files.len() > settings.merge_batch_size {
        let mut remaining_files = files.len();
        let batches = files.chunks(settings.merge_batch_size);
        let mut batches = batches.into_iter();
        let mut temporary_files = vec![];
        while remaining_files != 0 {
            // Work around the fact that `Chunks` is not an `ExactSizeIterator`.
            remaining_files = remaining_files.saturating_sub(settings.merge_batch_size);
            let merger = merge_without_limit(batches.next().unwrap(), settings)?;
            let mut tmp_file =
                Tmp::create(tmp_dir.next_file()?, settings.compress_prog.as_deref())?;
            merger.write_all_to(settings, tmp_file.as_write())?;
            temporary_files.push(tmp_file.finished_writing()?);
        }
        assert!(batches.next().is_none());
        merge_with_file_limit::<_, _, Tmp>(
            temporary_files
                .into_iter()
                .map(Box::new(|c: Tmp::Closed| c.reopen())
                    as Box<
                        dyn FnMut(Tmp::Closed) -> UResult<<Tmp::Closed as ClosedTmpFile>::Reopened>,
                    >),
            settings,
            tmp_dir,
        )
    } else {
        merge_without_limit(files, settings)
    }
}

/// Merge files without limiting how many files are concurrently open.
///
/// It is the responsibility of the caller to ensure that `files` yields only
/// as many files as we are allowed to open concurrently.
fn merge_without_limit<M: MergeInput + 'static, F: Iterator<Item = UResult<M>>>(
    files: F,
    settings: &GlobalSettings,
) -> UResult<FileMerger> {
    let (request_sender, request_receiver) = channel();
    let mut reader_files = Vec::with_capacity(files.size_hint().0);
    let mut loaded_receivers = Vec::with_capacity(files.size_hint().0);
    for (file_number, file) in files.enumerate() {
        let (sender, receiver) = sync_channel(2);
        loaded_receivers.push(receiver);
        reader_files.push(Some(ReaderFile {
            file: file?,
            sender,
            carry_over: vec![],
        }));
        // Send the initial chunk to trigger a read for each file
        request_sender
            .send((file_number, RecycledChunk::new(8 * 1024)))
            .unwrap();
    }

    // Send the second chunk for each file
    for file_number in 0..reader_files.len() {
        request_sender
            .send((file_number, RecycledChunk::new(8 * 1024)))
            .unwrap();
    }

    let reader_join_handle = thread::spawn({
        let settings = settings.clone();
        move || {
            reader(
                &request_receiver,
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
        if let Ok(chunk) = receiver.recv() {
            mergeable_files.push(MergeableFile {
                current_chunk: Rc::new(chunk),
                file_number,
                line_idx: 0,
                receiver,
            });
        }
    }

    Ok(FileMerger {
        heap: binary_heap_plus::BinaryHeap::from_vec_cmp(
            mergeable_files,
            FileComparator { settings },
        ),
        request_sender,
        prev: None,
        reader_join_handle,
    })
}
/// The struct on the reader thread representing an input file
struct ReaderFile<M: MergeInput> {
    file: M,
    sender: SyncSender<Chunk>,
    carry_over: Vec<u8>,
}

/// The function running on the reader thread.
fn reader(
    recycled_receiver: &Receiver<(usize, RecycledChunk)>,
    files: &mut [Option<ReaderFile<impl MergeInput>>],
    settings: &GlobalSettings,
    separator: u8,
) -> UResult<()> {
    for (file_idx, recycled_chunk) in recycled_receiver.iter() {
        if let Some(ReaderFile {
            file,
            sender,
            carry_over,
        }) = &mut files[file_idx]
        {
            let should_continue = chunks::read(
                sender,
                recycled_chunk,
                None,
                carry_over,
                file.as_read(),
                &mut iter::empty(),
                separator,
                settings,
            )?;
            if !should_continue {
                // Remove the file from the list by replacing it with `None`.
                let ReaderFile { file, .. } = files[file_idx].take().unwrap();
                // Depending on the kind of the `MergeInput`, this may delete the file:
                file.finished_reading()?;
            }
        }
    }
    Ok(())
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
    request_sender: Sender<(usize, RecycledChunk)>,
    prev: Option<PreviousLine>,
    reader_join_handle: JoinHandle<UResult<()>>,
}

impl<'a> FileMerger<'a> {
    /// Write the merged contents to the output file.
    pub fn write_all(self, settings: &GlobalSettings, output: Output) -> UResult<()> {
        let mut out = output.into_write();
        self.write_all_to(settings, &mut out)
    }

    pub fn write_all_to(mut self, settings: &GlobalSettings, out: &mut impl Write) -> UResult<()> {
        while self.write_next(settings, out) {}
        drop(self.request_sender);
        self.reader_join_handle.join().unwrap()
    }

    fn write_next(&mut self, settings: &GlobalSettings, out: &mut impl Write) -> bool {
        if let Some(file) = self.heap.peek() {
            let prev = self.prev.replace(PreviousLine {
                chunk: file.current_chunk.clone(),
                line_idx: file.line_idx,
                file_number: file.file_number,
            });

            file.current_chunk.with_contents(|contents| {
                let current_line = &contents.lines[file.line_idx];
                if settings.unique {
                    if let Some(prev) = &prev {
                        let cmp = compare_by(
                            &prev.chunk.lines()[prev.line_idx],
                            current_line,
                            settings,
                            prev.chunk.line_data(),
                            file.current_chunk.line_data(),
                        );
                        if cmp == Ordering::Equal {
                            return;
                        }
                    }
                }
                current_line.print(out, settings);
            });

            let was_last_line_for_file = file.current_chunk.lines().len() == file.line_idx + 1;

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
                        .send((prev.file_number, prev_chunk.recycle()))
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
            &a.current_chunk.lines()[a.line_idx],
            &b.current_chunk.lines()[b.line_idx],
            self.settings,
            a.current_chunk.line_data(),
            b.current_chunk.line_data(),
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
fn check_child_success(mut child: Child, program: &str) -> UResult<()> {
    if matches!(
        child.wait().map(|e| e.code()),
        Ok(Some(0)) | Ok(None) | Err(_)
    ) {
        Ok(())
    } else {
        Err(SortError::CompressProgTerminatedAbnormally {
            prog: program.to_owned(),
        }
        .into())
    }
}

/// A temporary file that can be written to.
pub trait WriteableTmpFile: Sized {
    type Closed: ClosedTmpFile;
    type InnerWrite: Write;
    fn create(file: (File, PathBuf), compress_prog: Option<&str>) -> UResult<Self>;
    /// Closes the temporary file.
    fn finished_writing(self) -> UResult<Self::Closed>;
    fn as_write(&mut self) -> &mut Self::InnerWrite;
}
/// A temporary file that is (temporarily) closed, but can be reopened.
pub trait ClosedTmpFile {
    type Reopened: MergeInput;
    /// Reopens the temporary file.
    fn reopen(self) -> UResult<Self::Reopened>;
}
/// A pre-sorted input for merging.
pub trait MergeInput: Send {
    type InnerRead: Read;
    /// Cleans this `MergeInput` up.
    /// Implementations may delete the backing file.
    fn finished_reading(self) -> UResult<()>;
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

    fn create((file, path): (File, PathBuf), _: Option<&str>) -> UResult<Self> {
        Ok(Self {
            file: BufWriter::new(file),
            path,
        })
    }

    fn finished_writing(self) -> UResult<Self::Closed> {
        Ok(ClosedPlainTmpFile { path: self.path })
    }

    fn as_write(&mut self) -> &mut Self::InnerWrite {
        &mut self.file
    }
}
impl ClosedTmpFile for ClosedPlainTmpFile {
    type Reopened = PlainTmpMergeInput;
    fn reopen(self) -> UResult<Self::Reopened> {
        Ok(PlainTmpMergeInput {
            file: File::open(&self.path).map_err(|error| SortError::OpenTmpFileFailed { error })?,
            path: self.path,
        })
    }
}
impl MergeInput for PlainTmpMergeInput {
    type InnerRead = File;

    fn finished_reading(self) -> UResult<()> {
        // we ignore failures to delete the temporary file,
        // because there is a race at the end of the execution and the whole
        // temporary directory might already be gone.
        let _ = fs::remove_file(self.path);
        Ok(())
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

    fn create((file, path): (File, PathBuf), compress_prog: Option<&str>) -> UResult<Self> {
        let compress_prog = compress_prog.unwrap();
        let mut command = Command::new(compress_prog);
        command.stdin(Stdio::piped()).stdout(file);
        let mut child = command
            .spawn()
            .map_err(|err| SortError::CompressProgExecutionFailed {
                code: err.raw_os_error().unwrap(),
            })?;
        let child_stdin = child.stdin.take().unwrap();
        Ok(Self {
            path,
            compress_prog: compress_prog.to_owned(),
            child,
            child_stdin: BufWriter::new(child_stdin),
        })
    }

    fn finished_writing(self) -> UResult<Self::Closed> {
        drop(self.child_stdin);
        check_child_success(self.child, &self.compress_prog)?;
        Ok(ClosedCompressedTmpFile {
            path: self.path,
            compress_prog: self.compress_prog,
        })
    }

    fn as_write(&mut self) -> &mut Self::InnerWrite {
        &mut self.child_stdin
    }
}
impl ClosedTmpFile for ClosedCompressedTmpFile {
    type Reopened = CompressedTmpMergeInput;

    fn reopen(self) -> UResult<Self::Reopened> {
        let mut command = Command::new(&self.compress_prog);
        let file = File::open(&self.path).unwrap();
        command.stdin(file).stdout(Stdio::piped()).arg("-d");
        let mut child = command
            .spawn()
            .map_err(|err| SortError::CompressProgExecutionFailed {
                code: err.raw_os_error().unwrap(),
            })?;
        let child_stdout = child.stdout.take().unwrap();
        Ok(CompressedTmpMergeInput {
            path: self.path,
            compress_prog: self.compress_prog,
            child,
            child_stdout,
        })
    }
}
impl MergeInput for CompressedTmpMergeInput {
    type InnerRead = ChildStdout;

    fn finished_reading(self) -> UResult<()> {
        drop(self.child_stdout);
        check_child_success(self.child, &self.compress_prog)?;
        let _ = fs::remove_file(self.path);
        Ok(())
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
    fn finished_reading(self) -> UResult<()> {
        Ok(())
    }
    fn as_read(&mut self) -> &mut Self::InnerRead {
        &mut self.inner
    }
}
