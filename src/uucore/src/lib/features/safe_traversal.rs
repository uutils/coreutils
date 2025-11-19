// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// Safe directory traversal using openat() and related syscalls
// This module provides TOCTOU-safe filesystem operations for recursive traversal
//
// Only available on Linux
//
// spell-checker:ignore CLOEXEC RDONLY TOCTOU closedir dirp fdopendir fstatat openat REMOVEDIR unlinkat smallfile
// spell-checker:ignore RAII dirfd fchownat fchown FchmodatFlags fchmodat fchmod

#[cfg(test)]
use std::os::unix::ffi::OsStringExt;

use std::ffi::{CString, OsStr, OsString};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};
use std::path::{Path, PathBuf};

use nix::dir::Dir;
use nix::fcntl::{OFlag, openat};
use nix::libc;
use nix::sys::stat::{FchmodatFlags, FileStat, Mode, fchmodat, fstatat};
use nix::unistd::{Gid, Uid, UnlinkatFlags, fchown, fchownat, unlinkat};
use os_display::Quotable;

use crate::translate;

// Custom error types for better error reporting
#[derive(thiserror::Error, Debug)]
pub enum SafeTraversalError {
    #[error("{}", translate!("safe-traversal-error-path-contains-null"))]
    PathContainsNull,

    #[error("{}", translate!("safe-traversal-error-open-failed", "path" => path.quote(), "source" => source))]
    OpenFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("{}", translate!("safe-traversal-error-stat-failed", "path" => path.quote(), "source" => source))]
    StatFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("{}", translate!("safe-traversal-error-read-dir-failed", "path" => path.quote(), "source" => source))]
    ReadDirFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("{}", translate!("safe-traversal-error-unlink-failed", "path" => path.quote(), "source" => source))]
    UnlinkFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

impl From<SafeTraversalError> for io::Error {
    fn from(err: SafeTraversalError) -> Self {
        match err {
            SafeTraversalError::PathContainsNull => Self::new(
                io::ErrorKind::InvalidInput,
                translate!("safe-traversal-error-path-contains-null"),
            ),
            SafeTraversalError::OpenFailed { source, .. } => source,
            SafeTraversalError::StatFailed { source, .. } => source,
            SafeTraversalError::ReadDirFailed { source, .. } => source,
            SafeTraversalError::UnlinkFailed { source, .. } => source,
        }
    }
}

// Helper function to read directory entries using nix
fn read_dir_entries(fd: &OwnedFd) -> io::Result<Vec<OsString>> {
    let mut entries = Vec::new();

    // Duplicate the fd for Dir (it takes ownership)
    let dup_fd = nix::unistd::dup(fd).map_err(|e| io::Error::from_raw_os_error(e as i32))?;

    let mut dir = Dir::from_fd(dup_fd).map_err(|e| io::Error::from_raw_os_error(e as i32))?;

    for entry_result in dir.iter() {
        let entry = entry_result.map_err(|e| io::Error::from_raw_os_error(e as i32))?;

        let name = entry.file_name();
        let name_os = OsStr::from_bytes(name.to_bytes());

        if name_os != "." && name_os != ".." {
            entries.push(name_os.to_os_string());
        }
    }

    Ok(entries)
}

/// A directory file descriptor that enables safe traversal
pub struct DirFd {
    fd: OwnedFd,
}

impl DirFd {
    /// Open a directory and return a file descriptor
    pub fn open(path: &Path) -> io::Result<Self> {
        let flags = OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC;
        let fd = nix::fcntl::open(path, flags, Mode::empty()).map_err(|e| {
            SafeTraversalError::OpenFailed {
                path: path.into(),
                source: io::Error::from_raw_os_error(e as i32),
            }
        })?;

        Ok(Self { fd })
    }

