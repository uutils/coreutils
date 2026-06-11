// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Traits and implementations for iterating over lines in a file-like object.
//!
//! This module provides a [`WordCountable`] trait and implementations
//! for some common file-like objects. Use the [`WordCountable::buffered`]
//! method to get an iterator over lines of a file-like object.
use std::fs::File;
use std::io::{BufRead, BufReader, Read, StdinLock};

#[cfg(any(unix, target_os = "wasi"))]
use std::os::fd::{AsFd, AsRawFd};

#[cfg(any(unix, target_os = "wasi"))]
pub trait WordCountable: AsFd + AsRawFd + Read {
    type Buffered: BufRead;
    fn buffered(self) -> Self::Buffered;
    #[cfg(not(target_os = "wasi"))]
    fn inner_file(&mut self) -> Option<&mut File>;
}

#[cfg(not(any(unix, target_os = "wasi")))]
pub trait WordCountable: Read {
    type Buffered: BufRead;
    fn buffered(self) -> Self::Buffered;
    fn inner_file(&mut self) -> Option<&mut File>;
}

impl WordCountable for StdinLock<'_> {
    type Buffered = Self;

    fn buffered(self) -> Self::Buffered {
        self
    }
    fn inner_file(&mut self) -> Option<&mut File> {
        None
    }
}

impl WordCountable for File {
    type Buffered = BufReader<Self>;

    fn buffered(self) -> Self::Buffered {
        BufReader::new(self)
    }

    fn inner_file(&mut self) -> Option<&mut File> {
        Some(self)
    }
}
