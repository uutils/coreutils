// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore lseek

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
    let Ok(input_info) = FileInformation::from_file(input) else {
        return false;
    };
    let Ok(output_info) = FileInformation::from_file(output) else {
        return false;
    };
    if input_info != output_info || output_info.file_size() == 0 {
        return false;
    }
    if is_appending(output) {
        return true;
    }
    let Ok(input_pos) = lseek(input.as_fd(), 0, Whence::SeekCur) else {
        return false;
    };
    let Ok(output_pos) = lseek(output.as_fd(), 0, Whence::SeekCur) else {
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
