//! Iterating over a file by chunks, starting at the end of the file.
//!
//! Use [`rchunk`] to create a new iterator over chunks of bytes from
//! the file.
use std::io::{Read, Seek, SeekFrom};

/// Returns an iterator over `chunk_size` elements of a file, starting
/// at the end of the file.
///
/// Each chunk is a [`Vec`]<[`u8`]>, and successive chunks do not
/// overlap. If `chunk_size` does not divide the length of the file (in
/// bytes), then the chunk that is yielded last (that is, the chunk at
/// the beginning of the file) will have length less than `chunk_size`.
///
/// As a side effect of this iteration, [`seek`] is called on the file
/// `f`.
///
/// See the [`slice::rchunks`] function for a similar function that
/// works on slices.
///
/// # Examples
///
/// ```rust,ignore
/// let iter = rchunks(&mut Cursor::new("abcde"), 2);
/// assert_eq!(iter.next(), Some(vec![b'd', b'e']);
/// assert_eq!(iter.next(), Some(vec![b'b', b'c']);
/// assert_eq!(iter.next(), Some(vec![b'a']);
/// assert_eq!(iter.next(), None);
/// ```
pub fn rchunks<T>(f: &mut T, chunk_size: usize) -> ReverseChunks<'_, T>
where
    T: Seek + Read,
{
    ReverseChunks::new(f, chunk_size)
}

/// An iterator over a file in non-overlapping chunks from the end of the file.
///
/// Each chunk is a [`Vec`]<[`u8`]> of size [`chunk_size`] (except
/// possibly the last chunk, which might be smaller). Each call to
/// [`next`] will seek backwards through the given file.
pub struct ReverseChunks<'a, T> {
    /// The size of each chunk, in bytes.
    chunk_size: usize,

    /// The file to iterate over, by blocks, from the end to the beginning.
    file: &'a mut T,

    /// The total number of bytes in the file.
    size: u64,

    /// The total number of blocks to read.
    max_blocks_to_read: usize,

    /// The index of the next block to read.
    block_idx: usize,
}

impl<'a, T> ReverseChunks<'a, T>
where
    T: Seek,
{
    fn new(file: &'a mut T, chunk_size: usize) -> ReverseChunks<'a, T> {
        let size = file.seek(SeekFrom::End(0)).unwrap();
        let max_blocks_to_read = (size as f64 / chunk_size as f64).ceil() as usize;
        let block_idx = 0;
        ReverseChunks {
            chunk_size,
            file,
            size,
            max_blocks_to_read,
            block_idx,
        }
    }
}

impl<'a, T> Iterator for ReverseChunks<'a, T>
where
    T: Seek + Read,
{
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        // If there are no more chunks to read, terminate the iterator.
        if self.block_idx >= self.max_blocks_to_read {
            return None;
        }

        // The chunk size is `BLOCK_SIZE` for all but the last chunk
        // (that is, the chunk closest to the beginning of the file),
        // which contains the remainder of the bytes.
        let block_size = if self.block_idx == self.max_blocks_to_read - 1 {
            ((self.size - 1) % (self.chunk_size as u64)) + 1
        } else {
            self.chunk_size as u64
        };

        // Seek backwards by the next chunk, read the full chunk into
        // `buf`, and then seek back to the start of the chunk again.
        let mut buf = vec![0; self.chunk_size as usize];
        let pos = self
            .file
            .seek(SeekFrom::Current(-(block_size as i64)))
            .unwrap();
        self.file
            .read_exact(&mut buf[0..(block_size as usize)])
            .unwrap();
        let pos2 = self
            .file
            .seek(SeekFrom::Current(-(block_size as i64)))
            .unwrap();
        assert_eq!(pos, pos2);

        self.block_idx += 1;

        Some(buf[0..(block_size as usize)].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use crate::features::rchunks;
    use std::io::Cursor;

    #[test]
    fn test_empty_input() {
        let chunk_size = 2;
        let actual: Vec<Vec<u8>> = rchunks(&mut Cursor::new(""), chunk_size).collect();
        let expected: Vec<Vec<u8>> = Vec::new();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_smaller_than_block() {
        let chunk_size = 2;
        let actual: Vec<Vec<u8>> = rchunks(&mut Cursor::new("a"), chunk_size).collect();
        let mut expected: Vec<Vec<u8>> = Vec::new();
        expected.push("a".bytes().collect());
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_one_plus_partial_block() {
        let chunk_size = 2;
        let actual: Vec<Vec<u8>> = rchunks(&mut Cursor::new("abc"), chunk_size).collect();
        let mut expected: Vec<Vec<u8>> = Vec::new();
        expected.push("bc".bytes().collect());
        expected.push("a".bytes().collect());
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_exactly_two_blocks() {
        let chunk_size = 2;
        let actual: Vec<Vec<u8>> = rchunks(&mut Cursor::new("abcd"), chunk_size).collect();
        let mut expected: Vec<Vec<u8>> = Vec::new();
        expected.push("cd".bytes().collect());
        expected.push("ab".bytes().collect());
        assert_eq!(actual, expected);
    }
}
