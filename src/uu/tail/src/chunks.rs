// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Iterating over a file by chunks, either starting at the end of the file with [`ReverseChunks`]
//! or else with [`LinesChunk`] or [`BytesChunk`].
//!
//! Use [`ReverseChunks::new`] to create a new iterator over chunks of bytes from the file.

// spell-checker:ignore (ToDO) filehandle BUFSIZ

use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};

/// When reading files in reverse in `bounded_tail`, this is the size of each
/// block read at a time.
pub const BLOCK_SIZE: u64 = 1 << 16;

/// The size of the backing buffer of a LinesChunk or BytesChunk in bytes. The value of BUFFER_SIZE
/// originates from the BUFSIZ constant in stdio.h and the libc crate to make stream IO efficient.
/// In the latter the value is constantly set to 8192 on all platforms, where the value in stdio.h
/// is determined on each platform differently. Since libc chose 8192 as a reasonable default the
/// value here is set to this value, too.
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
    pub fn new(file: &'a mut File) -> io::Result<ReverseChunks<'a>> {
        // TODO: why is this platform dependent ?
        let current = if cfg!(unix) {
            file.stream_position()?
        } else {
            0
        };
        let size = file.seek(SeekFrom::End(0))? - current;
        // TODO: is the cast to usize safe ? on 32-bit systems ?
        let max_blocks_to_read = (size as f64 / BLOCK_SIZE as f64).ceil() as usize;
        let block_idx = 0;
        Ok(ReverseChunks {
            file,
            size,
            max_blocks_to_read,
            block_idx,
        })
    }
}

impl<'a> Iterator for ReverseChunks<'a> {
    type Item = io::Result<Vec<u8>>;

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
        let pos = match self.file.seek(SeekFrom::Current(-(block_size as i64))) {
            Ok(pos) => pos,
            Err(error) => return Some(Err(error)),
        };

        if let Err(error) = self.file.read_exact(&mut buf[0..(block_size as usize)]) {
            return Some(Err(error));
        }

        let pos2 = match self.file.seek(SeekFrom::Current(-(block_size as i64))) {
            Ok(pos) => pos,
            Err(error) => return Some(Err(error)),
        };
        assert_eq!(pos, pos2);

        self.block_idx += 1;

        Some(Ok(buf[0..(block_size as usize)].to_vec()))
    }
}

/// The type of the backing buffer of [`BytesChunk`] and [`LinesChunk`] which can hold
/// [`BUFFER_SIZE`] elements at max.
type ChunkBuffer = [u8; BUFFER_SIZE];

/// A [`BytesChunk`] storing a fixed size number of bytes in a buffer.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BytesChunk {
    /// The [`ChunkBuffer`], an array storing the bytes, for example filled by
    /// [`BytesChunk::fill`]
    buffer: ChunkBuffer,

    /// Stores the number of bytes, this buffer holds. This is not equal to buffer.len(), since the
    /// [`BytesChunk`] may store less bytes than the internal buffer can hold. In addition
    /// [`BytesChunk`] may be reused, what makes it necessary to track the number of stored bytes.
    /// The choice of usize is sufficient here, since the number of bytes max value is
    /// [`BUFFER_SIZE`], which is a usize.
    bytes: usize,
}

impl BytesChunk {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
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
            return Self::new();
        }

        let mut buffer: ChunkBuffer = [0; BUFFER_SIZE];
        let slice = chunk.get_buffer_with(offset);
        buffer[..slice.len()].copy_from_slice(slice);
        Self {
            buffer,
            bytes: chunk.bytes - offset,
        }
    }

    /// Receive the internal buffer safely, so it returns a slice only containing as many bytes as
    /// large the `self.bytes` value is.
    ///
    /// returns: a slice containing the bytes of the internal buffer from `[0..self.bytes]`
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut chunk = BytesChunk::new();
    /// chunk.bytes = 1;
    /// assert_eq!(&[0], chunk.get_buffer());
    /// ```
    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer[..self.bytes]
    }

    /// Like [`BytesChunk::get_buffer`], but returning a slice from `[offset.self.bytes]`.
    ///
    /// returns: a slice containing the bytes of the internal buffer from `[offset..self.bytes]`
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut chunk = BytesChunk::new();
    /// chunk.bytes = 2;
    /// assert_eq!(&[0], chunk.get_buffer_with(1));
    /// ```
    pub fn get_buffer_with(&self, offset: usize) -> &[u8] {
        &self.buffer[offset..self.bytes]
    }

    /// Return true if the [`BytesChunk`] has bytes stored.
    pub fn has_data(&self) -> bool {
        self.bytes > 0
    }

    /// Return true if the [`BytesChunk`] has no bytes stored.
    pub fn is_empty(&self) -> bool {
        !self.has_data()
    }

    /// Return the amount of bytes stored in this [`BytesChunk`].
    pub fn len(&self) -> usize {
        self.bytes
    }

    /// Fills `self.buffer` with maximal [`BUFFER_SIZE`] number of bytes,
    /// draining the reader by that number of bytes. If EOF is reached (so 0
    /// bytes are read), then returns [`io::Result<None>`] or else the result
    /// with [`io::Result<Some(bytes))`] where bytes is the number of bytes read
    /// from the source.
    pub fn fill(&mut self, filehandle: &mut impl Read) -> io::Result<Option<usize>> {
        let num_bytes = filehandle.read(&mut self.buffer)?;
        self.bytes = num_bytes;
        if num_bytes == 0 {
            return Ok(None);
        }

        Ok(Some(self.bytes))
    }
}

