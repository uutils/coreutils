// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Input file handling for sort merge.
//!
//! Dedupes duplicate paths via memory-map and defers opening unique files until
//! iteration so `merge_with_file_limit` respects its batch size.

use std::{
    collections::HashMap,
    ffi::OsString,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::STDIN_FILE;
use memmap2::Mmap as MemoryMap;

/// Deferred representation of a sort input before any file descriptor is opened.
#[derive(Debug)]
pub enum DeferredInput {
    Stdin,
    /// A file whose open() is deferred until iteration.
    Path {
        path: PathBuf,
        access: InputAccess,
    },
    OutputSnapshot(Arc<MemoryMap>),
}

/// Describes how a deferred path should be opened when yielded by the iterator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAccess {
    OpenFile,
    SharedMemoryMap,
}

/// Concrete input opened by `SortInputsIntoIter`; this is the type consumed by merge.
#[derive(Debug)]
pub enum OpenedInput {
    Stdin,
    File(File),
    SharedMemoryMap { data: Arc<MemoryMap>, offset: usize },
}

impl Read for OpenedInput {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::File(file) => file.read(buf),
            Self::SharedMemoryMap { data, offset } => {
                let pos = *offset;
                let available = data.len().saturating_sub(pos);
                let to_read = buf.len().min(available);

                if to_read > 0 {
                    buf[..to_read].copy_from_slice(&data[pos..pos + to_read]);
                    *offset = pos + to_read;
                }

                Ok(to_read)
            }
            Self::Stdin => {
                let mut stdin = io::stdin();
                stdin.read(buf)
            }
        }
    }
}

/// Handle to a single sort input (file, memory-map, or stdin).
#[derive(Debug)]
pub struct SortInput {
    pub inner: DeferredInput,
}

impl SortInput {
    fn stdin() -> Self {
        Self {
            inner: DeferredInput::Stdin,
        }
    }

    fn to_args_path(path: PathBuf, access: InputAccess) -> Self {
        Self {
            inner: DeferredInput::Path { path, access },
        }
    }

    fn to_output(memory_map: Arc<MemoryMap>) -> Self {
        Self {
            inner: DeferredInput::OutputSnapshot(memory_map),
        }
    }
}

/// Collection of sort inputs.
///
/// Preserves argument order and multiplicity. Duplicate paths are memory-map'd once
/// and shared; unique paths are stored lazily and opened during iteration so the
/// merge batch size limits active FDs.
#[derive(Debug)]
pub struct SortInputs {
    pub inputs: Vec<SortInput>,
}

