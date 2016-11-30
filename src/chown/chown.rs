#![crate_name = "uu_chown"]

// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

#[macro_use]
extern crate uucore;
use uucore::libc::{self, uid_t, gid_t, lchown};
pub use uucore::entries::{self, Locate, Passwd, Group};
use uucore::fs::resolve_relative_path;

extern crate walkdir;
use walkdir::WalkDir;

use std::fs::{self, Metadata};
use std::os::unix::fs::MetadataExt;

use std::io::{self, Write};
use std::io::Result as IOResult;

use std::path::Path;
use std::convert::AsRef;

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;

static SYNTAX: &'static str = "[OPTION]... [OWNER][:[GROUP]] FILE...\n chown [OPTION]... --reference=RFILE FILE...";
static SUMMARY: &'static str = "change file owner and group";

const FTS_COMFOLLOW: u8 = 1;
const FTS_PHYSICAL: u8 = 1 << 1;
const FTS_LOGICAL: u8 = 1 << 2;

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = new_coreopts!(SYNTAX, SUMMARY, "");
    opts.optflag("c",
                 "changes",
                 "like verbose but report only when a change is made")
        .optflag("f", "silent", "")
        .optflag("", "quiet", "suppress most error messages")
        .optflag("v",
                 "verbose",
                 "output a diagnostic for every file processed")
        .optflag("", "dereference", "affect the referent of each symbolic link (this is the default), rather than the symbolic link itself")
        .optflag("h", "no-dereference", "affect symbolic links instead of any referenced file (useful only on systems that can change the ownership of a symlink)")

        .optopt("", "from", "change the owner and/or group of each file only if its current owner and/or group match those specified here. Either may be omitted, in which case a match is not required for the omitted attribute", "CURRENT_OWNER:CURRENT_GROUP")
        .optopt("",
                "reference",
                "use RFILE's owner and group rather than specifying OWNER:GROUP values",
                "RFILE")
        .optflag("",
                 "no-preserve-root",
                 "do not treat '/' specially (the default)")
        .optflag("", "preserve-root", "fail to operate recursively on '/'")

        .optflag("R",
                 "recursive",
                 "operate on files and directories recursively")
        .optflag("H",
                 "",
                 "if a command line argument is a symbolic link to a directory, traverse it")
        .optflag("L",
                 "",
                 "traverse every symbolic link to a directory encountered")
        .optflag("P", "", "do not traverse any symbolic links (default)");

    let mut bit_flag = FTS_PHYSICAL;
    let mut preserve_root = false;
    let mut derefer = -1;
    let flags: &[char] = &['H', 'L', 'P'];
    for opt in &args {
        match opt.as_str() {
            // If more than one is specified, only the final one takes effect.
            s if s.contains(flags) => {
                if let Some(idx) = s.rfind(flags) {
                    match s.chars().nth(idx).unwrap() {
                        'H' => bit_flag = FTS_COMFOLLOW | FTS_PHYSICAL,
                        'L' => bit_flag = FTS_LOGICAL,
                        'P' => bit_flag = FTS_PHYSICAL,
                        _ => (),
                    }
                }
            }
            "--no-preserve-root" => preserve_root = false,
            "--preserve-root" => preserve_root = true,
            "--dereference" => derefer = 1,
            "--no-dereference" => derefer = 0,
            _ => (),
        }
    }

    let matches = opts.parse(args);
    let recursive = matches.opt_present("recursive");
    if recursive {
        if bit_flag == FTS_PHYSICAL {
            if derefer == 1 {
                show_info!("-R --dereference requires -H or -L");
                return 1;
            }
            derefer = 0;
        }
    } else {
        bit_flag = FTS_PHYSICAL;
    }

    let verbosity = if matches.opt_present("changes") {
        Verbosity::Changes
    } else if matches.opt_present("silent") || matches.opt_present("quiet") {
        Verbosity::Silent
    } else if matches.opt_present("verbose") {
        Verbosity::Verbose
    } else {
        Verbosity::Normal
    };

    let filter = if let Some(spec) = matches.opt_str("from") {
        match parse_spec(&spec) {
            Ok((Some(uid), None)) => IfFrom::User(uid),
            Ok((None, Some(gid))) => IfFrom::Group(gid),
            Ok((Some(uid), Some(gid))) => IfFrom::UserGroup(uid, gid),
            Ok((None, None)) => IfFrom::All,
            Err(e) => {
                show_info!("{}", e);
                return 1;
            }
        }
    } else {
        IfFrom::All
    };

    if matches.free.len() < 1 {
        disp_err!("missing operand");
        return 1;
    } else if matches.free.len() < 2 && !matches.opt_present("reference") {
        disp_err!("missing operand after ‘{}’", matches.free[0]);
        return 1;
    }

    let dest_uid: Option<u32>;
    let dest_gid: Option<u32>;
    if let Some(file) = matches.opt_str("reference") {
        match fs::metadata(&file) {
            Ok(meta) => {
                dest_gid = Some(meta.gid());
                dest_uid = Some(meta.uid());
            }
            Err(e) => {
                show_info!("failed to get attributes of '{}': {}", file, e);
                return 1;
            }
        }
    } else {
        match parse_spec(&matches.free[0]) {
            Ok((u, g)) => {
                dest_uid = u;
                dest_gid = g;
            }
            Err(e) => {
                show_info!("{}", e);
                return 1;
            }
        }
    }
    let mut files = matches.free;
    files.remove(0);
    let executor = Chowner {
        bit_flag: bit_flag,
        dest_uid: dest_uid,
        dest_gid: dest_gid,
        verbosity: verbosity,
        recursive: recursive,
        dereference: derefer != 0,
        filter: filter,
        preserve_root: preserve_root,
        files: files,
    };
    executor.exec()
}

