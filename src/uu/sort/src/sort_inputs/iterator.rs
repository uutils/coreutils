use crate::SortError;
use crate::sort_inputs::{DeferredInput, InputAccess, OpenedInput, SortInput, SortInputs};
use memmap2::Mmap as MemoryMap;
use std::{collections::HashMap, fs::File, path::PathBuf, sync::Arc, vec::IntoIter};
use uucore::error::UResult;

/// Iterator that opens deferred input entries as they are yielded.
#[derive(Debug, Default)]
pub struct SortInputsIntoIter {
    inner: IntoIter<SortInput>,
    memory_map_files: HashMap<PathBuf, Arc<MemoryMap>>,
}

impl SortInputsIntoIter {
    fn open_file(path: PathBuf) -> UResult<OpenedInput> {
        File::open(&path)
            .map(OpenedInput::File)
            .map_err(|error| SortError::ReadFailed { path, error }.into())
    }

    fn shared_memory_map(&mut self, path: PathBuf) -> UResult<OpenedInput> {
        if let Some(memory_map) = self.memory_map_files.get(&path) {
            return Ok(OpenedInput::SharedMemoryMap {
                data: memory_map.clone(),
                offset: 0,
            });
        }

        let file = File::open(&path).map_err(|error| SortError::ReadFailed {
            path: path.clone(),
            error,
        })?;

        // SAFETY: This creates a read-only memory map for an input file. The map is
        // only exposed through `OpenedInput::SharedMemoryMap`, which implements
        // `Read` by copying bytes out and never mutates the mapped region.
        let memory_map =
            Arc::new(
                unsafe { MemoryMap::map(&file) }.map_err(|error| SortError::ReadFailed {
                    path: path.clone(),
                    error,
                })?,
            );

        self.memory_map_files.insert(path, memory_map.clone());

        Ok(OpenedInput::SharedMemoryMap {
            data: memory_map,
            offset: 0,
        })
    }
}

impl Iterator for SortInputsIntoIter {
    type Item = UResult<OpenedInput>;

