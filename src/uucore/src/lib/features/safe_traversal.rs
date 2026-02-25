// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// Safe directory traversal using openat() and related syscalls
// This module provides TOCTOU-safe filesystem operations for recursive traversal
//
// Available on Unix
//
// spell-checker:ignore CLOEXEC RDONLY TOCTOU closedir dirp fdopendir fstatat openat REMOVEDIR unlinkat smallfile
// spell-checker:ignore RAII dirfd fchownat fchown FchmodatFlags fchmodat fchmod mkdirat CREAT WRONLY ELOOP ENOTDIR

#[cfg(test)]
use std::os::unix::ffi::OsStringExt;

use std::ffi::{CString, OsStr, OsString};
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use std::path::{Path, PathBuf};

use nix::dir::Dir;
use nix::fcntl::{OFlag, openat};
use nix::libc;
use nix::sys::stat::{FchmodatFlags, FileStat, Mode, fchmodat, fstatat, mkdirat};
use nix::unistd::{Gid, Uid, UnlinkatFlags, fchown, fchownat, unlinkat};
use os_display::Quotable;

use crate::translate;

/// Enum to specify symlink following behavior.
///
/// This replaces boolean `follow_symlinks` parameters for better readability
/// at call sites. Instead of `open(path, true)`, use `open(path, SymlinkBehavior::Follow)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SymlinkBehavior {
    /// Follow symlinks (resolve to their target)
    #[default]
    Follow,
    /// Do not follow symlinks (operate on the symlink itself)
    NoFollow,
}

impl SymlinkBehavior {
    /// Returns `true` if symlinks should be followed
    #[inline]
    pub fn should_follow(self) -> bool {
        matches!(self, Self::Follow)
    }
}

