//! Iterating over a file by chunks, either starting at the end of the file with [`ReverseChunks`]
//! or at the end of piped stdin with [`LinesChunk`] or [`BytesChunk`].
//!
//! Use [`ReverseChunks::new`] to create a new iterator over chunks of bytes from the file.

// spell-checker:ignore (ToDO) filehandle

use std::collections::vec_deque::{Iter, VecDeque};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use uucore::error::UResult;

/// When reading files in reverse in `bounded_tail`, this is the size of each
/// block read at a time.
pub const BLOCK_SIZE: u64 = 1 << 16;

/// The size of the backing buffer of a LinesChunk or BytesChunk. Some calculations concerning the
/// buffer assume that the target system's usize is greater than this BUFFER_SIZE, and therefore
/// convert from u64 to usize as long as it is known, that the value resides somewhere between 0
/// and the BUFFER_SIZE.
pub const BUFFER_SIZE: usize = 8192;

/// An iterator over a file in non-overlapping chunks from the end of the file.
///
/// Each chunk is a [`Vec`]<[`u8`]> of size [`BLOCK_SIZE`] (except
/// possibly the last chunk, which might be smaller). Each call to
/// [`ReverseChunks::next`] will seek backwards through the given file.
pub struct ReverseChunks<'a> {
    /// The file to iterate over, by blocks, from the end to the beginning.
    file: &'a File,

    /// The total number of bytes in the file.
    size: u64,

    /// The total number of blocks to read.
    max_blocks_to_read: usize,

    /// The index of the next block to read.
    block_idx: usize,
}

impl<'a> ReverseChunks<'a> {
    pub fn new(file: &'a mut File) -> ReverseChunks<'a> {
        let size = file.seek(SeekFrom::End(0)).unwrap();
        let max_blocks_to_read = (size as f64 / BLOCK_SIZE as f64).ceil() as usize;
        let block_idx = 0;
        ReverseChunks {
            file,
            size,
            max_blocks_to_read,
            block_idx,
        }
    }
}

impl<'a> Iterator for ReverseChunks<'a> {
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
            self.size % BLOCK_SIZE
        } else {
            BLOCK_SIZE
        };

        // Seek backwards by the next chunk, read the full chunk into
        // `buf`, and then seek back to the start of the chunk again.
        let mut buf = vec![0; BLOCK_SIZE as usize];
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

/// The type of the backing buffer of the BytesChunk and LinesChunk structs.
type ChunkBuffer = [u8; BUFFER_SIZE];

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BytesChunk {
    buffer: ChunkBuffer,
    bytes: usize,
}

impl BytesChunk {
    pub(crate) fn new() -> Self {
        Self {
            buffer: [0; BUFFER_SIZE],
            bytes: 0,
        }
    }

    /// Create a new chunk from an existing chunk. The new chunk's buffer will be copied from the
    /// old chunk's buffer, copying the slice `[offset..old_chunk.bytes]` into the new chunk's
    /// buffer but starting at 0 instead of offset. If the offset is larger or equal to
    /// `chunk.lines` then a new empty `BytesChunk` is returned.
    ///
    /// # Arguments
    ///
    /// * `chunk`: The chunk to create a new `BytesChunk` chunk from
    /// * `offset`: Start to copy the old chunk's buffer from this position. May not be larger
    ///             than `chunk.bytes`.
    ///
    /// returns: BytesChunk
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut chunk = BytesChunk::new();
    /// chunk.buffer[1] = 1;
    /// chunk.bytes = 2;
    /// let new_chunk = BytesChunk::from_chunk(&chunk, 0);
    /// assert_eq!(2, new_chunk.get_buffer().len());
    /// assert_eq!(&[0, 1], new_chunk.get_buffer());
    ///
    /// let new_chunk = BytesChunk::from_chunk(&chunk, 1);
    /// assert_eq!(1, new_chunk.get_buffer().len());
    /// assert_eq!(&[1], new_chunk.get_buffer());
    /// ```
    fn from_chunk(chunk: &Self, offset: usize) -> Self {
        if offset >= chunk.bytes {
            Self::new();
        }

        let mut buffer: ChunkBuffer = [0; BUFFER_SIZE];
        let slice = chunk.get_buffer_with(offset);
        buffer[..slice.len()].copy_from_slice(slice);
        Self {
            buffer,
            bytes: chunk.bytes - offset,
        }
    }

    pub(crate) fn get_buffer(&self) -> &[u8] {
        &self.buffer[..self.bytes]
    }

    pub(crate) fn get_buffer_with(&self, offset: usize) -> &[u8] {
        &self.buffer[offset..self.bytes]
    }

