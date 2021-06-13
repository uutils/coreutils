// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) COMFOLLOW Chgrper RFILE RFILE's derefer dgid nonblank nonprint nonprinting

mod app;

#[macro_use]
extern crate uucore;
use app::get_app;
pub use uucore::entries;
use uucore::fs::resolve_relative_path;
use uucore::libc::gid_t;
use uucore::perms::{wrap_chgrp, Verbosity};

use clap::Arg;

extern crate walkdir;
use walkdir::WalkDir;

use std::fs;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;

use std::path::Path;
use uucore::InvalidEncodingHandling;

use crate::app::options;

const FTS_COMFOLLOW: u8 = 1;
const FTS_PHYSICAL: u8 = 1 << 1;
const FTS_LOGICAL: u8 = 1 << 2;

fn get_usage() -> String {
    format!(
        "{0} [OPTION]... GROUP FILE...\n    {0} [OPTION]... --reference=RFILE FILE...",
        executable!()
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let usage = get_usage();

    let mut app = get_app(executable!()).usage(&usage[..]);

    // we change the positional args based on whether
    // --reference was used.
    let mut reference = false;
    let mut help = false;
    // stop processing options on --
    for arg in args.iter().take_while(|s| *s != "--") {
        if arg.starts_with("--reference=") || arg == "--reference" {
            reference = true;
        } else if arg == "--help" {
            // we stop processing once we see --help,
            // as it doesn't matter if we've seen reference or not
            help = true;
            break;
        }
    }

    if help || !reference {
        // add both positional arguments
        app = app.arg(
            Arg::with_name(options::ARG_GROUP)
                .value_name(options::ARG_GROUP)
                .required(true)
                .takes_value(true)
                .multiple(false),
        )
    }
    app = app.arg(
        Arg::with_name(options::ARG_FILES)
            .value_name(options::ARG_FILES)
            .multiple(true)
            .takes_value(true)
            .required(true)
            .min_values(1),
    );

    let matches = app.get_matches_from(args);

    /* Get the list of files */
    let files: Vec<String> = matches
        .values_of(options::ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let preserve_root = matches.is_present(options::preserve_root::PRESERVE);

    let mut derefer = if matches.is_present(options::dereference::DEREFERENCE) {
        1
    } else if matches.is_present(options::dereference::NO_DEREFERENCE) {
        0
    } else {
        -1
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
                show_error!("-R --dereference requires -H or -L");
                return 1;
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

    let dest_gid: u32;
    if let Some(file) = matches.value_of(options::REFERENCE) {
        match fs::metadata(&file) {
            Ok(meta) => {
                dest_gid = meta.gid();
            }
            Err(e) => {
                show_error!("failed to get attributes of '{}': {}", file, e);
                return 1;
            }
        }
    } else {
        let group = matches.value_of(options::ARG_GROUP).unwrap_or_default();
        match entries::grp2gid(group) {
            Ok(g) => {
                dest_gid = g;
            }
            _ => {
                show_error!("invalid group: {}", group);
                return 1;
            }
        }
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
                    show_error!("it is dangerous to operate recursively on '/'");
                    show_error!("use --no-preserve-root to override this failsafe");
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
                show_error!("{}", n);
                0
            }
            Err(e) => {
                if self.verbosity != Verbosity::Silent {
                    show_error!("{}", e);
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

            ret = match wrap_chgrp(path, &meta, self.dest_gid, follow, self.verbosity.clone()) {
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
}
