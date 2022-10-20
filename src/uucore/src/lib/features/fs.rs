// This file is part of the uutils coreutils package.
//
// (c) Joseph Crail <jbcrail@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Set of functions to manage files and symlinks

// spell-checker:ignore backport

#[cfg(unix)]
use libc::{
    mode_t, S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFREG, S_IFSOCK, S_IRGRP,
    S_IROTH, S_IRUSR, S_ISGID, S_ISUID, S_ISVTX, S_IWGRP, S_IWOTH, S_IWUSR, S_IXGRP, S_IXOTH,
    S_IXUSR,
};
use std::borrow::Cow;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::fs::read_dir;
use std::hash::Hash;
use std::io::{Error, ErrorKind, Result as IOResult};
#[cfg(unix)]
use std::os::unix::{fs::MetadataExt, io::AsRawFd};
use std::path::{Component, Path, PathBuf, MAIN_SEPARATOR};
#[cfg(target_os = "windows")]
use winapi_util::AsHandleRef;

#[cfg(unix)]
#[macro_export]
macro_rules! has {
    ($mode:expr, $perm:expr) => {
        $mode & $perm != 0
    };
}

/// Information to uniquely identify a file
pub struct FileInformation(
    #[cfg(unix)] nix::sys::stat::FileStat,
    #[cfg(windows)] winapi_util::file::Information,
);

impl FileInformation {
    /// Get information from a currently open file
    #[cfg(unix)]
    pub fn from_file(file: &impl AsRawFd) -> IOResult<Self> {
        let stat = nix::sys::stat::fstat(file.as_raw_fd())?;
        Ok(Self(stat))
    }

    /// Get information from a currently open file
    #[cfg(target_os = "windows")]
    pub fn from_file(file: &impl AsHandleRef) -> IOResult<Self> {
        let info = winapi_util::file::information(file.as_handle_ref())?;
        Ok(Self(info))
    }

    /// Get information for a given path.
    ///
    /// If `path` points to a symlink and `dereference` is true, information about
    /// the link's target will be returned.
    pub fn from_path(path: impl AsRef<Path>, dereference: bool) -> IOResult<Self> {
        #[cfg(unix)]
        {
            let stat = if dereference {
                nix::sys::stat::stat(path.as_ref())
            } else {
                nix::sys::stat::lstat(path.as_ref())
            };
            Ok(Self(stat?))
        }
        #[cfg(target_os = "windows")]
        {
            use std::fs::OpenOptions;
            use std::os::windows::prelude::*;
            let mut open_options = OpenOptions::new();
            let mut custom_flags = 0;
            if !dereference {
                custom_flags |=
                    windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
            }
            custom_flags |= windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS;
            open_options.custom_flags(custom_flags);
            let file = open_options.read(true).open(path.as_ref())?;
            Self::from_file(&file)
        }
    }

    pub fn file_size(&self) -> u64 {
        #[cfg(unix)]
        {
            assert!(self.0.st_size >= 0, "File size is negative");
            self.0.st_size.try_into().unwrap()
        }
        #[cfg(target_os = "windows")]
        {
            self.0.file_size()
        }
    }

    #[cfg(windows)]
    pub fn file_index(&self) -> u64 {
        self.0.file_index()
    }

    pub fn number_of_links(&self) -> u64 {
        #[cfg(unix)]
        return self.0.st_nlink as u64;
        #[cfg(windows)]
        return self.0.number_of_links() as u64;
    }

    #[cfg(unix)]
    pub fn inode(&self) -> u64 {
        self.0.st_ino as u64
    }
}

#[cfg(unix)]
impl PartialEq for FileInformation {
    fn eq(&self, other: &Self) -> bool {
        self.0.st_dev == other.0.st_dev && self.0.st_ino == other.0.st_ino
    }
}

#[cfg(target_os = "windows")]
impl PartialEq for FileInformation {
    fn eq(&self, other: &Self) -> bool {
        self.0.volume_serial_number() == other.0.volume_serial_number()
            && self.0.file_index() == other.0.file_index()
    }
}

impl Eq for FileInformation {}

impl Hash for FileInformation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        #[cfg(unix)]
        {
            self.0.st_dev.hash(state);
            self.0.st_ino.hash(state);
        }
        #[cfg(target_os = "windows")]
        {
            self.0.volume_serial_number().hash(state);
            self.0.file_index().hash(state);
        }
    }
}

/// resolve a relative path
pub fn resolve_relative_path(path: &Path) -> Cow<Path> {
    if path.components().all(|e| e != Component::ParentDir) {
        return path.into();
    }
    let root = Component::RootDir.as_os_str();
    let mut result = env::current_dir().unwrap_or_else(|_| PathBuf::from(root));
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                if let Ok(p) = result.read_link() {
                    result = p;
                }
                result.pop();
            }
            Component::CurDir => (),
            Component::RootDir | Component::Normal(_) | Component::Prefix(_) => {
                result.push(comp.as_os_str());
            }
        }
    }
    result.into()
}

