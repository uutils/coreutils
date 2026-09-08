// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsStr;
use std::fs::{self, Metadata, Permissions};
use std::io;
use std::os::unix;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::Path;

use nix::sys::stat::{Mode, SFlag, mknod};

/// Recreate a special file using Redox's path-based filesystem operations.
pub(crate) fn copy_special_file(from: &Path, metadata: &Metadata, to: &Path) -> io::Result<()> {
    let file_type = metadata.file_type();
    let mode = Mode::from_bits_truncate(metadata.mode() as _);
    if file_type.is_fifo() {
        nix::unistd::mkfifo(to, mode)?;
    } else {
        let kind = if file_type.is_socket() {
            SFlag::S_IFSOCK
        } else if file_type.is_block_device() {
            SFlag::S_IFBLK
        } else {
            SFlag::S_IFCHR
        };
        mknod(to, kind, mode, metadata.rdev() as _)?;
    }
    let _ = crate::preserve_ownership(from, to);
    let _ = fs::set_permissions(to, Permissions::from_mode(metadata.mode() & 0o7777));
    Ok(())
}

/// Replace the destination with a recreated special file and then remove the source.
pub(crate) fn rename_special_fallback(
    from: &Path,
    to: &Path,
    metadata: &Metadata,
) -> io::Result<()> {
    let parent = to
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut urandom = fs::File::open("/dev/urandom")?;

    for _ in 0..32 {
        let tmp_bytes = crate::random_temp_name(&mut urandom)?;
        let tmp = parent.join(OsStr::from_bytes(&tmp_bytes));

        match copy_special_file(from, metadata, &tmp) {
            Ok(()) => {
                if let Err(error) = fs::rename(&tmp, to) {
                    let _ = fs::remove_file(&tmp);
                    return Err(error);
                }
                return fs::remove_file(from);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique temp name in destination directory",
    ))
}

/// Replace an existing symlink using Redox's path-based filesystem operations.
pub(crate) fn create_symlink_replace(target: &Path, to: &Path) -> io::Result<()> {
    fs::remove_file(to)?;
    unix::fs::symlink(target, to)
}
