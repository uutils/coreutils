// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Common functions to manage permissions

use crate::error::UResult;
pub use crate::features::entries;
use crate::fs::resolve_relative_path;
use crate::show_error;
use libc::{self, gid_t, lchown, uid_t};
use walkdir::WalkDir;

use std::io::Error as IOError;
use std::io::Result as IOResult;

use std::ffi::CString;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;

use std::os::unix::ffi::OsStrExt;
use std::path::Path;

/// The various level of verbosity
#[derive(PartialEq, Clone, Debug)]
pub enum VerbosityLevel {
    Silent,
    Changes,
    Verbose,
    Normal,
}
#[derive(PartialEq, Clone, Debug)]
pub struct Verbosity {
    pub groups_only: bool,
    pub level: VerbosityLevel,
}

/// Actually perform the change of owner on a path
fn chown<P: AsRef<Path>>(path: P, uid: uid_t, gid: gid_t, follow: bool) -> IOResult<()> {
    let path = path.as_ref();
    let s = CString::new(path.as_os_str().as_bytes()).unwrap();
    let ret = unsafe {
        if follow {
            libc::chown(s.as_ptr(), uid, gid)
        } else {
            lchown(s.as_ptr(), uid, gid)
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
    let dest_uid = dest_uid.unwrap_or_else(|| meta.uid());
    let dest_gid = dest_gid.unwrap_or_else(|| meta.gid());
    let path = path.as_ref();
    let mut out: String = String::new();

    if let Err(e) = chown(path, dest_uid, dest_gid, follow) {
        match verbosity.level {
            VerbosityLevel::Silent => (),
            level => {
                out = format!(
                    "changing {} of '{}': {}",
                    if verbosity.groups_only {
                        "group"
                    } else {
                        "ownership"
                    },
                    path.display(),
                    e
                );
                if level == VerbosityLevel::Verbose {
                    out = if verbosity.groups_only {
                        format!(
                            "{}\nfailed to change group of '{}' from {} to {}",
                            out,
                            path.display(),
                            entries::gid2grp(meta.gid()).unwrap(),
                            entries::gid2grp(dest_gid).unwrap()
                        )
                    } else {
                        format!(
                            "{}\nfailed to change ownership of '{}' from {}:{} to {}:{}",
                            out,
                            path.display(),
                            entries::uid2usr(meta.uid()).unwrap(),
                            entries::gid2grp(meta.gid()).unwrap(),
                            entries::uid2usr(dest_uid).unwrap(),
                            entries::gid2grp(dest_gid).unwrap()
                        )
                    };
                };
            }
        }
        return Err(out);
    } else {
        let changed = dest_uid != meta.uid() || dest_gid != meta.gid();
        if changed {
            match verbosity.level {
                VerbosityLevel::Changes | VerbosityLevel::Verbose => {
                    out = if verbosity.groups_only {
                        format!(
                            "changed group of '{}' from {} to {}",
                            path.display(),
                            entries::gid2grp(meta.gid()).unwrap(),
                            entries::gid2grp(dest_gid).unwrap()
                        )
                    } else {
                        format!(
                            "changed ownership of '{}' from {}:{} to {}:{}",
                            path.display(),
                            entries::uid2usr(meta.uid()).unwrap(),
                            entries::gid2grp(meta.gid()).unwrap(),
                            entries::uid2usr(dest_uid).unwrap(),
                            entries::gid2grp(dest_gid).unwrap()
                        )
                    };
                }
                _ => (),
            };
        } else if verbosity.level == VerbosityLevel::Verbose {
            out = if verbosity.groups_only {
                format!(
                    "group of '{}' retained as {}",
                    path.display(),
                    entries::gid2grp(dest_gid).unwrap_or_default()
                )
            } else {
                format!(
                    "ownership of '{}' retained as {}:{}",
                    path.display(),
                    entries::uid2usr(dest_uid).unwrap(),
                    entries::gid2grp(dest_gid).unwrap()
                )
            };
        }
    }
    Ok(out)
}

pub enum IfFrom {
    All,
    User(u32),
    Group(u32),
    UserGroup(u32, u32),
}

pub struct ChownExecutor {
    pub dest_uid: Option<u32>,
    pub dest_gid: Option<u32>,
    pub bit_flag: u8,
    pub verbosity: Verbosity,
    pub filter: IfFrom,
    pub files: Vec<String>,
    pub recursive: bool,
    pub preserve_root: bool,
    pub dereference: bool,
}

macro_rules! unwrap {
    ($m:expr, $e:ident, $err:block) => {
        match $m {
            Ok(meta) => meta,
            Err($e) => $err,
        }
    };
}

pub const FTS_COMFOLLOW: u8 = 1;
pub const FTS_PHYSICAL: u8 = 1 << 1;
pub const FTS_LOGICAL: u8 = 1 << 2;

impl ChownExecutor {
    pub fn exec(&self) -> UResult<()> {
        let mut ret = 0;
        for f in &self.files {
            ret |= self.traverse(f);
        }
        if ret != 0 {
            return Err(ret.into());
        }
        Ok(())
    }

    fn traverse<P: AsRef<Path>>(&self, root: P) -> i32 {
        let follow_arg = self.dereference || self.bit_flag != FTS_PHYSICAL;
        let path = root.as_ref();
        let meta = match self.obtain_meta(path, follow_arg) {
            Some(m) => m,
            _ => return 1,
        };

        // Prohibit only if:
        // (--preserve-root and -R present) &&
        // (
        //     (argument is not symlink && resolved to be '/') ||
        //     (argument is symlink && should follow argument && resolved to be '/')
        // )
        if self.recursive && self.preserve_root {
            let may_exist = if follow_arg {
                path.canonicalize().ok()
            } else {
                let real = resolve_relative_path(path);
                if real.is_dir() {
                    Some(real.canonicalize().expect("failed to get real path"))
                } else {
                    Some(real.into_owned())
                }
            };

            if let Some(p) = may_exist {
                if p.parent().is_none() {
                    show_error!("it is dangerous to operate recursively on '/'");
                    show_error!("use --no-preserve-root to override this failsafe");
                    return 1;
                }
            }
        }

        let ret = if self.matched(meta.uid(), meta.gid()) {
            match wrap_chown(
                path,
                &meta,
                self.dest_uid,
                self.dest_gid,
                follow_arg,
                self.verbosity.clone(),
            ) {
                Ok(n) => {
                    if !n.is_empty() {
                        show_error!("{}", n);
                    }
                    0
                }
                Err(e) => {
                    if self.verbosity.level != VerbosityLevel::Silent {
                        show_error!("{}", e);
                    }
                    1
                }
            }
        } else {
            0
        };

        if !self.recursive {
            ret
        } else {
            ret | self.dive_into(&root)
        }
    }

    fn dive_into<P: AsRef<Path>>(&self, root: P) -> i32 {
        let mut ret = 0;
        let root = root.as_ref();
        let follow = self.dereference || self.bit_flag & FTS_LOGICAL != 0;
        for entry in WalkDir::new(root).follow_links(follow).min_depth(1) {
            let entry = unwrap!(entry, e, {
                ret = 1;
                show_error!("{}", e);
                continue;
            });
            let path = entry.path();
            let meta = match self.obtain_meta(path, follow) {
                Some(m) => m,
                _ => {
                    ret = 1;
                    continue;
                }
            };

            if !self.matched(meta.uid(), meta.gid()) {
                continue;
            }

            ret = match wrap_chown(
                path,
                &meta,
                self.dest_uid,
                self.dest_gid,
                follow,
                self.verbosity.clone(),
            ) {
                Ok(n) => {
                    if !n.is_empty() {
                        show_error!("{}", n);
                    }
                    0
                }
                Err(e) => {
                    if self.verbosity.level != VerbosityLevel::Silent {
                        show_error!("{}", e);
                    }
                    1
                }
            }
        }
        ret
    }

    fn obtain_meta<P: AsRef<Path>>(&self, path: P, follow: bool) -> Option<Metadata> {
        let path = path.as_ref();
        let meta = if follow {
            unwrap!(path.metadata(), e, {
                match self.verbosity.level {
                    VerbosityLevel::Silent => (),
                    _ => show_error!("cannot access '{}': {}", path.display(), e),
                }
                return None;
            })
        } else {
            unwrap!(path.symlink_metadata(), e, {
                match self.verbosity.level {
                    VerbosityLevel::Silent => (),
                    _ => show_error!("cannot dereference '{}': {}", path.display(), e),
                }
                return None;
            })
        };
        Some(meta)
    }

    #[inline]
    fn matched(&self, uid: uid_t, gid: gid_t) -> bool {
        match self.filter {
            IfFrom::All => true,
            IfFrom::User(u) => u == uid,
            IfFrom::Group(g) => g == gid,
            IfFrom::UserGroup(u, g) => u == uid && g == gid,
        }
    }
}
