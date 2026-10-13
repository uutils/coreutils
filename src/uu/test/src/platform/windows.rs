// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::{self, IsTerminal};
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use uucore::fs::{FileInformation, infos_refer_to_same_file};
use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS;

/// Only the standard streams can be answered for: asking the CRT about a
/// descriptor it never handed out aborts the process.
pub fn fd_is_terminal(fd: i32) -> bool {
    match fd {
        0 => io::stdin().is_terminal(),
        1 => io::stdout().is_terminal(),
        2 => io::stderr().is_terminal(),
        _ => false,
    }
}

pub fn same_file(a: &OsStr, b: &OsStr) -> bool {
    // Asking for no access right leaves nothing a share mode or DACL could
    // refuse; BACKUP_SEMANTICS lets a directory be opened.
    fn information(path: &OsStr) -> io::Result<FileInformation> {
        let file = OpenOptions::new()
            .access_mode(0)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(Path::new(path))?;
        FileInformation::from_file(&file)
    }

    infos_refer_to_same_file(information(a), information(b))
}
