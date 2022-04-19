// spell-checker:ignore (ToDO) tempbuffer abcdefgh abcdefghij

//! Contains the trait `PeekRead` and type `PeekReader` implementing it.

use std::io;
use std::io::{Read, Write};

use crate::multifilereader::HasError;

/// A trait which supplies a function to peek into a stream without
/// actually reading it.
///
/// Like `std::io::Read`, it allows to read data from a stream, with
/// the additional possibility to reserve a part of the returned data
/// with the data which will be read in subsequent calls.
///
pub trait PeekRead {
    /// Reads data into a buffer.
    ///
    /// Fills `out` with data. The last `peek_size` bytes of `out` are
    /// used for data which keeps available on subsequent calls.
    /// `peek_size` must be smaller or equal to the size of `out`.
    ///
    /// Returns a tuple where the first number is the number of bytes
    /// read from the stream, and the second number is the number of
    /// bytes additionally read. Any of the numbers might be zero.
    /// It can also return an error.
    ///
    /// A type implementing this trait, will typically also implement
    /// `std::io::Read`.
    ///
    /// # Panics
    /// Might panic if `peek_size` is larger then the size of `out`
    fn peek_read(&mut self, out: &mut [u8], peek_size: usize) -> io::Result<(usize, usize)>;
}

/// Wrapper for `std::io::Read` allowing to peek into the data to be read.
pub struct PeekReader<R> {
    inner: R,
    temp_buffer: Vec<u8>,
}

impl<R> PeekReader<R> {
    /// Create a new `PeekReader` wrapping `inner`
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            temp_buffer: Vec::new(),
        }
    }
}

impl<R: Read> PeekReader<R> {
    fn read_from_tempbuffer(&mut self, mut out: &mut [u8]) -> usize {
        match out.write(self.temp_buffer.as_mut_slice()) {
            Ok(n) => {
                self.temp_buffer.drain(..n);
                n
            }
            Err(_) => 0,
        }
    }

    fn write_to_tempbuffer(&mut self, bytes: &[u8]) {
        // if temp_buffer is not empty, data has to be inserted in front
        let org_buffer: Vec<_> = self.temp_buffer.drain(..).collect();
        self.temp_buffer.write_all(bytes).unwrap();
        self.temp_buffer.extend(org_buffer);
    }
}

impl<R: Read> Read for PeekReader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let start_pos = self.read_from_tempbuffer(out);
        match self.inner.read(&mut out[start_pos..]) {
            Err(e) => Err(e),
            Ok(n) => Ok(n + start_pos),
        }
    }
}

impl<R: Read> PeekRead for PeekReader<R> {
    /// Reads data into a buffer.
    ///
    /// See `PeekRead::peek_read`.
    ///
    /// # Panics
    /// If `peek_size` is larger then the size of `out`
    fn peek_read(&mut self, out: &mut [u8], peek_size: usize) -> io::Result<(usize, usize)> {
        assert!(out.len() >= peek_size);
        match self.read(out) {
            Err(e) => Err(e),
            Ok(bytes_in_buffer) => {
                let unused = out.len() - bytes_in_buffer;
                if peek_size <= unused {
                    Ok((bytes_in_buffer, 0))
                } else {
                    let actual_peek_size = peek_size - unused;
                    let real_size = bytes_in_buffer - actual_peek_size;
                    self.write_to_tempbuffer(&out[real_size..bytes_in_buffer]);
                    Ok((real_size, actual_peek_size))
                }
            }
        }
    }
}

impl<R: HasError> HasError for PeekReader<R> {
    fn has_error(&self) -> bool {
        self.inner.has_error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};

    #[test]
    fn test_read_normal() {
        let mut sut = PeekReader::new(Cursor::new(&b"abcdefgh"[..]));

        let mut v = [0; 10];
        assert_eq!(sut.read(v.as_mut()).unwrap(), 8);
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0, 0]);
    }

    #[test]
    fn test_peek_read_without_buffer() {
        let mut sut = PeekReader::new(Cursor::new(&b"abcdefgh"[..]));

        let mut v = [0; 10];
        assert_eq!(sut.peek_read(v.as_mut(), 0).unwrap(), (8, 0));
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0, 0]);
    }

    #[test]
    fn test_peek_read_and_read() {
        let mut sut = PeekReader::new(Cursor::new(&b"abcdefghij"[..]));

        let mut v = [0; 8];
        assert_eq!(sut.peek_read(v.as_mut(), 4).unwrap(), (4, 4));
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68]);

        let mut v2 = [0; 8];
        assert_eq!(sut.read(v2.as_mut()).unwrap(), 6);
        assert_eq!(v2, [0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0, 0]);
    }

    #[test]
    fn test_peek_read_multiple_times() {
        let mut sut = PeekReader::new(Cursor::new(&b"abcdefghij"[..]));

        let mut s1 = [0; 8];
        assert_eq!(sut.peek_read(s1.as_mut(), 4).unwrap(), (4, 4));
        assert_eq!(s1, [0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68]);

        let mut s2 = [0; 8];
        assert_eq!(sut.peek_read(s2.as_mut(), 4).unwrap(), (4, 2));
        assert_eq!(s2, [0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0, 0]);

        let mut s3 = [0; 8];
        assert_eq!(sut.peek_read(s3.as_mut(), 4).unwrap(), (2, 0));
        assert_eq!(s3, [0x69, 0x6a, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_peek_read_and_read_with_small_buffer() {
        let mut sut = PeekReader::new(Cursor::new(&b"abcdefghij"[..]));

        let mut v = [0; 8];
        assert_eq!(sut.peek_read(v.as_mut(), 4).unwrap(), (4, 4));
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68]);

        let mut v2 = [0; 2];
        assert_eq!(sut.read(v2.as_mut()).unwrap(), 2);
        assert_eq!(v2, [0x65, 0x66]);
        assert_eq!(sut.read(v2.as_mut()).unwrap(), 2);
        assert_eq!(v2, [0x67, 0x68]);
        assert_eq!(sut.read(v2.as_mut()).unwrap(), 2);
        assert_eq!(v2, [0x69, 0x6a]);
    }

    #[test]
    fn test_peek_read_with_smaller_buffer() {
        let mut sut = PeekReader::new(Cursor::new(&b"abcdefghij"[..]));

        let mut v = [0; 8];
        assert_eq!(sut.peek_read(v.as_mut(), 4).unwrap(), (4, 4));
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68]);

        let mut v2 = [0; 2];
        assert_eq!(sut.peek_read(v2.as_mut(), 2).unwrap(), (0, 2));
        assert_eq!(v2, [0x65, 0x66]);
        assert_eq!(sut.peek_read(v2.as_mut(), 0).unwrap(), (2, 0));
        assert_eq!(v2, [0x65, 0x66]);
        assert_eq!(sut.peek_read(v2.as_mut(), 0).unwrap(), (2, 0));
        assert_eq!(v2, [0x67, 0x68]);
        assert_eq!(sut.peek_read(v2.as_mut(), 0).unwrap(), (2, 0));
        assert_eq!(v2, [0x69, 0x6a]);
    }

    #[test]
    fn test_peek_read_peek_with_larger_peek_buffer() {
        let mut sut = PeekReader::new(Cursor::new(&b"abcdefghij"[..]));

        let mut v = [0; 8];
        assert_eq!(sut.peek_read(v.as_mut(), 4).unwrap(), (4, 4));
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68]);

        let mut v2 = [0; 8];
        assert_eq!(sut.peek_read(v2.as_mut(), 8).unwrap(), (0, 6));
        assert_eq!(v2, [0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0, 0]);
    }
}
