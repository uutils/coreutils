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

use uucore::error::{FromIo, UError, UResult, USimpleError};

use clap::{crate_version, App, Arg};

use walkdir::WalkDir;

use std::fs::{self, Metadata};
use std::os::unix::fs::MetadataExt;

use std::convert::AsRef;
use std::path::Path;
use uucore::InvalidEncodingHandling;

static ABOUT: &str = "change file owner and group";

pub mod options {
    pub mod verbosity {
        pub static CHANGES: &str = "changes";
        pub static QUIET: &str = "quiet";
        pub static SILENT: &str = "silent";
        pub static VERBOSE: &str = "verbose";
    }
    pub mod preserve_root {
        pub static PRESERVE: &str = "preserve-root";
        pub static NO_PRESERVE: &str = "no-preserve-root";
    }
    pub mod dereference {
        pub static DEREFERENCE: &str = "dereference";
        pub static NO_DEREFERENCE: &str = "no-dereference";
    }
    pub static FROM: &str = "from";
    pub static RECURSIVE: &str = "recursive";
    pub mod traverse {
        pub static TRAVERSE: &str = "H";
        pub static NO_TRAVERSE: &str = "P";
        pub static EVERY: &str = "L";
    }
    pub static REFERENCE: &str = "reference";
}

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

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let usage = get_usage();

    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

    /* First arg is the owner/group */
    let owner = matches.value_of(ARG_OWNER).unwrap();

    /* Then the list of files */
    let files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let preserve_root = matches.is_present(options::preserve_root::PRESERVE);

    let mut derefer = if matches.is_present(options::dereference::NO_DEREFERENCE) {
        1
    } else {
        0
    };

    let mut bit_flag = if matches.is_present(options::traverse::TRAVERSE) {
        FTS_COMFOLLOW | FTS_PHYSICAL
    } else if matches.is_present(options::traverse::EVERY) {
        FTS_LOGICAL
    } else {
        FTS_PHYSICAL
    };

    let recursive = matches.is_present(options::RECURSIVE);
    if recursive {
        if bit_flag == FTS_PHYSICAL {
            if derefer == 1 {
                return Err(USimpleError::new(
                    1,
                    "-R --dereference requires -H or -L".to_string(),
                ));
            }
            derefer = 0;
        }
    } else {
        bit_flag = FTS_PHYSICAL;
    }

    let verbosity = if matches.is_present(options::verbosity::CHANGES) {
        Verbosity::Changes
    } else if matches.is_present(options::verbosity::SILENT)
        || matches.is_present(options::verbosity::QUIET)
    {
        Verbosity::Silent
    } else if matches.is_present(options::verbosity::VERBOSE) {
        Verbosity::Verbose
    } else {
        Verbosity::Normal
    };

    let filter = if let Some(spec) = matches.value_of(options::FROM) {
        match parse_spec(spec)? {
            (Some(uid), None) => IfFrom::User(uid),
            (None, Some(gid)) => IfFrom::Group(gid),
            (Some(uid), Some(gid)) => IfFrom::UserGroup(uid, gid),
            (None, None) => IfFrom::All,
        }
    } else {
        IfFrom::All
    };

    let dest_uid: Option<u32>;
    let dest_gid: Option<u32>;
    if let Some(file) = matches.value_of(options::REFERENCE) {
        let meta = fs::metadata(&file)
            .map_err_context(|| format!("failed to get attributes of '{}'", file))?;
        dest_gid = Some(meta.gid());
        dest_uid = Some(meta.uid());
    } else {
        let (u, g) = parse_spec(owner)?;
        dest_uid = u;
        dest_gid = g;
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

pub fn uu_app() -> App<'static, 'static> {
    App::new(executable!())
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::verbosity::CHANGES)
                .short("c")
                .long(options::verbosity::CHANGES)
                .help("like verbose but report only when a change is made"),
        )
        .arg(Arg::with_name(options::dereference::DEREFERENCE).long(options::dereference::DEREFERENCE).help(
            "affect the referent of each symbolic link (this is the default), rather than the symbolic link itself",
        ))
        .arg(
            Arg::with_name(options::dereference::NO_DEREFERENCE)
                .short("h")
                .long(options::dereference::NO_DEREFERENCE)
                .help(
                    "affect symbolic links instead of any referenced file (useful only on systems that can change the ownership of a symlink)",
                ),
        )
        .arg(
            Arg::with_name(options::FROM)
                .long(options::FROM)
                .help(
                    "change the owner and/or group of each file only if its current owner and/or group match those specified here. Either may be omitted, in which case a match is not required for the omitted attribute",
                )
                .value_name("CURRENT_OWNER:CURRENT_GROUP"),
        )
        .arg(
            Arg::with_name(options::preserve_root::PRESERVE)
                .long(options::preserve_root::PRESERVE)
                .help("fail to operate recursively on '/'"),
        )
        .arg(
            Arg::with_name(options::preserve_root::NO_PRESERVE)
                .long(options::preserve_root::NO_PRESERVE)
                .help("do not treat '/' specially (the default)"),
        )
        .arg(
            Arg::with_name(options::verbosity::QUIET)
                .long(options::verbosity::QUIET)
                .help("suppress most error messages"),
        )
        .arg(
            Arg::with_name(options::RECURSIVE)
                .short("R")
                .long(options::RECURSIVE)
                .help("operate on files and directories recursively"),
        )
        .arg(
            Arg::with_name(options::REFERENCE)
                .long(options::REFERENCE)
                .help("use RFILE's owner and group rather than specifying OWNER:GROUP values")
                .value_name("RFILE")
                .min_values(1),
        )
        .arg(Arg::with_name(options::verbosity::SILENT).short("f").long(options::verbosity::SILENT))
        .arg(
            Arg::with_name(options::traverse::TRAVERSE)
                .short(options::traverse::TRAVERSE)
                .help("if a command line argument is a symbolic link to a directory, traverse it")
                .overrides_with_all(&[options::traverse::EVERY, options::traverse::NO_TRAVERSE]),
        )
        .arg(
            Arg::with_name(options::traverse::EVERY)
                .short(options::traverse::EVERY)
                .help("traverse every symbolic link to a directory encountered")
                .overrides_with_all(&[options::traverse::TRAVERSE, options::traverse::NO_TRAVERSE]),
        )
        .arg(
            Arg::with_name(options::traverse::NO_TRAVERSE)
                .short(options::traverse::NO_TRAVERSE)
                .help("do not traverse any symbolic links (default)")
                .overrides_with_all(&[options::traverse::TRAVERSE, options::traverse::EVERY]),
        )
        .arg(
            Arg::with_name(options::verbosity::VERBOSE)
                .long(options::verbosity::VERBOSE)
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
}

