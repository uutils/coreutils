/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Joseph Crail <jbcrail@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

// Based on the pattern using by Cargo, I created a shim over the
// standard PathExt trait, so that the unstable path methods could
// be backported to stable (<= 1.1). This will likely be dropped
// when the path trait stabilizes.

use std::env;
use std::fs;
use std::io::{Error, ErrorKind, Result};
use std::path::{Component, Path, PathBuf};

pub trait UUPathExt {
    fn uu_exists(&self) -> bool;
    fn uu_is_file(&self) -> bool;
    fn uu_is_dir(&self) -> bool;
    fn uu_metadata(&self) -> Result<fs::Metadata>;
}

impl UUPathExt for Path {
    fn uu_exists(&self) -> bool {
        fs::metadata(self).is_ok()
    }

    fn uu_is_file(&self) -> bool {
        fs::metadata(self).map(|m| m.is_file()).unwrap_or(false)
    }

    fn uu_is_dir(&self) -> bool {
        fs::metadata(self).map(|m| m.is_dir()).unwrap_or(false)
    }

    fn uu_metadata(&self) -> Result<fs::Metadata> {
        fs::metadata(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub enum CanonicalizeMode {
    None,
    Normal,
    Existing,
    Missing,
}

#[allow(dead_code)]
fn resolve<P: AsRef<Path>>(original: P) -> Result<PathBuf> {
    const MAX_LINKS_FOLLOWED: u32 = 255;
    let mut followed = 0;
    let mut result = original.as_ref().to_path_buf();
    loop {
        if followed == MAX_LINKS_FOLLOWED {
            return Err(Error::new(ErrorKind::InvalidInput, "maximum links followed"));
        }

        match fs::metadata(&result) {
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

#[allow(dead_code)]
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
            }
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop();
            }
            Component::Normal(_) => {
                parts.push(part.as_os_str());
            }
        }
    }

    // Resolve the symlinks where possible
    if parts.len() > 0 {
        for part in parts[..parts.len()-1].iter() {
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
