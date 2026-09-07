// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::{CString, OsStr};
use std::fs;

use crate::{PathCondition, modified_since_read};

/// WASI has no uid/gid and no permission bits, so the conditions that depend
/// on them (`-u`, `-g`, `-k`, `-O`, `-G`, `-x`) are always false, and a file
/// that can be stat'ed counts as readable. The special file types are only
/// reachable through the unstable `wasi_ext` feature, so they are false too.
/// Everything the platform does report - file type, size, timestamps -
/// behaves as it does elsewhere.
pub fn path(path: &OsStr, condition: &PathCondition) -> bool {
    let metadata = if condition == &PathCondition::SymLink {
        fs::symlink_metadata(path)
    } else {
        fs::metadata(path)
    };

    let Ok(metadata) = metadata else {
        return false;
    };

    let file_type = metadata.file_type();

    match condition {
        PathCondition::Directory => file_type.is_dir(),
        PathCondition::Exists | PathCondition::Readable => true,
        PathCondition::ExistsModifiedLastRead => modified_since_read(&metadata),
        PathCondition::Regular => file_type.is_file(),
        PathCondition::SymLink => file_type.is_symlink(),
        PathCondition::NonEmpty => metadata.len() > 0,
        PathCondition::Writable => !metadata.permissions().readonly(),
        PathCondition::BlockSpecial
        | PathCondition::CharacterSpecial
        | PathCondition::Fifo
        | PathCondition::Socket
        | PathCondition::Executable
        | PathCondition::GroupIdFlag
        | PathCondition::GroupOwns
        | PathCondition::Sticky
        | PathCondition::UserIdFlag
        | PathCondition::UserOwns => false,
    }
}

/// Whether `a` and `b` are the same file, the condition behind `-ef`.
/// `std::os::wasi`'s dev/ino accessors are unstable, so go through `stat`.
pub fn same_file(a: &OsStr, b: &OsStr) -> bool {
    fn stat(path: &OsStr) -> Option<libc::stat> {
        // WASI paths are UTF-8; anything else cannot name a file here.
        let path = CString::new(path.to_str()?).ok()?;
        let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
        (unsafe { libc::stat(path.as_ptr(), &raw mut stat) } == 0).then_some(stat)
    }

    match (stat(a), stat(b)) {
        (Some(a), Some(b)) => a.st_dev == b.st_dev && a.st_ino == b.st_ino,
        _ => false,
    }
}