    pub(crate) fn fill(&mut self, filehandle: &mut BufReader<impl Read>) -> UResult<Option<usize>> {
        let num_bytes = filehandle.read(&mut self.buffer)?;
        self.bytes = num_bytes;
        if num_bytes == 0 {
            return Ok(None);
        }

        Ok(Some(self.bytes))
    }
}

pub(crate) struct BytesChunkBuffer {
    num_print: u64,
    bytes: u64,
    chunks: VecDeque<Box<BytesChunk>>,
}

impl BytesChunkBuffer {
    pub(crate) fn new(num_print: u64) -> Self {
        Self {
            bytes: 0,
            num_print,
            chunks: VecDeque::new(),
        }
    }

    /// Fills the chunks collection with chunks and consumes the reader completely. This method
    /// ensures that there are exactly as many chunks as needed to match `self.num_print` bytes, so
    /// there are in sum exactly `self.num_print` bytes stored in all chunks. The method returns
    /// an iterator over these chunks. If there are no chunks, for example because the piped stdin
    /// contained no bytes, or `num_print = 0` then `iterator.next` returns None.
    ///
    /// # Arguments
    ///
    /// * `reader`: A buffered reader with an inner element implementing the [`Read`] trait.
    ///
    /// returns: Result<Iter<Box<BytesChunk, Global>>, Box<dyn UError, Global>>
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::io::{BufReader, Cursor};
    ///
    /// let mut reader = BufReader::new(Cursor::new(""));
    /// let num_print = 0;
    /// let mut chunks = BytesChunkBuffer::new(num_print);
    /// let mut iter = chunks.fill(&mut reader).unwrap();
    ///
    /// let chunk = iter.next();
    /// assert!(chunk.is_none());
    ///
    /// let mut reader = BufReader::new(Cursor::new("a"));
    /// let num_print = 1;
    /// let mut chunks = BytesChunkBuffer::new(num_print);
    /// let mut iter = chunks.fill(&mut reader).unwrap();
    ///
    /// let chunk = iter.next();
    /// assert!(chunk.is_some());
    /// assert_eq!(&[b'a'], chunk.unwrap().get_buffer());
    /// assert_eq!(None, iter.next());
    /// ```
    pub(crate) fn fill(
        &mut self,
        reader: &mut BufReader<impl Read>,
    ) -> UResult<Iter<Box<BytesChunk>>> {
        let mut chunk = Box::new(BytesChunk::new());

        // fill chunks with all bytes from reader and reuse already instantiated chunks if possible
        while (chunk.fill(reader)?).is_some() {
            self.bytes += chunk.bytes as u64;
            self.chunks.push_back(chunk);

            let first = &self.chunks[0];
            if self.bytes - first.bytes as u64 > self.num_print {
                chunk = self.chunks.pop_front().unwrap();
                self.bytes -= chunk.bytes as u64;
            } else {
                chunk = Box::new(BytesChunk::new());
            }
        }

        // quit early if there are no chunks for example in case the pipe was empty
        if self.chunks.is_empty() {
            return Ok(self.chunks.iter());
        }

        let chunk = self.chunks.pop_front().unwrap();
        // calculate the offset in the first chunk and put the calculated chunk as first element in
        // the self.chunks collection.
        let offset = if self.num_print >= self.bytes {
            // ignore a passed in value exceeding the number of actually read bytes and treat it
            // like a value equal to the number of bytes.
            0
        } else {
            // the calculated offset must be in the range 0 to BUFFER_SIZE and is therefore safely
            // convertible to a usize without losses.
            (self.bytes - self.num_print) as usize
        };

        self.chunks
            .push_front(Box::new(BytesChunk::from_chunk(&chunk, offset)));

        Ok(self.chunks.iter())
    }
}

pub(crate) struct LinesChunk {
    buffer: ChunkBuffer,
    bytes: usize,
    lines: usize,
    delimiter: u8,
}

impl LinesChunk {
    pub(crate) fn new(delimiter: u8) -> Self {
        Self {
            buffer: [0; BUFFER_SIZE],
            bytes: 0,
            lines: 0,
            delimiter,
        }
    }

    fn count_lines(&self) -> usize {
        memchr::memchr_iter(self.delimiter, self.get_buffer()).count()
    }