    /// Open a subdirectory relative to this directory
    pub fn open_subdir(&self, name: &OsStr) -> io::Result<Self> {
        let name_cstr =
            CString::new(name.as_bytes()).map_err(|_| SafeTraversalError::PathContainsNull)?;

        let flags = OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC;
        let fd = openat(&self.fd, name_cstr.as_c_str(), flags, Mode::empty()).map_err(|e| {
            SafeTraversalError::OpenFailed {
                path: name.into(),
                source: io::Error::from_raw_os_error(e as i32),
            }
        })?;

        Ok(Self { fd })
    }

    /// Get raw stat data for a file relative to this directory
    pub fn stat_at(&self, name: &OsStr, follow_symlinks: bool) -> io::Result<FileStat> {
        let name_cstr =
            CString::new(name.as_bytes()).map_err(|_| SafeTraversalError::PathContainsNull)?;

        let flags = if follow_symlinks {
            nix::fcntl::AtFlags::empty()
        } else {
            nix::fcntl::AtFlags::AT_SYMLINK_NOFOLLOW
        };

        let stat = fstatat(&self.fd, name_cstr.as_c_str(), flags).map_err(|e| {
            SafeTraversalError::StatFailed {
                path: name.into(),
                source: io::Error::from_raw_os_error(e as i32),
            }
        })?;

        Ok(stat)
    }

    /// Get metadata for a file relative to this directory
    pub fn metadata_at(&self, name: &OsStr, follow_symlinks: bool) -> io::Result<Metadata> {
        self.stat_at(name, follow_symlinks).map(Metadata::from_stat)
    }

    /// Get metadata for this directory
    pub fn metadata(&self) -> io::Result<Metadata> {
        self.fstat().map(Metadata::from_stat)
    }

    /// Get raw stat data for this directory
    pub fn fstat(&self) -> io::Result<FileStat> {
        let stat = nix::sys::stat::fstat(&self.fd).map_err(|e| SafeTraversalError::StatFailed {
            path: translate!("safe-traversal-current-directory").into(),
            source: io::Error::from_raw_os_error(e as i32),
        })?;

        Ok(stat)
    }

    /// Read directory entries
    pub fn read_dir(&self) -> io::Result<Vec<OsString>> {
        read_dir_entries(&self.fd).map_err(|e| {
            SafeTraversalError::ReadDirFailed {
                path: translate!("safe-traversal-directory").into(),
                source: e,
            }
            .into()
        })
    }

    /// Remove a file or empty directory relative to this directory
    pub fn unlink_at(&self, name: &OsStr, is_dir: bool) -> io::Result<()> {
        let name_cstr =
            CString::new(name.as_bytes()).map_err(|_| SafeTraversalError::PathContainsNull)?;
        let flags = if is_dir {
            UnlinkatFlags::RemoveDir
        } else {
            UnlinkatFlags::NoRemoveDir
        };

        unlinkat(&self.fd, name_cstr.as_c_str(), flags).map_err(|e| {
            SafeTraversalError::UnlinkFailed {
                path: name.into(),
                source: io::Error::from_raw_os_error(e as i32),
            }
        })?;

        Ok(())
    }

    /// Change ownership of a file relative to this directory
    /// Use uid/gid of None to keep the current value
    pub fn chown_at(
        &self,
        name: &OsStr,
        uid: Option<u32>,
        gid: Option<u32>,
        follow_symlinks: bool,
    ) -> io::Result<()> {
        let name_cstr =
            CString::new(name.as_bytes()).map_err(|_| SafeTraversalError::PathContainsNull)?;

        let flags = if follow_symlinks {
            nix::fcntl::AtFlags::empty()
        } else {
            nix::fcntl::AtFlags::AT_SYMLINK_NOFOLLOW
        };

        let uid = uid.map(Uid::from_raw);
        let gid = gid.map(Gid::from_raw);

        fchownat(&self.fd, name_cstr.as_c_str(), uid, gid, flags)
            .map_err(|e| io::Error::from_raw_os_error(e as i32))?;

        Ok(())
    }

