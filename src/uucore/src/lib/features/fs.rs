// This file is part of the uutils coreutils package.
//
// (c) Joseph Crail <jbcrail@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Set of functions to manage files and symlinks

#[cfg(unix)]
use libc::{
    mode_t, S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFREG, S_IFSOCK, S_IRGRP,
    S_IROTH, S_IRUSR, S_ISGID, S_ISUID, S_ISVTX, S_IWGRP, S_IWOTH, S_IWUSR, S_IXGRP, S_IXOTH,
    S_IXUSR,
};
use std::borrow::Cow;
use std::env;
use std::fs;
use std::hash::Hash;
use std::io::Error as IOError;
use std::io::Result as IOResult;
use std::io::{Error, ErrorKind};
#[cfg(unix)]
use std::os::unix::{fs::MetadataExt, io::AsRawFd};
use std::path::{Component, Path, PathBuf};
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
    pub fn from_file(file: &impl AsRawFd) -> Option<Self> {
        if let Ok(x) = nix::sys::stat::fstat(file.as_raw_fd()) {
            Some(Self(x))
        } else {
            None
        }
    }

    /// Get information from a currently open file
    #[cfg(target_os = "windows")]
    pub fn from_file(file: &impl AsHandleRef) -> Option<Self> {
        if let Ok(x) = winapi_util::file::information(file.as_handle_ref()) {
            Some(Self(x))
        } else {
            None
        }
    }

    /// Get information for a given path.
    ///
    /// If `path` points to a symlink and `dereference` is true, information about
    /// the link's target will be returned.
    pub fn from_path(path: impl AsRef<Path>, dereference: bool) -> Option<Self> {
        #[cfg(unix)]
        {
            let stat = if dereference {
                nix::sys::stat::stat(path.as_ref())
            } else {
                nix::sys::stat::lstat(path.as_ref())
            };
            if let Ok(stat) = stat {
                Some(Self(stat))
            } else {
                None
            }
        }
        #[cfg(target_os = "windows")]
        {
            use std::fs::OpenOptions;
            use std::os::windows::prelude::*;
            let mut open_options = OpenOptions::new();
            if !dereference {
                open_options.custom_flags(winapi::um::winbase::FILE_FLAG_OPEN_REPARSE_POINT);
            }
            open_options
                .read(true)
                .open(path.as_ref())
                .ok()
                .and_then(|file| Self::from_file(&file))
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

// copied from https://github.com/rust-lang/cargo/blob/2e4cfc2b7d43328b207879228a2ca7d427d188bb/src/cargo/util/paths.rs#L65-L90
// both projects are MIT https://github.com/rust-lang/cargo/blob/master/LICENSE-MIT
// for std impl progress see rfc https://github.com/rust-lang/rfcs/issues/2208
// replace this once that lands
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

fn resolve<P: AsRef<Path>>(original: P) -> Result<PathBuf, (bool, PathBuf, IOError)> {
    const MAX_LINKS_FOLLOWED: u32 = 255;
    let mut followed = 0;
    let mut result = original.as_ref().to_path_buf();
    let mut symlink_is_absolute = false;
    let mut first_resolution = None;

    loop {
        if followed == MAX_LINKS_FOLLOWED {
            return Err((
                symlink_is_absolute,
                // When we hit MAX_LINKS_FOLLOWED we should return the first resolution (that's what GNU does - for whatever reason)
                first_resolution.unwrap(),
                Error::new(ErrorKind::InvalidInput, "maximum links followed"),
            ));
        }

        match fs::symlink_metadata(&result) {
            Ok(meta) => {
                if !meta.file_type().is_symlink() {
                    break;
                }
            }
            Err(e) => return Err((symlink_is_absolute, result, e)),
        }

        followed += 1;
        match fs::read_link(&result) {
            Ok(path) => {
                result.pop();
                symlink_is_absolute = path.is_absolute();
                result.push(path);
            }
            Err(e) => return Err((symlink_is_absolute, result, e)),
        }

        if first_resolution.is_none() {
            first_resolution = Some(result.clone());
        }
    }
    Ok(result)
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
    // Create an absolute path
    let original = original.as_ref();
    let original = if original.is_absolute() {
        original.to_path_buf()
    } else {
        let current_dir = env::current_dir()?;
        dunce::canonicalize(current_dir)?.join(original)
    };

    let mut result = PathBuf::new();
    let mut parts = vec![];

    // Split path by directory separator; add prefix (Windows-only) and root
    // directory to final path buffer; add remaining parts to temporary
    // vector for canonicalization.
    for part in original.components() {
        match part {
            Component::Prefix(_) | Component::RootDir => {
                result.push(part.as_os_str());
            }
            Component::CurDir => (),
            Component::ParentDir => {
                if res_mode == ResolveMode::Logical {
                    parts.pop();
                } else {
                    parts.push(part.as_os_str());
                }
            }
            Component::Normal(_) => {
                parts.push(part.as_os_str());
            }
        }
    }

    // Resolve the symlinks where possible
    if !parts.is_empty() {
        for part in parts[..parts.len() - 1].iter() {
            result.push(part);

            //resolve as we go to handle long relative paths on windows
            if res_mode == ResolveMode::Physical {
                result = normalize_path(&result);
            }

            if res_mode == ResolveMode::None {
                continue;
            }

            match resolve(&result) {
                Err((_, path, e)) => {
                    if miss_mode == MissingHandling::Missing {
                        result = path;
                    } else {
                        return Err(e);
                    }
                }
                Ok(path) => {
                    result = path;
                }
            }
        }

        result.push(parts.last().unwrap());

        if res_mode == ResolveMode::None {
            return Ok(result);
        }

        match resolve(&result) {
            Err((is_absolute, path, err)) => {
                // If the resolved symlink is an absolute path and non-existent,
                // `realpath` throws no such file error.
                if miss_mode == MissingHandling::Existing
                    || (err.kind() == ErrorKind::NotFound
                        && is_absolute
                        && miss_mode == MissingHandling::Normal)
                {
                    return Err(err);
                } else {
                    result = path;
                }
            }
            Ok(path) => {
                result = path;
            }
        }
        if res_mode == ResolveMode::Physical {
            result = normalize_path(&result);
        }
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
pub fn display_permissions(metadata: &fs::Metadata, display_file_type: bool) -> String {
    let mode: mode_t = metadata.mode() as mode_t;
    display_permissions_unix(mode, display_file_type)
}

#[cfg(unix)]
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

// For some programs like install or mkdir, dir/. can be provided
// Special case to match GNU's behavior:
// install -d foo/. should work and just create foo/
// std::fs::create_dir("foo/."); fails in pure Rust
// See also mkdir.rs for another occurrence of this
pub fn dir_strip_dot_for_creation(path: &Path) -> PathBuf {
    if path.to_string_lossy().ends_with("/.") {
        // Do a simple dance to strip the "/."
        Path::new(&path).components().collect::<PathBuf>()
    } else {
        path.to_path_buf()
    }
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