    /// Creates a new [`LinesChunk`] from an existing one with an offset in lines. The new chunk
    /// contains exactly `chunk.lines - offset` lines. The offset in bytes is calculated and applied
    /// to the new chunk, so the new chunk contains only the bytes encountered after the offset in
    /// number of lines and the `delimiter`. If the offset is larger or equal to `chunk.lines` then
    /// a new empty `LinesChunk` is returned.
    ///
    /// # Arguments
    ///
    /// * `chunk`: The chunk to create the new chunk from
    /// * `offset`: The offset in number of lines (not bytes)
    ///
    /// returns: LinesChunk
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut chunk = LinesChunk::new(b'\n');
    /// // manually filling the buffer and setting the correct values for bytes and lines
    /// chunk.buffer[0..12].copy_from_slice("hello\nworld\n".as_bytes());
    /// chunk.bytes = 12;
    /// chunk.lines = 2;
    ///
    /// let offset = 1; // offset in number of lines
    /// let new_chunk = LinesChunk::from(&chunk, offset);
    /// assert_eq!("world\n".as_bytes(), new_chunk.get_buffer());
    /// assert_eq!(6, new_chunk.bytes);
    /// assert_eq!(1, new_chunk.lines);
    ///
    /// let offset = 13; // offset larger
    /// ```
    fn from_chunk(chunk: &Self, offset: usize) -> Self {
        if offset >= chunk.lines {
            Self::new(chunk.delimiter);
        }

        let mut buffer: ChunkBuffer = [0; BUFFER_SIZE];

        let bytes_offset = chunk.calculate_bytes_offset_from(offset);
        let slice = chunk.get_buffer_with(bytes_offset);
        buffer[..slice.len()].copy_from_slice(slice);

        Self {
            buffer,
            lines: chunk.lines - offset,
            bytes: chunk.bytes - bytes_offset,
            delimiter: chunk.delimiter,
        }
    }

    pub(crate) fn has_data(&self) -> bool {
        self.bytes > 0
    }

    pub(crate) fn get_buffer(&self) -> &[u8] {
        &self.buffer[..self.bytes]
    }

    pub(crate) fn get_buffer_with(&self, offset: usize) -> &[u8] {
        &self.buffer[offset..self.bytes]
    }

    pub(crate) fn get_lines(&self) -> usize {
        self.lines
    }

    pub(crate) fn increment_lines(&mut self) -> usize {
        self.lines += 1;
        self.lines
    }

    pub(crate) fn fill(&mut self, filehandle: &mut BufReader<impl Read>) -> UResult<Option<usize>> {
        let num_bytes = filehandle.read(&mut self.buffer)?;
        self.bytes = num_bytes;

        if num_bytes == 0 {
            self.lines = 0;
            return Ok(None);
        }

        self.lines = self.count_lines();
        Ok(Some(self.bytes))
    }

    fn calculate_bytes_offset_from(&self, offset: usize) -> usize {
        let mut lines_offset = offset;
        let mut bytes_offset = 0;
        for byte in self.get_buffer().iter() {
            if lines_offset == 0 {
                break;
            }
            if byte == &self.delimiter {
                lines_offset -= 1;
            }
            bytes_offset += 1;
        }
        bytes_offset
    }

    pub(crate) fn print_lines(&self, writer: &mut impl Write, offset: usize) -> UResult<()> {
        self.print_bytes(writer, self.calculate_bytes_offset_from(offset))
    }

    pub(crate) fn print_bytes(&self, writer: &mut impl Write, offset: usize) -> UResult<()> {
        writer.write_all(&self.buffer[offset..self.bytes])?;
        Ok(())
    }
}

pub struct LinesChunkBuffer {
    delimiter: u8,
    lines: u64,
    num_print: u64,
    chunks: VecDeque<Box<LinesChunk>>,
}

impl LinesChunkBuffer {
    pub(crate) fn new(delimiter: u8, num_print: u64) -> Self {
        Self {
            delimiter,
            num_print,
            lines: 0,
            chunks: VecDeque::new(),
        }
    }

    pub(crate) fn fill(
        &mut self,
        reader: &mut BufReader<impl Read>,
    ) -> UResult<Iter<Box<LinesChunk>>> {
        let mut chunk = Box::new(LinesChunk::new(self.delimiter));

        while (chunk.fill(reader)?).is_some() {
            self.lines += chunk.lines as u64;
            self.chunks.push_back(chunk);

            let first = &self.chunks[0];
            if self.lines - first.lines as u64 > self.num_print {
                chunk = self.chunks.pop_front().unwrap();

                self.lines -= chunk.lines as u64;
            } else {
                chunk = Box::new(LinesChunk::new(self.delimiter));
            }
        }

        if !&self.chunks.is_empty() {
            let length = &self.chunks.len();
            let last = &mut self.chunks[length - 1];
            if !last.buffer[..last.bytes].ends_with(&[self.delimiter]) {
                last.lines += 1;
                self.lines += 1;
            }
        } else {
            // chunks is empty when a file is empty so quitting early here
            return Ok(self.chunks.iter());
        }

        // skip unnecessary chunks and save the first chunk which may hold some lines we have to
        // print
        let chunk = loop {
            // it's safe to call unwrap here because there is at least one chunk and sorting out
            // more chunks than exist shouldn't be possible.
            let chunk = self.chunks.pop_front().unwrap();

            // skip is true as long there are enough lines left in the other stored chunks.
            let skip = self.lines - chunk.lines as u64 > self.num_print;
            if skip {
                self.lines -= chunk.lines as u64;
            } else {
                break chunk;
            }
        };

        // calculate the number of lines to skip in the chunk
        let skip_lines = if self.num_print >= self.lines {
            0
        } else {
            (self.lines - self.num_print) as usize
        };

        let chunk = LinesChunk::from_chunk(&chunk, skip_lines);
        self.chunks.push_front(Box::new(chunk));

        Ok(self.chunks.iter())
    }
}

#[cfg(test)]
mod tests {
    use crate::chunks::{BytesChunk, ChunkBuffer, BUFFER_SIZE};