    /// Change ownership of this directory
    pub fn fchown(&self, uid: Option<u32>, gid: Option<u32>) -> io::Result<()> {
        let uid = uid.map(Uid::from_raw);
        let gid = gid.map(Gid::from_raw);

        fchown(&self.fd, uid, gid).map_err(|e| io::Error::from_raw_os_error(e as i32))?;

        Ok(())
    }

    /// Change mode of a file relative to this directory
    pub fn chmod_at(&self, name: &OsStr, mode: u32, follow_symlinks: bool) -> io::Result<()> {
        let flags = if follow_symlinks {
            FchmodatFlags::FollowSymlink
        } else {
            FchmodatFlags::NoFollowSymlink
        };

        let mode = Mode::from_bits_truncate(mode);

        let name_cstr =
            CString::new(name.as_bytes()).map_err(|_| SafeTraversalError::PathContainsNull)?;

        fchmodat(&self.fd, name_cstr.as_c_str(), mode, flags)
            .map_err(|e| io::Error::from_raw_os_error(e as i32))?;

        Ok(())
    }

    /// Change mode of this directory
    pub fn fchmod(&self, mode: u32) -> io::Result<()> {
        let mode = Mode::from_bits_truncate(mode);

        nix::sys::stat::fchmod(&self.fd, mode)
            .map_err(|e| io::Error::from_raw_os_error(e as i32))?;

        Ok(())
    }

    /// Create a DirFd from an existing file descriptor (takes ownership)
    pub fn from_raw_fd(fd: RawFd) -> io::Result<Self> {
        if fd < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                translate!("safe-traversal-error-invalid-fd"),
            ));
        }
        // SAFETY: We've verified fd >= 0, and the caller is transferring ownership
        let owned_fd = unsafe { OwnedFd::from_raw_fd(fd) };
        Ok(Self { fd: owned_fd })
    }
}

impl AsRawFd for DirFd {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl AsFd for DirFd {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

/// File information for tracking inodes
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FileInfo {
    pub dev: u64,
    pub ino: u64,
}

impl FileInfo {
    pub fn from_stat(stat: &libc::stat) -> Self {
        // Allow unnecessary cast because st_dev and st_ino have different types on different platforms
        #[allow(clippy::unnecessary_cast)]
        Self {
            dev: stat.st_dev as u64,
            ino: stat.st_ino as u64,
        }
    }

    /// Create FileInfo from device and inode numbers
    pub fn new(dev: u64, ino: u64) -> Self {
        Self { dev, ino }
    }

    /// Get the device number
    pub fn device(&self) -> u64 {
        self.dev
    }

    /// Get the inode number
    pub fn inode(&self) -> u64 {
        self.ino
    }
}

/// File type enumeration for better type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Directory,
    RegularFile,
    Symlink,
    Other,
}

impl FileType {
    pub fn from_mode(mode: libc::mode_t) -> Self {
        match mode & libc::S_IFMT {
            libc::S_IFDIR => Self::Directory,
            libc::S_IFREG => Self::RegularFile,
            libc::S_IFLNK => Self::Symlink,
            _ => Self::Other,
        }
    }

    pub fn is_directory(&self) -> bool {
        matches!(self, Self::Directory)
    }

    pub fn is_regular_file(&self) -> bool {
        matches!(self, Self::RegularFile)
    }

    pub fn is_symlink(&self) -> bool {
        matches!(self, Self::Symlink)
    }
}

/// Metadata wrapper for safer access to file information
#[derive(Debug, Clone)]
pub struct Metadata {
    stat: FileStat,
}

impl Metadata {
    pub fn from_stat(stat: FileStat) -> Self {
        Self { stat }
    }

    pub fn file_type(&self) -> FileType {
        FileType::from_mode(self.stat.st_mode)
    }

    pub fn file_info(&self) -> FileInfo {
        FileInfo::from_stat(&self.stat)
    }

    pub fn size(&self) -> u64 {
        self.stat.st_size as u64
    }

    pub fn mode(&self) -> u32 {
        self.stat.st_mode
    }

