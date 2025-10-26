// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Tests for EINTR (ErrorKind::Interrupted) handling across utilities
//!
//! This module provides test utilities and integration tests to verify that
//! utilities properly handle signal interruptions during I/O operations.
//!
//! # CI Integration
//! EINTR handling tests are NOW visible in CI logs through integration tests:
//! - `test_cat_eintr_handling` in `tests/by-util/test_cat.rs`
//! - `test_comm_eintr_handling` in `tests/by-util/test_comm.rs`  
//! - `test_od_eintr_handling` in `tests/by-util/test_od.rs`
//! 
//! These integration tests use the mock utilities from this module to verify
//! that each utility properly handles signal interruptions during I/O operations.
//! Test results appear in CI logs under the "Test" steps when running `cargo nextest run`.
//!
//! # Note
//! EINTR is a POSIX error code for interrupted system calls
//! cspell:ignore EINTR worl

use std::io::{self, Read, Write};

/// A mock reader that injects `ErrorKind::Interrupted` errors
///
/// This reader wraps another reader and injects a specified number of
/// `Interrupted` errors before allowing reads to succeed. This simulates
/// the behavior of system calls being interrupted by signals (EINTR).
pub struct InterruptingReader<R: Read> {
    inner: R,
    interrupts_remaining: usize,
    bytes_read: usize,
}

impl<R: Read> InterruptingReader<R> {
    /// Create a new interrupting reader
    ///
    /// # Arguments
    /// * `inner` - The underlying reader to wrap
    /// * `num_interrupts` - Number of times to return `Interrupted` error before succeeding
    pub fn new(inner: R, num_interrupts: usize) -> Self {
        Self {
            inner,
            interrupts_remaining: num_interrupts,
            bytes_read: 0,
        }
    }

    /// Get the total number of bytes successfully read
    pub fn bytes_read(&self) -> usize {
        self.bytes_read
    }
}

impl<R: Read> Read for InterruptingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.interrupts_remaining > 0 {
            self.interrupts_remaining -= 1;
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        match self.inner.read(buf) {
            Ok(n) => {
                self.bytes_read += n;
                Ok(n)
            }
            err => err,
        }
    }
}

/// A mock writer that injects `ErrorKind::Interrupted` errors
pub struct InterruptingWriter<W: Write> {
    inner: W,
    interrupts_remaining: usize,
    bytes_written: usize,
}

impl<W: Write> InterruptingWriter<W> {
    /// Create a new interrupting writer
    pub fn new(inner: W, num_interrupts: usize) -> Self {
        Self {
            inner,
            interrupts_remaining: num_interrupts,
            bytes_written: 0,
        }
    }

    /// Get the total number of bytes successfully written
    pub fn bytes_written(&self) -> usize {
        self.bytes_written
    }
}

impl<W: Write> Write for InterruptingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.interrupts_remaining > 0 {
            self.interrupts_remaining -= 1;
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        match self.inner.write(buf) {
            Ok(n) => {
                self.bytes_written += n;
                Ok(n)
            }
            err => err,
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_interrupting_reader_no_interrupts() {
        let data = b"hello world";
        let mut reader = InterruptingReader::new(Cursor::new(data), 0);
        let mut buf = [0u8; 11];

        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 11);
        assert_eq!(&buf, data);
        assert_eq!(reader.bytes_read(), 11);
    }

    #[test]
    fn test_interrupting_reader_with_interrupts() {
        let data = b"hello world";
        let mut reader = InterruptingReader::new(Cursor::new(data), 3);
        let mut buf = [0u8; 11];

        // First 3 reads should fail with Interrupted
        for i in 0..3 {
            let result = reader.read(&mut buf);
            assert!(result.is_err());
            assert_eq!(
                result.unwrap_err().kind(),
                io::ErrorKind::Interrupted,
                "Read {} should be interrupted",
                i
            );
        }

        // Fourth read should succeed
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 11);
        assert_eq!(&buf, data);
        assert_eq!(reader.bytes_read(), 11);
    }

    #[test]
    fn test_interrupting_reader_multiple_chunks() {
        let data = b"hello world";
        let mut reader = InterruptingReader::new(Cursor::new(data), 2);
        let mut buf = [0u8; 5];

        // First 2 reads should fail
        assert!(reader.read(&mut buf).is_err());
        assert!(reader.read(&mut buf).is_err());

        // Third read gets first 5 bytes
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf, b"hello");

        // Read rest of data without interruption  
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf, b" worl"); // Second chunk of "hello world"

        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf[0], b'd'); // Final 'd' from "world"

        assert_eq!(reader.bytes_read(), 11);
    }

    #[test]
    fn test_interrupting_writer_no_interrupts() {
        let mut buffer = Vec::new();
        let mut writer = InterruptingWriter::new(&mut buffer, 0);

        let n = writer.write(b"test").unwrap();
        assert_eq!(n, 4);
        writer.flush().unwrap();
        assert_eq!(buffer, b"test");
        assert_eq!(writer.bytes_written(), 4);
    }

    #[test]
    fn test_interrupting_writer_with_interrupts() {
        let mut buffer = Vec::new();
        let mut writer = InterruptingWriter::new(&mut buffer, 2);

        // First 2 writes should fail
        assert!(writer.write(b"test").is_err());
        assert!(writer.write(b"test").is_err());

        // Third write should succeed
        let n = writer.write(b"test").unwrap();
        assert_eq!(n, 4);
        writer.flush().unwrap();
        assert_eq!(buffer, b"test");
        assert_eq!(writer.bytes_written(), 4);
    }
}
