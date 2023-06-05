//! Traits and implementations for iterating over lines in a file-like object.
//!
//! This module provides a [`WordCountable`] trait and implementations
//! for some common file-like objects. Use the [`WordCountable::buffered`]
//! method to get an iterator over lines of a file-like object.
use std::fs::File;
use std::io::{BufRead, BufReader, Read, StdinLock};

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

#[cfg(unix)]
pub trait WordCountable: AsRawFd + Read {
    type Buffered: BufRead;
    fn buffered(self) -> Self::Buffered;
}

#[cfg(not(unix))]
pub trait WordCountable: Read {
    type Buffered: BufRead;
    fn buffered(self) -> Self::Buffered;
}

impl WordCountable for StdinLock<'_> {
    type Buffered = Self;

    fn buffered(self) -> Self::Buffered {
        self
    }
}

impl WordCountable for File {
    type Buffered = BufReader<Self>;

    fn buffered(self) -> Self::Buffered {
        BufReader::new(self)
    }
}