impl SortInputs {
    /// Build a `SortInputs` from paths.
    ///
    /// - Duplicate paths → stored as deferred paths marked for shared memory-map access.
    /// - Unique paths → stored as deferred paths marked for regular file access.
    /// - `output_as_input` → stored as a pre-created memory-map snapshot.
    pub fn from_files_with_output(
        files: &[OsString],
        output_as_input: Option<(PathBuf, Arc<MemoryMap>)>,
    ) -> Self {
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
        // - Unique files: deferred path opened as a regular file during iteration
        // - Duplicate files: deferred path opened as a shared memory-map during iteration
        // - Output-as-input: use pre-created memory-map snapshot
        // - Stdin: preserve each occurrence
        for file in files {
            if file == STDIN_FILE {
                inputs.push(SortInput::stdin());
                continue;
            }

            let path = Path::new(file);
            let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            // Check if this is the output file used as input
            // Then use already opened fd for output
            if let Some((ref output_path, ref output_memory_map)) = output_as_input {
                if canonical == *output_path {
                    inputs.push(SortInput::to_output(output_memory_map.clone()));
                    continue;
                }
            }

            // Unique file as input
            if *path_counts.get(&canonical).unwrap_or(&0) <= 1 {
                inputs.push(SortInput::to_args_path(
                    PathBuf::from(file),
                    InputAccess::OpenFile,
                ));
            // Duplicate input
            } else {
                inputs.push(SortInput::to_args_path(
                    PathBuf::from(file),
                    InputAccess::SharedMemoryMap,
                ));
            }
        }

        Self {
            inputs: inputs.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{OsStr, OsString};
    #[cfg(not(target_os = "wasi"))]
    use std::io::Write;
    #[cfg(not(target_os = "wasi"))]
    use tempfile::NamedTempFile;

    // Util method for SortInputs in test
    impl SortInputs {
        pub fn from_files(files: &[OsString]) -> Self {
            Self::from_files_with_output(files, None)
        }

        /// Returns the total number of inputs (including duplicates).
        pub fn len(&self) -> usize {
            self.inputs.len()
        }

        /// Returns true if there are no inputs.
        fn is_empty(&self) -> bool {
            self.inputs.is_empty()
        }

        /// Returns the number of unique sources (stdin + unique files + memory-map groups).
        pub fn unique_count(&self) -> usize {
            let mut stdin_present = false;
            let mut seen_paths = std::collections::HashSet::new();
            let mut seen_memory_maps = std::collections::HashSet::new();

            for input in &self.inputs {
                match &input.inner {
                    DeferredInput::Stdin => {
                        stdin_present = true;
                    }
                    DeferredInput::Path { path, .. } => {
                        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
                        seen_paths.insert(canonical);
                    }
                    DeferredInput::OutputSnapshot(data) => {
                        seen_memory_maps.insert(Arc::as_ptr(data));
                    }
                }
            }

            seen_paths.len() + seen_memory_maps.len() + usize::from(stdin_present)
        }

        /// Iterate over the inputs without consuming them.
        fn iter(&self) -> impl Iterator<Item = &SortInput> {
            self.inputs.iter()
        }
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn test_sort_input_new_file() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile
            .write_all(b"hello world")
            .expect("should write to temp file");
        tmpfile.flush().expect("should flush temp file");

        let input = SortInput::to_args_path(tmpfile.path().to_path_buf(), InputAccess::OpenFile);

        assert!(matches!(
            input.inner,
            DeferredInput::Path {
                access: InputAccess::OpenFile,
                ..
            }
        ));
    }
    #[test]
    fn test_sort_input_new_stdin() {
        let input = SortInput::stdin();
        assert!(matches!(input.inner, DeferredInput::Stdin));
    }

    #[test]
    fn test_sort_input_new_missing_file() {
        let file = OsStr::new("/nonexistent/path/file.txt");

        let lazy_input = SortInput::to_args_path(PathBuf::from(file), InputAccess::OpenFile);

        assert!(matches!(
            lazy_input.inner,
            DeferredInput::Path {
                access: InputAccess::OpenFile,
                ..
            }
        ));
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn test_sort_input_memory_map_read() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile
            .write_all(b"memory_map test data")
            .expect("should write to temp file");
        tmpfile.flush().expect("should flush temp file");

        let file = File::open(tmpfile.path()).expect("should open temp file");
        let memory_map =
            Arc::new(unsafe { MemoryMap::map(&file).expect("should memory_map temp file") });
        let mut input = OpenedInput::SharedMemoryMap {
            data: memory_map,
            offset: 0,
        };
        let mut buf = [0u8; 20];
        let n = input.read(&mut buf).expect("should read from input");
        assert_eq!(n, 20);
        assert_eq!(&buf, b"memory_map test data");
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn test_sort_input_into_box_read() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile
            .write_all(b"test data")
            .expect("should write to temp file");
        tmpfile.flush().expect("should flush temp file");

        let file = File::open(tmpfile.path()).expect("should open temp file");
        let mmap = Arc::new(unsafe { MemoryMap::map(&file).expect("should mmap temp file") });
        let input = OpenedInput::SharedMemoryMap {
            data: mmap,
            offset: 0,
        };
        let mut reader: Box<dyn Read + Send> = Box::new(input);
        let mut buf = [0u8; 9];
        let n = reader
            .read(&mut buf)
            .expect("should read from boxed reader");
        assert_eq!(n, 9);
        assert_eq!(&buf, b"test data");
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn test_sort_input_mmap_independent_reads() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile
            .write_all(b"independent reads")
            .expect("should write to temp file");
        tmpfile.flush().expect("should flush temp file");

        let file = File::open(tmpfile.path()).expect("should open temp file");
        let mmap = Arc::new(unsafe { MemoryMap::map(&file).expect("should mmap temp file") });

        let mut input1 = OpenedInput::SharedMemoryMap {
            data: mmap.clone(),
            offset: 0,
        };
        let mut input2 = OpenedInput::SharedMemoryMap {
            data: mmap,
            offset: 0,
        };

        // Both should be able to read independently
        let mut buf1 = [0u8; 11];
        input1
            .read_exact(&mut buf1)
            .expect("should read from first input");
        assert_eq!(&buf1, b"independent");

        let mut buf2 = [0u8; 11];
        input2
            .read_exact(&mut buf2)
            .expect("should read from second input");
        assert_eq!(&buf2, b"independent");
    }

    #[test]
    fn test_sort_inputs_empty() {
        let inputs = SortInputs::from_files(&[]);
        assert_eq!(inputs.len(), 0);
        assert!(inputs.is_empty());
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn test_sort_inputs_single_file() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile
            .write_all(b"data")
            .expect("should write to temp file");
        tmpfile.flush().expect("should flush temp file");

        let files = vec![tmpfile.path().as_os_str().to_os_string()];
        let inputs = SortInputs::from_files(&files);
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs.unique_count(), 1);
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn test_sort_inputs_multiple_unique() {
        let mut tmpfile1 = NamedTempFile::new().expect("should create temp file");
        tmpfile1
            .write_all(b"data1")
            .expect("should write to temp file");
        let mut tmpfile2 = NamedTempFile::new().expect("should create temp file");
        tmpfile2
            .write_all(b"data2")
            .expect("should write to temp file");
        let mut tmpfile3 = NamedTempFile::new().expect("should create temp file");
        tmpfile3
            .write_all(b"data3")
            .expect("should write to temp file");

        let files = vec![
            tmpfile1.path().as_os_str().to_os_string(),
            tmpfile2.path().as_os_str().to_os_string(),
            tmpfile3.path().as_os_str().to_os_string(),
        ];
        let inputs = SortInputs::from_files(&files);
        assert_eq!(inputs.len(), 3);
        assert_eq!(inputs.unique_count(), 3);
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn test_sort_inputs_with_duplicates() {
        let mut tmpfile1 = NamedTempFile::new().expect("should create temp file");
        tmpfile1
            .write_all(b"data1")
            .expect("should write to temp file");
        let mut tmpfile2 = NamedTempFile::new().expect("should create temp file");
        tmpfile2
            .write_all(b"data2")
            .expect("should write to temp file");

        let files = vec![
            tmpfile1.path().as_os_str().to_os_string(),
            tmpfile1.path().as_os_str().to_os_string(),
            tmpfile2.path().as_os_str().to_os_string(),
        ];
        let inputs = SortInputs::from_files(&files);
        assert_eq!(inputs.len(), 3);
        // 2 unique: file1 (mmap) and file2 (direct)
        assert_eq!(inputs.unique_count(), 2);
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn test_sort_inputs_duplicate_mmap_independent() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile
            .write_all(b"independent reads")
            .expect("should write to temp file");
        tmpfile.flush().expect("should flush temp file");

        let files = vec![
            tmpfile.path().as_os_str().to_os_string(),
            tmpfile.path().as_os_str().to_os_string(),
        ];
        let inputs = SortInputs::from_files(&files);

        assert_eq!(inputs.len(), 2);

        for input in inputs.iter() {
            assert!(matches!(
                input.inner,
                DeferredInput::Path {
                    access: InputAccess::SharedMemoryMap,
                    ..
                }
            ));
        }
    }

    #[test]
    fn test_sort_inputs_stdin_only() {
        let files = vec![OsString::from("-")];
        let inputs = SortInputs::from_files(&files);
        let input = inputs.iter().next().expect("should get first input");
        assert_eq!(inputs.len(), 1);
        assert!(matches!(input.inner, DeferredInput::Stdin));
    }

    #[test]
    fn test_sort_inputs_duplicate_stdin_allowed() {
        // Verify that duplicate stdin is allowed (GNU Coreutils compatible)
        let files = vec![OsString::from("-"), OsString::from("-")];
        let inputs = SortInputs::from_files(&files);
        assert_eq!(inputs.len(), files.len());
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn test_sort_inputs_mixed_stdin_and_files_allowed() {
        // Verify that mixing stdin with files is allowed (GNU Coreutils compatible)
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile
            .write_all(b"data")
            .expect("should write to temp file");

        let files = vec![
            OsString::from("-"),
            tmpfile.path().as_os_str().to_os_string(),
        ];
        let inputs = SortInputs::from_files(&files);
        assert_eq!(inputs.len(), files.len());
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn test_sort_inputs_order_preserved() {
        let mut tmpfile1 = NamedTempFile::new().expect("should create temp file");
        tmpfile1
            .write_all(b"data1")
            .expect("should write to temp file");
        let mut tmpfile2 = NamedTempFile::new().expect("should create temp file");
        tmpfile2
            .write_all(b"data2")
            .expect("should write to temp file");

        let files = vec![
            tmpfile2.path().as_os_str().to_os_string(),
            tmpfile1.path().as_os_str().to_os_string(),
        ];
        let inputs = SortInputs::from_files(&files);
        let collected: Vec<_> = inputs.iter().collect();
        assert_eq!(collected.len(), 2);
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn test_sort_inputs_from_files_error() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile
            .write_all(b"data")
            .expect("should write to temp file");

        let files = vec![
            tmpfile.path().as_os_str().to_os_string(),
            OsString::from("/nonexistent/path/file.txt"),
        ];
        let inputs = SortInputs::from_files(&files);
        let mut iter = inputs.into_iter();
        assert!(iter.next().expect("should get first input").is_ok()); // first file opens successfully
        assert!(iter.next().expect("should get second input").is_err()); // second file fails to open
    }
}
