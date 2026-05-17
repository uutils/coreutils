// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! This module provides several buffer-based copy/write functions that leverage
//! the `splice` system call in Linux systems, thus increasing the I/O
//! performance of copying between two file descriptors. This module is mostly
//! used by utilities to work around the limitations of Rust's `fs::copy` which
//! does not handle copying special files (e.g pipes, character/block devices).

#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod linux;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub use linux::*;

#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub mod other;
#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub use other::copy_stream;

#[cfg(test)]
#[cfg(any(target_os = "linux", target_os = "android"))] // copy_stream is a thin wrapper for io::copy. nothing to test...
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    use {
        std::fs::OpenOptions,
        std::{
            io::{Seek, SeekFrom},
            thread,
        },
    };

    use std::io::{Read, Write};

    fn new_temp_file() -> File {
        let temp_dir = tempdir().unwrap();
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(temp_dir.path().join("file.txt"))
            .unwrap()
    }

    #[test]
    fn test_copy_stream() {
        let mut dest_file = new_temp_file();

        let (pipe_read, pipe_write) = rustix::pipe::pipe().unwrap();
        let mut pipe_read: File = pipe_read.into();
        let mut pipe_write: File = pipe_write.into();
        let data = b"Hello, world!";
        let thread = thread::spawn(move || {
            pipe_write.write_all(data).unwrap();
        });
        copy_stream(&mut pipe_read, &mut dest_file).unwrap();
        thread.join().unwrap();

        // We would have been at the end already, so seek again to the start.
        dest_file.seek(SeekFrom::Start(0)).unwrap();

        let mut buf = Vec::new();
        dest_file.read_to_end(&mut buf).unwrap();

        assert_eq!(buf, data);
    }
}