/// An abstraction layer on top of [`BytesChunk`] mainly to simplify filling only the needed amount
/// of chunks. See also [`Self::fill`].
pub struct BytesChunkBuffer {
    /// The number of bytes to print
    num_print: u64,
    /// The current number of bytes summed over all stored chunks in [`Self::chunks`]. Use u64 here
    /// to support files > 4GB on 32-bit systems. Note, this differs from `BytesChunk::bytes` which
    /// is a usize. The choice of u64 is based on `tail::FilterMode::Bytes`.
    bytes: u64,
    /// The buffer to store [`BytesChunk`] in
    chunks: VecDeque<Box<BytesChunk>>,
}

impl BytesChunkBuffer {
    /// Creates a new [`BytesChunkBuffer`].
    ///
    /// # Arguments
    ///
    /// * `num_print`: The number of bytes to print
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
    pub fn new(num_print: u64) -> Self {
        Self {
            bytes: 0,
            num_print,
            chunks: VecDeque::new(),
        }
    }

    /// Fills this buffer with chunks and consumes the reader completely. This method ensures that
    /// there are exactly as many chunks as needed to match `self.num_print` bytes, so there are
    /// in sum exactly `self.num_print` bytes stored in all chunks. The method returns an iterator
    /// over these chunks. If there are no chunks, for example because the piped stdin contained no
    /// bytes, or `num_print = 0` then `iterator.next` returns None.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::chunks::BytesChunkBuffer;
    /// use std::io::{BufReader, Cursor};
    ///
    /// let mut reader = BufReader::new(Cursor::new(""));
    /// let num_print = 0;
    /// let mut chunks = BytesChunkBuffer::new(num_print);
    /// chunks.fill(&mut reader).unwrap();
    ///
    /// let mut reader = BufReader::new(Cursor::new("a"));
    /// let num_print = 1;
    /// let mut chunks = BytesChunkBuffer::new(num_print);
    /// chunks.fill(&mut reader).unwrap();
    /// ```
    pub fn fill(&mut self, reader: &mut impl Read) -> io::Result<u64> {
        if self.num_print == 0 {
            return Ok(0);
        }

        let mut chunk = Box::new(BytesChunk::new());

        // fill chunks with all bytes from reader and reuse already instantiated chunks if possible
        while (chunk.fill(reader)?).is_some() {
            self.push_back(chunk);

            let first = &self.chunks[0];
            if self.bytes - first.bytes as u64 > self.num_print {
                chunk = self.pop_front().unwrap();
            } else {
                chunk = Box::new(BytesChunk::new());
            }
        }

        // quit early if there are no chunks for example in case the pipe was empty
        if self.chunks.is_empty() {
            return Ok(0);
        }

        // calculate the offset in the first chunk and put the calculated chunk as first element in
        // the self.chunks collection. The calculated offset must be in the range 0 to BUFFER_SIZE
        // and is therefore safely convertible to a usize without losses.
        let offset = self.bytes.saturating_sub(self.num_print) as usize;

        let chunk = self.pop_front().unwrap();
        let chunk = Box::new(BytesChunk::from_chunk(&chunk, offset));
        self.push_front(chunk);

        Ok(self.bytes)
    }

    /// Print the whole [`BytesChunkBuffer`].
    pub fn print(&self, writer: &mut impl Write) -> io::Result<()> {
        for chunk in &self.chunks {
            writer.write_all(chunk.get_buffer())?;
        }
        Ok(())
    }

