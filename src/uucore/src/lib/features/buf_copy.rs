// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! This module provides several buffer-based copy/write functions that leverage
//! the `splice` system call in Linux systems, thus increasing the I/O
//! performance of copying between two file descriptors. This module is mostly
//! used by utilities to work around the limitations of Rust's `fs::copy` which
//! does not handle copying special files (e.g pipes, character/block devices).

pub mod common;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod linux;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub use linux::*;

#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub mod other;
#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub use other::copy_stream;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[cfg(unix)]
    use {
        crate::pipes,
        std::fs::OpenOptions,
        std::{
            io::{Seek, SeekFrom},
            thread,
        },
    };

    #[cfg(any(target_os = "linux", target_os = "android"))]
    use std::os::fd::AsRawFd;

    use std::io::{Read, Write};

    #[cfg(unix)]
    fn new_temp_file() -> File {
        let temp_dir = tempdir().unwrap();
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(temp_dir.path().join("file.txt"))
            .unwrap()
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn test_copy_exact() {
        let (mut pipe_read, mut pipe_write) = pipes::pipe().unwrap();
        let data = b"Hello, world!";
        let n = pipe_write.write(data).unwrap();
        assert_eq!(n, data.len());
        let mut buf = [0; 1024];
        let n = copy_exact(pipe_read.as_raw_fd(), &pipe_write, data.len()).unwrap();
        let n2 = pipe_read.read(&mut buf).unwrap();
        assert_eq!(n, n2);
        assert_eq!(&buf[..n], data);
    }

    #[test]
    #[cfg(unix)]
    fn test_copy_stream() {
        let mut dest_file = new_temp_file();

        let (mut pipe_read, mut pipe_write) = pipes::pipe().unwrap();
        let data = b"Hello, world!";
        let thread = thread::spawn(move || {
            pipe_write.write_all(data).unwrap();
        });
        let result = copy_stream(&mut pipe_read, &mut dest_file).unwrap();
        thread.join().unwrap();
        assert!(result == data.len() as u64);

        // We would have been at the end already, so seek again to the start.
        dest_file.seek(SeekFrom::Start(0)).unwrap();

        let mut buf = Vec::new();
        dest_file.read_to_end(&mut buf).unwrap();

        assert_eq!(buf, data);
    }

    #[test]
    #[cfg(not(unix))]
    // Test for non-unix platforms. We use regular files instead.
    fn test_copy_stream() {
        let temp_dir = tempdir().unwrap();
        let src_path = temp_dir.path().join("src.txt");
        let dest_path = temp_dir.path().join("dest.txt");

        let mut src_file = File::create(&src_path).unwrap();
        let mut dest_file = File::create(&dest_path).unwrap();

        let data = b"Hello, world!";
        src_file.write_all(data).unwrap();
        src_file.sync_all().unwrap();

        let mut src_file = File::open(&src_path).unwrap();
        let bytes_copied = copy_stream(&mut src_file, &mut dest_file).unwrap();

        let mut dest_file = File::open(&dest_path).unwrap();
        let mut buf = Vec::new();
        dest_file.read_to_end(&mut buf).unwrap();

        assert_eq!(bytes_copied as usize, data.len());
        assert_eq!(buf, data);
    }
}