    fn next(&mut self) -> Option<Self::Item> {
        let input = self.inner.next()?;

        let opened_input = match input.inner {
            DeferredInput::Stdin => Ok(OpenedInput::Stdin),
            DeferredInput::Path {
                path,
                access: InputAccess::OpenFile,
            } => Self::open_file(path),
            DeferredInput::Path {
                path,
                access: InputAccess::SharedMemoryMap,
            } => self.shared_memory_map(path),
            DeferredInput::OutputSnapshot(data) => {
                Ok(OpenedInput::SharedMemoryMap { data, offset: 0 })
            }
        };

        Some(opened_input)
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
    type Item = UResult<OpenedInput>;
    type IntoIter = SortInputsIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        SortInputsIntoIter {
            inner: self.inputs.into_iter(),
            memory_map_files: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::SortInputsIntoIter;
    use crate::sort_inputs::{DeferredInput, InputAccess, OpenedInput, SortInput, SortInputs};
    use memmap2::Mmap as MemoryMap;
    #[cfg(not(target_os = "wasi"))]
    use std::{
        fs::File,
        io::{Read, Write},
        sync::Arc,
    };
    #[cfg(not(target_os = "wasi"))]
    use tempfile::NamedTempFile;

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn test_open_file_opens_as_file() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile
            .write_all(b"unique file data")
            .expect("should write to temp file");
        tmpfile.flush().expect("should flush temp file");

        let opened_input =
            SortInputsIntoIter::open_file(tmpfile.path().to_path_buf()).expect("should open input");

        match opened_input {
            OpenedInput::File(mut file) => {
                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .expect("should read opened file");
                assert_eq!(contents, "unique file data");
            }
            other => panic!("expected opened file, got {other:?}"),
        }
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn test_shared_memory_map_opens_as_shared_memory_map() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile
            .write_all(b"duplicate data")
            .expect("should write to temp file");
        tmpfile.flush().expect("should flush temp file");

        let mut iter = SortInputsIntoIter::default();
        let opened_input = iter
            .shared_memory_map(tmpfile.path().to_path_buf())
            .expect("should open shared memory map");

        match opened_input {
            OpenedInput::SharedMemoryMap { data, offset } => {
                assert_eq!(offset, 0);
                assert_eq!(&data[..], b"duplicate data");
            }
            other => panic!("expected shared memory map, got {other:?}"),
        }
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn test_shared_memory_maps_have_independent_offsets() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile
            .write_all(b"independent offsets")
            .expect("should write to temp file");
        tmpfile.flush().expect("should flush temp file");

        let path = tmpfile.path().to_path_buf();
        let mut iter = SortInputsIntoIter::default();
        let mut first = iter
            .shared_memory_map(path.clone())
            .expect("should open first shared memory map");
        let mut second = iter
            .shared_memory_map(path)
            .expect("should open second shared memory map");

        let mut first_buf = [0u8; 11];
        first
            .read_exact(&mut first_buf)
            .expect("should read from first mmap");
        assert_eq!(&first_buf, b"independent");

        let mut second_buf = [0u8; 11];
        second
            .read_exact(&mut second_buf)
            .expect("should read from second mmap");
        assert_eq!(&second_buf, b"independent");
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn test_sort_inputs_into_iter_opens_each_input_kind() {
        let mut open_file_input = NamedTempFile::new().expect("should create open-file input");
        open_file_input
            .write_all(b"open file input")
            .expect("should write open-file input");
        open_file_input
            .flush()
            .expect("should flush open-file input");

        let mut shared_memory_input =
            NamedTempFile::new().expect("should create shared-memory input");
        shared_memory_input
            .write_all(b"shared memory input")
            .expect("should write shared-memory input");
        shared_memory_input
            .flush()
            .expect("should flush shared-memory input");

        let mut output = NamedTempFile::new().expect("should create output file");
        output
            .write_all(b"output snapshot")
            .expect("should write output file");
        output.flush().expect("should flush output file");
        let output_file = File::open(output.path()).expect("should open output for snapshot");
        let output_snapshot =
            Arc::new(unsafe { MemoryMap::map(&output_file).expect("should mmap output snapshot") });

        let inputs = SortInputs {
            inputs: vec![
                SortInput {
                    inner: DeferredInput::Stdin,
                },
                SortInput {
                    inner: DeferredInput::Path {
                        path: open_file_input.path().to_path_buf(),
                        access: InputAccess::OpenFile,
                    },
                },
                SortInput {
                    inner: DeferredInput::Path {
                        path: shared_memory_input.path().to_path_buf(),
                        access: InputAccess::SharedMemoryMap,
                    },
                },
                SortInput {
                    inner: DeferredInput::OutputSnapshot(output_snapshot),
                },
            ],
        };

        let mut iter = inputs.into_iter();
        assert_eq!(iter.len(), 4);

        assert!(matches!(
            iter.next()
                .expect("should yield stdin")
                .expect("stdin is valid"),
            OpenedInput::Stdin
        ));
        assert_eq!(iter.len(), 3);

        match iter
            .next()
            .expect("should yield open-file input")
            .expect("should open file")
        {
            OpenedInput::File(mut file) => {
                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .expect("should read opened file");
                assert_eq!(contents, "open file input");
            }
            other => panic!("expected opened file, got {other:?}"),
        }

        match iter
            .next()
            .expect("should yield shared-memory input")
            .expect("should open shared-memory input")
        {
            OpenedInput::SharedMemoryMap { data, offset } => {
                assert_eq!(offset, 0);
                assert_eq!(&data[..], b"shared memory input");
            }
            other => panic!("expected shared memory map, got {other:?}"),
        }

        match iter
            .next()
            .expect("should yield output snapshot")
            .expect("should open output snapshot")
        {
            OpenedInput::SharedMemoryMap { data, offset } => {
                assert_eq!(offset, 0);
                assert_eq!(&data[..], b"output snapshot");
            }
            other => panic!("expected output snapshot memory map, got {other:?}"),
        }

        assert!(iter.next().is_none());
    }
}
