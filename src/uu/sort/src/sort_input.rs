// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Input file handling for sort merge.
//!
//! Dedupes duplicate paths via mmap and defers opening unique files until
//! iteration so `merge_with_file_limit` respects its batch size.

use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use memmap2::Mmap;
use uucore::error::UResult;

use crate::{STDIN_FILE, SortError};

/// An inner enum representing the actual source of a sort input.
#[derive(Debug)]
enum SortInputInner {
    /// An already-opened regular file.
    File(File),
    /// A memory-mapped file shared across duplicate paths, with an
    /// independent cursor per instance.
    Mmap {
        data: Arc<Mmap>,
        offset: AtomicUsize,
    },
    Stdin,
    /// A unique file whose open() is deferred until iteration.
    LazyFile(PathBuf),
}

/// Handle to a single sort input (file, mmap, or stdin).
#[derive(Debug)]
pub struct SortInput {
    inner: SortInputInner,
}

impl SortInput {
    /// Open a path directly (stdin if `"-"`).
    pub fn new(path: &OsStr) -> UResult<Self> {
        if path == STDIN_FILE {
            return Ok(Self {
                inner: SortInputInner::Stdin,
            });
        }

        let path = Path::new(path);
        match File::open(path) {
            Ok(f) => Ok(Self {
                inner: SortInputInner::File(f),
            }),
            Err(error) => Err(SortError::ReadFailed {
                path: path.to_owned(),
                error,
            }
            .into()),
        }
    }

    fn from_mmap(data: Arc<Mmap>) -> Self {
        Self {
            inner: SortInputInner::Mmap {
                data,
                offset: AtomicUsize::new(0),
            },
        }
    }

    /// Returns true if this input is stdin.
    pub fn is_stdin(&self) -> bool {
        matches!(self.inner, SortInputInner::Stdin)
    }
}

impl Read for SortInput {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Open LazyFile on first read (fallback if iterator didn't do it).
        if let SortInputInner::LazyFile(path) = &self.inner {
            let file = File::open(path)?;
            self.inner = SortInputInner::File(file);
        }

        match &mut self.inner {
            SortInputInner::File(file) => file.read(buf),
            SortInputInner::Mmap { data, offset } => {
                let pos = offset.load(Ordering::Relaxed);
                let available = data.len().saturating_sub(pos);
                let to_read = buf.len().min(available);
                if to_read > 0 {
                    buf[..to_read].copy_from_slice(&data[pos..pos + to_read]);
                    offset.fetch_add(to_read, Ordering::Relaxed);
                }
                Ok(to_read)
            }
            SortInputInner::Stdin => {
                let mut stdin = io::stdin();
                stdin.read(buf)
            }
            SortInputInner::LazyFile(_) => unreachable!(),
        }
    }
}

impl From<SortInput> for Box<dyn Read + Send> {
    fn from(input: SortInput) -> Self {
        Box::new(input) as Box<dyn Read + Send>
    }
}

/// Collection of sort inputs.
///
/// Preserves argument order and multiplicity. Duplicate paths are mmap'd once
/// and shared; unique paths are stored lazily and opened during iteration so the
/// merge batch size limits active FDs.
#[derive(Debug)]
pub struct SortInputs {
    inputs: Vec<SortInput>,
}

impl SortInputs {
    #[allow(dead_code)]
    // Used only in unit tests.
    pub fn from_files(files: &[std::ffi::OsString]) -> UResult<Self> {
        Self::from_files_with_output(files, None)
    }