/// Controls how symbolic links should be handled when canonicalizing a path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissingHandling {
    /// Return an error if any part of the path is missing.
    Normal,

    /// Resolve symbolic links, ignoring errors on the final component.
    Existing,

    /// Resolve symbolic links, ignoring errors on the non-final components.
    Missing,
}

/// Controls when symbolic links are resolved
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolveMode {
    /// Do not resolve any symbolic links.
    None,

    /// Resolve symlinks as encountered when processing the path
    Physical,

    /// Resolve '..' elements before symlinks
    Logical,
}

/// Normalize a path by removing relative information
/// For example, convert 'bar/../foo/bar.txt' => 'foo/bar.txt'
/// copied from `<https://github.com/rust-lang/cargo/blob/2e4cfc2b7d43328b207879228a2ca7d427d188bb/src/cargo/util/paths.rs#L65-L90>`
/// both projects are MIT `<https://github.com/rust-lang/cargo/blob/master/LICENSE-MIT>`
/// for std impl progress see rfc `<https://github.com/rust-lang/rfcs/issues/2208>`
/// replace this once that lands
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

fn resolve_symlink<P: AsRef<Path>>(path: P) -> IOResult<Option<PathBuf>> {
    let result = if fs::symlink_metadata(&path)?.file_type().is_symlink() {
        Some(fs::read_link(&path)?)
    } else {
        None
    };
    Ok(result)
}

enum OwningComponent {
    Prefix(OsString),
    RootDir,
    CurDir,
    ParentDir,
    Normal(OsString),
}

impl OwningComponent {
    fn as_os_str(&self) -> &OsStr {
        match self {
            Self::Prefix(s) => s.as_os_str(),
            Self::RootDir => Component::RootDir.as_os_str(),
            Self::CurDir => Component::CurDir.as_os_str(),
            Self::ParentDir => Component::ParentDir.as_os_str(),
            Self::Normal(s) => s.as_os_str(),
        }
    }
}

impl<'a> From<Component<'a>> for OwningComponent {
    fn from(comp: Component<'a>) -> Self {
        match comp {
            Component::Prefix(_) => Self::Prefix(comp.as_os_str().to_os_string()),
            Component::RootDir => Self::RootDir,
            Component::CurDir => Self::CurDir,
            Component::ParentDir => Self::ParentDir,
            Component::Normal(s) => Self::Normal(s.to_os_string()),
        }
    }
}

/// Return the canonical, absolute form of a path.
///
/// This function is a generalization of [`std::fs::canonicalize`] that
/// allows controlling how symbolic links are resolved and how to deal
/// with missing components. It returns the canonical, absolute form of
/// a path.
/// The `miss_mode` parameter controls how missing path elements are handled
///
/// * [`MissingHandling::Normal`] makes this function behave like
///   [`std::fs::canonicalize`], resolving symbolic links and returning
///   an error if the path does not exist.
/// * [`MissingHandling::Missing`] makes this function ignore non-final
///   components of the path that could not be resolved.
/// * [`MissingHandling::Existing`] makes this function return an error
///   if the final component of the path does not exist.
///
/// The `res_mode` parameter controls how symbolic links are
/// resolved:
///
/// * [`ResolveMode::None`] makes this function not try to resolve
///   any symbolic links.
/// * [`ResolveMode::Physical`] makes this function resolve symlinks as they
///   are encountered
/// * [`ResolveMode::Logical`] makes this function resolve '..' components
///   before symlinks
///
pub fn canonicalize<P: AsRef<Path>>(
    original: P,
    miss_mode: MissingHandling,
    res_mode: ResolveMode,
) -> IOResult<PathBuf> {
    const SYMLINKS_TO_LOOK_FOR_LOOPS: i32 = 20;
    let original = original.as_ref();
    let has_to_be_directory =
        (miss_mode == MissingHandling::Normal || miss_mode == MissingHandling::Existing) && {
            let path_str = original.to_string_lossy();
            path_str.ends_with(MAIN_SEPARATOR) || path_str.ends_with('/')
        };
    let original = if original.is_absolute() {
        original.to_path_buf()
    } else {
        let current_dir = env::current_dir()?;
        dunce::canonicalize(current_dir)?.join(original)
    };
    let path = if res_mode == ResolveMode::Logical {
        normalize_path(&original)
    } else {
        original
    };
    let mut parts: VecDeque<OwningComponent> = path.components().map(|part| part.into()).collect();
    let mut result = PathBuf::new();
    let mut followed_symlinks = 0;
    let mut visited_files = HashSet::new();
    while let Some(part) = parts.pop_front() {
        match part {
            OwningComponent::Prefix(s) => {
                result.push(s);
                continue;
            }
            OwningComponent::RootDir | OwningComponent::Normal(..) => {
                result.push(part.as_os_str());
            }
            OwningComponent::CurDir => {}
            OwningComponent::ParentDir => {
                result.pop();
            }
        }
        if res_mode == ResolveMode::None {
            continue;
        }
        match resolve_symlink(&result) {
            Ok(Some(link_path)) => {
                for link_part in link_path.components().rev() {
                    parts.push_front(link_part.into());
                }
                if followed_symlinks < SYMLINKS_TO_LOOK_FOR_LOOPS {
                    followed_symlinks += 1;
                } else {
                    let file_info =
                        FileInformation::from_path(result.parent().unwrap(), false).unwrap();
                    let mut path_to_follow = PathBuf::new();
                    for part in &parts {
                        path_to_follow.push(part.as_os_str());
                    }
                    if !visited_files.insert((file_info, path_to_follow)) {
                        return Err(Error::new(
                            ErrorKind::InvalidInput,
                            "Too many levels of symbolic links",
                        )); // TODO use ErrorKind::FilesystemLoop when stable
                    }
                }
                result.pop();
            }
            Err(e) => {
                if miss_mode == MissingHandling::Existing
                    || (miss_mode == MissingHandling::Normal && !parts.is_empty())
                {
                    return Err(e);
                }
            }
            _ => {}
        }
    }
    // raise Not a directory if required
    match miss_mode {
        MissingHandling::Existing => {
            if has_to_be_directory {
                read_dir(&result)?;
            }
        }
        MissingHandling::Normal => {
            if result.exists() {
                if has_to_be_directory {
                    read_dir(&result)?;
                }
            } else if let Some(parent) = result.parent() {
                read_dir(parent)?;
            }
        }
        _ => {}
    }
    Ok(result)
}

