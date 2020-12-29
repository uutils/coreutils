// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) COMFOLLOW Chowner Passwd RFILE RFILE's derefer dgid duid

#[macro_use]
extern crate uucore;
pub use uucore::entries::{self, Group, Locate, Passwd};
use uucore::fs::resolve_relative_path;
use uucore::libc::{gid_t, uid_t};
use uucore::perms::{wrap_chown, Verbosity};

use clap::{App, Arg};

use walkdir::WalkDir;

use std::fs::{self, Metadata};
use std::os::unix::fs::MetadataExt;

use std::convert::AsRef;
use std::path::Path;

static ABOUT: &str = "change file owner and group";
static VERSION: &str = env!("CARGO_PKG_VERSION");

static OPT_CHANGES: &str = "changes";
static OPT_DEREFERENCE: &str = "dereference";
static OPT_NO_DEREFERENCE: &str = "no-dereference";
static OPT_FROM: &str = "from";
static OPT_PRESERVE_ROOT: &str = "preserve-root";
static OPT_NO_PRESERVE_ROOT: &str = "no-preserve-root";
static OPT_QUIET: &str = "quiet";
static OPT_RECURSIVE: &str = "recursive";
static OPT_REFERENCE: &str = "reference";
static OPT_SILENT: &str = "silent";
static OPT_TRAVERSE: &str = "H";
static OPT_NO_TRAVERSE: &str = "P";
static OPT_TRAVERSE_EVERY: &str = "L";
static OPT_VERBOSE: &str = "verbose";

static ARG_OWNER: &str = "owner";
static ARG_FILES: &str = "files";

const FTS_COMFOLLOW: u8 = 1;
const FTS_PHYSICAL: u8 = 1 << 1;
const FTS_LOGICAL: u8 = 1 << 2;

