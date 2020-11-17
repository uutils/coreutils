// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

pub use crate::features::entries;
use libc::{self, gid_t, lchown, uid_t};

#[macro_use]
pub use crate::*;

use std::io::Error as IOError;
use std::io::Result as IOResult;

use std::ffi::CString;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;

use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(PartialEq, Clone, Debug)]
pub enum Verbosity {
    Silent,
    Changes,
    Verbose,
    Normal,
}

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

pub fn wrap_chgrp<P: AsRef<Path>>(
    path: P,
    meta: &Metadata,
    dest_gid: gid_t,
    follow: bool,
    verbosity: Verbosity,
) -> i32 {
    use self::Verbosity::*;
    let mut ret = 0;
    let path = path.as_ref();
    if let Err(e) = chgrp(path, dest_gid, follow) {
        match verbosity {
            Silent => (),
            _ => {
                show_info!("changing group of '{}': {}", path.display(), e);
                if verbosity == Verbose {
                    println!(
                        "failed to change group of {} from {} to {}",
                        path.display(),
                        entries::gid2grp(meta.gid()).unwrap(),
                        entries::gid2grp(dest_gid).unwrap()
                    );
                };
            }
        }
        ret = 1;
    } else {
        let changed = dest_gid != meta.gid();
        if changed {
            match verbosity {
                Changes | Verbose => {
                    println!(
                        "changed group of {} from {} to {}",
                        path.display(),
                        entries::gid2grp(meta.gid()).unwrap(),
                        entries::gid2grp(dest_gid).unwrap()
                    );
                }
                _ => (),
            };
        } else if verbosity == Verbose {
            println!(
                "group of {} retained as {}",
                path.display(),
                entries::gid2grp(dest_gid).unwrap()
            );
        }
    }
    ret
}

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

pub fn wrap_chown<P: AsRef<Path>>(
    path: P,
    meta: &Metadata,
    dest_uid: Option<u32>,
    dest_gid: Option<u32>,
    follow: bool,
    verbosity: Verbosity,
) -> i32 {
    use self::Verbosity::*;
    let mut ret = 0;
    let dest_uid = dest_uid.unwrap_or_else(|| meta.uid());
    let dest_gid = dest_gid.unwrap_or_else(|| meta.gid());
    let path = path.as_ref();
    if let Err(e) = chown(path, dest_uid, dest_gid, follow) {
        match verbosity {
            Silent => (),
            _ => {
                show_info!("changing ownership of '{}': {}", path.display(), e);
                if verbosity == Verbose {
                    println!(
                        "failed to change ownership of {} from {}:{} to {}:{}",
                        path.display(),
                        entries::uid2usr(meta.uid()).unwrap(),
                        entries::gid2grp(meta.gid()).unwrap(),
                        entries::uid2usr(dest_uid).unwrap(),
                        entries::gid2grp(dest_gid).unwrap()
                    );
                };
            }
        }
        ret = 1;
    } else {
        let changed = dest_uid != meta.uid() || dest_gid != meta.gid();
        if changed {
            match verbosity {
                Changes | Verbose => {
                    println!(
                        "changed ownership of {} from {}:{} to {}:{}",
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
            println!(
                "ownership of {} retained as {}:{}",
                path.display(),
                entries::uid2usr(dest_uid).unwrap(),
                entries::gid2grp(dest_gid).unwrap()
            );
        }
    }
    ret
}