    /// Return true if the [`BytesChunkBuffer`] has bytes stored.
    pub fn has_data(&self) -> bool {
        !self.chunks.is_empty()
    }

    /// Return the amount of bytes this [`BytesChunkBuffer`] has stored.
    pub fn get_bytes(&self) -> u64 {
        self.bytes
    }

    /// Return and remove the first [`BytesChunk`] stored in this [`BytesChunkBuffer`].
    fn pop_front(&mut self) -> Option<Box<BytesChunk>> {
        let chunk = self.chunks.pop_front();
        if let Some(chunk) = chunk {
            self.bytes -= chunk.bytes as u64;
            Some(chunk)
        } else {
            None
        }
    }

    /// Add a [`BytesChunk`] at the start of this [`BytesChunkBuffer`].
    fn push_front(&mut self, chunk: Box<BytesChunk>) {
        self.bytes += chunk.bytes as u64;
        self.chunks.push_front(chunk);
    }

    /// Add a [`BytesChunk`] at the end of this [`BytesChunkBuffer`].
    fn push_back(&mut self, chunk: Box<BytesChunk>) {
        self.bytes += chunk.bytes as u64;
        self.chunks.push_back(chunk);
    }
}

/// Works similar to a [`BytesChunk`] but also stores the number of lines encountered in the current
/// buffer. The size of the buffer is limited to a fixed size number of bytes.
#[derive(Debug)]
pub struct LinesChunk {
    /// Work on top of a [`BytesChunk`]
    chunk: BytesChunk,
    /// The number of lines delimited by `delimiter`. The choice of usize is sufficient here,
    /// because lines max value is the number of bytes contained in this chunk's buffer, and the
    /// number of bytes max value is [`BUFFER_SIZE`], which is a usize.
    lines: usize,
    /// The delimiter to use, to count the lines
    delimiter: u8,
}

impl LinesChunk {
    /// Create a new [`LinesChunk`] with an arbitrary `u8` as `delimiter`.
    pub fn new(delimiter: u8) -> Self {
        Self {
            chunk: BytesChunk::new(),
            lines: 0,
            delimiter,
        }
    }

    /// Count the number of lines delimited with [`Self::delimiter`] contained in the buffer.
    /// Currently [`memchr`] is used because performance is better than using an iterator or for
    /// loop.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut chunk = LinesChunk::new(b'\n');
    /// chunk.buffer[0..12].copy_from_slice("hello\nworld\n".as_bytes());
    /// chunk.bytes = 12;
    /// assert_eq!(2, chunk.count_lines());
    ///
    /// chunk.buffer[0..14].copy_from_slice("hello\r\nworld\r\n".as_bytes());
    /// chunk.bytes = 14;
    /// assert_eq!(2, chunk.count_lines());
    /// ```
    fn count_lines(&self) -> usize {
        memchr::memchr_iter(self.delimiter, self.get_buffer()).count()
    }

    /// Creates a new [`LinesChunk`] from an existing one with an offset in lines. The new chunk
    /// contains exactly `chunk.lines - offset` lines. The offset in bytes is calculated and applied
    /// to the new chunk, so the new chunk contains only the bytes encountered after the offset in
    /// number of lines and the `delimiter`. If the offset is larger than `chunk.lines` then a new
    /// empty `LinesChunk` is returned.
    ///
    /// # Arguments
    ///
    /// * `chunk`: The chunk to create the new chunk from
    /// * `offset`: The offset in number of lines (not bytes)
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
    /// ```
    fn from_chunk(chunk: &Self, offset: usize) -> Self {
        if offset > chunk.lines {
            return Self::new(chunk.delimiter);
        }

        let bytes_offset = chunk.calculate_bytes_offset_from(offset);
        let new_chunk = BytesChunk::from_chunk(&chunk.chunk, bytes_offset);

        Self {
            chunk: new_chunk,
            lines: chunk.lines - offset,
            delimiter: chunk.delimiter,
        }
    }

    /// Returns true if this buffer has stored any bytes.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut chunk = LinesChunk::new(b'\n');
    /// assert!(!chunk.has_data());
    ///
    /// chunk.buffer[0] = 1;
    /// assert!(!chunk.has_data());
    ///
    /// chunk.bytes = 1;
    /// assert!(chunk.has_data());
    /// ```
    pub fn has_data(&self) -> bool {
        self.chunk.has_data()
    }

