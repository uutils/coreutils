// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! External sort: sort large inputs that may not fit in memory.
//!
//! On most platforms this uses a multi-threaded chunked approach with
//! temporary files. On WASI without atomics, a synchronous variant is used
//! instead. The two implementations live in sibling modules and are selected
//! via cfg at the module boundary.

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use uucore::error::UResult;

use crate::Line;
use crate::chunks::Chunk;
use crate::merge::WriteableTmpFile;

#[cfg(not(wasi_no_threads))]
mod threaded;
#[cfg(not(wasi_no_threads))]
pub use threaded::ext_sort;

#[cfg(wasi_no_threads)]
mod sync;
#[cfg(wasi_no_threads)]
pub use sync::ext_sort;

// Note: update `test_sort::test_start_buffer` if this size is changed
// Fixed to 8 KiB (equivalent to `std::sys::io::DEFAULT_BUF_SIZE` on most targets)
pub(super) const DEFAULT_BUF_SIZE: usize = 8 * 1024;

/// Write the lines in `chunk` to `file`, separated by `separator`.
/// `compress_prog` is used to optionally compress file contents.
pub(super) fn write<I: WriteableTmpFile>(
    chunk: &Chunk,
    file: (File, PathBuf),
    compress_prog: Option<&str>,
    separator: u8,
) -> UResult<I::Closed> {
    let mut tmp_file = I::create(file, compress_prog)?;
    write_lines(chunk.lines(), tmp_file.as_write(), separator)?;
    tmp_file.finished_writing()
}

fn write_lines<T: Write>(lines: &[Line], writer: &mut T, separator: u8) -> std::io::Result<()> {
    for s in lines {
        writer.write_all(s.line)?;
        writer.write_all(&[separator])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};

    use super::write_lines;
    use crate::Line;

    struct FailAfterFirstWrite {
        writes: usize,
    }

    impl Write for FailAfterFirstWrite {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if self.writes == 0 {
                self.writes += 1;
                Ok(buf.len())
            } else {
                Err(io::Error::other("write failed"))
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn write_lines_propagates_write_errors() {
        let lines = [Line {
            line: b"line",
            index: 0,
        }];
        let mut writer = FailAfterFirstWrite { writes: 0 };

        let error = write_lines(&lines, &mut writer, b'\n').unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::Other);
    }
}