    #[test]
    fn test_bytes_chunk_from_when_offset_is_zero() {
        let mut chunk = BytesChunk::new();
        chunk.bytes = BUFFER_SIZE;
        chunk.buffer[1] = 1;
        let other = BytesChunk::from_chunk(&chunk, 0);
        assert_eq!(other, chunk);

        chunk.bytes = 2;
        let other = BytesChunk::from_chunk(&chunk, 0);
        assert_eq!(other, chunk);

        chunk.bytes = 1;
        let other = BytesChunk::from_chunk(&chunk, 0);
        assert_eq!(other.buffer, [0; BUFFER_SIZE]);
        assert_eq!(other.bytes, chunk.bytes);

        chunk.bytes = BUFFER_SIZE;
        let other = BytesChunk::from_chunk(&chunk, 2);
        assert_eq!(other.buffer, [0; BUFFER_SIZE]);
        assert_eq!(other.bytes, BUFFER_SIZE - 2);
    }

    #[test]
    fn test_bytes_chunk_from_when_offset_is_not_zero() {
        let mut chunk = BytesChunk::new();
        chunk.bytes = BUFFER_SIZE;
        chunk.buffer[1] = 1;

        let other = BytesChunk::from_chunk(&chunk, 1);
        let mut expected_buffer = [0; BUFFER_SIZE];
        expected_buffer[0] = 1;
        assert_eq!(other.buffer, expected_buffer);
        assert_eq!(other.bytes, BUFFER_SIZE - 1);

        let other = BytesChunk::from_chunk(&chunk, 2);
        assert_eq!(other.buffer, [0; BUFFER_SIZE]);
        assert_eq!(other.bytes, BUFFER_SIZE - 2);
    }

    #[test]
    fn test_bytes_chunk_from_when_offset_is_larger_than_chunk_size_1() {
        let mut chunk = BytesChunk::new();
        chunk.bytes = BUFFER_SIZE;
        let new_chunk = BytesChunk::from_chunk(&chunk, BUFFER_SIZE + 1);
        assert_eq!(0, new_chunk.bytes);
    }

    #[test]
    fn test_bytes_chunk_from_when_offset_is_larger_than_chunk_size_2() {
        let mut chunk = BytesChunk::new();
        chunk.bytes = 0;
        let new_chunk = BytesChunk::from_chunk(&chunk, 1);
        assert_eq!(0, new_chunk.bytes);
    }

    #[test]
    fn test_bytes_chunk_from_when_offset_is_larger_than_chunk_size_3() {
        let mut chunk = BytesChunk::new();
        chunk.bytes = 1;
        let new_chunk = BytesChunk::from_chunk(&chunk, 2);
        assert_eq!(0, new_chunk.bytes);
    }

    #[test]
    fn test_bytes_chunk_from_when_offset_is_equal_to_chunk_size() {
        let mut chunk = BytesChunk::new();
        chunk.buffer[0] = 1;
        chunk.bytes = 1;
        let new_chunk = BytesChunk::from_chunk(&chunk, 1);
        assert_eq!(0, new_chunk.bytes);
    }

    #[test]
    fn example() {
        let mut chunk = BytesChunk::new();
        chunk.buffer[1] = 1;
        chunk.bytes = 2;
        let new_chunk = BytesChunk::from_chunk(&chunk, 0);
        assert_eq!(2, new_chunk.get_buffer().len());
        assert_eq!(&[0, 1], new_chunk.get_buffer());

        let new_chunk = BytesChunk::from_chunk(&chunk, 1);
        assert_eq!(1, new_chunk.get_buffer().len());
        assert_eq!(&[1], new_chunk.get_buffer());
    }
}
