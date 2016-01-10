/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Joseph Crail <jbcrail@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[cfg(unix)]
use super::libc;
use std::env;
use std::fs;
use std::io::{Error, ErrorKind, Result};
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalizeMode {
    None,
    Normal,
    Existing,
    Missing,
}

fn resolve<P: AsRef<Path>>(original: P) -> Result<PathBuf> {
    const MAX_LINKS_FOLLOWED: u32 = 255;
    let mut followed = 0;
    let mut result = original.as_ref().to_path_buf();
    loop {
        if followed == MAX_LINKS_FOLLOWED {
            return Err(Error::new(ErrorKind::InvalidInput, "maximum links followed"));
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
                    },
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }
    }
    Ok(result)
}

pub fn canonicalize<P: AsRef<Path>>(original: P, can_mode: CanonicalizeMode) -> Result<PathBuf> {
    // Create an absolute path
    let original = original.as_ref();
    let original = if original.is_absolute() {
        original.to_path_buf()
    } else {
        env::current_dir().unwrap().join(original)
    };

    let mut result = PathBuf::new();
    let mut parts = vec!();

    // Split path by directory separator; add prefix (Windows-only) and root
    // directory to final path buffer; add remaining parts to temporary
    // vector for canonicalization.
    for part in original.components() {
        match part {
            Component::Prefix(_) | Component::RootDir => {
                result.push(part.as_os_str());
            },
            Component::CurDir => {},
            Component::ParentDir => {
                parts.pop();
            },
            Component::Normal(_) => {
                parts.push(part.as_os_str());
            }
        }
    }

    // Resolve the symlinks where possible
    if !parts.is_empty() {
        for part in parts[..parts.len()-1].iter() {
            result.push(part);

            if can_mode == CanonicalizeMode::None {
                continue;
            }

            match resolve(&result) {
                Err(e) => match can_mode {
                    CanonicalizeMode::Missing => continue,
                    _ => return Err(e)
                },
                Ok(path) => {
                    result.pop();
                    result.push(path);
                }
            }
        }

        result.push(parts.last().unwrap());

        match resolve(&result) {
            Err(e) => { if can_mode == CanonicalizeMode::Existing { return Err(e); } },
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

#[cfg(unix)]
pub fn is_stdout_interactive() -> bool {
    unsafe { libc::isatty(libc::STDOUT_FILENO) == 1 }
}

#[cfg(windows)]
pub fn is_stdout_interactive() -> bool {
    false
}

#[cfg(unix)]
pub fn is_stderr_interactive() -> bool {
    unsafe { libc::isatty(libc::STDERR_FILENO) == 1 }
}

#[cfg(windows)]
pub fn is_stderr_interactive() -> bool {
    false
}