#[cfg(not(unix))]
#[allow(unused_variables)]
pub fn display_permissions(metadata: &fs::Metadata, display_file_type: bool) -> String {
    if display_file_type {
        return String::from("----------");
    }
    String::from("---------")
}

#[cfg(unix)]
/// Display the permissions of a file
/// On non unix like system, just show '----------'
pub fn display_permissions(metadata: &fs::Metadata, display_file_type: bool) -> String {
    let mode: mode_t = metadata.mode() as mode_t;
    display_permissions_unix(mode, display_file_type)
}

#[cfg(unix)]
/// Display the permissions of a file on a unix like system
pub fn display_permissions_unix(mode: mode_t, display_file_type: bool) -> String {
    let mut result;
    if display_file_type {
        result = String::with_capacity(10);
        result.push(match mode & S_IFMT {
            S_IFDIR => 'd',
            S_IFCHR => 'c',
            S_IFBLK => 'b',
            S_IFREG => '-',
            S_IFIFO => 'p',
            S_IFLNK => 'l',
            S_IFSOCK => 's',
            // TODO: Other file types
            _ => '?',
        });
    } else {
        result = String::with_capacity(9);
    }

    result.push(if has!(mode, S_IRUSR) { 'r' } else { '-' });
    result.push(if has!(mode, S_IWUSR) { 'w' } else { '-' });
    result.push(if has!(mode, S_ISUID as mode_t) {
        if has!(mode, S_IXUSR) {
            's'
        } else {
            'S'
        }
    } else if has!(mode, S_IXUSR) {
        'x'
    } else {
        '-'
    });

    result.push(if has!(mode, S_IRGRP) { 'r' } else { '-' });
    result.push(if has!(mode, S_IWGRP) { 'w' } else { '-' });
    result.push(if has!(mode, S_ISGID as mode_t) {
        if has!(mode, S_IXGRP) {
            's'
        } else {
            'S'
        }
    } else if has!(mode, S_IXGRP) {
        'x'
    } else {
        '-'
    });

    result.push(if has!(mode, S_IROTH) { 'r' } else { '-' });
    result.push(if has!(mode, S_IWOTH) { 'w' } else { '-' });
    result.push(if has!(mode, S_ISVTX as mode_t) {
        if has!(mode, S_IXOTH) {
            't'
        } else {
            'T'
        }
    } else if has!(mode, S_IXOTH) {
        'x'
    } else {
        '-'
    });

    result
}

/// For some programs like install or mkdir, dir/. can be provided
/// Special case to match GNU's behavior:
/// install -d foo/. should work and just create foo/
/// std::fs::create_dir("foo/."); fails in pure Rust
pub fn dir_strip_dot_for_creation(path: &Path) -> PathBuf {
    if path.to_string_lossy().ends_with("/.") {
        // Do a simple dance to strip the "/."
        Path::new(&path).components().collect::<PathBuf>()
    } else {
        path.to_path_buf()
    }
}

