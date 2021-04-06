// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) COMFOLLOW Chgrper RFILE RFILE's derefer dgid nonblank nonprint nonprinting

#[macro_use]
extern crate uucore;
pub use uucore::entries;
use uucore::fs::resolve_relative_path;
use uucore::libc::gid_t;
use uucore::perms::{wrap_chgrp, Verbosity};

extern crate walkdir;
use walkdir::WalkDir;

use std::fs;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;

use std::path::Path;

static SYNTAX: &str =
    "chgrp [OPTION]... GROUP FILE...\n or :  chgrp [OPTION]... --reference=RFILE FILE...";
static SUMMARY: &str = "Change the group of each FILE to GROUP.";

const FTS_COMFOLLOW: u8 = 1;
const FTS_PHYSICAL: u8 = 1 << 1;
const FTS_LOGICAL: u8 = 1 << 2;

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();

    let mut opts = app!(SYNTAX, SUMMARY, "");
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
        .optflag("",
                 "no-preserve-root",
                 "do not treat '/' specially (the default)")
        .optflag("", "preserve-root", "fail to operate recursively on '/'")
        .optopt("",
                "reference",
                "use RFILE's owner and group rather than specifying OWNER:GROUP values",
                "RFILE")
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

    if matches.free.is_empty() {
        show_usage_error!("missing operand");
        return 1;
    } else if matches.free.len() < 2 && !matches.opt_present("reference") {
        show_usage_error!("missing operand after ‘{}’", matches.free[0]);
        return 1;
    }

    let dest_gid: gid_t;
    let mut files;
    if let Some(file) = matches.opt_str("reference") {
        match fs::metadata(&file) {
            Ok(meta) => {
                dest_gid = meta.gid();
            }
            Err(e) => {
                show_info!("failed to get attributes of '{}': {}", file, e);
                return 1;
            }
        }
        files = matches.free;
    } else {
        match entries::grp2gid(&matches.free[0]) {
            Ok(g) => {
                dest_gid = g;
            }
            _ => {
                show_info!("invalid group: {}", matches.free[0].as_str());
                return 1;
            }
        }
        files = matches.free;
        files.remove(0);
    }

    let executor = Chgrper {
        bit_flag,
        dest_gid,
        verbosity,
        recursive,
        dereference: derefer != 0,
        preserve_root,
        files,
    };
    executor.exec()
}

struct Chgrper {
    dest_gid: gid_t,
    bit_flag: u8,
    verbosity: Verbosity,
    files: Vec<String>,
    recursive: bool,
    preserve_root: bool,
    dereference: bool,
}

macro_rules! unwrap {
    ($m:expr, $e:ident, $err:block) => {
        match $m {
            Ok(meta) => meta,
            Err($e) => $err,
        }
    };
}

impl Chgrper {
    fn exec(&self) -> i32 {
        let mut ret = 0;
        for f in &self.files {
            ret |= self.traverse(f);
        }
        ret
    }

    #[cfg(windows)]
    fn is_bind_root<P: AsRef<Path>>(&self, root: P) -> bool {
        // TODO: is there an equivalent on Windows?
        false
    }

    #[cfg(unix)]
    fn is_bind_root<P: AsRef<Path>>(&self, path: P) -> bool {
        if let (Ok(given), Ok(root)) = (fs::metadata(path), fs::metadata("/")) {
            given.dev() == root.dev() && given.ino() == root.ino()
        } else {
            // FIXME: not totally sure if it's okay to just ignore an error here
            false
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
                if p.parent().is_none() || self.is_bind_root(p) {
                    show_info!("it is dangerous to operate recursively on '/'");
                    show_info!("use --no-preserve-root to override this failsafe");
                    return 1;
                }
            }
        }

        let ret = match wrap_chgrp(
            path,
            &meta,
            self.dest_gid,
            follow_arg,
            self.verbosity.clone(),
        ) {
            Ok(n) => {
                show_info!("{}", n);
                0
            }
            Err(e) => {
                if self.verbosity != Verbosity::Silent {
                    show_info!("{}", e);
                }
                1
            }
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

            ret = match wrap_chgrp(path, &meta, self.dest_gid, follow, self.verbosity.clone()) {
                Ok(n) => {
                    if n != "" {
                        show_info!("{}", n);
                    }
                    0
                }
                Err(e) => {
                    if self.verbosity != Verbosity::Silent {
                        show_info!("{}", e);
                    }
                    1
                }
            }
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
}