    /// Build a `SortInputs` from paths.
    ///
    /// - Duplicate paths → one mmap shared across all instances.
    /// - Unique paths → stored as `LazyFile` and opened when the iterator yields them.
    /// - `output_as_input`: pre-created mmap used when the output file is also an input.
    pub fn from_files_with_output(
        files: &[std::ffi::OsString],
        output_as_input: Option<(PathBuf, Arc<Mmap>)>,
    ) -> UResult<Self> {
        let mut inputs = Vec::with_capacity(files.len());

        // First pass: count occurrences of each path to identify duplicates
        let mut path_counts: HashMap<PathBuf, usize> = HashMap::new();
        for file in files {
            if file != STDIN_FILE {
                let path = Path::new(file);
                let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                *path_counts.entry(canonical).or_insert(0) += 1;
            }
        }

        // Second pass: build inputs
        // - Unique files: LazyFile (opened during iteration)
        // - Duplicate files: mmap once, share it
        // - Output-as-input: use pre-created mmap
        // - Stdin: single occurrence
        let mut opened_files: HashMap<PathBuf, Arc<Mmap>> = HashMap::new();

        for file in files {
            if file == STDIN_FILE {
                inputs.push(SortInput {
                    inner: SortInputInner::Stdin,
                });
            } else {
                let path = Path::new(file);
                let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

                // Check if this is the output file used as input
                if let Some((ref output_path, ref output_mmap)) = output_as_input {
                    if canonical == *output_path {
                        inputs.push(SortInput::from_mmap(output_mmap.clone()));
                        continue;
                    }
                }

                if *path_counts.get(&canonical).unwrap_or(&0) > 1 {
                    // Duplicate file: use mmap
                    let mmap = if let Some(mmap) = opened_files.get(&canonical) {
                        mmap.clone()
                    } else {
                        let f = File::open(path).map_err(|error| SortError::ReadFailed {
                            path: path.to_owned(),
                            error,
                        })?;
                        // SAFETY: We keep the file open for the lifetime of the mmap,
                        // and we only read from it. The file is not modified.
                        let mmap = Arc::new(unsafe { Mmap::map(&f) }.map_err(|error| {
                            SortError::ReadFailed {
                                path: path.to_owned(),
                                error,
                            }
                        })?);
                        opened_files.insert(canonical, mmap.clone());
                        mmap
                    };
                    inputs.push(SortInput::from_mmap(mmap));
                } else {
                    // Unique file: defer opening until iteration so that
                    // merge_with_file_limit can respect its batch_size and
                    // avoid exceeding the file-descriptor soft limit.
                    inputs.push(SortInput {
                        inner: SortInputInner::LazyFile(path.to_path_buf()),
                    });
                }
            }
        }

        Ok(Self { inputs })
    }

    /// Returns the total number of inputs (including duplicates).
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.inputs.len()
    }

    /// Returns true if there are no inputs.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }

    /// Returns the number of unique sources (stdin + unique files + mmap groups).
    #[allow(dead_code)]
    pub fn unique_count(&self) -> usize {
        let mut file_count = 0;
        let mut stdin_present = false;

        // Count unique mmap instances by Arc pointer
        let mut seen_mmaps = std::collections::HashSet::new();

        for input in &self.inputs {
            match &input.inner {
                SortInputInner::Stdin => {
                    stdin_present = true;
                }
                SortInputInner::File(_) | SortInputInner::LazyFile(_) => {
                    file_count += 1;
                }
                SortInputInner::Mmap { data, .. } => {
                    seen_mmaps.insert(Arc::as_ptr(data));
                }
            }
        }

        file_count + seen_mmaps.len() + usize::from(stdin_present)
    }

    /// Iterate over the inputs without consuming them.
    #[allow(dead_code)]
    pub fn iter(&self) -> impl Iterator<Item = &SortInput> {
        self.inputs.iter()
    }
}

/// Iterator that opens LazyFile entries as they are yielded.
#[derive(Debug)]
pub struct SortInputsIntoIter {
    inner: std::vec::IntoIter<SortInput>,
}

impl Iterator for SortInputsIntoIter {
    type Item = UResult<SortInput>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut input = self.inner.next()?;

