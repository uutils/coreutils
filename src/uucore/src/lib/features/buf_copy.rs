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
use std::os::fd::AsFd;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn copy_stream(
    src: &mut (impl std::io::Read + AsFd),
    dest: &mut impl AsFd,
) -> std::io::Result<()> {
    // try to splice() system call for throughput
    if crate::pipes::splice_unbounded_auto(src, dest)?.is_err() {
        // fall back on writing "without buffering", or order of output would be wrong
        // unrelated for cp /dev/stdin since cp does not have multiple input? <https://github.com/uutils/coreutils/issues/5186>
        // RawWriter also removes io::copy's specialization slower than our splice
        std::io::copy(src, &mut crate::io::RawWriter(dest))?;
    }
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub use std::io::copy as copy_stream;

#[cfg(test)]
#[cfg(any(target_os = "linux", target_os = "android"))] // copy_stream is io::copy on other platforms. nothing to test.
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

        let (mut pipe_read, mut pipe_write) = std::io::pipe().unwrap();
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
