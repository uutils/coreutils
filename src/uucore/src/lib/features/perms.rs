// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Common functions to manage permissions

use crate::display::Quotable;
use crate::error::strip_errno;
use crate::error::UResult;
use crate::error::USimpleError;
pub use crate::features::entries;
use crate::fs::resolve_relative_path;
use crate::show_error;
use clap::Arg;
use clap::ArgMatches;
use clap::Command;
use libc::{self, gid_t, uid_t};
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
            libc::lchown(s.as_ptr(), uid, gid)
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
                    "changing {} of {}: {}",
                    if verbosity.groups_only {
                        "group"
                    } else {
                        "ownership"
                    },
                    path.quote(),
                    e
                );
                if level == VerbosityLevel::Verbose {
                    out = if verbosity.groups_only {
                        format!(
                            "{}\nfailed to change group of {} from {} to {}",
                            out,
                            path.quote(),
                            entries::gid2grp(meta.gid()).unwrap(),
                            entries::gid2grp(dest_gid).unwrap()
                        )
                    } else {
                        format!(
                            "{}\nfailed to change ownership of {} from {}:{} to {}:{}",
                            out,
                            path.quote(),
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
                            "changed group of {} from {} to {}",
                            path.quote(),
                            entries::gid2grp(meta.gid()).unwrap(),
                            entries::gid2grp(dest_gid).unwrap()
                        )
                    } else {
                        format!(
                            "changed ownership of {} from {}:{} to {}:{}",
                            path.quote(),
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
                    "group of {} retained as {}",
                    path.quote(),
                    entries::gid2grp(dest_gid).unwrap_or_default()
                )
            } else {
                format!(
                    "ownership of {} retained as {}:{}",
                    path.quote(),
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

#[derive(PartialEq, Eq)]
pub enum TraverseSymlinks {
    None,
    First,
    All,
}

pub struct ChownExecutor {
    pub dest_uid: Option<u32>,
    pub dest_gid: Option<u32>,
    pub traverse_symlinks: TraverseSymlinks,
    pub verbosity: Verbosity,
    pub filter: IfFrom,
    pub files: Vec<String>,
    pub recursive: bool,
    pub preserve_root: bool,
    pub dereference: bool,
}

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
        let path = root.as_ref();
        let meta = match self.obtain_meta(path, self.dereference) {
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
            let may_exist = if self.dereference {
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
                self.dereference,
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
        let root = root.as_ref();

        // walkdir always dereferences the root directory, so we have to check it ourselves
        // TODO: replace with `root.is_symlink()` once it is stable
        if self.traverse_symlinks == TraverseSymlinks::None
            && std::fs::symlink_metadata(root)
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false)
        {
            return 0;
        }

        let mut ret = 0;
        let mut iterator = WalkDir::new(root)
            .follow_links(self.traverse_symlinks == TraverseSymlinks::All)
            .min_depth(1)
            .into_iter();
        // We can't use a for loop because we need to manipulate the iterator inside the loop.
        while let Some(entry) = iterator.next() {
            let entry = match entry {
                Err(e) => {
                    ret = 1;
                    if let Some(path) = e.path() {
                        show_error!(
                            "cannot access '{}': {}",
                            path.display(),
                            if let Some(error) = e.io_error() {
                                strip_errno(error)
                            } else {
                                "Too many levels of symbolic links".into()
                            }
                        );
                    } else {
                        show_error!("{}", e);
                    }
                    continue;
                }
                Ok(entry) => entry,
            };
            let path = entry.path();
            let meta = match self.obtain_meta(path, self.dereference) {
                Some(m) => m,
                _ => {
                    ret = 1;
                    if entry.file_type().is_dir() {
                        // Instruct walkdir to skip this directory to avoid getting another error
                        // when walkdir tries to query the children of this directory.
                        iterator.skip_current_dir();
                    }
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
                self.dereference,
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
            path.metadata()
        } else {
            path.symlink_metadata()
        };
        match meta {
            Err(e) => {
                match self.verbosity.level {
                    VerbosityLevel::Silent => (),
                    _ => show_error!(
                        "cannot {} {}: {}",
                        if follow { "dereference" } else { "access" },
                        path.quote(),
                        strip_errno(&e)
                    ),
                }
                None
            }
            Ok(meta) => Some(meta),
        }
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

pub mod options {
    pub mod verbosity {
        pub const CHANGES: &str = "changes";
        pub const QUIET: &str = "quiet";
        pub const SILENT: &str = "silent";
        pub const VERBOSE: &str = "verbose";
    }
    pub mod preserve_root {
        pub const PRESERVE: &str = "preserve-root";
        pub const NO_PRESERVE: &str = "no-preserve-root";
    }
    pub mod dereference {
        pub const DEREFERENCE: &str = "dereference";
        pub const NO_DEREFERENCE: &str = "no-dereference";
    }
    pub const FROM: &str = "from";
    pub const RECURSIVE: &str = "recursive";
    pub mod traverse {
        pub const TRAVERSE: &str = "H";
        pub const NO_TRAVERSE: &str = "P";
        pub const EVERY: &str = "L";
    }
    pub const REFERENCE: &str = "reference";
    pub const ARG_OWNER: &str = "OWNER";
    pub const ARG_GROUP: &str = "GROUP";
    pub const ARG_FILES: &str = "FILE";
}

type GidUidFilterParser = fn(&ArgMatches) -> UResult<(Option<u32>, Option<u32>, IfFrom)>;

/// Base implementation for `chgrp` and `chown`.
///
/// An argument called `add_arg_if_not_reference` will be added to `command` if
/// `args` does not contain the `--reference` option.
/// `parse_gid_uid_and_filter` will be called to obtain the target gid and uid, and the filter,
/// from `ArgMatches`.
/// `groups_only` determines whether verbose output will only mention the group.
pub fn chown_base<'a>(
    mut command: Command<'a>,
    args: impl crate::Args,
    add_arg_if_not_reference: &'a str,
    parse_gid_uid_and_filter: GidUidFilterParser,
    groups_only: bool,
) -> UResult<()> {
    let args: Vec<_> = args.collect();
    let mut reference = false;
    let mut help = false;
    // stop processing options on --
    for arg in args.iter().take_while(|s| *s != "--") {
        if arg.to_string_lossy().starts_with("--reference=") || arg == "--reference" {
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
        // arg_group is only required if
        command = command.arg(
            Arg::new(add_arg_if_not_reference)
                .value_name(add_arg_if_not_reference)
                .required(true)
                .takes_value(true)
                .multiple_occurrences(false),
        );
    }
    command = command.arg(
        Arg::new(options::ARG_FILES)
            .value_name(options::ARG_FILES)
            .multiple_occurrences(true)
            .takes_value(true)
            .required(true)
            .min_values(1),
    );
    let matches = command.get_matches_from(args);

    let files: Vec<String> = matches
        .values_of(options::ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let preserve_root = matches.is_present(options::preserve_root::PRESERVE);

    let mut dereference = if matches.is_present(options::dereference::DEREFERENCE) {
        Some(true)
    } else if matches.is_present(options::dereference::NO_DEREFERENCE) {
        Some(false)
    } else {
        None
    };

    let mut traverse_symlinks = if matches.is_present(options::traverse::TRAVERSE) {
        TraverseSymlinks::First
    } else if matches.is_present(options::traverse::EVERY) {
        TraverseSymlinks::All
    } else {
        TraverseSymlinks::None
    };

    let recursive = matches.is_present(options::RECURSIVE);
    if recursive {
        if traverse_symlinks == TraverseSymlinks::None {
            if dereference == Some(true) {
                return Err(USimpleError::new(1, "-R --dereference requires -H or -L"));
            }
            dereference = Some(false);
        }
    } else {
        traverse_symlinks = TraverseSymlinks::None;
    }

    let verbosity_level = if matches.is_present(options::verbosity::CHANGES) {
        VerbosityLevel::Changes
    } else if matches.is_present(options::verbosity::SILENT)
        || matches.is_present(options::verbosity::QUIET)
    {
        VerbosityLevel::Silent
    } else if matches.is_present(options::verbosity::VERBOSE) {
        VerbosityLevel::Verbose
    } else {
        VerbosityLevel::Normal
    };
    let (dest_gid, dest_uid, filter) = parse_gid_uid_and_filter(&matches)?;

    let executor = ChownExecutor {
        traverse_symlinks,
        dest_gid,
        dest_uid,
        verbosity: Verbosity {
            groups_only,
            level: verbosity_level,
        },
        recursive,
        dereference: dereference.unwrap_or(true),
        preserve_root,
        files,
        filter,
    };
    executor.exec()
}
