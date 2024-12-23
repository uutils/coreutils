// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
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
                        let (mut temp_file, copy_path) = tmp_dir.next_file()?;
                        let mut source_file = File::open(&file_path)?;
                        std::io::copy(&mut source_file, &mut temp_file)?;
                        *file = OsString::from(&copy_path);
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
pub fn merge(
    files: &mut [OsString],
    settings: &GlobalSettings,
    output: Output,
    tmp_dir: &mut TmpDirWrapper,
) -> UResult<()> {
    replace_output_file_in_input_files(files, output.as_output_name(), tmp_dir)?;
    if settings.compress_prog.is_none() {
        merge_with_retry::<WriteablePlainTmpFile>(files, settings, output, tmp_dir)
    } else {
        merge_with_retry::<WriteableCompressedTmpFile>(files, settings, output, tmp_dir)
    }
}

/// Merge logic to handle file-descriptor exhaustion.
/// First, an optimal merge (i.e. no temp-file preallocation) is attempted. If this succeeds then
/// we're all done. There exist GNU-compatibility tests that require that no temp-files
/// be created if one is not required, so we have to make our first attempt without a pre-allocating
/// a temp file.
/// If we fail to merge (due to file-descriptor exhaustion) on our first attempt, then retry with a
/// pre-allocated temp-file. This ensures that when we hit the file-descriptor exhaustion again we
/// will have a output file to merge our already opened input files into, and we can continue
/// to make progress.
fn merge_with_retry<Tmp: WriteableTmpFile>(
    files: &mut [OsString],
    settings: &GlobalSettings,
    output: Output,
    tmp_dir: &mut TmpDirWrapper,
) -> UResult<()> {
    match merge_with_file_limit::<Tmp>(
        files.iter().map(|file| PlainInputFile { path: file }),
        settings,
        &output,
        tmp_dir,
        None,
    ) {
        Ok(()) => Ok(()),
        Err(_err) => {
            // Pre-allocate the temp file that we can use for partial merges.
            let tmp_file_option = Some(Tmp::create(
                tmp_dir.next_file()?,
                settings.compress_prog.as_deref(),
            )?);
            merge_with_file_limit::<Tmp>(
                files.iter().map(|file| PlainInputFile { path: file }),
                settings,
                &output,
                tmp_dir,
                tmp_file_option,
            )
        }
    }
}

/// Open and merge the input files.
/// If possible, merge to the final output file.
/// Reasons not to merge to the final-output file...
/// 1 - if the number of input files exceeds the batch-size.
/// 2 - if we run out of file-descriptors..
/// In either case that we don't manage to merge to the final output file, collect a vector
/// of the intermediate partial-merges (i.e. the temp files) and tail-call this function again.
/// Eventually the initial input set will be reduced to a set small enough that we can finally
/// merge to the target output file.
pub fn merge_with_file_limit<Tmp: WriteableTmpFile>(
    input_files: impl ExactSizeIterator<Item = impl ClosedFile>,
    settings: &GlobalSettings,
    output: &Output,
    tmp_dir: &mut TmpDirWrapper,
    mut tmp_file_option: Option<Tmp>,
) -> UResult<()> {
    // Code assumes settings.merge_batch_size >= 2. Assert it!
    assert!(settings.merge_batch_size >= 2);
    // Merge down all the input files into as few temporary files as we can (ideally 0
    // if possible - i.e. merge directly to output).
    let mut output_temporary_files = vec![];
    let mut opened_files = vec![];
    for input_file in input_files {
        if opened_files.len() >= settings.merge_batch_size {
            // Check that we've not somehow accidentally violated our merge-size requirement.
            assert_eq!(opened_files.len(), settings.merge_batch_size);
            // We have a full batch. Merge them.
            let merger = merge_without_limit(opened_files.into_iter(), settings)?;
            // If we haven't already got a temp-file to merge into, make one now...
            let mut tmp_file = match tmp_file_option {
                Some(t) => t,
                None => Tmp::create(tmp_dir.next_file()?, settings.compress_prog.as_deref())?,
            };
            merger.write_all_to(settings, tmp_file.as_write())?;
            output_temporary_files.push(tmp_file.finished_writing()?);
            tmp_file_option = Some(Tmp::create(
                tmp_dir.next_file()?,
                settings.compress_prog.as_deref(),
            )?);
            opened_files = vec![];
        }

        // Make a backup in case our first attempt to open fails (due to file-descriptor exhaustion).
        let input_file_backup = input_file.clone();

        match input_file.open() {
            Ok(opened_file) => opened_files.push(opened_file),
            Err(err) => {
                // We've run out of descriptors. If we've only managed to open one file then give up,
                // otherwise merge what we've got.
                if opened_files.len() < 2 {
                    return Err(err);
                }
                let merger = merge_without_limit(opened_files.into_iter(), settings)?;
                // If we haven't already got a temp-file to merge into, make one now...
                let mut tmp_file = match tmp_file_option {
                    Some(t) => t,
                    None => Tmp::create(tmp_dir.next_file()?, settings.compress_prog.as_deref())?,
                };
                merger.write_all_to(settings, tmp_file.as_write())?;
                output_temporary_files.push(tmp_file.finished_writing()?);
                tmp_file_option = Some(Tmp::create(
                    tmp_dir.next_file()?,
                    settings.compress_prog.as_deref(),
                )?);

                // Now retry the open. If we fail this time, give up completely and return error.
                opened_files = vec![input_file_backup.open()?];
            }
        }
    }
    // If we've opened some files but not yet merged them...
    if !opened_files.is_empty() {
        let merger = merge_without_limit(opened_files.into_iter(), settings)?;
        // If we have no output temp files at this point, we can just merge to the final output and be done.
        if output_temporary_files.is_empty() {
            return merger.write_all_to(settings, &mut output.writer());
        }
        // If we get to here then we have at least one other open temp file, need to do another round of merges.
        // If we haven't already got a temp-file to merge into, make one now...
        let mut tmp_file = match tmp_file_option {
            Some(t) => t,
            None => Tmp::create(tmp_dir.next_file()?, settings.compress_prog.as_deref())?,
        };
        merger.write_all_to(settings, tmp_file.as_write())?;
        output_temporary_files.push(tmp_file.finished_writing()?);
        tmp_file_option = Some(Tmp::create(
            tmp_dir.next_file()?,
            settings.compress_prog.as_deref(),
        )?);
    }
    if output_temporary_files.is_empty() {
        // If there are no temporary files we must be done.
        Ok(())
    } else {
        merge_with_file_limit::<Tmp>(
            output_temporary_files.into_iter(),
            settings,
            output,
            tmp_dir,
            tmp_file_option,
        )
    }
}

