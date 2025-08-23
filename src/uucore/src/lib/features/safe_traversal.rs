// Safe directory traversal using openat() and related syscalls
// This module provides TOCTOU-safe filesystem operations for recursive traversal
// Only available on Linux
// spell-checker:ignore CLOEXEC RDONLY TOCTOU closedir dirp fdopendir fstatat openat

#![cfg(target_os = "linux")]

#[cfg(test)]
use std::os::unix::ffi::OsStringExt;

use std::ffi::{CStr, CString, OsStr, OsString};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;

// Custom error types for better error reporting
#[derive(thiserror::Error, Debug)]
pub enum SafeTraversalError {
    #[error("path contains null byte")]
    PathContainsNull,

    #[error("failed to open '{path}': {source}")]
    OpenFailed {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("failed to stat '{path}': {source}")]
    StatFailed {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("failed to read directory '{path}': {source}")]
    ReadDirFailed {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("failed to unlink '{path}': {source}")]
    UnlinkFailed {
        path: String,
        #[source]
        source: io::Error,
    },
}

impl From<SafeTraversalError> for io::Error {
    fn from(err: SafeTraversalError) -> Self {
        match err {
            SafeTraversalError::PathContainsNull => {
                io::Error::new(io::ErrorKind::InvalidInput, "path contains null byte")
            }
            SafeTraversalError::OpenFailed { source, .. } => source,
            SafeTraversalError::StatFailed { source, .. } => source,
            SafeTraversalError::ReadDirFailed { source, .. } => source,
            SafeTraversalError::UnlinkFailed { source, .. } => source,
        }
    }
}

// RAII wrapper for DIR pointer
struct Dir {
    dirp: *mut libc::DIR,
}

impl Dir {
    fn from_fd(fd: RawFd) -> io::Result<Self> {
        let dirp = unsafe { libc::fdopendir(fd) };
        if dirp.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Dir { dirp })
        }
    }

    fn read_entries(&self) -> io::Result<Vec<OsString>> {
        let mut entries = Vec::new();

        loop {
            // Clear errno before readdir as per POSIX requirements
            unsafe { *libc::__errno_location() = 0 };

            let entry = unsafe { libc::readdir(self.dirp) };
            if entry.is_null() {
                let errno = unsafe { *libc::__errno_location() };
                if errno != 0 {
                    return Err(io::Error::from_raw_os_error(errno));
                }
                break;
            }

            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
            let name_os = OsStr::from_bytes(name.to_bytes());

            if name_os != "." && name_os != ".." {
                entries.push(name_os.to_os_string());
            }
        }

        Ok(entries)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        if !self.dirp.is_null() {
            unsafe {
                libc::closedir(self.dirp);
            }
        }
    }
}

/// A directory file descriptor that enables safe traversal
pub struct DirFd {
    fd: RawFd,
    owned: bool,
}

impl DirFd {
    /// Open a directory and return a file descriptor
    pub fn open(path: &Path) -> io::Result<Self> {
        let path_str = path.to_string_lossy();
        let path_cstr = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| SafeTraversalError::PathContainsNull)?;

        let fd = unsafe {
            libc::open(
                path_cstr.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };

        if fd < 0 {
            Err(SafeTraversalError::OpenFailed {
                path: path_str.to_string(),
                source: io::Error::last_os_error(),
            }
            .into())
        } else {
            Ok(DirFd { fd, owned: true })
        }
    }

