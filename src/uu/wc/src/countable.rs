//! Traits and implementations for iterating over lines in a file-like object.
//!
//! This module provides a [`WordCountable`] trait and implementations
//! for some common file-like objects. Use the [`WordCountable::lines`]
//! method to get an iterator over lines of a file-like object.
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, StdinLock};

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

#[cfg(unix)]
pub trait WordCountable: AsRawFd + Read {
    type Buffered: BufRead;
    fn lines(self) -> Lines<Self::Buffered>;
}

#[cfg(not(unix))]
pub trait WordCountable: Read {
    type Buffered: BufRead;
    fn lines(self) -> Lines<Self::Buffered>;
}

impl WordCountable for StdinLock<'_> {
    type Buffered = Self;

    fn lines(self) -> Lines<Self::Buffered>
    where
        Self: Sized,
    {
        Lines::new(self)
    }
}
impl WordCountable for File {
    type Buffered = BufReader<Self>;

    fn lines(self) -> Lines<Self::Buffered>
    where
        Self: Sized,
    {
        Lines::new(BufReader::new(self))
    }
}

/// An iterator over the lines of an instance of `BufRead`.
///
/// Similar to [`io::Lines`] but yields each line as a `Vec<u8>` and
/// includes the newline character (`\n`, the `0xA` byte) that
/// terminates the line.
///
/// [`io::Lines`]:: io::Lines
pub struct Lines<B> {
    buf: B,
    line: Vec<u8>,
}

impl<B: BufRead> Lines<B> {
    fn new(reader: B) -> Self {
        Lines {
            buf: reader,
            line: Vec::new(),
        }
    }

    pub fn next(&mut self) -> Option<io::Result<&[u8]>> {
        self.line.clear();

        // reading from a TTY seems to raise a condition on, rather than return Some(0) like a file.
        // hence the option wrapped in a result here
        match self.buf.read_until(b'\n', &mut self.line) {
            Ok(0) => None,
            Ok(_n) => Some(Ok(&self.line)),
            Err(e) => Some(Err(e)),
        }
    }
}
