// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

pub use crate::features::entries;
use libc::{self, gid_t, lchown, uid_t};

use std::io::Error as IOError;
use std::io::Result as IOResult;

use std::ffi::CString;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;

use std::os::unix::ffi::OsStrExt;
use std::path::Path;

/// The various level of verbosity
#[derive(PartialEq, Clone, Debug)]
pub enum Verbosity {
    Silent,
    Changes,
    Verbose,
    Normal,
}

/// Actually perform the change of group on a path
fn chgrp<P: AsRef<Path>>(path: P, dgid: gid_t, follow: bool) -> IOResult<()> {
    let path = path.as_ref();
    let s = CString::new(path.as_os_str().as_bytes()).unwrap();
    let ret = unsafe {
        if follow {
            libc::chown(s.as_ptr(), (0 as gid_t).wrapping_sub(1), dgid)
        } else {
            lchown(s.as_ptr(), (0 as gid_t).wrapping_sub(1), dgid)
        }
    };
    if ret == 0 {
        Ok(())
    } else {
        Err(IOError::last_os_error())
    }
}

/// Perform the change of group on a path
/// with the various options
/// and error messages management
pub fn wrap_chgrp<P: AsRef<Path>>(
    path: P,
    meta: &Metadata,
    dest_gid: gid_t,
    follow: bool,
    verbosity: Verbosity,
) -> Result<String, String> {
    use self::Verbosity::*;
    let path = path.as_ref();
    let mut out: String = String::new();

    if let Err(e) = chgrp(path, dest_gid, follow) {
        match verbosity {
            Silent => (),
            _ => {
                out = format!("changing group of '{}': {}", path.display(), e);
                if verbosity == Verbose {
                    out = format!(
                        "{}\nfailed to change group of '{}' from {} to {}",
                        out,
                        path.display(),
                        entries::gid2grp(meta.gid()).unwrap(),
                        entries::gid2grp(dest_gid).unwrap()
                    );
                };
            }
        }
        return Err(out);
    } else {
        let changed = dest_gid != meta.gid();
        if changed {
            match verbosity {
                Changes | Verbose => {
                    out = format!(
                        "changed group of '{}' from {} to {}",
                        path.display(),
                        entries::gid2grp(meta.gid()).unwrap(),
                        entries::gid2grp(dest_gid).unwrap()
                    );
                }
                _ => (),
            };
        } else if verbosity == Verbose {
            out = format!(
                "group of '{}' retained as {}",
                path.display(),
                entries::gid2grp(dest_gid).unwrap()
            );
        }
    }
    Ok(out)
}

/// Actually perform the change of owner on a path
fn chown<P: AsRef<Path>>(path: P, duid: uid_t, dgid: gid_t, follow: bool) -> IOResult<()> {
    let path = path.as_ref();
    let s = CString::new(path.as_os_str().as_bytes()).unwrap();
    let ret = unsafe {
        if follow {
            libc::chown(s.as_ptr(), duid, dgid)
        } else {
            lchown(s.as_ptr(), duid, dgid)
        }
    };
    if ret == 0 {
        Ok(())
    } else {
        Err(IOError::last_os_error())
    }
}

/// Perform the change of owner on a path
/// with the various options
/// and error messages management
pub fn wrap_chown<P: AsRef<Path>>(
    path: P,
    meta: &Metadata,
    dest_uid: Option<u32>,
    dest_gid: Option<u32>,
    follow: bool,
    verbosity: Verbosity,
) -> Result<String, String> {
    use self::Verbosity::*;
    let dest_uid = dest_uid.unwrap_or_else(|| meta.uid());
    let dest_gid = dest_gid.unwrap_or_else(|| meta.gid());
    let path = path.as_ref();
    let mut out: String = String::new();

    if let Err(e) = chown(path, dest_uid, dest_gid, follow) {
        match verbosity {
            Silent => (),
            _ => {
                out = format!("changing ownership of '{}': {}", path.display(), e);
                if verbosity == Verbose {
                    out = format!(
                        "{}\nfailed to change ownership of '{}' from {}:{} to {}:{}",
                        out,
                        path.display(),
                        entries::uid2usr(meta.uid()).unwrap(),
                        entries::gid2grp(meta.gid()).unwrap(),
                        entries::uid2usr(dest_uid).unwrap(),
                        entries::gid2grp(dest_gid).unwrap()
                    );
                };
            }
        }
        return Err(out);
    } else {
        let changed = dest_uid != meta.uid() || dest_gid != meta.gid();
        if changed {
            match verbosity {
                Changes | Verbose => {
                    out = format!(
                        "changed ownership of '{}' from {}:{} to {}:{}",
                        path.display(),
                        entries::uid2usr(meta.uid()).unwrap(),
                        entries::gid2grp(meta.gid()).unwrap(),
                        entries::uid2usr(dest_uid).unwrap(),
                        entries::gid2grp(dest_gid).unwrap()
                    );
                }
                _ => (),
            };
        } else if verbosity == Verbose {
            out = format!(
                "ownership of '{}' retained as {}:{}",
                path.display(),
                entries::uid2usr(dest_uid).unwrap(),
                entries::gid2grp(dest_gid).unwrap()
            );
        }
    }
    Ok(out)
}
