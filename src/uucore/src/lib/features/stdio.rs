// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Abstractions for raw access to stdin and stdout, without buffering.

use core::ops::{Deref, DerefMut};
use std::fs::File;
use std::mem::ManuallyDrop;
use std::os::fd::{FromRawFd as _, OwnedFd, RawFd};

pub struct StdinRaw(ManuallyDrop<File>);

pub struct StdoutRaw(ManuallyDrop<File>);

pub struct StderrRaw(ManuallyDrop<File>);

pub fn stdin_raw() -> StdinRaw {
    // SAFETY: We ensure that the file descriptor is never closed by
    // wrapping the `File` in `ManuallyDrop`.
    let fd = unsafe { OwnedFd::from_raw_fd(libc::STDIN_FILENO as RawFd) };
    StdinRaw(ManuallyDrop::new(File::from(fd)))
}

pub fn stdout_raw() -> StdoutRaw {
    // SAFETY: We ensure that the file descriptor is never closed by
    // wrapping the `File` in `ManuallyDrop`.
    let fd = unsafe { OwnedFd::from_raw_fd(libc::STDOUT_FILENO as RawFd) };
    StdoutRaw(ManuallyDrop::new(File::from(fd)))
}

pub fn stderr_raw() -> StderrRaw {
    // SAFETY: We ensure that the file descriptor is never closed by
    // wrapping the `File` in `ManuallyDrop`.
    let fd = unsafe { OwnedFd::from_raw_fd(libc::STDERR_FILENO as RawFd) };
    StderrRaw(ManuallyDrop::new(File::from(fd)))
}

impl Deref for StdinRaw {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StdinRaw {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for StdoutRaw {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StdoutRaw {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for StderrRaw {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StderrRaw {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
