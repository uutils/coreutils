// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore lseek seekable

use nix::fcntl::{FcntlArg, OFlag, fcntl};
use nix::unistd::{Whence, lseek};
use std::os::fd::AsFd;
use uucore::fs::FileInformation;

/// An unsafe overwrite occurs when the same nonempty file is used as both stdin and stdout,
/// and the file offset of stdin is positioned earlier than that of stdout.
/// In this scenario, bytes read from stdin are written to a later part of the file
/// via stdout, which can then be read again by stdin and written again by stdout,
/// causing an infinite loop and potential file corruption.
pub fn is_unsafe_overwrite<I: AsFd, O: AsFd>(input: &I, output: &O) -> bool {
    // `FileInformation::from_file` returns an error if the file descriptor is closed, invalid,
    // or refers to a non-regular file (e.g., socket, pipe, or special device).
    let Ok(input_info) = FileInformation::from_file(input) else {
        return false;
    };
    let Ok(output_info) = FileInformation::from_file(output) else {
        return false;
    };
    if input_info != output_info {
        return false;
    }
    let file_size = output_info.file_size();
    if file_size == 0 {
        return false;
    }
    // `lseek` returns an error if the file descriptor is closed or it refers to
    // a non-seekable resource (e.g., pipe, socket, or some devices).
    let input_pos = lseek(input.as_fd(), 0, Whence::SeekCur);
    let output_pos = lseek(output.as_fd(), 0, Whence::SeekCur);
    if is_appending(output) {
        if let Ok(pos) = input_pos {
            if pos >= 0 && (pos as u64) >= file_size {
                return false;
            }
        }
        return true;
    }
    let Ok(input_pos) = input_pos else {
        return false;
    };
    let Ok(output_pos) = output_pos else {
        return false;
    };
    input_pos < output_pos
}

/// Whether the file is opened with the `O_APPEND` flag
fn is_appending<F: AsFd>(file: &F) -> bool {
    let flags_raw = fcntl(file.as_fd(), FcntlArg::F_GETFL).unwrap_or_default();
    let flags = OFlag::from_bits_truncate(flags_raw);
    flags.contains(OFlag::O_APPEND)
}

#[cfg(test)]
mod tests {
    use crate::platform::unix::{is_appending, is_unsafe_overwrite};
    use std::fs::OpenOptions;
    use std::io::{Seek, SeekFrom, Write};
    use tempfile::NamedTempFile;

    #[test]
    fn test_is_appending() {
        let temp_file = NamedTempFile::new().unwrap();
        assert!(!is_appending(&temp_file));

        let read_file = OpenOptions::new().read(true).open(&temp_file).unwrap();
        assert!(!is_appending(&read_file));

        let write_file = OpenOptions::new().write(true).open(&temp_file).unwrap();
        assert!(!is_appending(&write_file));

        let append_file = OpenOptions::new().append(true).open(&temp_file).unwrap();
        assert!(is_appending(&append_file));
    }

    #[test]
    fn test_is_unsafe_overwrite() {
        // Create two temp files one of which is empty
        let empty = NamedTempFile::new().unwrap();
        let mut nonempty = NamedTempFile::new().unwrap();
        nonempty.write_all(b"anything").unwrap();
        nonempty.seek(SeekFrom::Start(0)).unwrap();

        // Using a different file as input and output does not result in an overwrite
        assert!(!is_unsafe_overwrite(&empty, &nonempty));

        // Overwriting an empty file is always safe
        assert!(!is_unsafe_overwrite(&empty, &empty));

        // Overwriting a nonempty file with itself is safe
        assert!(!is_unsafe_overwrite(&nonempty, &nonempty));

        // Overwriting an empty file opened in append mode is safe
        let empty_append = OpenOptions::new().append(true).open(&empty).unwrap();
        assert!(!is_unsafe_overwrite(&empty, &empty_append));

        // Overwriting a nonempty file opened in append mode is unsafe
        let nonempty_append = OpenOptions::new().append(true).open(&nonempty).unwrap();
        assert!(is_unsafe_overwrite(&nonempty, &nonempty_append));

        // Overwriting a file opened in write mode is safe
        let mut nonempty_write = OpenOptions::new().write(true).open(&nonempty).unwrap();
        assert!(!is_unsafe_overwrite(&nonempty, &nonempty_write));

        // Overwriting a file when the input and output file descriptors are pointing to
        // different offsets is safe if the input offset is further than the output offset
        nonempty_write.seek(SeekFrom::Start(1)).unwrap();
        assert!(!is_unsafe_overwrite(&nonempty_write, &nonempty));
        assert!(is_unsafe_overwrite(&nonempty, &nonempty_write));
    }
}