/// Merge a batch of files. Files are already opened at this point, all considerations of
/// batch sizes and dealing with potential file-descriptor exhaustion have already been taken
/// of.
fn merge_without_limit<F: Iterator<Item = impl MergeInput>>(
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
            file,
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
                settings.line_ending.into(),
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
    for (file_idx, recycled_chunk) in recycled_receiver {
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
struct FileMerger<'a> {
    heap: binary_heap_plus::BinaryHeap<MergeableFile, FileComparator<'a>>,
    request_sender: Sender<(usize, RecycledChunk)>,
    prev: Option<PreviousLine>,
    reader_join_handle: JoinHandle<UResult<()>>,
}

impl FileMerger<'_> {
    fn write_all_to(mut self, settings: &GlobalSettings, out: &mut impl Write) -> UResult<()> {
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

            file.current_chunk.with_dependent(|_, contents| {
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

impl Compare<MergeableFile> for FileComparator<'_> {
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
    type Closed: ClosedFile;
    type InnerWrite: Write;
    fn create(file: (File, PathBuf), compress_prog: Option<&str>) -> UResult<Self>;
    /// Closes the temporary file.
    fn finished_writing(self) -> UResult<Self::Closed>;
    fn as_write(&mut self) -> &mut Self::InnerWrite;
}

/// A file that is (temporarily) closed, but can be reopened.
pub trait ClosedFile: Clone {
    type Opened: MergeInput;
    /// Opens temporary file.
    fn open(self) -> UResult<Self::Opened>;
}

/// A pre-sorted input for merging.
pub trait MergeInput: Send + 'static {
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
#[derive(Clone)]
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
impl ClosedFile for ClosedPlainTmpFile {
    type Opened = PlainTmpMergeInput;
    fn open(self) -> UResult<Self::Opened> {
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
#[derive(Clone)]
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
impl ClosedFile for ClosedCompressedTmpFile {
    type Opened = CompressedTmpMergeInput;

    fn open(self) -> UResult<Self::Opened> {
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

#[derive(Clone)]
pub struct PlainInputFile<'a> {
    path: &'a OsString,
}

impl ClosedFile for PlainInputFile<'_> {
    type Opened = PlainMergeInput;
    fn open(self) -> UResult<Self::Opened> {
        Ok(PlainMergeInput {
            inner: open(self.path)?,
        })
    }
}

pub struct PlainMergeInput {
    inner: Box<dyn Read + Send>,
}

impl MergeInput for PlainMergeInput {
    type InnerRead = Box<dyn Read + Send>;
    fn finished_reading(self) -> UResult<()> {
        Ok(())
    }
    fn as_read(&mut self) -> &mut Self::InnerRead {
        &mut self.inner
    }
}
