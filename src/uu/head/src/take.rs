// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Take all but the last elements of an iterator.
use memchr::memchr_iter;
use std::collections::VecDeque;
use std::io::{ErrorKind, Read, Write};

const BUF_SIZE: usize = 65536;

struct TakeAllBuffer {
    buffer: Vec<u8>,
    start_index: usize,
}

impl TakeAllBuffer {
    fn new() -> Self {
        TakeAllBuffer {
            buffer: vec![],
            start_index: 0,
        }
    }

    fn fill_buffer(&mut self, reader: &mut impl Read) -> std::io::Result<usize> {
        self.buffer.resize(BUF_SIZE, 0);
        self.start_index = 0;
        loop {
            match reader.read(&mut self.buffer[..]) {
                Ok(n) => {
                    self.buffer.truncate(n);
                    return Ok(n);
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
    }

    fn write_bytes_exact(&mut self, writer: &mut impl Write, bytes: usize) -> std::io::Result<()> {
        let buffer_to_write = &self.remaining_buffer()[..bytes];
        writer.write_all(buffer_to_write)?;
        self.start_index += bytes;
        assert!(self.start_index <= self.buffer.len());
        Ok(())
    }

    fn write_all(&mut self, writer: &mut impl Write) -> std::io::Result<usize> {
        let remaining_bytes = self.remaining_bytes();
        self.write_bytes_exact(writer, remaining_bytes)?;
        Ok(remaining_bytes)
    }

    fn write_bytes_limit(
        &mut self,
        writer: &mut impl Write,
        max_bytes: usize,
    ) -> std::io::Result<usize> {
        let bytes_to_write = self.remaining_bytes().min(max_bytes);
        self.write_bytes_exact(writer, bytes_to_write)?;
        Ok(bytes_to_write)
    }

    fn remaining_buffer(&self) -> &[u8] {
        &self.buffer[self.start_index..]
    }

    fn remaining_bytes(&self) -> usize {
        self.remaining_buffer().len()
    }

    fn is_empty(&self) -> bool {
        assert!(self.start_index <= self.buffer.len());
        self.start_index == self.buffer.len()
    }
}

pub fn copy_all_but_bytes(
    reader: &mut impl Read,
    writer: &mut impl Write,
    n: usize,
) -> std::io::Result<usize> {
    let mut buffers: VecDeque<TakeAllBuffer> = VecDeque::new();
    let mut empty_buffer_pool: Vec<TakeAllBuffer> = vec![];
    let mut buffered_bytes: usize = 0;
    let mut total_bytes_copied = 0;
    loop {
        loop {
            // Try to buffer at least enough to write the entire first buffer.
            let front_buffer = buffers.front();
            if let Some(front_buffer) = front_buffer {
                if buffered_bytes >= n + front_buffer.remaining_bytes() {
                    break;
                }
            }
            let mut new_buffer = empty_buffer_pool.pop().unwrap_or_else(TakeAllBuffer::new);
            let filled_bytes = new_buffer.fill_buffer(reader)?;
            if filled_bytes == 0 {
                // filled_bytes==0 => Eof
                break;
            }
            buffers.push_back(new_buffer);
            buffered_bytes += filled_bytes;
        }

        // If we've got <=n bytes buffered here we have nothing let to do.
        if buffered_bytes <= n {
            break;
        }

        let excess_buffered_bytes = buffered_bytes - n;
        // Since we have some data buffered, can assume we have >=1 buffer - i.e. safe to unwrap.
        let front_buffer = buffers.front_mut().unwrap();
        let bytes_written = front_buffer.write_bytes_limit(writer, excess_buffered_bytes)?;
        buffered_bytes -= bytes_written;
        total_bytes_copied += bytes_written;
        // If the front buffer is empty (which it probably is), push it into the empty-buffer-pool.
        if front_buffer.is_empty() {
            empty_buffer_pool.push(buffers.pop_front().unwrap());
        }
    }
    Ok(total_bytes_copied)
}

struct TakeAllLinesBuffer {
    inner: TakeAllBuffer,
    lines: usize,
}

struct BytesAndLines {
    bytes: usize,
    lines: usize,
}

impl TakeAllLinesBuffer {
    fn new() -> Self {
        TakeAllLinesBuffer {
            inner: TakeAllBuffer::new(),
            lines: 0,
        }
    }

    fn fill_buffer(
        &mut self,
        reader: &mut impl Read,
        separator: u8,
    ) -> std::io::Result<BytesAndLines> {
        let bytes_read = self.inner.fill_buffer(reader)?;
        // Count the number of lines...
        self.lines = memchr_iter(separator, self.inner.remaining_buffer()).count();
        Ok(BytesAndLines {
            bytes: bytes_read,
            lines: self.lines,
        })
    }

    fn write_lines(
        &mut self,
        writer: &mut impl Write,
        max_lines: usize,
        separator: u8,
    ) -> std::io::Result<BytesAndLines> {
        assert!(max_lines > 0, "Must request at least 1 line.");
        let ret;
        if max_lines > self.lines {
            ret = BytesAndLines {
                bytes: self.inner.write_all(writer)?,
                lines: self.lines,
            };
            self.lines = 0;
        } else {
            let index = memchr_iter(separator, self.inner.remaining_buffer()).nth(max_lines - 1);
            assert!(
                index.is_some(),
                "Somehow we're being asked to write more lines than we have, that's a bug in copy_all_but_lines."
            );
            let index = index.unwrap();
            // index is the offset of the separator character, zero indexed. Need to add 1 to get the number
            // of bytes to write.
            let bytes_to_write = index + 1;
            self.inner.write_bytes_exact(writer, bytes_to_write)?;
            ret = BytesAndLines {
                bytes: bytes_to_write,
                lines: max_lines,
            };
            self.lines -= max_lines;
        }
        Ok(ret)
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn lines(&self) -> usize {
        self.lines
    }
}

pub fn copy_all_but_lines<R: Read, W: Write>(
    mut reader: R,
    writer: &mut W,
    n: usize,
    separator: u8,
) -> std::io::Result<usize> {
    let mut buffers: VecDeque<TakeAllLinesBuffer> = VecDeque::new();
    let mut buffered_lines: usize = 0;
    let mut empty_buffers = vec![];
    let mut total_bytes_copied = 0;
    loop {
        // Try to buffer enough such that we can write out the entire first buffer.
        loop {
            // First check if we have enough lines buffered that we can write out the entire
            // front buffer. If so, break.
            let front_buffer = buffers.front();
            if let Some(front_buffer) = front_buffer {
                if buffered_lines > n + front_buffer.lines() {
                    break;
                }
            }
            // We need to try to buffer more data...
            let mut new_buffer = empty_buffers.pop().unwrap_or_else(TakeAllLinesBuffer::new);
            let fill_result = new_buffer.fill_buffer(&mut reader, separator)?;
            if fill_result.bytes == 0 {
                // fill_result.bytes == 0 => EoF.
                break;
            }
            buffered_lines += fill_result.lines;
            buffers.push_back(new_buffer);
        }

        // If we've not buffered more lines than we need to hold back we must be done.
        if buffered_lines <= n {
            break;
        }

        // Since we have some data buffered can assume we have at least 1 buffer, so safe to unwrap.
        let front_buffer = buffers.front_mut().unwrap();
        let excess_buffered_lines = buffered_lines - n;
        let write_result = front_buffer.write_lines(writer, excess_buffered_lines, separator)?;
        buffered_lines -= write_result.lines;
        total_bytes_copied += write_result.bytes;
        // If the front buffer is empty (which it probably is), push it into the empty-buffer-pool.
        if front_buffer.is_empty() {
            empty_buffers.push(buffers.pop_front().unwrap());
        }
    }
    Ok(total_bytes_copied)
}

/// Like `std::io::Take`, but for lines instead of bytes.
///
/// This struct is generally created by calling [`take_lines`] on a
/// reader. Please see the documentation of [`take_lines`] for more
/// details.
pub struct TakeLines<T> {
    inner: T,
    limit: u64,
    separator: u8,
}

impl<T: Read> Read for TakeLines<T> {
    /// Read bytes from a buffer up to the requested number of lines.
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.limit == 0 {
            return Ok(0);
        }
        match self.inner.read(buf) {
            Ok(0) => Ok(0),
            Ok(n) => {
                for i in memchr_iter(self.separator, &buf[..n]) {
                    self.limit -= 1;
                    if self.limit == 0 {
                        return Ok(i + 1);
                    }
                }
                Ok(n)
            }
            Err(e) => Err(e),
        }
    }
}

/// Create an adaptor that will read at most `limit` lines from a given reader.
///
/// This function returns a new instance of `Read` that will read at
/// most `limit` lines, after which it will always return EOF
/// (`Ok(0)`).
///
/// The `separator` defines the character to interpret as the line
/// ending. For the usual notion of "line", set this to `b'\n'`.
pub fn take_lines<R>(reader: R, limit: u64, separator: u8) -> TakeLines<R> {
    TakeLines {
        inner: reader,
        limit,
        separator,
    }
}

#[cfg(test)]
mod tests {

    use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

    use crate::take::{take_lines, TakeAllBuffer, TakeAllLinesBuffer};

    #[test]
    fn test_take_all_buffer_exact_bytes() {
        let input_buffer = "abc";
        let mut input_reader = std::io::Cursor::new(input_buffer);
        let mut take_all_buffer = TakeAllBuffer::new();
        let bytes_read = take_all_buffer.fill_buffer(&mut input_reader).unwrap();
        assert_eq!(bytes_read, input_buffer.len());
        assert_eq!(take_all_buffer.remaining_bytes(), input_buffer.len());
        assert_eq!(take_all_buffer.remaining_buffer(), input_buffer.as_bytes());
        assert!(!take_all_buffer.is_empty());
        let mut output_reader = std::io::Cursor::new(vec![0x10; 0]);
        for (index, c) in input_buffer.bytes().enumerate() {
            take_all_buffer
                .write_bytes_exact(&mut output_reader, 1)
                .unwrap();
            let buf_ref = output_reader.get_ref();
            assert_eq!(buf_ref.len(), index + 1);
            assert_eq!(buf_ref[index], c);
            assert_eq!(
                take_all_buffer.remaining_bytes(),
                input_buffer.len() - (index + 1)
            );
            assert_eq!(
                take_all_buffer.remaining_buffer(),
                &input_buffer.as_bytes()[index + 1..]
            );
        }

        assert!(take_all_buffer.is_empty());
        assert_eq!(take_all_buffer.remaining_bytes(), 0);
        assert_eq!(take_all_buffer.remaining_buffer(), "".as_bytes());
    }

    #[test]
    fn test_take_all_buffer_all_bytes() {
        let input_buffer = "abc";
        let mut input_reader = std::io::Cursor::new(input_buffer);
        let mut take_all_buffer = TakeAllBuffer::new();
        let bytes_read = take_all_buffer.fill_buffer(&mut input_reader).unwrap();
        assert_eq!(bytes_read, input_buffer.len());
        assert_eq!(take_all_buffer.remaining_bytes(), input_buffer.len());
        let mut output_reader = std::io::Cursor::new(vec![0x10; 0]);
        let bytes_written = take_all_buffer.write_all(&mut output_reader).unwrap();
        assert_eq!(bytes_written, input_buffer.len());
        assert_eq!(output_reader.get_ref().as_slice(), input_buffer.as_bytes());

        assert!(take_all_buffer.is_empty());
        assert_eq!(take_all_buffer.remaining_bytes(), 0);
        assert_eq!(take_all_buffer.remaining_buffer(), "".as_bytes());

        // Now do a write_all on an empty TakeAllBuffer. Confirm correct behavior.
        let mut output_reader = std::io::Cursor::new(vec![0x10; 0]);
        let bytes_written = take_all_buffer.write_all(&mut output_reader).unwrap();
        assert_eq!(bytes_written, 0);
        assert_eq!(output_reader.get_ref().as_slice().len(), 0);
    }

    #[test]
    fn test_take_all_buffer_limit_bytes() {
        let input_buffer = "abc";
        let mut input_reader = std::io::Cursor::new(input_buffer);
        let mut take_all_buffer = TakeAllBuffer::new();
        let bytes_read = take_all_buffer.fill_buffer(&mut input_reader).unwrap();
        assert_eq!(bytes_read, input_buffer.len());
        assert_eq!(take_all_buffer.remaining_bytes(), input_buffer.len());
        let mut output_reader = std::io::Cursor::new(vec![0x10; 0]);
        // Write all but 1 bytes.
        let bytes_to_write = input_buffer.len() - 1;
        let bytes_written = take_all_buffer
            .write_bytes_limit(&mut output_reader, bytes_to_write)
            .unwrap();
        assert_eq!(bytes_written, bytes_to_write);
        assert_eq!(
            output_reader.get_ref().as_slice(),
            &input_buffer.as_bytes()[..bytes_to_write]
        );
        assert!(!take_all_buffer.is_empty());
        assert_eq!(take_all_buffer.remaining_bytes(), 1);
        assert_eq!(
            take_all_buffer.remaining_buffer(),
            &input_buffer.as_bytes()[bytes_to_write..]
        );

        // Write 1 more byte - i.e. last byte in buffer.
        let bytes_to_write = 1;
        let bytes_written = take_all_buffer
            .write_bytes_limit(&mut output_reader, bytes_to_write)
            .unwrap();
        assert_eq!(bytes_written, bytes_to_write);
        assert_eq!(output_reader.get_ref().as_slice(), input_buffer.as_bytes());
        assert!(take_all_buffer.is_empty());
        assert_eq!(take_all_buffer.remaining_bytes(), 0);
        assert_eq!(take_all_buffer.remaining_buffer(), "".as_bytes());

        // Write 1 more byte - i.e. confirm behavior on already empty buffer.
        let mut output_reader = std::io::Cursor::new(vec![0x10; 0]);
        let bytes_to_write = 1;
        let bytes_written = take_all_buffer
            .write_bytes_limit(&mut output_reader, bytes_to_write)
            .unwrap();
        assert_eq!(bytes_written, 0);
        assert_eq!(output_reader.get_ref().as_slice().len(), 0);
        assert!(take_all_buffer.is_empty());
        assert_eq!(take_all_buffer.remaining_bytes(), 0);
        assert_eq!(take_all_buffer.remaining_buffer(), "".as_bytes());
    }

    #[test]
    fn test_take_all_lines_buffer() {
        // 3 lines with new-lines and one partial line.
        let input_buffer = "a\nb\nc\ndef";
        let separator = b'\n';
        let mut input_reader = std::io::Cursor::new(input_buffer);
        let mut take_all_lines_buffer = TakeAllLinesBuffer::new();
        let fill_result = take_all_lines_buffer
            .fill_buffer(&mut input_reader, separator)
            .unwrap();
        assert_eq!(fill_result.bytes, input_buffer.len());
        assert_eq!(fill_result.lines, 3);
        assert_eq!(take_all_lines_buffer.lines(), 3);
        assert!(!take_all_lines_buffer.is_empty());

        // Write 1st line.
        let mut output_reader = std::io::Cursor::new(vec![0x10; 0]);
        let lines_to_write = 1;
        let write_result = take_all_lines_buffer
            .write_lines(&mut output_reader, lines_to_write, separator)
            .unwrap();
        assert_eq!(write_result.bytes, 2);
        assert_eq!(write_result.lines, lines_to_write);
        assert_eq!(output_reader.get_ref().as_slice(), "a\n".as_bytes());
        assert!(!take_all_lines_buffer.is_empty());
        assert_eq!(take_all_lines_buffer.lines(), 2);

        // Write 2nd line.
        let mut output_reader = std::io::Cursor::new(vec![0x10; 0]);
        let lines_to_write = 1;
        let write_result = take_all_lines_buffer
            .write_lines(&mut output_reader, lines_to_write, separator)
            .unwrap();
        assert_eq!(write_result.bytes, 2);
        assert_eq!(write_result.lines, lines_to_write);
        assert_eq!(output_reader.get_ref().as_slice(), "b\n".as_bytes());
        assert!(!take_all_lines_buffer.is_empty());
        assert_eq!(take_all_lines_buffer.lines(), 1);

        // Now try to write 3 lines. Should write everything left in the buffer.
        let mut output_reader = std::io::Cursor::new(vec![0x10; 0]);
        let lines_to_write = 3;
        let write_result = take_all_lines_buffer
            .write_lines(&mut output_reader, lines_to_write, separator)
            .unwrap();
        assert_eq!(write_result.bytes, 5);
        assert_eq!(write_result.lines, 1);
        assert_eq!(output_reader.get_ref().as_slice(), "c\ndef".as_bytes());
        assert!(take_all_lines_buffer.is_empty());
        assert_eq!(take_all_lines_buffer.lines(), 0);
    }

    #[test]
    fn test_zero_lines() {
        let input_reader = std::io::Cursor::new("a\nb\nc\n");
        let output_reader = BufReader::new(take_lines(input_reader, 0, b'\n'));
        let mut iter = output_reader.lines().map(|l| l.unwrap());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_fewer_lines() {
        let input_reader = std::io::Cursor::new("a\nb\nc\n");
        let output_reader = BufReader::new(take_lines(input_reader, 2, b'\n'));
        let mut iter = output_reader.lines().map(|l| l.unwrap());
        assert_eq!(Some(String::from("a")), iter.next());
        assert_eq!(Some(String::from("b")), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_more_lines() {
        let input_reader = std::io::Cursor::new("a\nb\nc\n");
        let output_reader = BufReader::new(take_lines(input_reader, 4, b'\n'));
        let mut iter = output_reader.lines().map(|l| l.unwrap());
        assert_eq!(Some(String::from("a")), iter.next());
        assert_eq!(Some(String::from("b")), iter.next());
        assert_eq!(Some(String::from("c")), iter.next());
        assert_eq!(None, iter.next());
    }
}
