use crate::SortError;
use crate::sort_inputs::{SortInput, SortInputInner, SortInputs};
use std::{fs::File, vec::IntoIter};
use uucore::error::UResult;

/// Iterator that opens LazyFile entries as they are yielded.
#[derive(Debug)]
pub struct SortInputsIntoIter {
    inner: IntoIter<SortInput>,
}

impl Iterator for SortInputsIntoIter {
    type Item = UResult<SortInput>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut input = self.inner.next()?;

        if let SortInputInner::LazyFile(path) = &input.inner {
            match File::open(path) {
                Ok(file) => input.inner = SortInputInner::File(file),
                Err(error) => {
                    return Some(Err(SortError::ReadFailed {
                        path: path.clone(),
                        error,
                    }
                    .into()));
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
mod test {

    use crate::sort_inputs::SortInputs;
    #[cfg(not(target_os = "wasi"))]
    use std::io::Write;
    #[cfg(not(target_os = "wasi"))]
    use tempfile::NamedTempFile;

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn test_sort_inputs_into_iter() {
        let mut tmpfile = NamedTempFile::new().expect("should create temp file");
        tmpfile
            .write_all(b"data")
            .expect("should write to temp file");

        let files = vec![tmpfile.path().as_os_str().to_os_string()];
        let inputs = SortInputs::from_files(&files).expect("should build sort inputs");
        let count = inputs.into_iter().count();
        assert_eq!(count, 1);
    }
}