fn parse_spec(spec: &str) -> UResult<(Option<u32>, Option<u32>)> {
    let args = spec.split_terminator(':').collect::<Vec<_>>();
    let usr_only = args.len() == 1 && !args[0].is_empty();
    let grp_only = args.len() == 2 && args[0].is_empty();
    let usr_grp = args.len() == 2 && !args[0].is_empty() && !args[1].is_empty();
    let uid = if usr_only || usr_grp {
        Some(
            Passwd::locate(args[0])
                .map_err(|_| USimpleError::new(1, format!("invalid user: '{}'", spec)))?
                .uid(),
        )
    } else {
        None
    };
    let gid = if grp_only || usr_grp {
        Some(
            Group::locate(args[1])
                .map_err(|_| USimpleError::new(1, format!("invalid group: '{}'", spec)))?
                .gid(),
        )
    } else {
        None
    };
    Ok((uid, gid))
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
    fn exec(&self) -> UResult<()> {
        let mut ret = 0;
        for f in &self.files {
            ret |= self.traverse(f);
        }
        if ret != 0 {
            return Err(UError::from(ret));
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
                    if self.verbosity != Verbosity::Silent {
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
                    if self.verbosity != Verbosity::Silent {
                        show_error!("{}", e);
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
                    _ => show_error!("cannot access '{}': {}", path.display(), e),
                }
                return None;
            })
        } else {
            unwrap!(path.symlink_metadata(), e, {
                match self.verbosity {
                    Silent => (),
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_spec() {
        assert!(matches!(parse_spec(":"), Ok((None, None))));
        assert!(format!("{}", parse_spec("::").err().unwrap()).starts_with("invalid group: "));
    }
}