    pub fn nlink(&self) -> u64 {
        // st_nlink is u32 on most platforms except x86_64
        #[cfg(target_arch = "x86_64")]
        {
            self.stat.st_nlink
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            self.stat.st_nlink.into()
        }
    }

    /// Compatibility methods to match std::fs::Metadata interface
    pub fn is_dir(&self) -> bool {
        self.file_type().is_directory()
    }

    pub fn len(&self) -> u64 {
        self.size()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// Add MetadataExt trait implementation for compatibility
impl std::os::unix::fs::MetadataExt for Metadata {
    fn dev(&self) -> u64 {
        self.stat.st_dev
    }

    fn ino(&self) -> u64 {
        #[cfg(target_pointer_width = "32")]
        {
            self.stat.st_ino.into()
        }
        #[cfg(not(target_pointer_width = "32"))]
        {
            self.stat.st_ino
        }
    }

    fn mode(&self) -> u32 {
        self.stat.st_mode
    }

    fn nlink(&self) -> u64 {
        // st_nlink is u32 on most platforms except x86_64
        #[cfg(target_arch = "x86_64")]
        {
            self.stat.st_nlink
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            self.stat.st_nlink.into()
        }
    }

    fn uid(&self) -> u32 {
        self.stat.st_uid
    }

    fn gid(&self) -> u32 {
        self.stat.st_gid
    }

    fn rdev(&self) -> u64 {
        self.stat.st_rdev
    }

    fn size(&self) -> u64 {
        self.stat.st_size as u64
    }

    fn atime(&self) -> i64 {
        #[cfg(target_pointer_width = "32")]
        {
            self.stat.st_atime.into()
        }
        #[cfg(not(target_pointer_width = "32"))]
        {
            self.stat.st_atime
        }
    }

    fn atime_nsec(&self) -> i64 {
        #[cfg(target_pointer_width = "32")]
        {
            self.stat.st_atime_nsec.into()
        }
        #[cfg(not(target_pointer_width = "32"))]
        {
            self.stat.st_atime_nsec
        }
    }

    fn mtime(&self) -> i64 {
        #[cfg(target_pointer_width = "32")]
        {
            self.stat.st_mtime.into()
        }
        #[cfg(not(target_pointer_width = "32"))]
        {
            self.stat.st_mtime
        }
    }

    fn mtime_nsec(&self) -> i64 {
        #[cfg(target_pointer_width = "32")]
        {
            self.stat.st_mtime_nsec.into()
        }
        #[cfg(not(target_pointer_width = "32"))]
        {
            self.stat.st_mtime_nsec
        }
    }

    fn ctime(&self) -> i64 {
        #[cfg(target_pointer_width = "32")]
        {
            self.stat.st_ctime.into()
        }
        #[cfg(not(target_pointer_width = "32"))]
        {
            self.stat.st_ctime
        }
    }

    fn ctime_nsec(&self) -> i64 {
        #[cfg(target_pointer_width = "32")]
        {
            self.stat.st_ctime_nsec.into()
        }
        #[cfg(not(target_pointer_width = "32"))]
        {
            self.stat.st_ctime_nsec
        }
    }

    fn blksize(&self) -> u64 {
        self.stat.st_blksize as u64
    }

    fn blocks(&self) -> u64 {
        self.stat.st_blocks as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::os::unix::io::IntoRawFd;
    use tempfile::TempDir;

    #[test]
    fn test_dirfd_open_valid_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_fd = DirFd::open(temp_dir.path()).unwrap();
        assert!(dir_fd.as_raw_fd() >= 0);
    }

    #[test]
    fn test_dirfd_open_nonexistent_directory() {
        let result = DirFd::open("/nonexistent/path".as_ref());
        assert!(result.is_err());
        if let Err(e) = result {
            // The error should be the underlying io::Error
            assert!(
                e.kind() == io::ErrorKind::NotFound || e.kind() == io::ErrorKind::PermissionDenied
            );
        }
    }

    #[test]
    fn test_dirfd_open_file_not_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file");
        fs::write(&file_path, "test content").unwrap();

        let result = DirFd::open(&file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_dirfd_open_subdir() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let parent_fd = DirFd::open(temp_dir.path()).unwrap();
        let subdir_fd = parent_fd.open_subdir(OsStr::new("subdir")).unwrap();
        assert!(subdir_fd.as_raw_fd() >= 0);
    }

    #[test]
    fn test_dirfd_open_nonexistent_subdir() {
        let temp_dir = TempDir::new().unwrap();
        let parent_fd = DirFd::open(temp_dir.path()).unwrap();

        let result = parent_fd.open_subdir(OsStr::new("nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn test_dirfd_stat_at() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file");
        fs::write(&file_path, "test content").unwrap();

        let dir_fd = DirFd::open(temp_dir.path()).unwrap();
        let stat = dir_fd.stat_at(OsStr::new("test_file"), true).unwrap();

        assert!(stat.st_size > 0);
        assert_eq!(stat.st_mode & libc::S_IFMT, libc::S_IFREG);
    }

    #[test]
    fn test_dirfd_stat_at_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("target");
        let symlink_file = temp_dir.path().join("link");

        fs::write(&target_file, "target content").unwrap();
        symlink(&target_file, &symlink_file).unwrap();

        let dir_fd = DirFd::open(temp_dir.path()).unwrap();

        // Follow symlinks
        let stat_follow = dir_fd.stat_at(OsStr::new("link"), true).unwrap();
        assert_eq!(stat_follow.st_mode & libc::S_IFMT, libc::S_IFREG);

        // Don't follow symlinks
        let stat_nofollow = dir_fd.stat_at(OsStr::new("link"), false).unwrap();
        assert_eq!(stat_nofollow.st_mode & libc::S_IFMT, libc::S_IFLNK);
    }

    #[test]
    fn test_dirfd_fstat() {
        let temp_dir = TempDir::new().unwrap();
        let dir_fd = DirFd::open(temp_dir.path()).unwrap();
        let stat = dir_fd.fstat().unwrap();

        assert_eq!(stat.st_mode & libc::S_IFMT, libc::S_IFDIR);
    }

    #[test]
    fn test_dirfd_read_dir() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1");
        let file2 = temp_dir.path().join("file2");

        fs::write(&file1, "content1").unwrap();
        fs::write(&file2, "content2").unwrap();

        let dir_fd = DirFd::open(temp_dir.path()).unwrap();
        let entries = dir_fd.read_dir().unwrap();

        assert_eq!(entries.len(), 2);
        assert!(entries.contains(&OsString::from("file1")));
        assert!(entries.contains(&OsString::from("file2")));
    }

    #[test]
    fn test_dirfd_unlink_at_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file");
        fs::write(&file_path, "test content").unwrap();

        let dir_fd = DirFd::open(temp_dir.path()).unwrap();
        dir_fd.unlink_at(OsStr::new("test_file"), false).unwrap();

        assert!(!file_path.exists());
    }

