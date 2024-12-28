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
    use {nix::unistd, std::os::fd::AsRawFd};

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
    fn test_file_is_pipe() {
        let temp_file = new_temp_file();
        let (pipe_read, pipe_write) = pipes::pipe().unwrap();

        assert!(is_pipe(&pipe_read).unwrap());
        assert!(is_pipe(&pipe_write).unwrap());
        assert!(!is_pipe(&temp_file).unwrap());
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn test_valid_splice_errs() {
        use nix::errno::Errno;
        use nix::Error;

        let err = Error::from(Errno::EINVAL);
        assert_eq!(maybe_unsupported(err).unwrap(), (0, true));

        let err = Error::from(Errno::ENOSYS);
        assert_eq!(maybe_unsupported(err).unwrap(), (0, true));

        let err = Error::from(Errno::EBADF);
        assert_eq!(maybe_unsupported(err).unwrap(), (0, true));

        let err = Error::from(Errno::EPERM);
        assert!(maybe_unsupported(err).is_err());
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn test_splice_data_to_pipe() {
        let (pipe_read, pipe_write) = pipes::pipe().unwrap();
        let data = b"Hello, world!";
        let (bytes, _) = splice_data_to_pipe(data, &pipe_write).unwrap();
        let mut buf = [0; 1024];
        let n = unistd::read(pipe_read.as_raw_fd(), &mut buf).unwrap();
        assert_eq!(&buf[..n], data);
        assert_eq!(bytes as usize, data.len());
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn test_splice_data_to_file() {
        use std::io::{Read, Seek, SeekFrom};

        let mut temp_file = new_temp_file();
        let (pipe_read, pipe_write) = pipes::pipe().unwrap();
        let data = b"Hello, world!";
        let (bytes, _) = splice_data_to_fd(data, &pipe_read, &pipe_write, &temp_file).unwrap();
        assert_eq!(bytes as usize, data.len());

        // We would have been at the end already, so seek again to the start.
        temp_file.seek(SeekFrom::Start(0)).unwrap();

        let mut buf = Vec::new();
        temp_file.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, data);
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

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn test_splice_write() {
        use std::{
            io::{Read, Seek, SeekFrom, Write},
            thread,
        };

        let (pipe_read, mut pipe_write) = pipes::pipe().unwrap();
        let mut dest_file = new_temp_file();
        let data = b"Hello, world!";
        let thread = thread::spawn(move || {
            pipe_write.write_all(data).unwrap();
        });
        let (bytes, _) = splice_write(&pipe_read, &dest_file).unwrap();
        thread.join().unwrap();

        assert!(bytes == data.len() as u64);

        // We would have been at the end already, so seek again to the start.
        dest_file.seek(SeekFrom::Start(0)).unwrap();

        let mut buf = Vec::new();
        dest_file.read_to_end(&mut buf).unwrap();

        assert_eq!(buf, data);
    }
}
