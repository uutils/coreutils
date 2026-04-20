// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Merge already sorted files.
//!
//! On most platforms this uses a multi-threaded reader/merger setup. On WASI
//! without atomics, a synchronous variant is used instead. The two
//! implementations live in sibling modules and are selected via cfg at the
//! module boundary.

use std::{
    ffi::{OsStr, OsString},
    fs::{self, File},
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    rc::Rc,
};

use uucore::error::UResult;

use crate::{
    GlobalSettings, Output, SortError, chunks::Chunk, current_open_fd_count, fd_soft_limit, open,
    tmp_dir::TmpDirWrapper,
};

#[cfg(not(wasi_no_threads))]
mod threaded;
#[cfg(not(wasi_no_threads))]
use threaded as runner;

#[cfg(wasi_no_threads)]
mod sync;
#[cfg(wasi_no_threads)]
use sync as runner;

/// If the output file occurs in the input files as well, copy the contents of the output file
/// and replace its occurrences in the inputs with that copy.
fn replace_output_file_in_input_files(
    files: &mut [OsString],
    output: Option<&OsStr>,
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
                        fs::copy(file_path, &copy_path)
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

/// Determine the effective merge batch size, enforcing a minimum and respecting the
/// file-descriptor soft limit after reserving stdio/output and a safety margin.
fn effective_merge_batch_size(settings: &GlobalSettings) -> usize {
    const MIN_BATCH_SIZE: usize = 2;
    const RESERVED_TMP_OUTPUT: usize = 1;
    const RESERVED_CTRL_C: usize = 2;
    const RESERVED_RANDOM_SOURCE: usize = 1;
    const SAFETY_MARGIN: usize = 1;
    let mut batch_size = settings.merge_batch_size.max(MIN_BATCH_SIZE);

    if let Some(limit) = fd_soft_limit() {
        let open_fds = current_open_fd_count().unwrap_or(3);
        let mut reserved = RESERVED_TMP_OUTPUT + RESERVED_CTRL_C + SAFETY_MARGIN;
        if settings.salt.is_some() {
            reserved = reserved.saturating_add(RESERVED_RANDOM_SOURCE);
        }
        let available_inputs = limit.saturating_sub(open_fds.saturating_add(reserved));
        if available_inputs >= MIN_BATCH_SIZE {
            batch_size = batch_size.min(available_inputs);
        } else {
            batch_size = MIN_BATCH_SIZE;
        }
    }

    batch_size
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
    let files = files
        .iter()
        .map(|file| open(file).map(|file| PlainMergeInput { inner: file }));

    if !runner::SUPPORTS_COMPRESSION && settings.compress_prog.is_some() {
        let _ = writeln!(
            std::io::stderr(),
            "sort: warning: --compress-program is ignored on this platform"
        );
        return merge_with_file_limit::<_, _, WriteablePlainTmpFile>(
            files, settings, output, tmp_dir,
        );
    }

    if settings.compress_prog.is_none() {
        merge_with_file_limit::<_, _, WriteablePlainTmpFile>(files, settings, output, tmp_dir)
    } else {
        merge_with_file_limit::<_, _, WriteableCompressedTmpFile>(files, settings, output, tmp_dir)
    }
}

/// Merge and write to output, dispatching to the active runner.
fn do_merge_to_output<M: MergeInput + 'static>(
    files: impl Iterator<Item = UResult<M>>,
    settings: &GlobalSettings,
    output: Output,
) -> UResult<()> {
    runner::merge_without_limit(files, settings)?.write_all(settings, output)
}

/// Merge and write to a writer, dispatching to the active runner.
fn do_merge_to_writer<M: MergeInput + 'static>(
    files: impl Iterator<Item = UResult<M>>,
    settings: &GlobalSettings,
    out: &mut impl Write,
) -> UResult<()> {
    runner::merge_without_limit(files, settings)?.write_all_to(settings, out)
}

// Merge already sorted `MergeInput`s.
pub fn merge_with_file_limit<
    M: MergeInput + 'static,
    F: ExactSizeIterator<Item = UResult<M>>,
    Tmp: WriteableTmpFile + 'static,
>(
    files: F,
    settings: &GlobalSettings,
    output: Output,
    tmp_dir: &mut TmpDirWrapper,
) -> UResult<()> {
    let batch_size = effective_merge_batch_size(settings);
    debug_assert!(batch_size >= 2);

    if files.len() <= batch_size {
        do_merge_to_output(files, settings, output)
    } else {
        let mut temporary_files = vec![];
        let mut batch = Vec::with_capacity(batch_size);
        for file in files {
            batch.push(file);
            if batch.len() >= batch_size {
                assert_eq!(batch.len(), batch_size);
                let full_batch = std::mem::replace(&mut batch, Vec::with_capacity(batch_size));

                let mut tmp_file =
                    Tmp::create(tmp_dir.next_file()?, settings.compress_prog.as_deref())?;
                do_merge_to_writer(full_batch.into_iter(), settings, tmp_file.as_write())?;
                temporary_files.push(tmp_file.finished_writing()?);
            }
        }
        // Merge any remaining files that didn't get merged in a full batch above.
        if !batch.is_empty() {
            assert!(batch.len() < batch_size);

            let mut tmp_file =
                Tmp::create(tmp_dir.next_file()?, settings.compress_prog.as_deref())?;
            do_merge_to_writer(batch.into_iter(), settings, tmp_file.as_write())?;
            temporary_files.push(tmp_file.finished_writing()?);
        }
        merge_with_file_limit::<_, _, Tmp>(
            temporary_files
                .into_iter()
                .map(Box::new(|c: Tmp::Closed| c.reopen())
                    as Box<
                        dyn FnMut(Tmp::Closed) -> UResult<<Tmp::Closed as ClosedTmpFile>::Reopened>,
                    >),
            settings,
            output,
            tmp_dir,
        )
    }
}

/// A struct to keep track of the previous line we encountered.
///
/// This is required for deduplication purposes.
pub(super) struct PreviousLine {
    pub chunk: Rc<Chunk>,
    pub line_idx: usize,
    // Only the threaded merger reads this back to recycle chunks.
    #[cfg_attr(wasi_no_threads, allow(dead_code))]
    pub file_number: usize,
}

/// Compares files by their current line.
pub(super) struct FileComparator<'a> {
    pub settings: &'a GlobalSettings,
}

/// Wait for the child to exit and check its exit code.
fn check_child_success(mut child: Child, program: &str) -> UResult<()> {
    if matches!(child.wait().map(|e| e.code()), Ok(Some(0) | None) | Err(_)) {
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
                prog: compress_prog.to_owned(),
                error: err,
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
        // mirroring what is done for ClosedPlainTmpFile
        let file =
            File::open(&self.path).map_err(|error| SortError::OpenTmpFileFailed { error })?;
        command.stdin(file).stdout(Stdio::piped()).arg("-d");
        let mut child = command
            .spawn()
            .map_err(|err| SortError::CompressProgExecutionFailed {
                prog: self.compress_prog.clone(),
                error: err,
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
        // Explicitly close stdout before waiting on the child process.
        #[allow(clippy::drop_non_drop)]
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