        // Convert LazyFile to File so errors surface during iteration.
        if let SortInputInner::LazyFile(path) = &input.inner {
            match File::open(path) {
                Ok(file) => input.inner = SortInputInner::File(file),
                Err(error) => {
                    return Some(Err(SortError::ReadFailed { path: path.clone(), error }.into()));
                }
            }
        }
        Some(Ok(input))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for SortInputsIntoIter {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl IntoIterator for SortInputs {
    type Item = UResult<SortInput>;
    type IntoIter = SortInputsIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        SortInputsIntoIter {
            inner: self.inputs.into_iter(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_sort_input_new_file() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile.write_all(b"hello world").expect("should write to temp file");
        tmpfile.flush().expect("should flush temp file");

        let input = SortInput::new(tmpfile.path().as_os_str()).expect("should create sort input");
        assert!(!input.is_stdin());
    }

    #[test]
    fn test_sort_input_new_stdin() {
        let input = SortInput::new(OsStr::new("-")).expect("should create sort input for stdin");
        assert!(input.is_stdin());
    }

    #[test]
    fn test_sort_input_new_missing_file() {
        let result = SortInput::new(OsStr::new("/nonexistent/path/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_sort_input_mmap_read() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile.write_all(b"mmap test data").expect("should write to temp file");
        tmpfile.flush().expect("should flush temp file");

        let file = File::open(tmpfile.path()).expect("should open temp file");
        let mmap = Arc::new(unsafe { Mmap::map(&file).expect("should mmap temp file") });
        let mut input = SortInput::from_mmap(mmap);

        let mut buf = [0u8; 14];
        let n = input.read(&mut buf).expect("should read from input");
        assert_eq!(n, 14);
        assert_eq!(&buf, b"mmap test data");
    }

    #[test]
    fn test_sort_input_into_box_read() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile.write_all(b"test data").expect("should write to temp file");
        tmpfile.flush().expect("should flush temp file");

        let file = File::open(tmpfile.path()).expect("should open temp file");
        let mmap = Arc::new(unsafe { Mmap::map(&file).expect("should mmap temp file") });
        let input = SortInput::from_mmap(mmap);
        let mut reader: Box<dyn Read + Send> = input.into();
        let mut buf = [0u8; 9];
        let n = reader.read(&mut buf).expect("should read from boxed reader");
        assert_eq!(n, 9);
        assert_eq!(&buf, b"test data");
    }

    #[test]
    fn test_sort_input_mmap_independent_reads() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile.write_all(b"independent reads").expect("should write to temp file");
        tmpfile.flush().expect("should flush temp file");

        let file = File::open(tmpfile.path()).expect("should open temp file");
        let mmap = Arc::new(unsafe { Mmap::map(&file).expect("should mmap temp file") });

        let mut input1 = SortInput::from_mmap(mmap.clone());
        let mut input2 = SortInput::from_mmap(mmap);

        // Both should be able to read independently
        let mut buf1 = [0u8; 11];
        input1.read(&mut buf1).expect("should read from first input");
        assert_eq!(&buf1, b"independent");

        let mut buf2 = [0u8; 11];
        input2.read(&mut buf2).expect("should read from second input");
        assert_eq!(&buf2, b"independent");
    }

    #[test]
    fn test_sort_inputs_empty() {
        let inputs = SortInputs::from_files(&[]).expect("should build empty sort inputs");
        assert_eq!(inputs.len(), 0);
        assert!(inputs.is_empty());
    }

    #[test]
    fn test_sort_inputs_single_file() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile.write_all(b"data").expect("should write to temp file");
        tmpfile.flush().expect("should flush temp file");

        let files = vec![tmpfile.path().as_os_str().to_os_string()];
        let inputs = SortInputs::from_files(&files).expect("should build sort inputs");
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs.unique_count(), 1);
    }

    #[test]
    fn test_sort_inputs_multiple_unique() {
        let mut tmpfile1 = NamedTempFile::new().expect("should create temp file");
        tmpfile1.write_all(b"data1").expect("should write to temp file");
        let mut tmpfile2 = NamedTempFile::new().expect("should create temp file");
        tmpfile2.write_all(b"data2").expect("should write to temp file");
        let mut tmpfile3 = NamedTempFile::new().expect("should create temp file");
        tmpfile3.write_all(b"data3").expect("should write to temp file");

        let files = vec![
            tmpfile1.path().as_os_str().to_os_string(),
            tmpfile2.path().as_os_str().to_os_string(),
            tmpfile3.path().as_os_str().to_os_string(),
        ];
        let inputs = SortInputs::from_files(&files).expect("should build sort inputs");
        assert_eq!(inputs.len(), 3);
        assert_eq!(inputs.unique_count(), 3);
    }

    #[test]
    fn test_sort_inputs_with_duplicates() {
        let mut tmpfile1 = NamedTempFile::new().expect("should create temp file");
        tmpfile1.write_all(b"data1").expect("should write to temp file");
        let mut tmpfile2 = NamedTempFile::new().expect("should create temp file");
        tmpfile2.write_all(b"data2").expect("should write to temp file");

        let files = vec![
            tmpfile1.path().as_os_str().to_os_string(),
            tmpfile1.path().as_os_str().to_os_string(),
            tmpfile2.path().as_os_str().to_os_string(),
        ];
        let inputs = SortInputs::from_files(&files).expect("should build sort inputs");
        assert_eq!(inputs.len(), 3);
        // 2 unique: file1 (mmap) and file2 (direct)
        assert_eq!(inputs.unique_count(), 2);
    }

    #[test]
    fn test_sort_inputs_duplicate_mmap_independent() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile.write_all(b"independent reads").expect("should write to temp file");
        tmpfile.flush().expect("should flush temp file");

        let files = vec![
            tmpfile.path().as_os_str().to_os_string(),
            tmpfile.path().as_os_str().to_os_string(),
        ];
        let inputs = SortInputs::from_files(&files).expect("should build sort inputs");

        // Both inputs should be able to read independently
        let mut buf1 = [0u8; 11];
        let mut input1 = SortInput {
            inner: match &inputs.iter().nth(0).expect("should get first input").inner {
                SortInputInner::Mmap { data, offset } => SortInputInner::Mmap {
                    data: data.clone(),
                    offset: AtomicUsize::new(offset.load(Ordering::Relaxed)),
                },
                _ => panic!("Expected mmap"),
            },
        };
        input1.read(&mut buf1).expect("should read from first input");
        assert_eq!(&buf1, b"independent");

        let mut buf2 = [0u8; 11];
        let mut input2 = SortInput {
            inner: match &inputs.iter().nth(1).expect("should get second input").inner {
                SortInputInner::Mmap { data, offset } => SortInputInner::Mmap {
                    data: data.clone(),
                    offset: AtomicUsize::new(offset.load(Ordering::Relaxed)),
                },
                _ => panic!("Expected mmap"),
            },
        };
        input2.read(&mut buf2).expect("should read from second input");
        assert_eq!(&buf2, b"independent");
    }

