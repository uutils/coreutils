// This file is part of the uutils coreutils package.
//
// (c) Joseph Crail <jbcrail@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(unix)]
use libc::{
    mode_t, S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFREG, S_IFSOCK, S_IRGRP,
    S_IROTH, S_IRUSR, S_ISGID, S_ISUID, S_ISVTX, S_IWGRP, S_IWOTH, S_IWUSR, S_IXGRP, S_IXOTH,
    S_IXUSR,
};
use std::borrow::Cow;
use std::env;
use std::ffi::OsStr;
use std::fs;
#[cfg(target_os = "redox")]
use std::io;
use std::io::Result as IOResult;
use std::io::{Error, ErrorKind};
#[cfg(any(unix, target_os = "redox"))]
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
#[macro_export]
macro_rules! has {
    ($mode:expr, $perm:expr) => {
        $mode & $perm != 0
    };
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
                result.push(comp.as_os_str())
            }
        }
    }
    result.into()
}

/// Controls how symbolic links should be handled when canonicalizing a path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalizeMode {
    /// Do not resolve any symbolic links.
    None,

    /// Resolve all symbolic links.
    Normal,

    /// Resolve symbolic links, ignoring errors on the final component.
    Existing,

    /// Resolve symbolic links, ignoring errors on the non-final components.
    Missing,
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

fn resolve<P: AsRef<Path>>(original: P) -> IOResult<PathBuf> {
    const MAX_LINKS_FOLLOWED: u32 = 255;
    let mut followed = 0;
    let mut result = original.as_ref().to_path_buf();
    loop {
        if followed == MAX_LINKS_FOLLOWED {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "maximum links followed",
            ));
        }

        if !fs::symlink_metadata(&result)?.file_type().is_symlink() {
            break;
        }

        followed += 1;
        let path = fs::read_link(&result)?;
        result.pop();
        result.push(path);
    }
    Ok(result)
}

fn get_absolute(path: &Path) -> IOResult<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let current_dir = env::current_dir()?;
        let canonical_dir = dunce::canonicalize(current_dir)?;
        Ok(canonical_dir.join(path))
    }
}

fn get_parts(path: &Path) -> (Option<&OsStr>, Vec<&OsStr>) {
    let mut parts = vec![];

    // Split path by directory separator; add prefix (Windows-only) and root
    // directory to final path buffer; add remaining parts to temporary
    // vector for canonicalization.
    let mut prefix = None;
    for part in path.components() {
        match part {
            Component::Prefix(_) | Component::RootDir => prefix = Some(part.as_os_str()),
            Component::CurDir => (),
            Component::ParentDir => {
                parts.pop();
            }
            Component::Normal(_) => {
                parts.push(part.as_os_str());
            }
        }
    }
    (prefix, parts)
}

fn path_buf_append<P>(buf: &mut PathBuf, paths: &[P])
where
    P: AsRef<Path>,
{
    for path in paths {
        buf.push(path)
    }
}

/// Resolve all symbolic links in a path.
///
/// `result` is a mutable reference to a [`PathBuf`] that initially
/// contains the prefix of the path (for example, `/` on Unix, `C:\` on
/// Windows). It will be modified in-place by this function. It must be
/// an absolute path.
///
/// `parts` comprises the remaining components of the path that for
/// which we will resolve symbolic links, from left to right. After
/// visiting each element of `parts`, the path that results from
/// resolving the symbolic links will be assigned to `result`.
///
/// `mode` specifies the strategy to use to resolve links and/or handle
/// errors that occur.
fn resolve_all_links(
    result: &mut PathBuf,
    parts: Vec<&OsStr>,
    mode: CanonicalizeMode,
) -> IOResult<()> {
    // As a pre-condition of calling this function, `result` must be an
    // absolute path. In the code below, we will repeatedly add a
    // component to the end of the path and then call `resolve()`, which
    // resolves a symbolic link if the new path is one. Since
    // `resolve()` will also return an absolute path, we will *replace*
    // the value of `result` each time we `resolve()`.
    match mode {
        // Resolve no links.
        CanonicalizeMode::None => {
            path_buf_append(result, &parts);
        }
        // Resolve all links, ignoring all errors.
        CanonicalizeMode::Missing => {
            for part in parts {
                result.push(part);
                if let Ok(path) = resolve(&result) {
                    *result = path;
                }
            }
        }
        // Resolve all links, propagating all errors.
        CanonicalizeMode::Existing => {
            for part in parts {
                result.push(part);
                *result = resolve(&result)?;
            }
        }
        // Resolve all links, propagating errors on all but the last component.
        CanonicalizeMode::Normal => {
            let n = parts.len();
            for part in &parts[..n - 1] {
                result.push(part);
                *result = resolve(&result)?;
            }
            let p = parts.last().unwrap();
            result.push(p);
            if let Ok(path) = resolve(&result) {
                *result = path;
            }
        }
    }
    Ok(())
}

/// Return the canonical, absolute form of a path.
///
/// This function is a generalization of [`std::fs::canonicalize`] that
/// allows controlling how symbolic links are resolved and how to deal
/// with missing components. It returns the canonical, absolute form of
/// a path. The `can_mode` parameter controls how symbolic links are
/// resolved:
///
/// * [`CanonicalizeMode::Normal`] makes this function behave like
///   [`std::fs::canonicalize`], resolving symbolic links and returning
///   an error if the path does not exist.
/// * [`CanonicalizeMode::Missing`] makes this function ignore non-final
///   components of the path that could not be resolved.
/// * [`CanonicalizeMode::Existing`] makes this function return an error
///   if the final component of the path does not exist.
/// * [`CanonicalizeMode::None`] makes this function not try to resolve
///   any symbolic links.
///
pub fn canonicalize<P: AsRef<Path>>(original: P, can_mode: CanonicalizeMode) -> IOResult<PathBuf> {
    // Get the absolute path. For example, convert "a/b" into "/a/b".
    let absolute_path = get_absolute(original.as_ref())?;

    // Convert the absolute path into its components, resolving ".." entries.
    let (maybe_prefix, parts) = get_parts(&absolute_path);

    // If there is a prefix, insert it into the `PathBuf` as the first element.
    let mut result = PathBuf::new();
    if let Some(prefix) = maybe_prefix {
        result.push(prefix);
    }

    // If there were no other components in the path, then do nothing else.
    if parts.is_empty() {
        return Ok(result);
    }

    // Resolve all links in the path.
    //
    // This function modifies `result` in-place.
    resolve_all_links(&mut result, parts, can_mode)?;
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
        for test in NORMALIZE_PATH_TESTS.iter() {
            let path = Path::new(test.path);
            let normalized = normalize_path(path);
            assert_eq!(
                test.test
                    .replace("/", std::path::MAIN_SEPARATOR.to_string().as_str()),
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
