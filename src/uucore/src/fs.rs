// This file is part of the uutils coreutils package.
//
// (c) Joseph Crail <jbcrail@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(windows)]
extern crate dunce;
#[cfg(target_os = "redox")]
extern crate termion;

#[cfg(unix)]
use super::libc;
#[cfg(unix)]
use super::libc::{
    mode_t, S_IRGRP, S_IROTH, S_IRUSR, S_ISGID, S_ISUID, S_ISVTX, S_IWGRP, S_IWOTH, S_IWUSR,
    S_IXGRP, S_IXOTH, S_IXUSR,
};
use std::borrow::Cow;
use std::env;
use std::fs;
#[cfg(target_os = "redox")]
use std::io;
use std::io::Result as IOResult;
use std::io::{Error, ErrorKind};
#[cfg(any(unix, target_os = "redox"))]
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
macro_rules! has {
    ($mode:expr, $perm:expr) => {
        $mode & ($perm as u32) != 0
    };
}

pub fn resolve_relative_path(path: &Path) -> Cow<Path> {
    if path.components().all(|e| e != Component::ParentDir) {
        return path.into();
    }
    let root = Component::RootDir.as_os_str();
    let mut result = env::current_dir().unwrap_or(PathBuf::from(root));
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalizeMode {
    None,
    Normal,
    Existing,
    Missing,
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

        match fs::symlink_metadata(&result) {
            Err(e) => return Err(e),
            Ok(ref m) if !m.file_type().is_symlink() => break,
            Ok(..) => {
                followed += 1;
                match fs::read_link(&result) {
                    Ok(path) => {
                        result.pop();
                        result.push(path);
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }
    }
    Ok(result)
}

pub fn canonicalize<P: AsRef<Path>>(original: P, can_mode: CanonicalizeMode) -> IOResult<PathBuf> {
    // Create an absolute path
    let original = original.as_ref();
    let original = if original.is_absolute() {
        original.to_path_buf()
    } else {
        dunce::canonicalize(env::current_dir().unwrap())
            .unwrap()
            .join(original)
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
                parts.pop();
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

            if can_mode == CanonicalizeMode::None {
                continue;
            }

            match resolve(&result) {
                Err(e) => match can_mode {
                    CanonicalizeMode::Missing => continue,
                    _ => return Err(e),
                },
                Ok(path) => {
                    result.pop();
                    result.push(path);
                }
            }
        }

        result.push(parts.last().unwrap());

        match resolve(&result) {
            Err(e) => {
                if can_mode == CanonicalizeMode::Existing {
                    return Err(e);
                }
            }
            Ok(path) => {
                result.pop();
                result.push(path);
            }
        }
    }
    Ok(result)
}

#[cfg(unix)]
pub fn is_stdin_interactive() -> bool {
    unsafe { libc::isatty(libc::STDIN_FILENO) == 1 }
}

#[cfg(windows)]
pub fn is_stdin_interactive() -> bool {
    false
}

#[cfg(target_os = "redox")]
pub fn is_stdin_interactive() -> bool {
    termion::is_tty(&io::stdin())
}

#[cfg(unix)]
pub fn is_stdout_interactive() -> bool {
    unsafe { libc::isatty(libc::STDOUT_FILENO) == 1 }
}

#[cfg(windows)]
pub fn is_stdout_interactive() -> bool {
    false
}

#[cfg(target_os = "redox")]
pub fn is_stdout_interactive() -> bool {
    termion::is_tty(&io::stdout())
}

#[cfg(unix)]
pub fn is_stderr_interactive() -> bool {
    unsafe { libc::isatty(libc::STDERR_FILENO) == 1 }
}

#[cfg(windows)]
pub fn is_stderr_interactive() -> bool {
    false
}

#[cfg(target_os = "redox")]
pub fn is_stderr_interactive() -> bool {
    termion::is_tty(&io::stderr())
}

#[cfg(not(unix))]
#[allow(unused_variables)]
pub fn display_permissions(metadata: &fs::Metadata) -> String {
    String::from("---------")
}

#[cfg(unix)]
pub fn display_permissions(metadata: &fs::Metadata) -> String {
    let mode: mode_t = metadata.mode() as mode_t;
    display_permissions_unix(mode as u32)
}

#[cfg(unix)]
pub fn display_permissions_unix(mode: u32) -> String {
    let mut result = String::with_capacity(9);
    result.push(if has!(mode, S_IRUSR) { 'r' } else { '-' });
    result.push(if has!(mode, S_IWUSR) { 'w' } else { '-' });
    result.push(if has!(mode, S_ISUID) {
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
    result.push(if has!(mode, S_ISGID) {
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
    result.push(if has!(mode, S_ISVTX) {
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