    /// Open a subdirectory relative to this directory
    pub fn open_subdir(&self, name: &OsStr) -> io::Result<Self> {
        let name_str = name.to_string_lossy();
        let name_cstr =
            CString::new(name.as_bytes()).map_err(|_| SafeTraversalError::PathContainsNull)?;

        let fd = unsafe {
            libc::openat(
                self.fd,
                name_cstr.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };

        if fd < 0 {
            Err(SafeTraversalError::OpenFailed {
                path: name_str.to_string(),
                source: io::Error::last_os_error(),
            }
            .into())
        } else {
            Ok(DirFd { fd, owned: true })
        }
    }

    /// Get raw stat data for a file relative to this directory
    pub fn stat_at(&self, name: &OsStr, follow_symlinks: bool) -> io::Result<libc::stat> {
        let name_str = name.to_string_lossy();
        let name_cstr =
            CString::new(name.as_bytes()).map_err(|_| SafeTraversalError::PathContainsNull)?;

        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        let flags = if follow_symlinks {
            0
        } else {
            libc::AT_SYMLINK_NOFOLLOW
        };

        let ret = unsafe { libc::fstatat(self.fd, name_cstr.as_ptr(), &mut stat, flags) };

        if ret < 0 {
            Err(SafeTraversalError::StatFailed {
                path: name_str.to_string(),
                source: io::Error::last_os_error(),
            }
            .into())
        } else {
            Ok(stat)
        }
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
    pub fn fstat(&self) -> io::Result<libc::stat> {
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };

        let ret = unsafe { libc::fstat(self.fd, &mut stat) };

        if ret < 0 {
            Err(SafeTraversalError::StatFailed {
                path: "<current directory>".to_string(),
                source: io::Error::last_os_error(),
            }
            .into())
        } else {
            Ok(stat)
        }
    }

    /// Read directory entries
    pub fn read_dir(&self) -> io::Result<Vec<OsString>> {
        // Duplicate the fd for fdopendir (it takes ownership)
        let dup_fd = unsafe { libc::dup(self.fd) };
        if dup_fd < 0 {
            return Err(SafeTraversalError::ReadDirFailed {
                path: "<directory>".to_string(),
                source: io::Error::last_os_error(),
            }
            .into());
        }

        let dir = Dir::from_fd(dup_fd).map_err(|e| {
            unsafe { libc::close(dup_fd) };
            SafeTraversalError::ReadDirFailed {
                path: "<directory>".to_string(),
                source: e,
            }
        })?;

        dir.read_entries().map_err(|e| {
            SafeTraversalError::ReadDirFailed {
                path: "<directory>".to_string(),
                source: e,
            }
            .into()
        })
    }

    pub fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for DirFd {
    fn drop(&mut self) {
        if self.owned && self.fd >= 0 {
            unsafe {
                libc::close(self.fd);
            }
        }
    }
}

impl AsRawFd for DirFd {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
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
            libc::S_IFDIR => FileType::Directory,
            libc::S_IFREG => FileType::RegularFile,
            libc::S_IFLNK => FileType::Symlink,
            _ => FileType::Other,
        }
    }

    pub fn is_directory(&self) -> bool {
        matches!(self, FileType::Directory)
    }

    pub fn is_regular_file(&self) -> bool {
        matches!(self, FileType::RegularFile)
    }

    pub fn is_symlink(&self) -> bool {
        matches!(self, FileType::Symlink)
    }
}

/// Metadata wrapper for safer access to file information
#[derive(Debug, Clone)]
pub struct Metadata {
    stat: libc::stat,
}

impl Metadata {
    pub fn from_stat(stat: libc::stat) -> Self {
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

    /// Get the raw libc::stat for compatibility with existing code
    pub fn as_raw_stat(&self) -> &libc::stat {
        &self.stat
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;
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
        let raw_fd = dir_fd.as_raw_fd();

        let borrowed_fd = DirFd::from_raw_fd(raw_fd).unwrap();
        assert_eq!(borrowed_fd.as_raw_fd(), raw_fd);
        assert!(!borrowed_fd.owned); // Should not own the FD
    }

    #[test]
    fn test_from_raw_fd_invalid() {
        let result = DirFd::from_raw_fd(-1);
        assert!(result.is_err());
    }

    #[test]
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

        // Test raw stat access
        let raw_stat = metadata.as_raw_stat();
        assert_eq!(raw_stat.st_size, metadata.size() as i64);
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