    #[test]
    fn test_dirfd_unlink_at_directory() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("empty_dir");
        fs::create_dir(&subdir).unwrap();

        let dir_fd = DirFd::open(temp_dir.path()).unwrap();
        dir_fd.unlink_at(OsStr::new("empty_dir"), true).unwrap();

        assert!(!subdir.exists());
    }

    #[test]
    fn test_from_raw_fd() {
        let temp_dir = TempDir::new().unwrap();
        let dir_fd = DirFd::open(temp_dir.path()).unwrap();

        // Duplicate the fd first so we don't have ownership conflicts
        let dup_fd = nix::unistd::dup(&dir_fd).unwrap();
        let from_raw_fd = DirFd::from_raw_fd(dup_fd.into_raw_fd()).unwrap();

        // Both should refer to the same directory
        let stat1 = dir_fd.fstat().unwrap();
        let stat2 = from_raw_fd.fstat().unwrap();
        assert_eq!(stat1.st_ino, stat2.st_ino);
        assert_eq!(stat1.st_dev, stat2.st_dev);
    }

    #[test]
    fn test_from_raw_fd_invalid() {
        let result = DirFd::from_raw_fd(-1);
        assert!(result.is_err());
    }

    #[test]
    #[allow(clippy::unnecessary_cast)]
    fn test_file_info() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file");
        fs::write(&file_path, "test content").unwrap();