fn parse_spec(spec: &str) -> Result<(Option<u32>, Option<u32>), String> {
    let args = spec.split(':').collect::<Vec<_>>();
    let usr_only = args.len() == 1;
    let grp_only = args.len() == 2 && args[0].is_empty() && !args[1].is_empty();
    let usr_grp = args.len() == 2 && !args[0].is_empty() && !args[1].is_empty();

    if usr_only {
        Ok((Some(match Passwd::locate(args[0]) {
            Ok(v) => v.uid(),
            _ => return Err(format!("invalid user: ‘{}’", spec)),
        }),
            None))
    } else if grp_only {
        Ok((None,
            Some(match Group::locate(args[1]) {
            Ok(v) => v.gid(),
            _ => return Err(format!("invalid group: ‘{}’", spec)),
        })))
    } else if usr_grp {
        Ok((Some(match Passwd::locate(args[0]) {
            Ok(v) => v.uid(),
            _ => return Err(format!("invalid user: ‘{}’", spec)),
        }),
            Some(match Group::locate(args[1]) {
            Ok(v) => v.gid(),
            _ => return Err(format!("invalid group: ‘{}’", spec)),
        })))
    } else {
        Ok((None, None))
    }
}

#[derive(PartialEq, Debug)]
enum Verbosity {
    Silent,
    Changes,
    Verbose,
    Normal,
}

enum IfFrom {
    All,
    User(u32),
    Group(u32),
    UserGroup(u32, u32),
}

struct Chowner {
    dest_uid: Option<u32>,
    dest_gid: Option<u32>,
    bit_flag: u8,
    verbosity: Verbosity,
    filter: IfFrom,
    files: Vec<String>,
    recursive: bool,
    preserve_root: bool,
    dereference: bool,
}

macro_rules! unwrap {
    ($m:expr, $e:ident, $err:block) => (
        match $m {
            Ok(meta) => meta,
            Err($e) => $err,
        }
    )
}

impl Chowner {
    fn exec(&self) -> i32 {
        let mut ret = 0;
        for f in &self.files {
            ret |= self.traverse(f);
        }
        ret
    }

    fn chown<P: AsRef<Path>>(&self, path: P, duid: uid_t, dgid: gid_t, follow: bool) -> IOResult<()> {
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
            Err(io::Error::last_os_error())
        }
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
                    show_info!("it is dangerous to operate recursively on '/'");
                    show_info!("use --no-preserve-root to override this failsafe");
                    return 1;
                }
            }
        }

        let ret = if self.matched(meta.uid(), meta.gid()) {
            self.wrap_chown(path, &meta, follow_arg)
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
                show_info!("{}", e);
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

            ret = self.wrap_chown(path, &meta, follow);
        }
        ret
    }

    fn obtain_meta<P: AsRef<Path>>(&self, path: P, follow: bool) -> Option<Metadata> {
        use self::Verbosity::*;
        let path = path.as_ref();
        let meta = if follow {
            unwrap!(path.metadata(), e, {
                match self.verbosity {
                    Silent => (),
                    _ => show_info!("cannot access '{}': {}", path.display(), e),
                }
                return None;
            })
        } else {
            unwrap!(path.symlink_metadata(), e, {
                match self.verbosity {
                    Silent => (),
                    _ => show_info!("cannot dereference '{}': {}", path.display(), e),
                }
                return None;
            })
        };
        Some(meta)
    }

    fn wrap_chown<P: AsRef<Path>>(&self, path: P, meta: &Metadata, follow: bool) -> i32 {
        use self::Verbosity::*;
        let mut ret = 0;
        let dest_uid = self.dest_uid.unwrap_or(meta.uid());
        let dest_gid = self.dest_gid.unwrap_or(meta.gid());
        let path = path.as_ref();
        if let Err(e) = self.chown(path, dest_uid, dest_gid, follow) {
            match self.verbosity {
                Silent => (),
                _ => {
                    show_info!("changing ownership of '{}': {}", path.display(), e);
                    if self.verbosity == Verbose {
                        println!("failed to change ownership of {} from {}:{} to {}:{}",
                                 path.display(),
                                 entries::uid2usr(meta.uid()).unwrap(),
                                 entries::gid2grp(meta.gid()).unwrap(),
                                 entries::uid2usr(dest_uid).unwrap(),
                                 entries::gid2grp(dest_gid).unwrap());
                    };
                }
            }
            ret = 1;
        } else {
            let changed = dest_uid != meta.uid() || dest_gid != meta.gid();
            if changed {
                match self.verbosity {
                    Changes | Verbose => {
                        println!("changed ownership of {} from {}:{} to {}:{}",
                                 path.display(),
                                 entries::uid2usr(meta.uid()).unwrap(),
                                 entries::gid2grp(meta.gid()).unwrap(),
                                 entries::uid2usr(dest_uid).unwrap(),
                                 entries::gid2grp(dest_gid).unwrap());
                    }
                    _ => (),
                };
            } else if self.verbosity == Verbose {
                println!("ownership of {} retained as {}:{}",
                         path.display(),
                         entries::uid2usr(dest_uid).unwrap(),
                         entries::gid2grp(dest_gid).unwrap());
            }
        }
        ret
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