/// Checks if `p1` and `p2` are the same file.
/// If error happens when trying to get files' metadata, returns false
pub fn paths_refer_to_same_file<P: AsRef<Path>>(p1: P, p2: P, dereference: bool) -> bool {
    infos_refer_to_same_file(
        FileInformation::from_path(p1, dereference),
        FileInformation::from_path(p2, dereference),
    )
}

/// Checks if `p1` and `p2` are the same file information.
/// If error happens when trying to get files' metadata, returns false
pub fn infos_refer_to_same_file(
    info1: IOResult<FileInformation>,
    info2: IOResult<FileInformation>,
) -> bool {
    if let Ok(info1) = info1 {
        if let Ok(info2) = info2 {
            return info1 == info2;
        }
    }
    false
}

/// Converts absolute `path` to be relative to absolute `to` path.
pub fn make_path_relative_to<P1: AsRef<Path>, P2: AsRef<Path>>(path: P1, to: P2) -> PathBuf {
    let path = path.as_ref();
    let to = to.as_ref();
    let common_prefix_size = path
        .components()
        .zip(to.components())
        .take_while(|(first, second)| first == second)
        .count();
    let path_suffix = path
        .components()
        .skip(common_prefix_size)
        .map(|x| x.as_os_str());
    let mut components: Vec<_> = to
        .components()
        .skip(common_prefix_size)
        .map(|_| Component::ParentDir.as_os_str())
        .chain(path_suffix)
        .collect();
    if components.is_empty() {
        components.push(Component::CurDir.as_os_str());
    }
    components.iter().collect()
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    struct NormalizePathTestCase<'a> {
        path: &'a str,
        test: &'a str,
    }

    const NORMALIZE_PATH_TESTS: [NormalizePathTestCase; 8] = [
        NormalizePathTestCase {
            path: "./foo/bar.txt",
            test: "foo/bar.txt",
        },
        NormalizePathTestCase {
            path: "bar/../foo/bar.txt",
            test: "foo/bar.txt",
        },
        NormalizePathTestCase {
            path: "foo///bar.txt",
            test: "foo/bar.txt",
        },
        NormalizePathTestCase {
            path: "foo///bar",
            test: "foo/bar",
        },
        NormalizePathTestCase {
            path: "foo//./bar",
            test: "foo/bar",
        },
        NormalizePathTestCase {
            path: "/foo//./bar",
            test: "/foo/bar",
        },
        NormalizePathTestCase {
            path: r"C:/you/later/",
            test: "C:/you/later",
        },
        NormalizePathTestCase {
            path: "\\networkShare/a//foo//./bar",
            test: "\\networkShare/a/foo/bar",
        },
    ];

    #[test]
    fn test_normalize_path() {
        for test in &NORMALIZE_PATH_TESTS {
            let path = Path::new(test.path);
            let normalized = normalize_path(path);
            assert_eq!(
                test.test
                    .replace('/', std::path::MAIN_SEPARATOR.to_string().as_str()),
                normalized.to_str().expect("Path is not valid utf-8!")
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_display_permissions() {
        // spell-checker:ignore (perms) brwsr drwxr rwxr
        assert_eq!(
            "drwxr-xr-x",
            display_permissions_unix(S_IFDIR | 0o755, true)
        );
        assert_eq!(
            "rwxr-xr-x",
            display_permissions_unix(S_IFDIR | 0o755, false)
        );
        assert_eq!(
            "-rw-r--r--",
            display_permissions_unix(S_IFREG | 0o644, true)
        );
        assert_eq!(
            "srw-r-----",
            display_permissions_unix(S_IFSOCK | 0o640, true)
        );
        assert_eq!(
            "lrw-r-xr-x",
            display_permissions_unix(S_IFLNK | 0o655, true)
        );
        assert_eq!("?rw-r-xr-x", display_permissions_unix(0o655, true));

        assert_eq!(
            "brwSr-xr-x",
            display_permissions_unix(S_IFBLK | S_ISUID as mode_t | 0o655, true)
        );
        assert_eq!(
            "brwsr-xr-x",
            display_permissions_unix(S_IFBLK | S_ISUID as mode_t | 0o755, true)
        );

        assert_eq!(
            "prw---sr--",
            display_permissions_unix(S_IFIFO | S_ISGID as mode_t | 0o614, true)
        );
        assert_eq!(
            "prw---Sr--",
            display_permissions_unix(S_IFIFO | S_ISGID as mode_t | 0o604, true)
        );

        assert_eq!(
            "c---r-xr-t",
            display_permissions_unix(S_IFCHR | S_ISVTX as mode_t | 0o055, true)
        );
        assert_eq!(
            "c---r-xr-T",
            display_permissions_unix(S_IFCHR | S_ISVTX as mode_t | 0o054, true)
        );
    }
}
