// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore TOCTOU NOFOLLOW CLOEXEC ELOOP RDONLY WRONLY WUSR RUSR

//! Path-based primitives for security-sensitive file copies.
//!
//! These wrap `open(2)` with the security defaults that copy-style
//! utilities (`cp`, `mv`) need to avoid TOCTOU races and permission-leak
//! windows:
//!
//! * [`open_source`] adds `O_NOFOLLOW` when requested, so a path swap
//!   between an `lstat` check and the open cannot redirect the read
//!   through an attacker-supplied symlink (issue #10017).
//! * [`create_dest_restrictive`] creates the destination with mode
//!   `0o600` instead of the umask-derived 0o644, so another user in a
//!   shared directory cannot open the file before the caller narrows the
//!   final permissions via `set_permissions` (issue #10011), and with
//!   [`ExistingDest::Refuse`] fails if anything is already at the path.

use std::fs::File;
use std::io;
use std::os::fd::OwnedFd;
use std::path::Path;

use rustix::fs::{Mode, OFlags, open};

/// Mode the destination file is created with, before the caller applies
/// the final permissions via `set_permissions`. `0o600` ensures no other
/// user can open the file during the copy — see issue #10011.
pub const DEST_INITIAL_MODE: u32 = 0o600;

const SOURCE_FLAGS: OFlags = OFlags::RDONLY.union(OFlags::CLOEXEC);
const DEST_FLAGS: OFlags = OFlags::WRONLY
    .union(OFlags::CREATE)
    .union(OFlags::TRUNC)
    .union(OFlags::CLOEXEC);
const DEST_EXCLUSIVE_FLAGS: OFlags = OFlags::WRONLY
    .union(OFlags::CREATE)
    .union(OFlags::EXCL)
    .union(OFlags::CLOEXEC);

/// What [`create_dest_restrictive`] does with an entry already at the path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExistingDest {
    /// Truncate it, following a symlink.
    Truncate,
    /// Fail with `EEXIST`, even for a dangling symlink.
    Refuse,
}

/// Open `path` for reading, optionally with `O_NOFOLLOW`.
///
/// Pass `nofollow = true` whenever the caller has already verified via
/// `lstat`/`symlink_metadata` that the source is not a symlink, or when
/// the user has explicitly requested no-dereference behavior (e.g. cp's
/// `-P`). With `O_NOFOLLOW` set, an attacker who swaps the path to a
/// symlink between the metadata check and this open gets `ELOOP`
/// instead of redirecting the read.
pub fn open_source<P: AsRef<Path>>(path: P, nofollow: bool) -> io::Result<File> {
    let mut flags = SOURCE_FLAGS;
    if nofollow {
        flags |= OFlags::NOFOLLOW;
    }
    let fd: OwnedFd = open(path.as_ref(), flags, Mode::empty())?;
    Ok(File::from(fd))
}

/// Create `path` with the restrictive [`DEST_INITIAL_MODE`].
///
/// On a pre-existing file `O_TRUNC` empties the contents but the existing
/// inode's mode is preserved; the restrictive mode only applies to a
/// freshly created inode. Callers that need to widen the destination to
/// the source's permissions should do so via `set_permissions` *after*
/// the content copy completes.
///
/// An entry already at `path` is truncated or refused, as `existing` says.
pub fn create_dest_restrictive<P: AsRef<Path>>(
    path: P,
    existing: ExistingDest,
) -> io::Result<File> {
    let flags = match existing {
        ExistingDest::Truncate => DEST_FLAGS,
        ExistingDest::Refuse => DEST_EXCLUSIVE_FLAGS,
    };
    let fd: OwnedFd = open(path.as_ref(), flags, Mode::RUSR.union(Mode::WUSR))?;
    Ok(File::from(fd))
}

/// The permission bits to give a copy of a file whose mode is `src_mode`.
///
/// If the chown of the copy did not take, it belongs to whoever ran the
/// copy, and keeping setuid/setgid would hand them a binary running as
/// themselves that used to run as someone else, so those bits are dropped.
pub fn preserved_mode(src_mode: u32, ownership_preserved: bool) -> u32 {
    let mode = src_mode & 0o7777;
    if ownership_preserved {
        mode
    } else {
        mode & !0o6000
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt, symlink};
    use tempfile::tempdir;

    #[test]
    fn preserved_mode_drops_setuid_setgid_only_without_ownership() {
        assert_eq!(preserved_mode(0o10_7755, true), 0o7755);
        assert_eq!(preserved_mode(0o10_7755, false), 0o1755);
    }

    #[test]
    fn open_source_follows_when_not_nofollow() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        let link = dir.path().join("link");
        File::create(&target).unwrap().write_all(b"ok").unwrap();
        symlink(&target, &link).unwrap();

        let mut f = open_source(&link, false).unwrap();
        let mut buf = String::new();
        f.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "ok");
    }

    #[test]
    fn open_source_rejects_symlink_with_nofollow() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        let link = dir.path().join("link");
        File::create(&target).unwrap();
        symlink(&target, &link).unwrap();

        let err = open_source(&link, true).unwrap_err();
        assert_eq!(
            err.raw_os_error(),
            Some(rustix::io::Errno::LOOP.raw_os_error())
        );
    }

    #[test]
    fn open_source_nofollow_accepts_regular_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("regular");
        File::create(&path).unwrap();
        open_source(&path, true).unwrap();
    }

    #[test]
    fn open_source_accepts_regular_file_without_nofollow() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("regular");
        File::create(&path).unwrap().write_all(b"data").unwrap();
        let mut f = open_source(&path, false).unwrap();
        let mut buf = String::new();
        f.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "data");
    }

    #[test]
    fn create_dest_uses_restrictive_initial_mode() {
        let dir = tempdir().unwrap();
        for (name, existing) in [
            ("new", ExistingDest::Truncate),
            ("new2", ExistingDest::Refuse),
        ] {
            let f = create_dest_restrictive(dir.path().join(name), existing).unwrap();
            let mode = f.metadata().unwrap().mode() & 0o777;
            assert_eq!(mode, DEST_INITIAL_MODE);
        }
    }

    #[test]
    fn create_dest_truncates_but_preserves_existing_mode() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("existing");
        {
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o644)
                .open(&path)
                .unwrap();
            f.write_all(b"old contents").unwrap();
        }
        // Re-open via the helper — mode of the existing inode stays 0o644,
        // only the contents are truncated.
        create_dest_restrictive(&path, ExistingDest::Truncate).unwrap();
        let mode = std::fs::metadata(&path).unwrap().mode() & 0o777;
        assert_eq!(mode, 0o644);
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);
    }

    #[test]
    fn create_dest_exclusive_rejects_existing_symlink() {
        // The exclusive create fails with EEXIST on a symlink, dangling or not,
        // and leaves its target untouched.
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        let dst = dir.path().join("dst");
        let dangling = dir.path().join("dangling");
        std::fs::write(&target, b"do not truncate me").unwrap();
        symlink(&target, &dst).unwrap();
        symlink(dir.path().join("missing"), &dangling).unwrap();

        for path in [&dst, &dangling] {
            let err = create_dest_restrictive(path, ExistingDest::Refuse).unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        }
        assert_eq!(std::fs::read(&target).unwrap(), b"do not truncate me");
        assert!(!dir.path().join("missing").exists());
    }

    #[test]
    fn create_dest_exclusive_rejects_existing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("existing");
        std::fs::write(&path, b"keep").unwrap();

        let err = create_dest_restrictive(&path, ExistingDest::Refuse).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&path).unwrap(), b"keep");
    }
}