        let dir_fd = DirFd::open(temp_dir.path()).unwrap();
        let stat = dir_fd.stat_at(OsStr::new("test_file"), true).unwrap();
        let file_info = FileInfo::from_stat(&stat);
        assert_eq!(file_info.device(), stat.st_dev as u64);
        assert_eq!(file_info.inode(), stat.st_ino as u64);
    }

    #[test]
    fn test_file_info_new() {
        let file_info = FileInfo::new(123, 456);
        assert_eq!(file_info.device(), 123);
        assert_eq!(file_info.inode(), 456);
    }

    #[test]
    fn test_file_type() {
        // Test directory
        let dir_mode = libc::S_IFDIR | 0o755;
        let file_type = FileType::from_mode(dir_mode);
        assert_eq!(file_type, FileType::Directory);
        assert!(file_type.is_directory());
        assert!(!file_type.is_regular_file());
        assert!(!file_type.is_symlink());

        // Test regular file
        let file_mode = libc::S_IFREG | 0o644;
        let file_type = FileType::from_mode(file_mode);
        assert_eq!(file_type, FileType::RegularFile);
        assert!(!file_type.is_directory());
        assert!(file_type.is_regular_file());
        assert!(!file_type.is_symlink());

        // Test symlink
        let link_mode = libc::S_IFLNK | 0o777;
        let file_type = FileType::from_mode(link_mode);
        assert_eq!(file_type, FileType::Symlink);
        assert!(!file_type.is_directory());
        assert!(!file_type.is_regular_file());
        assert!(file_type.is_symlink());
    }

    #[test]
    #[allow(clippy::unnecessary_cast)]
    fn test_metadata_wrapper() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file");
        fs::write(&file_path, "test content with some length").unwrap();

        let dir_fd = DirFd::open(temp_dir.path()).unwrap();
        let metadata = dir_fd.metadata_at(OsStr::new("test_file"), true).unwrap();

        assert_eq!(metadata.file_type(), FileType::RegularFile);
        assert!(metadata.size() > 0);
        assert_eq!(metadata.mode() & libc::S_IFMT as u32, libc::S_IFREG as u32);
        assert_eq!(metadata.nlink(), 1);

        assert!(metadata.size() > 0);
    }

    #[test]
    fn test_metadata_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_fd = DirFd::open(temp_dir.path()).unwrap();
        let metadata = dir_fd.metadata().unwrap();

        assert_eq!(metadata.file_type(), FileType::Directory);
        assert!(metadata.file_type().is_directory());
    }

    #[test]
    fn test_path_with_null_byte() {
        let path_with_null = std::ffi::OsString::from_vec(b"test\0file".to_vec());
        let temp_dir = TempDir::new().unwrap();
        let dir_fd = DirFd::open(temp_dir.path()).unwrap();

        let result = dir_fd.open_subdir(&path_with_null);
        assert!(result.is_err());
        if let Err(e) = result {
            // Should be InvalidInput for null byte error
            assert_eq!(e.kind(), io::ErrorKind::InvalidInput);
        }
    }

    #[test]
    fn test_error_chain() {
        let result = DirFd::open("/nonexistent/deeply/nested/path".as_ref());
        assert!(result.is_err());

        if let Err(e) = result {
            // Test that we get the proper underlying error
            let io_err: io::Error = e;
            assert!(
                io_err.kind() == io::ErrorKind::NotFound
                    || io_err.kind() == io::ErrorKind::PermissionDenied
            );
        }
    }
}