fn get_usage() -> String {
    format!(
        "{0} [OPTION]... [OWNER][:[GROUP]] FILE...\n{0} [OPTION]... --reference=RFILE FILE...",
        executable!()
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();

    let usage = get_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(OPT_CHANGES)
                .short("c")
                .long(OPT_CHANGES)
                .help("like verbose but report only when a change is made"),
        )
        .arg(Arg::with_name(OPT_DEREFERENCE).long(OPT_DEREFERENCE).help(
            "affect the referent of each symbolic link (this is the default), rather than the symbolic link itself",
        ))
        .arg(
            Arg::with_name(OPT_NO_DEREFERENCE)
                .short("h")
                .long(OPT_NO_DEREFERENCE)
                .help(
                    "affect symbolic links instead of any referenced file (useful only on systems that can change the ownership of a symlink)",
                ),
        )
        .arg(
            Arg::with_name(OPT_FROM)
                .long(OPT_FROM)
                .help(
                    "change the owner and/or group of each file only if its current owner and/or group match those specified here. Either may be omitted, in which case a match is not required for the omitted attribute",
                )
                .value_name("CURRENT_OWNER:CURRENT_GROUP"),
        )
        .arg(
            Arg::with_name(OPT_PRESERVE_ROOT)
                .long(OPT_PRESERVE_ROOT)
                .help("fail to operate recursively on '/'"),
        )
        .arg(
            Arg::with_name(OPT_NO_PRESERVE_ROOT)
                .long(OPT_NO_PRESERVE_ROOT)
                .help("do not treat '/' specially (the default)"),
        )
        .arg(
            Arg::with_name(OPT_QUIET)
                .long(OPT_QUIET)
                .help("suppress most error messages"),
        )
        .arg(
            Arg::with_name(OPT_RECURSIVE)
                .short("R")
                .long(OPT_RECURSIVE)
                .help("operate on files and directories recursively"),
        )
        .arg(
            Arg::with_name(OPT_REFERENCE)
                .long(OPT_REFERENCE)
                .help("use RFILE's owner and group rather than specifying OWNER:GROUP values")
                .value_name("RFILE")
                .min_values(1),
        )
        .arg(Arg::with_name(OPT_SILENT).short("f").long(OPT_SILENT))
        .arg(
            Arg::with_name(OPT_TRAVERSE)
                .short(OPT_TRAVERSE)
                .help("if a command line argument is a symbolic link to a directory, traverse it")
                .overrides_with_all(&[OPT_TRAVERSE_EVERY, OPT_NO_TRAVERSE]),
        )
        .arg(
            Arg::with_name(OPT_TRAVERSE_EVERY)
                .short(OPT_TRAVERSE_EVERY)
                .help("traverse every symbolic link to a directory encountered")
                .overrides_with_all(&[OPT_TRAVERSE, OPT_NO_TRAVERSE]),
        )
        .arg(
            Arg::with_name(OPT_NO_TRAVERSE)
                .short(OPT_NO_TRAVERSE)
                .help("do not traverse any symbolic links (default)")
                .overrides_with_all(&[OPT_TRAVERSE, OPT_TRAVERSE_EVERY]),
        )
        .arg(
            Arg::with_name(OPT_VERBOSE)
                .long(OPT_VERBOSE)
                .help("output a diagnostic for every file processed"),
        )
        .arg(
            Arg::with_name(ARG_OWNER)
                .multiple(false)
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(ARG_FILES)
                .multiple(true)
                .takes_value(true)
                .required(true)
                .min_values(1),
        )
        .get_matches_from(args);

    /* First arg is the owner/group */
    let owner = matches.value_of(ARG_OWNER).unwrap();

    /* Then the list of files */
    let files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let preserve_root = matches.is_present(OPT_PRESERVE_ROOT);

    let mut derefer = if matches.is_present(OPT_NO_DEREFERENCE) {
        1
    } else {
        0
    };

    let mut bit_flag = if matches.is_present(OPT_TRAVERSE) {
        FTS_COMFOLLOW | FTS_PHYSICAL
    } else if matches.is_present(OPT_TRAVERSE_EVERY) {
        FTS_LOGICAL
    } else {
        FTS_PHYSICAL
    };

    let recursive = matches.is_present(OPT_RECURSIVE);
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

    let verbosity = if matches.is_present(OPT_CHANGES) {
        Verbosity::Changes
    } else if matches.is_present(OPT_SILENT) || matches.is_present(OPT_QUIET) {
        Verbosity::Silent
    } else if matches.is_present(OPT_VERBOSE) {
        Verbosity::Verbose
    } else {
        Verbosity::Normal
    };

    let filter = if let Some(spec) = matches.value_of(OPT_FROM) {
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

    let dest_uid: Option<u32>;
    let dest_gid: Option<u32>;
    if let Some(file) = matches.value_of(OPT_REFERENCE) {
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
        match parse_spec(&owner) {
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
    let executor = Chowner {
        bit_flag,
        dest_uid,
        dest_gid,
        verbosity,
        recursive,
        dereference: derefer != 0,
        filter,
        preserve_root,
        files,
    };
    executor.exec()
}

fn parse_spec(spec: &str) -> Result<(Option<u32>, Option<u32>), String> {
    let args = spec.split(':').collect::<Vec<_>>();
    let usr_only = args.len() == 1;
    let grp_only = args.len() == 2 && args[0].is_empty() && !args[1].is_empty();
    let usr_grp = args.len() == 2 && !args[0].is_empty() && !args[1].is_empty();

    if usr_only {
        Ok((
            Some(match Passwd::locate(args[0]) {
                Ok(v) => v.uid(),
                _ => return Err(format!("invalid user: '{}'", spec)),
            }),
            None,
        ))
    } else if grp_only {
        Ok((
            None,
            Some(match Group::locate(args[1]) {
                Ok(v) => v.gid(),
                _ => return Err(format!("invalid group: '{}'", spec)),
            }),
        ))
    } else if usr_grp {
        Ok((
            Some(match Passwd::locate(args[0]) {
                Ok(v) => v.uid(),
                _ => return Err(format!("invalid user: '{}'", spec)),
            }),
            Some(match Group::locate(args[1]) {
                Ok(v) => v.gid(),
                _ => return Err(format!("invalid group: '{}'", spec)),
            }),
        ))
    } else {
        Ok((None, None))
    }
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
    ($m:expr, $e:ident, $err:block) => {
        match $m {
            Ok(meta) => meta,
            Err($e) => $err,
        }
    };
}

impl Chowner {
    fn exec(&self) -> i32 {
        let mut ret = 0;
        for f in &self.files {
            ret |= self.traverse(f);
        }
        ret
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
            match wrap_chown(
                path,
                &meta,
                self.dest_uid,
                self.dest_gid,
                follow_arg,
                self.verbosity.clone(),
            ) {
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

            ret = match wrap_chown(
                path,
                &meta,
                self.dest_uid,
                self.dest_gid,
                follow,
                self.verbosity.clone(),
            ) {
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
