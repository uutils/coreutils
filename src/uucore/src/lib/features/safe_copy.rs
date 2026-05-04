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
//!   final permissions via `set_permissions` (issue #10011). The same
//!   `nofollow` flag refuses to truncate through a symlink that may have
//!   been swapped in at the destination path.
//! * [`safe_copy_file`] composes the two for callers that just want a
//!   secure replacement for `std::fs::copy`.

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
/// With `nofollow = true`, an existing symlink at `path` causes `ELOOP`
/// rather than truncating the symlink's target. Pass `true` whenever the
/// caller has not just unlinked `path` itself: without it, an attacker
/// who plants `path` as a symlink between the caller's check and this
/// open can redirect the truncate (and the subsequent write) to any file
/// the caller has permission to write.
pub fn create_dest_restrictive<P: AsRef<Path>>(path: P, nofollow: bool) -> io::Result<File> {
    let mut flags = DEST_FLAGS;
    if nofollow {
        flags |= OFlags::NOFOLLOW;
    }
    let fd: OwnedFd = open(path.as_ref(), flags, Mode::RUSR.union(Mode::WUSR))?;
    Ok(File::from(fd))
}

/// Like [`std::fs::copy`] but uses [`open_source`] and
/// [`create_dest_restrictive`]. The same `nofollow` flag is applied to
/// both ends, so an attacker-planted symlink at either path returns
/// `ELOOP` instead of being followed.
///
/// Intentionally does *not* preserve source permissions on the
/// destination — doing so would widen the destination's mode mid-copy
/// and reopen the race that `DEST_INITIAL_MODE` closes. The caller is
/// expected to call `set_permissions` later, once content has been
/// fully written.
///
/// On error from `io::copy`, a partial destination file may remain on
/// disk (truncated to whatever was written before the failure, or empty
/// if the failure was in `create_dest_restrictive`). Cleanup of `dest`
/// on `Err` is the caller's responsibility — symmetric with
/// [`std::fs::copy`].
pub fn safe_copy_file<P: AsRef<Path>, Q: AsRef<Path>>(
    source: P,
    dest: Q,
    nofollow: bool,
) -> io::Result<u64> {
    let mut src = open_source(source, nofollow)?;
    let mut dst = create_dest_restrictive(dest, nofollow)?;
    io::copy(&mut src, &mut dst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt, symlink};
    use tempfile::tempdir;

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
        let path = dir.path().join("new");
        let f = create_dest_restrictive(&path, false).unwrap();
        let mode = f.metadata().unwrap().mode() & 0o777;
        assert_eq!(mode, DEST_INITIAL_MODE);
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
        create_dest_restrictive(&path, false).unwrap();
        let mode = std::fs::metadata(&path).unwrap().mode() & 0o777;
        assert_eq!(mode, 0o644);
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);
    }

    #[test]
    fn create_dest_rejects_symlink_with_nofollow() {
        // Without nofollow on the dest, an attacker swapping `dst` to a
        // symlink between check and open would silently truncate the
        // symlink's target. With nofollow=true the open returns ELOOP and
        // the would-be victim file is left untouched.
        let dir = tempdir().unwrap();
        let victim = dir.path().join("victim");
        let dst = dir.path().join("dst");
        std::fs::write(&victim, b"do not truncate me").unwrap();
        symlink(&victim, &dst).unwrap();

        let err = create_dest_restrictive(&dst, true).unwrap_err();
        assert_eq!(
            err.raw_os_error(),
            Some(rustix::io::Errno::LOOP.raw_os_error())
        );
        assert_eq!(std::fs::read(&victim).unwrap(), b"do not truncate me");
    }

    #[test]
    fn safe_copy_file_copies_bytes_and_keeps_dest_restrictive() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        let dst = dir.path().join("dst");
        File::create(&src).unwrap().write_all(b"payload").unwrap();

        let n = safe_copy_file(&src, &dst, false).unwrap();
        assert_eq!(n, b"payload".len() as u64);
        let mode = std::fs::metadata(&dst).unwrap().mode() & 0o777;
        assert_eq!(mode, DEST_INITIAL_MODE);
        assert_eq!(std::fs::read(&dst).unwrap(), b"payload");
    }

    #[test]
    fn safe_copy_file_nofollow_rejects_symlink_source() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        let link = dir.path().join("link");
        let dst = dir.path().join("dst");
        File::create(&target).unwrap().write_all(b"x").unwrap();
        symlink(&target, &link).unwrap();

        let err = safe_copy_file(&link, &dst, true).unwrap_err();
        assert_eq!(
            err.raw_os_error(),
            Some(rustix::io::Errno::LOOP.raw_os_error())
        );
        assert!(!dst.exists(), "dst should not be created on error");
    }

    #[test]
    fn safe_copy_file_nofollow_rejects_symlink_dest() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        let victim = dir.path().join("victim");
        let dst = dir.path().join("dst");
        File::create(&src).unwrap().write_all(b"payload").unwrap();
        std::fs::write(&victim, b"keep").unwrap();
        symlink(&victim, &dst).unwrap();

        let err = safe_copy_file(&src, &dst, true).unwrap_err();
        assert_eq!(
            err.raw_os_error(),
            Some(rustix::io::Errno::LOOP.raw_os_error())
        );
        assert_eq!(std::fs::read(&victim).unwrap(), b"keep");
    }
}