    /// Return true if the [`LinesChunk`] has no bytes stored.
    pub fn is_empty(&self) -> bool {
        !self.has_data()
    }

    /// Return the amount of bytes stored in this [`LinesChunk`].
    pub fn len(&self) -> usize {
        self.chunk.len()
    }

    /// Returns this buffer safely. See [`BytesChunk::get_buffer`]
    ///
    /// returns: &[u8] with length `self.bytes`
    pub fn get_buffer(&self) -> &[u8] {
        self.chunk.get_buffer()
    }

    /// Returns this buffer safely with an offset applied. See [`BytesChunk::get_buffer_with`].
    ///
    /// returns: &[u8] with length `self.bytes - offset`
    pub fn get_buffer_with(&self, offset: usize) -> &[u8] {
        self.chunk.get_buffer_with(offset)
    }

    /// Return the number of lines the buffer contains. `self.lines` needs to be set before the call
    /// to this function returns the correct value. If the calculation of lines is needed then
    /// use `self.count_lines`.
    pub fn get_lines(&self) -> usize {
        self.lines
    }

    /// Fills `self.buffer` with maximal [`BUFFER_SIZE`] number of bytes, draining the reader by
    /// that number of bytes. This function works like the [`BytesChunk::fill`] function besides
    /// that this function also counts and stores the number of lines encountered while reading from
    /// the `filehandle`.
    pub fn fill(&mut self, filehandle: &mut impl Read) -> io::Result<Option<usize>> {
        match self.chunk.fill(filehandle)? {
            None => {
                self.lines = 0;
                Ok(None)
            }
            Some(bytes) => {
                self.lines = self.count_lines();
                Ok(Some(bytes))
            }
        }
    }