impl From<bool> for SymlinkBehavior {
    fn from(follow: bool) -> Self {
        if follow { Self::Follow } else { Self::NoFollow }
    }
}

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
    ///
    /// # Arguments
    /// * `path` - The path to the directory to open
    /// * `symlink_behavior` - Whether to follow symlinks when opening
    pub fn open(path: &Path, symlink_behavior: SymlinkBehavior) -> io::Result<Self> {
        let mut flags = OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC;
        if !symlink_behavior.should_follow() {
            flags |= OFlag::O_NOFOLLOW;
        }
        let fd = nix::fcntl::open(path, flags, Mode::empty()).map_err(|e| {
            SafeTraversalError::OpenFailed {
                path: path.into(),
                source: io::Error::from_raw_os_error(e as i32),
            }
        })?;
        Ok(Self { fd })
    }

    /// Open a subdirectory relative to this directory
    ///
    /// # Arguments
    /// * `name` - The name of the subdirectory to open
    /// * `symlink_behavior` - Whether to follow symlinks when opening
    pub fn open_subdir(&self, name: &OsStr, symlink_behavior: SymlinkBehavior) -> io::Result<Self> {
        let name_cstr =
            CString::new(name.as_bytes()).map_err(|_| SafeTraversalError::PathContainsNull)?;
        let mut flags = OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC;
        if !symlink_behavior.should_follow() {
            flags |= OFlag::O_NOFOLLOW;
        }
        let fd = openat(&self.fd, name_cstr.as_c_str(), flags, Mode::empty()).map_err(|e| {
            SafeTraversalError::OpenFailed {
                path: name.into(),
                source: io::Error::from_raw_os_error(e as i32),
            }
        })?;
        Ok(Self { fd })
    }

    /// Get raw stat data for a file relative to this directory
    pub fn stat_at(&self, name: &OsStr, symlink_behavior: SymlinkBehavior) -> io::Result<FileStat> {
        let name_cstr =
            CString::new(name.as_bytes()).map_err(|_| SafeTraversalError::PathContainsNull)?;

        let flags = if symlink_behavior.should_follow() {
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
    pub fn metadata_at(
        &self,
        name: &OsStr,
        symlink_behavior: SymlinkBehavior,
    ) -> io::Result<Metadata> {
        self.stat_at(name, symlink_behavior)
            .map(Metadata::from_stat)
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
        symlink_behavior: SymlinkBehavior,
    ) -> io::Result<()> {
        let name_cstr =
            CString::new(name.as_bytes()).map_err(|_| SafeTraversalError::PathContainsNull)?;

        let flags = if symlink_behavior.should_follow() {
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
    pub fn chmod_at(
        &self,
        name: &OsStr,
        mode: u32,
        symlink_behavior: SymlinkBehavior,
    ) -> io::Result<()> {
        let flags = if symlink_behavior.should_follow() {
            FchmodatFlags::FollowSymlink
        } else {
            FchmodatFlags::NoFollowSymlink
        };

        let mode = Mode::from_bits_truncate(mode as libc::mode_t);

        let name_cstr =
            CString::new(name.as_bytes()).map_err(|_| SafeTraversalError::PathContainsNull)?;

        fchmodat(&self.fd, name_cstr.as_c_str(), mode, flags)
            .map_err(|e| io::Error::from_raw_os_error(e as i32))?;

        Ok(())
    }

    /// Change mode of this directory
    pub fn fchmod(&self, mode: u32) -> io::Result<()> {
        let mode = Mode::from_bits_truncate(mode as libc::mode_t);

        nix::sys::stat::fchmod(&self.fd, mode)
            .map_err(|e| io::Error::from_raw_os_error(e as i32))?;

        Ok(())
    }

    /// Create a directory relative to this directory
    pub fn mkdir_at(&self, name: &OsStr, mode: u32) -> io::Result<()> {
        let name_cstr =
            CString::new(name.as_bytes()).map_err(|_| SafeTraversalError::PathContainsNull)?;
        let mode = Mode::from_bits_truncate(mode as libc::mode_t);

        if let Err(e) = mkdirat(self.fd.as_fd(), name_cstr.as_c_str(), mode) {
            let err = io::Error::from_raw_os_error(e as i32);
            return Err(SafeTraversalError::OpenFailed {
                path: name.into(),
                source: err,
            }
            .into());
        }
        Ok(())
    }

    /// Open a file for writing relative to this directory
    /// Creates the file if it doesn't exist, truncates if it does
    pub fn open_file_at(&self, name: &OsStr) -> io::Result<fs::File> {
        let name_cstr =
            CString::new(name.as_bytes()).map_err(|_| SafeTraversalError::PathContainsNull)?;
        let flags = OFlag::O_CREAT | OFlag::O_WRONLY | OFlag::O_TRUNC | OFlag::O_CLOEXEC;
        let mode = Mode::from_bits_truncate(0o666); // Default file permissions

        let fd: OwnedFd = openat(self.fd.as_fd(), name_cstr.as_c_str(), flags, mode)
            .map_err(|e| io::Error::from_raw_os_error(e as i32))?;

        // Convert OwnedFd to raw fd and create File
        let raw_fd = fd.into_raw_fd();
        Ok(unsafe { fs::File::from_raw_fd(raw_fd) })
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

/// Find the deepest existing real directory ancestor for a path.
///
/// Returns the existing ancestor path and a list of components that need to be created.
/// Uses `symlink_metadata` to detect symlinks - symlinks are NOT followed and are
/// treated as components that need to be created/replaced.
fn find_existing_ancestor(path: &Path) -> io::Result<(PathBuf, Vec<OsString>)> {
    let mut current = path.to_path_buf();
    let mut components: Vec<OsString> = Vec::new();

    loop {
        // Use symlink_metadata to NOT follow symlinks
        match fs::symlink_metadata(&current) {
            Ok(meta) => {
                if meta.is_dir() && !meta.file_type().is_symlink() {
                    // Found a real directory (not a symlink to a directory)
                    components.reverse();
                    return Ok((current, components));
                }
                // It's a symlink, file, or other non-directory - treat as needing creation
                // This ensures symlinks get replaced by open_or_create_subdir
                if let Some(file_name) = current.file_name() {
                    components.push(file_name.to_os_string());
                }
                if let Some(parent) = current.parent() {
                    if parent.as_os_str().is_empty() {
                        // Reached empty parent (for relative paths), use "."
                        components.reverse();
                        return Ok((PathBuf::from("."), components));
                    }
                    current = parent.to_path_buf();
                } else {
                    // Reached filesystem root
                    let root = if path.is_absolute() {
                        PathBuf::from("/")
                    } else {
                        PathBuf::from(".")
                    };
                    components.reverse();
                    return Ok((root, components));
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                // Doesn't exist, record component and move up to parent
                if let Some(file_name) = current.file_name() {
                    components.push(file_name.to_os_string());
                }
                if let Some(parent) = current.parent() {
                    if parent.as_os_str().is_empty() {
                        // Reached empty parent (for relative paths), use "."
                        components.reverse();
                        return Ok((PathBuf::from("."), components));
                    }
                    current = parent.to_path_buf();
                } else {
                    // Reached filesystem root
                    let root = if path.is_absolute() {
                        PathBuf::from("/")
                    } else {
                        PathBuf::from(".")
                    };
                    components.reverse();
                    return Ok((root, components));
                }
            }
            Err(e) => return Err(e),
        }
    }
}

/// Open or create a subdirectory using fd-based operations only.
///
/// This is a helper function for `create_dir_all_safe` that handles a single
/// path component. If a symlink exists where a directory should be, it is
/// removed and replaced with a real directory.
///
/// # Arguments
/// * `parent_fd` - The parent directory file descriptor
/// * `name` - The name of the subdirectory to open or create
/// * `mode` - The mode to use when creating a new directory
///
/// # Returns
/// A DirFd for the subdirectory
fn open_or_create_subdir(parent_fd: &DirFd, name: &OsStr, mode: u32) -> io::Result<DirFd> {
    match parent_fd.stat_at(name, SymlinkBehavior::NoFollow) {
        Ok(stat) => {
            let file_type = (stat.st_mode as libc::mode_t) & libc::S_IFMT;
            match file_type {
                libc::S_IFDIR => parent_fd.open_subdir(name, SymlinkBehavior::NoFollow),
                libc::S_IFLNK => {
                    parent_fd.unlink_at(name, false)?;
                    parent_fd.mkdir_at(name, mode)?;
                    parent_fd.open_subdir(name, SymlinkBehavior::NoFollow)
                }
                _ => Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!(
                        "path component exists but is not a directory: {}",
                        name.display()
                    ),
                )),
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            parent_fd.mkdir_at(name, mode)?;
            parent_fd.open_subdir(name, SymlinkBehavior::NoFollow)
        }
        Err(e) => Err(e),
    }
}

/// Safely create all parent directories for a path using directory file descriptors.
/// This prevents symlink race conditions by anchoring all operations to directory fds.
///
/// # Security
/// This function prevents TOCTOU race conditions by:
/// 1. Finding the deepest existing ancestor directory (path-based, but safe since it exists)
/// 2. Opening that ancestor with a file descriptor
/// 3. Creating all new directories using fd-based operations (mkdirat, openat with O_NOFOLLOW)
///
/// Once we have a fd for an existing ancestor, all subsequent operations use that fd
/// as the anchor. If an attacker replaces a newly-created directory with a symlink,
/// our openat with O_NOFOLLOW will fail, preventing the attack.
///
/// Existing symlinks in the path (like /var -> /private/var on macOS) are followed
/// when finding the ancestor, which is safe since they already exist.
///
/// # Arguments
/// * `path` - The path to create directories for
/// * `mode` - The mode to use when creating new directories (e.g., 0o755). The actual
///   mode will be modified by the process umask.
///
/// # Returns
/// A DirFd for the final created directory, or the first existing parent if
/// all directories already exist.
#[cfg(unix)]
pub fn create_dir_all_safe(path: &Path, mode: u32) -> io::Result<DirFd> {
    let (existing_ancestor, components_to_create) = find_existing_ancestor(path)?;
    let mut dir_fd = DirFd::open(&existing_ancestor, SymlinkBehavior::Follow)?;

    for component in &components_to_create {
        dir_fd = open_or_create_subdir(&dir_fd, component.as_os_str(), mode)?;
    }

    Ok(dir_fd)
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

    pub fn is_directory(self) -> bool {
        matches!(self, Self::Directory)
    }

    pub fn is_regular_file(self) -> bool {
        matches!(self, Self::RegularFile)
    }

    pub fn is_symlink(self) -> bool {
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
        FileType::from_mode(self.stat.st_mode as libc::mode_t)
    }

    pub fn file_info(&self) -> FileInfo {
        FileInfo::from_stat(&self.stat)
    }

    // st_size type varies by platform (i64 vs u64)
    #[allow(clippy::unnecessary_cast)]
    pub fn size(&self) -> u64 {
        self.stat.st_size as u64
    }

    // st_mode type varies by platform (u16 on macOS, u32 on Linux)
    #[allow(clippy::unnecessary_cast)]
    pub fn mode(&self) -> u32 {
        self.stat.st_mode as u32
    }

    pub fn nlink(&self) -> u64 {
        // st_nlink type varies by platform (u16 on FreeBSD, u32/u64 on others)
        #[allow(clippy::unnecessary_cast)]
        {
            self.stat.st_nlink as u64
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
    // st_dev type varies by platform (i32 on macOS, u64 on Linux)
    #[allow(clippy::unnecessary_cast)]
    fn dev(&self) -> u64 {
        self.stat.st_dev as u64
    }

    fn ino(&self) -> u64 {
        // st_ino type varies by platform (u32 on FreeBSD, u64 on Linux)
        #[allow(clippy::unnecessary_cast)]
        {
            self.stat.st_ino as u64
        }
    }

    // st_mode type varies by platform (u16 on macOS, u32 on Linux)
    #[allow(clippy::unnecessary_cast)]
    fn mode(&self) -> u32 {
        self.stat.st_mode as u32
    }

    fn nlink(&self) -> u64 {
        // st_nlink type varies by platform (u16 on FreeBSD, u32/u64 on others)
        #[allow(clippy::unnecessary_cast)]
        {
            self.stat.st_nlink as u64
        }
    }

    fn uid(&self) -> u32 {
        self.stat.st_uid
    }

    fn gid(&self) -> u32 {
        self.stat.st_gid
    }

    // st_rdev type varies by platform (i32 on macOS, u64 on Linux)
    #[allow(clippy::unnecessary_cast)]
    fn rdev(&self) -> u64 {
        self.stat.st_rdev as u64
    }

    // st_size type varies by platform (i64 on some platforms, u64 on others)
    #[allow(clippy::unnecessary_cast)]
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

    // st_blksize type varies by platform (i32/i64/u32/u64 depending on platform)
    #[allow(clippy::unnecessary_cast)]
    fn blksize(&self) -> u64 {
        self.stat.st_blksize as u64
    }

    // st_blocks type varies by platform (i64 on some platforms, u64 on others)
    #[allow(clippy::unnecessary_cast)]
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
        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();
        assert!(dir_fd.as_raw_fd() >= 0);
    }

    #[test]
    fn test_dirfd_open_nonexistent_directory() {
        let result = DirFd::open("/nonexistent/path".as_ref(), SymlinkBehavior::Follow);
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

        let result = DirFd::open(&file_path, SymlinkBehavior::Follow);
        assert!(result.is_err());
    }

    #[test]
    fn test_dirfd_open_subdir() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let parent_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();
        let subdir_fd = parent_fd
            .open_subdir(OsStr::new("subdir"), SymlinkBehavior::Follow)
            .unwrap();
        assert!(subdir_fd.as_raw_fd() >= 0);
    }

    #[test]
    fn test_dirfd_open_nonexistent_subdir() {
        let temp_dir = TempDir::new().unwrap();
        let parent_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();

        let result = parent_fd.open_subdir(OsStr::new("nonexistent"), SymlinkBehavior::Follow);
        assert!(result.is_err());
    }

    #[test]
    fn test_dirfd_stat_at() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file");
        fs::write(&file_path, "test content").unwrap();

        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();
        let stat = dir_fd
            .stat_at(OsStr::new("test_file"), SymlinkBehavior::Follow)
            .unwrap();

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

        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();

        // Follow symlinks
        let stat_follow = dir_fd
            .stat_at(OsStr::new("link"), SymlinkBehavior::Follow)
            .unwrap();
        assert_eq!(stat_follow.st_mode & libc::S_IFMT, libc::S_IFREG);

        // Don't follow symlinks
        let stat_nofollow = dir_fd
            .stat_at(OsStr::new("link"), SymlinkBehavior::NoFollow)
            .unwrap();
        assert_eq!(stat_nofollow.st_mode & libc::S_IFMT, libc::S_IFLNK);
    }

    #[test]
    fn test_dirfd_fstat() {
        let temp_dir = TempDir::new().unwrap();
        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();
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

        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();
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

        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();
        dir_fd.unlink_at(OsStr::new("test_file"), false).unwrap();

        assert!(!file_path.exists());
    }

    #[test]
    fn test_dirfd_unlink_at_directory() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("empty_dir");
        fs::create_dir(&subdir).unwrap();

        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();
        dir_fd.unlink_at(OsStr::new("empty_dir"), true).unwrap();

        assert!(!subdir.exists());
    }

    #[test]
    fn test_from_raw_fd() {
        let temp_dir = TempDir::new().unwrap();
        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();

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

        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();
        let stat = dir_fd
            .stat_at(OsStr::new("test_file"), SymlinkBehavior::Follow)
            .unwrap();
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

        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();
        let metadata = dir_fd
            .metadata_at(OsStr::new("test_file"), SymlinkBehavior::Follow)
            .unwrap();

        assert_eq!(metadata.file_type(), FileType::RegularFile);
        assert!(metadata.size() > 0);
        assert_eq!(metadata.mode() & libc::S_IFMT as u32, libc::S_IFREG as u32);
        assert_eq!(metadata.nlink(), 1);

        assert!(metadata.size() > 0);
    }

    #[test]
    fn test_metadata_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();
        let metadata = dir_fd.metadata().unwrap();

        assert_eq!(metadata.file_type(), FileType::Directory);
        assert!(metadata.file_type().is_directory());
    }

    #[test]
    fn test_path_with_null_byte() {
        let path_with_null = OsString::from_vec(b"test\0file".to_vec());
        let temp_dir = TempDir::new().unwrap();
        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();

        let result = dir_fd.open_subdir(&path_with_null, SymlinkBehavior::Follow);
        assert!(result.is_err());
        if let Err(e) = result {
            // Should be InvalidInput for null byte error
            assert_eq!(e.kind(), io::ErrorKind::InvalidInput);
        }
    }

    #[test]
    fn test_error_chain() {
        let result = DirFd::open(
            "/nonexistent/deeply/nested/path".as_ref(),
            SymlinkBehavior::Follow,
        );
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

    #[test]
    fn test_mkdir_at_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();

        dir_fd.mkdir_at(OsStr::new("new_subdir"), 0o755).unwrap();

        assert!(temp_dir.path().join("new_subdir").is_dir());
    }

    #[test]
    fn test_mkdir_at_fails_if_exists() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("existing");
        fs::create_dir(&subdir).unwrap();

        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();
        let result = dir_fd.mkdir_at(OsStr::new("existing"), 0o755);

        assert!(result.is_err());
    }

    #[test]
    fn test_open_file_at_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();

        let mut file = dir_fd.open_file_at(OsStr::new("new_file.txt")).unwrap();
        use std::io::Write;
        file.write_all(b"test content").unwrap();

        let content = fs::read_to_string(temp_dir.path().join("new_file.txt")).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_open_file_at_truncates_existing() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("existing.txt");
        fs::write(&file_path, "old content that is longer").unwrap();

        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();
        let mut file = dir_fd.open_file_at(OsStr::new("existing.txt")).unwrap();
        use std::io::Write;
        file.write_all(b"new").unwrap();
        drop(file);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "new");
    }

    #[test]
    fn test_create_dir_all_safe_creates_nested_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("a/b/c");

        let dir_fd = create_dir_all_safe(&nested_path, 0o755).unwrap();
        assert!(dir_fd.as_raw_fd() >= 0);
        assert!(nested_path.is_dir());
    }

    #[test]
    fn test_create_dir_all_safe_existing_path() {
        let temp_dir = TempDir::new().unwrap();
        let existing_path = temp_dir.path().join("existing");
        fs::create_dir(&existing_path).unwrap();

        let dir_fd = create_dir_all_safe(&existing_path, 0o755).unwrap();
        assert!(dir_fd.as_raw_fd() >= 0);
    }

    #[test]
    fn test_create_dir_all_safe_replaces_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();

        // Create a symlink where we want to create a directory
        let symlink_path = temp_dir.path().join("link_to_replace");
        symlink(&target_dir, &symlink_path).unwrap();
        assert!(symlink_path.is_symlink());

        // create_dir_all_safe should replace the symlink with a real directory
        let dir_fd = create_dir_all_safe(&symlink_path, 0o755).unwrap();
        assert!(dir_fd.as_raw_fd() >= 0);

        // Verify the symlink was replaced with a real directory
        assert!(symlink_path.is_dir());
        assert!(!symlink_path.is_symlink());
    }

    #[test]
    fn test_create_dir_all_safe_fails_on_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file");
        fs::write(&file_path, "content").unwrap();

        let result = create_dir_all_safe(&file_path, 0o755);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_dir_all_safe_nested_symlink_in_path() {
        let temp_dir = TempDir::new().unwrap();

        // Create: parent/symlink -> target
        // Then try to create: parent/symlink/subdir
        let parent = temp_dir.path().join("parent");
        let target = temp_dir.path().join("target");
        fs::create_dir(&parent).unwrap();
        fs::create_dir(&target).unwrap();

        let symlink_in_path = parent.join("link");
        symlink(&target, &symlink_in_path).unwrap();

        // Try to create parent/link/subdir - the symlink should be replaced
        let nested_path = symlink_in_path.join("subdir");
        let dir_fd = create_dir_all_safe(&nested_path, 0o755).unwrap();
        assert!(dir_fd.as_raw_fd() >= 0);

        // The symlink should have been replaced with a real directory
        assert!(!symlink_in_path.is_symlink());
        assert!(symlink_in_path.is_dir());
        assert!(nested_path.is_dir());

        // Target directory should not contain subdir (race attack prevented)
        assert!(!target.join("subdir").exists());
    }

    #[test]
    fn test_open_subdir_nofollow_fails_on_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target");
        fs::create_dir(&target).unwrap();

        let link = temp_dir.path().join("link");
        symlink(&target, &link).unwrap();

        let dir_fd = DirFd::open(temp_dir.path(), SymlinkBehavior::Follow).unwrap();

        // With follow_symlinks=true, should succeed
        let result_follow = dir_fd.open_subdir(OsStr::new("link"), SymlinkBehavior::Follow);
        assert!(result_follow.is_ok());

        // With follow_symlinks=false, should fail (ELOOP or ENOTDIR)
        let result_nofollow = dir_fd.open_subdir(OsStr::new("link"), SymlinkBehavior::NoFollow);
        assert!(result_nofollow.is_err());
    }

    #[test]
    fn test_open_nofollow_fails_on_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target");
        fs::create_dir(&target).unwrap();

        let link = temp_dir.path().join("link");
        symlink(&target, &link).unwrap();

        // With follow_symlinks=true, should succeed
        let result_follow = DirFd::open(&link, SymlinkBehavior::Follow);
        assert!(result_follow.is_ok());

        // With follow_symlinks=false, should fail
        let result_nofollow = DirFd::open(&link, SymlinkBehavior::NoFollow);
        assert!(result_nofollow.is_err());
    }
}