    #[test]
    fn test_sort_inputs_stdin_only() {
        let files = vec![OsString::from("-")];
        let inputs = SortInputs::from_files(&files).expect("should build sort inputs");
        assert_eq!(inputs.len(), 1);
        assert!(inputs.iter().next().expect("should get first input").is_stdin());
    }

    #[test]
    fn test_sort_inputs_duplicate_stdin_allowed() {
        // Verify that duplicate stdin is allowed (GNU Coreutils compatible)
        let files = vec![OsString::from("-"), OsString::from("-")];
        let result = SortInputs::from_files(&files);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sort_inputs_mixed_stdin_and_files_allowed() {
        // Verify that mixing stdin with files is allowed (GNU Coreutils compatible)
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile.write_all(b"data").expect("should write to temp file");

        let files = vec![
            OsString::from("-"),
            tmpfile.path().as_os_str().to_os_string(),
        ];
        let result = SortInputs::from_files(&files);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sort_inputs_order_preserved() {
        let mut tmpfile1 = NamedTempFile::new().expect("should create temp file");
        tmpfile1.write_all(b"data1").expect("should write to temp file");
        let mut tmpfile2 = NamedTempFile::new().expect("should create temp file");
        tmpfile2.write_all(b"data2").expect("should write to temp file");

        let files = vec![
            tmpfile2.path().as_os_str().to_os_string(),
            tmpfile1.path().as_os_str().to_os_string(),
        ];
        let inputs = SortInputs::from_files(&files).expect("should build sort inputs");
        let collected: Vec<_> = inputs.iter().collect();
        assert_eq!(collected.len(), 2);
    }

    #[test]
    fn test_sort_inputs_from_files_error() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile.write_all(b"data").expect("should write to temp file");

        let files = vec![
            tmpfile.path().as_os_str().to_os_string(),
            OsString::from("/nonexistent/path/file.txt"),
        ];
        let inputs = SortInputs::from_files(&files).expect("should build sort inputs");
        let mut iter = inputs.into_iter();
        assert!(iter.next().expect("should get first input").is_ok()); // first file opens successfully
        assert!(iter.next().expect("should get second input").is_err()); // second file fails to open
    }

    #[test]
    fn test_sort_inputs_into_iter() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile.write_all(b"data").expect("should write to temp file");

        let files = vec![tmpfile.path().as_os_str().to_os_string()];
        let inputs = SortInputs::from_files(&files).expect("should build sort inputs");
        let count = inputs.into_iter().count();
        assert_eq!(count, 1);
    }
}