    /// Calculates the offset in bytes within this buffer from the offset in number of lines. The
    /// resulting offset is 0-based and points to the byte after the delimiter.
    ///
    /// # Arguments
    ///
    /// * `offset`: the offset in number of lines. If offset is 0 then 0 is returned, if larger than
    ///             the contained lines then self.bytes is returned.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut chunk = LinesChunk::new(b'\n');
    /// chunk.buffer[0..12].copy_from_slice("hello\nworld\n".as_bytes());
    /// chunk.bytes = 12;
    /// chunk.lines = 2; // note that if not setting lines the result might not be what is expected
    /// let bytes_offset = chunk.calculate_bytes_offset_from(1);
    /// assert_eq!(6, bytes_offset);
    /// assert_eq!(
    ///     "world\n",
    ///     String::from_utf8_lossy(chunk.get_buffer_with(bytes_offset)));
    /// ```
    fn calculate_bytes_offset_from(&self, offset: usize) -> usize {
        let mut lines_offset = offset;
        let mut bytes_offset = 0;
        for byte in self.get_buffer() {
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

    /// Print the bytes contained in this buffer calculated with the given offset in number of
    /// lines.
    ///
    /// # Arguments
    ///
    /// * `writer`: must implement [`Write`]
    /// * `offset`: An offset in number of lines.
    pub fn print_lines(&self, writer: &mut impl Write, offset: usize) -> io::Result<usize> {
        self.print_bytes(writer, self.calculate_bytes_offset_from(offset))
    }

    /// Print the bytes contained in this buffer beginning from the given offset in number of bytes.
    ///
    /// # Arguments
    ///
    /// * `writer`: must implement [`Write`]
    /// * `offset`: An offset in number of bytes.
    pub fn print_bytes(&self, writer: &mut impl Write, offset: usize) -> io::Result<usize> {
        let buffer = self.get_buffer_with(offset);
        writer.write_all(buffer)?;
        Ok(buffer.len())
    }
}

/// An abstraction layer on top of [`LinesChunk`] mainly to simplify filling only the needed amount
/// of chunks. See also [`Self::fill`]. Works similar like [`BytesChunkBuffer`], but works on top
/// of lines delimited by `self.delimiter` instead of bytes.
pub struct LinesChunkBuffer {
    /// The delimiter to recognize a line. Any [`u8`] is allowed.
    delimiter: u8,
    /// The amount of lines occurring in all currently stored [`LinesChunk`]s. Use u64 here to
    /// support files > 4GB on 32-bit systems. Note, this differs from [`LinesChunk::lines`] which
    /// is a usize. The choice of u64 is based on `tail::FilterMode::Lines`.
    lines: u64,
    /// The amount of lines to print.
    num_print: u64,
    /// Stores the [`LinesChunk`]
    chunks: VecDeque<Box<LinesChunk>>,
    /// The total amount of bytes stored in all chunks
    bytes: u64,
}

impl LinesChunkBuffer {
    /// Create a new [`LinesChunkBuffer`]
    pub fn new(delimiter: u8, num_print: u64) -> Self {
        Self {
            delimiter,
            num_print,
            lines: 0,
            chunks: VecDeque::new(),
            bytes: 0,
        }
    }

    /// Fills this buffer with chunks and consumes the reader completely.
    ///
    /// This method ensures that there are exactly as many chunks as needed to match
    /// `self.num_print` lines, so there are in sum exactly `self.num_print` lines stored in all
    /// chunks.
    pub fn fill(&mut self, reader: &mut impl Read) -> io::Result<u64> {
        if self.num_print == 0 {
            return Ok(0);
        }

        let mut chunk = Box::new(LinesChunk::new(self.delimiter));
        while (chunk.fill(reader)?).is_some() {
            self.push_back(chunk);

            let first = &self.chunks[0];
            if self.lines - first.lines as u64 > self.num_print {
                chunk = self.pop_front().unwrap();
            } else {
                chunk = Box::new(LinesChunk::new(self.delimiter));
            }
        }

        if self.has_data() {
            let length = &self.chunks.len();
            let last = &mut self.chunks[length - 1];
            if !last.get_buffer().ends_with(&[self.delimiter]) {
                last.lines += 1;
                self.lines += 1;
            }
        }

        // skip unnecessary chunks and save the first chunk which may hold some lines we have to
        // print
        while let Some(chunk) = self.pop_front() {
            // this is false as long there are enough lines left in the other stored chunks.
            if self.lines <= self.num_print {
                // Calculate the number of lines to skip in the current chunk. The calculated value must be
                // in the range 0 to BUFFER_SIZE and is therefore safely convertible to a usize without
                // losses.
                let skip_lines =
                    (self.lines + chunk.lines as u64).saturating_sub(self.num_print) as usize;

                let chunk = Box::new(LinesChunk::from_chunk(&chunk, skip_lines));
                self.push_front(chunk);
                break;
            }
        }

        Ok(self.bytes)
    }

    /// Writes the whole [`LinesChunkBuffer`] into a `writer`.
    pub fn print(&self, writer: &mut impl Write) -> io::Result<()> {
        for chunk in &self.chunks {
            chunk.print_bytes(writer, 0)?;
        }
        Ok(())
    }

    /// Return the amount of lines this buffer has stored.
    pub fn get_lines(&self) -> u64 {
        self.lines
    }

    /// Return true if this buffer has stored any bytes.
    pub fn has_data(&self) -> bool {
        !self.chunks.is_empty()
    }

    /// Return and remove the first [`LinesChunk`] stored in this [`LinesChunkBuffer`].
    fn pop_front(&mut self) -> Option<Box<LinesChunk>> {
        let chunk = self.chunks.pop_front();
        if let Some(chunk) = chunk {
            self.bytes -= chunk.len() as u64;
            self.lines -= chunk.lines as u64;
            Some(chunk)
        } else {
            None
        }
    }

    /// Add a [`LinesChunk`] at the end of this [`LinesChunkBuffer`].
    fn push_back(&mut self, chunk: Box<LinesChunk>) {
        self.bytes += chunk.len() as u64;
        self.lines += chunk.lines as u64;
        self.chunks.push_back(chunk);
    }

    /// Add a [`LinesChunk`] at the start of this [`LinesChunkBuffer`].
    fn push_front(&mut self, chunk: Box<LinesChunk>) {
        self.bytes += chunk.len() as u64;
        self.lines += chunk.lines as u64;
        self.chunks.push_front(chunk);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufWriter, Cursor};

    fn fill_lines_chunk(chunk: &mut LinesChunk, data: &str) -> usize {
        let mut cursor = Cursor::new(data);
        let result = chunk.fill(&mut cursor);
        assert!(result.is_ok());
        let option = result.unwrap();
        option.unwrap_or(0)
    }

    fn fill_lines_chunk_buffer(buffer: &mut LinesChunkBuffer, data: &str) -> u64 {
        let mut cursor = Cursor::new(data);
        let result = buffer.fill(&mut cursor);
        assert!(result.is_ok());
        result.unwrap()
    }

    fn fill_bytes_chunk_buffer(buffer: &mut BytesChunkBuffer, data: &str) -> u64 {
        let mut cursor = Cursor::new(data);
        let result = buffer.fill(&mut cursor);
        assert!(result.is_ok());
        result.unwrap()
    }

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
    fn test_lines_chunk_fill_when_unix_line_endings() {
        let mut chunk = LinesChunk::new(b'\n');

        let bytes = fill_lines_chunk(&mut chunk, "");
        assert_eq!(bytes, 0);
        assert_eq!(chunk.get_lines(), 0);

        let bytes = fill_lines_chunk(&mut chunk, "\n");
        assert_eq!(bytes, 1);
        assert_eq!(chunk.get_lines(), 1);

        let bytes = fill_lines_chunk(&mut chunk, "a");
        assert_eq!(bytes, 1);
        assert_eq!(chunk.get_lines(), 0);

        let bytes = fill_lines_chunk(&mut chunk, "aa");
        assert_eq!(bytes, 2);
        assert_eq!(chunk.get_lines(), 0);

        let bytes = fill_lines_chunk(&mut chunk, "a".repeat(BUFFER_SIZE).as_str());
        assert_eq!(bytes, BUFFER_SIZE);
        assert_eq!(chunk.get_lines(), 0);

        let bytes = fill_lines_chunk(&mut chunk, "a".repeat(BUFFER_SIZE + 1).as_str());
        assert_eq!(bytes, BUFFER_SIZE);
        assert_eq!(chunk.get_lines(), 0);

        let bytes = fill_lines_chunk(&mut chunk, "a\n".repeat(BUFFER_SIZE / 2).as_str());
        assert_eq!(bytes, BUFFER_SIZE);
        assert_eq!(chunk.get_lines(), BUFFER_SIZE / 2);

        let bytes = fill_lines_chunk(&mut chunk, "a\n".repeat(BUFFER_SIZE).as_str());
        assert_eq!(bytes, BUFFER_SIZE);
        assert_eq!(chunk.get_lines(), BUFFER_SIZE / 2);

        let bytes = fill_lines_chunk(&mut chunk, "\n".repeat(BUFFER_SIZE).as_str());
        assert_eq!(bytes, BUFFER_SIZE);
        assert_eq!(chunk.get_lines(), BUFFER_SIZE);

        let bytes = fill_lines_chunk(&mut chunk, "\n".repeat(BUFFER_SIZE + 1).as_str());
        assert_eq!(bytes, BUFFER_SIZE);
        assert_eq!(chunk.get_lines(), BUFFER_SIZE);
    }

    #[test]
    fn test_lines_chunk_fill_when_windows_line_endings() {
        let mut chunk = LinesChunk::new(b'\n');

        let bytes = fill_lines_chunk(&mut chunk, "\r\n");
        assert_eq!(bytes, 2);
        assert_eq!(chunk.get_lines(), 1);

        let bytes = fill_lines_chunk(&mut chunk, "a\r\n");
        assert_eq!(bytes, 3);
        assert_eq!(chunk.get_lines(), 1);

        let bytes = fill_lines_chunk(&mut chunk, "a\r\na");
        assert_eq!(bytes, 4);
        assert_eq!(chunk.get_lines(), 1);

        let bytes = fill_lines_chunk(&mut chunk, "a\r\na\r\n");
        assert_eq!(bytes, 6);
        assert_eq!(chunk.get_lines(), 2);

        let bytes = fill_lines_chunk(&mut chunk, "\r\n".repeat(BUFFER_SIZE / 2).as_str());
        assert_eq!(bytes, BUFFER_SIZE);
        assert_eq!(chunk.get_lines(), BUFFER_SIZE / 2);

        // tests the correct amount of lines when \r\n is split across different chunks
        let mut data = "\r\n".repeat(BUFFER_SIZE / 2 - 1);
        data.push('a');
        data.push('\r');
        data.push('\n');
        let bytes = fill_lines_chunk(&mut chunk, data.as_str());
        assert_eq!(bytes, BUFFER_SIZE);
        assert_eq!(chunk.get_lines(), BUFFER_SIZE / 2 - 1);
    }

    #[test]
    fn test_lines_chunk_when_print_lines_no_offset_then_correct_amount_of_bytes() {
        let mut chunk = LinesChunk::new(b'\n');
        let expected = fill_lines_chunk(&mut chunk, "");

        let mut writer = BufWriter::new(vec![]);
        let result = chunk.print_lines(&mut writer, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        let expected = fill_lines_chunk(&mut chunk, "a");
        let result = chunk.print_lines(&mut writer, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        let expected = fill_lines_chunk(&mut chunk, "\n");
        let result = chunk.print_lines(&mut writer, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        let expected = fill_lines_chunk(&mut chunk, "a\n");
        let result = chunk.print_lines(&mut writer, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        let expected = fill_lines_chunk(&mut chunk, "\n".repeat(BUFFER_SIZE).as_str());
        let result = chunk.print_lines(&mut writer, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        fill_lines_chunk(&mut chunk, "a\n".repeat(BUFFER_SIZE / 2 + 1).as_str());
        let result = chunk.print_lines(&mut writer, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BUFFER_SIZE);
    }

    #[test]
    fn test_lines_chunk_when_print_lines_with_offset_then_correct_amount_of_bytes() {
        let mut chunk = LinesChunk::new(b'\n');
        fill_lines_chunk(&mut chunk, "");

        let mut writer = BufWriter::new(vec![]);
        let result = chunk.print_lines(&mut writer, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        fill_lines_chunk(&mut chunk, "a");
        let result = chunk.print_lines(&mut writer, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        fill_lines_chunk(&mut chunk, "a");
        let result = chunk.print_lines(&mut writer, 2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        fill_lines_chunk(&mut chunk, "a");
        let result = chunk.print_lines(&mut writer, 100);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        fill_lines_chunk(&mut chunk, "a\n");
        let result = chunk.print_lines(&mut writer, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        fill_lines_chunk(&mut chunk, "a\n\n");
        let result = chunk.print_lines(&mut writer, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        fill_lines_chunk(&mut chunk, "a\na\n");
        let result = chunk.print_lines(&mut writer, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);

        fill_lines_chunk(&mut chunk, "a\na\n");
        let result = chunk.print_lines(&mut writer, 2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        fill_lines_chunk(&mut chunk, "a\naa\n");
        let result = chunk.print_lines(&mut writer, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);

        fill_lines_chunk(&mut chunk, "a".repeat(BUFFER_SIZE).as_str());
        let result = chunk.print_lines(&mut writer, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BUFFER_SIZE);

        fill_lines_chunk(&mut chunk, "a".repeat(BUFFER_SIZE).as_str());
        let result = chunk.print_lines(&mut writer, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        fill_lines_chunk(&mut chunk, "\n".repeat(BUFFER_SIZE).as_str());
        let result = chunk.print_lines(&mut writer, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BUFFER_SIZE - 1);

        fill_lines_chunk(&mut chunk, "\n".repeat(BUFFER_SIZE).as_str());
        let result = chunk.print_lines(&mut writer, BUFFER_SIZE - 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        fill_lines_chunk(&mut chunk, "\n".repeat(BUFFER_SIZE).as_str());
        let result = chunk.print_lines(&mut writer, BUFFER_SIZE);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_lines_chunk_buffer_fill_when_num_print_is_equal_to_size() {
        let size = 0;
        let mut buffer = LinesChunkBuffer::new(b'\n', size as u64);
        let bytes = fill_lines_chunk_buffer(&mut buffer, "");
        assert_eq!(buffer.get_lines(), 0);
        assert_eq!(bytes, size as u64);
        assert!(!buffer.has_data());

        let size = 1;
        let mut buffer = LinesChunkBuffer::new(b'\n', size as u64);
        let bytes = fill_lines_chunk_buffer(&mut buffer, "a");
        assert_eq!(buffer.get_lines(), 1);
        assert_eq!(bytes, size as u64);

        let size = 1;
        let mut buffer = LinesChunkBuffer::new(b'\n', size as u64);
        let bytes = fill_lines_chunk_buffer(&mut buffer, "\n");
        assert_eq!(buffer.get_lines(), 1);
        assert_eq!(bytes, size as u64);

        let size = BUFFER_SIZE + 1;
        let mut buffer = LinesChunkBuffer::new(b'\n', size as u64);
        let bytes = fill_lines_chunk_buffer(&mut buffer, "\n".repeat(size).as_str());
        assert_eq!(buffer.get_lines(), size as u64);
        assert_eq!(bytes, size as u64);

        let size = BUFFER_SIZE + 1;
        let mut data = "a".repeat(BUFFER_SIZE);
        data.push('\n');
        let mut buffer = LinesChunkBuffer::new(b'\n', size as u64);
        let bytes = fill_lines_chunk_buffer(&mut buffer, data.as_str());
        assert_eq!(buffer.get_lines(), 1);
        assert_eq!(bytes, size as u64);

        let size = BUFFER_SIZE + 1;
        let mut data = "a".repeat(BUFFER_SIZE - 1);
        data.push('\n');
        data.push('\n');
        let mut buffer = LinesChunkBuffer::new(b'\n', size as u64);
        let bytes = fill_lines_chunk_buffer(&mut buffer, data.as_str());
        assert_eq!(buffer.get_lines(), 2);
        assert_eq!(bytes, size as u64);

        let size = BUFFER_SIZE * 2;
        let mut buffer = LinesChunkBuffer::new(b'\n', size as u64);
        let bytes = fill_lines_chunk_buffer(&mut buffer, "a".repeat(size).as_str());
        assert_eq!(buffer.get_lines(), 1);
        assert_eq!(bytes, size as u64);
    }

    #[test]
    fn test_lines_chunk_buffer_fill_when_num_print_is_not_equal_to_size() {
        let size = 0;
        let mut buffer = LinesChunkBuffer::new(b'\n', 1);
        let bytes = fill_lines_chunk_buffer(&mut buffer, "");
        assert_eq!(buffer.get_lines(), 0);
        assert_eq!(bytes, size as u64);
        assert!(!buffer.has_data());

        let size = 1;
        let mut buffer = LinesChunkBuffer::new(b'\n', 2);
        let bytes = fill_lines_chunk_buffer(&mut buffer, "a");
        assert_eq!(buffer.get_lines(), 1);
        assert_eq!(bytes, size as u64);

        let mut buffer = LinesChunkBuffer::new(b'\n', 0);
        let bytes = fill_lines_chunk_buffer(&mut buffer, "a");
        assert_eq!(buffer.get_lines(), 0);
        assert_eq!(bytes, 0);
        assert!(!buffer.has_data());
    }

    #[test]
    fn test_bytes_chunk_buffer_fill_when_num_print_is_equal_to_size() {
        let size = 0;
        let mut buffer = BytesChunkBuffer::new(size as u64);
        let bytes = fill_bytes_chunk_buffer(&mut buffer, "");
        assert_eq!(buffer.get_bytes(), 0);
        assert_eq!(bytes, size as u64);
        assert!(!buffer.has_data());

        let size = 1;
        let mut buffer = BytesChunkBuffer::new(size as u64);
        let bytes = fill_bytes_chunk_buffer(&mut buffer, "a");
        assert_eq!(buffer.get_bytes(), 1);
        assert_eq!(bytes, size as u64);
        assert!(buffer.has_data());

        let size = BUFFER_SIZE;
        let mut buffer = BytesChunkBuffer::new(size as u64);
        let bytes = fill_bytes_chunk_buffer(&mut buffer, "a".repeat(size).as_str());
        assert_eq!(buffer.get_bytes(), size as u64);
        assert_eq!(bytes, size as u64);
        assert!(buffer.has_data());

        let size = BUFFER_SIZE + 1;
        let mut buffer = BytesChunkBuffer::new(size as u64);
        let bytes = fill_bytes_chunk_buffer(&mut buffer, "a".repeat(size).as_str());
        assert_eq!(buffer.get_bytes(), size as u64);
        assert_eq!(bytes, size as u64);
        assert!(buffer.has_data());

        let size = BUFFER_SIZE * 2;
        let mut buffer = BytesChunkBuffer::new(size as u64);
        let bytes = fill_bytes_chunk_buffer(&mut buffer, "a".repeat(size).as_str());
        assert_eq!(buffer.get_bytes(), size as u64);
        assert_eq!(bytes, size as u64);
        assert!(buffer.has_data());
    }

    #[test]
    fn test_bytes_chunk_buffer_fill_when_num_print_is_not_equal_to_size() {
        let mut buffer = BytesChunkBuffer::new(0);
        let bytes = fill_bytes_chunk_buffer(&mut buffer, "a");
        assert_eq!(buffer.get_bytes(), 0);
        assert_eq!(bytes, 0);
        assert!(!buffer.has_data());

        let mut buffer = BytesChunkBuffer::new(1);
        let bytes = fill_bytes_chunk_buffer(&mut buffer, "");
        assert_eq!(buffer.get_bytes(), 0);
        assert_eq!(bytes, 0);
        assert!(!buffer.has_data());

        let mut buffer = BytesChunkBuffer::new(2);
        let bytes = fill_bytes_chunk_buffer(&mut buffer, "a");
        assert_eq!(buffer.get_bytes(), 1);
        assert_eq!(bytes, 1);
        assert!(buffer.has_data());
    }
}
