// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Common functions to manage permissions

// spell-checker:ignore (jargon) TOCTOU

use crate::display::Quotable;
use crate::error::{strip_errno, UResult, USimpleError};
pub use crate::features::entries;
use crate::show_error;
use clap::{Arg, ArgMatches, Command};
use libc::{gid_t, uid_t};
use walkdir::WalkDir;

use std::io::Error as IOError;
use std::io::Result as IOResult;

use std::ffi::CString;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;

use std::os::unix::ffi::OsStrExt;
use std::path::{Path, MAIN_SEPARATOR_STR};

/// The various level of verbosity
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum VerbosityLevel {
    Silent,
    Changes,
    Verbose,
    Normal,
}
#[derive(PartialEq, Eq, Clone, Debug)]
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
                        let gid = meta.gid();
                        format!(
                            "{}\nfailed to change group of {} from {} to {}",
                            out,
                            path.quote(),
                            entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string()),
                            entries::gid2grp(dest_gid).unwrap_or_else(|_| dest_gid.to_string())
                        )
                    } else {
                        let uid = meta.uid();
                        let gid = meta.gid();
                        format!(
                            "{}\nfailed to change ownership of {} from {}:{} to {}:{}",
                            out,
                            path.quote(),
                            entries::uid2usr(uid).unwrap_or_else(|_| uid.to_string()),
                            entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string()),
                            entries::uid2usr(dest_uid).unwrap_or_else(|_| dest_uid.to_string()),
                            entries::gid2grp(dest_gid).unwrap_or_else(|_| dest_gid.to_string())
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
                    let gid = meta.gid();
                    out = if verbosity.groups_only {
                        format!(
                            "changed group of {} from {} to {}",
                            path.quote(),
                            entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string()),
                            entries::gid2grp(dest_gid).unwrap_or_else(|_| dest_gid.to_string())
                        )
                    } else {
                        let gid = meta.gid();
                        let uid = meta.uid();
                        format!(
                            "changed ownership of {} from {}:{} to {}:{}",
                            path.quote(),
                            entries::uid2usr(uid).unwrap_or_else(|_| uid.to_string()),
                            entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string()),
                            entries::uid2usr(dest_uid).unwrap_or_else(|_| dest_uid.to_string()),
                            entries::gid2grp(dest_gid).unwrap_or_else(|_| dest_gid.to_string())
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
                    entries::uid2usr(dest_uid).unwrap_or_else(|_| dest_uid.to_string()),
                    entries::gid2grp(dest_gid).unwrap_or_else(|_| dest_gid.to_string())
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
    pub raw_owner: String, // The owner of the file as input by the user in the command line.
    pub traverse_symlinks: TraverseSymlinks,
    pub verbosity: Verbosity,
    pub filter: IfFrom,
    pub files: Vec<String>,
    pub recursive: bool,
    pub preserve_root: bool,
    pub dereference: bool,
}

#[cfg(test)]
pub fn check_root(path: &Path, would_recurse_symlink: bool) -> bool {
    is_root(path, would_recurse_symlink)
}

/// In the context of chown and chgrp, check whether we are in a "preserve-root" scenario.
///
/// In particular, we want to prohibit further traversal only if:
///     (--preserve-root and -R present) &&
///     (path canonicalizes to "/") &&
///     (
///         (path is a symlink && would traverse/recurse this symlink) ||
///         (path is not a symlink)
///     )
/// The first clause is checked by the caller, the second and third clause is checked here.
/// The caller has to evaluate -P/-H/-L into 'would_recurse_symlink'.
/// Recall that canonicalization resolves both relative paths (e.g. "..") and symlinks.
fn is_root(path: &Path, would_traverse_symlink: bool) -> bool {
    // The third clause can be evaluated without any syscalls, so we do that first.
    // If we would_recurse_symlink, then the clause is true no matter whether the path is a symlink
    // or not. Otherwise, we only need to check here if the path can syntactically be a symlink:
    if !would_traverse_symlink {
        // We cannot check path.is_dir() here, as this would resolve symlinks,
        // which we need to avoid here.
        // All directory-ish paths match "*/", except ".", "..", "*/.", and "*/..".
        let looks_like_dir = match path.as_os_str().to_str() {
            // If it contains special character, prefer to err on the side of safety, i.e. forbidding the chown operation:
            None => false,
            Some(".") | Some("..") => true,
            Some(path_str) => {
                (path_str.ends_with(MAIN_SEPARATOR_STR))
                    || (path_str.ends_with(&format!("{}.", MAIN_SEPARATOR_STR)))
                    || (path_str.ends_with(&format!("{}..", MAIN_SEPARATOR_STR)))
            }
        };
        // TODO: Once we reach MSRV 1.74.0, replace this abomination by something simpler, e.g. this:
        // let path_bytes = path.as_os_str().as_encoded_bytes();
        // let looks_like_dir = path_bytes == [b'.']
        //     || path_bytes == [b'.', b'.']
        //     || path_bytes.ends_with(&[MAIN_SEPARATOR as u8])
        //     || path_bytes.ends_with(&[MAIN_SEPARATOR as u8, b'.'])
        //     || path_bytes.ends_with(&[MAIN_SEPARATOR as u8, b'.', b'.']);
        if !looks_like_dir {
            return false;
        }
    }

    // FIXME: TOCTOU bug! canonicalize() runs at a different time than WalkDir's recursion decision.
    // However, we're forced to make the decision whether to warn about --preserve-root
    // *before* even attempting to chown the path, let alone doing the stat inside WalkDir.
    if let Ok(p) = path.canonicalize() {
        let path_buf = path.to_path_buf();
        if p.parent().is_none() {
            if path_buf.as_os_str() == "/" {
                show_error!("it is dangerous to operate recursively on '/'");
            } else {
                show_error!(
                    "it is dangerous to operate recursively on {} (same as '/')",
                    path_buf.quote()
                );
            }
            show_error!("use --no-preserve-root to override this failsafe");
            return true;
        }
    }

    false
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

    #[allow(clippy::cognitive_complexity)]
    fn traverse<P: AsRef<Path>>(&self, root: P) -> i32 {
        let path = root.as_ref();
        let meta = match self.obtain_meta(path, self.dereference) {
            Some(m) => m,
            _ => {
                if self.verbosity.level == VerbosityLevel::Verbose {
                    println!(
                        "failed to change ownership of {} to {}",
                        path.quote(),
                        self.raw_owner
                    );
                }
                return 1;
            }
        };

        if self.recursive
            && self.preserve_root
            && is_root(path, self.traverse_symlinks != TraverseSymlinks::None)
        {
            // Fail-fast, do not attempt to recurse.
            return 1;
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
            self.print_verbose_ownership_retained_as(
                path,
                meta.uid(),
                self.dest_gid.map(|_| meta.gid()),
            );
            0
        };

        if self.recursive {
            ret | self.dive_into(&root)
        } else {
            ret
        }
    }

    #[allow(clippy::cognitive_complexity)]
    fn dive_into<P: AsRef<Path>>(&self, root: P) -> i32 {
        let root = root.as_ref();

        // walkdir always dereferences the root directory, so we have to check it ourselves
        if self.traverse_symlinks == TraverseSymlinks::None && root.is_symlink() {
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

            if self.preserve_root && is_root(path, self.traverse_symlinks == TraverseSymlinks::All)
            {
                // Fail-fast, do not recurse further.
                return 1;
            }

            if !self.matched(meta.uid(), meta.gid()) {
                self.print_verbose_ownership_retained_as(
                    path,
                    meta.uid(),
                    self.dest_gid.map(|_| meta.gid()),
                );
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

    fn print_verbose_ownership_retained_as(&self, path: &Path, uid: u32, gid: Option<u32>) {
        if self.verbosity.level == VerbosityLevel::Verbose {
            match (self.dest_uid, self.dest_gid, gid) {
                (Some(_), Some(_), Some(gid)) => {
                    println!(
                        "ownership of {} retained as {}:{}",
                        path.quote(),
                        entries::uid2usr(uid).unwrap_or_else(|_| uid.to_string()),
                        entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string()),
                    );
                }
                (None, Some(_), Some(gid)) => {
                    println!(
                        "ownership of {} retained as {}",
                        path.quote(),
                        entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string()),
                    );
                }
                (_, _, _) => {
                    println!(
                        "ownership of {} retained as {}",
                        path.quote(),
                        entries::uid2usr(uid).unwrap_or_else(|_| uid.to_string()),
                    );
                }
            }
        }
    }
}

pub mod options {
    pub const HELP: &str = "help";
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

pub struct GidUidOwnerFilter {
    pub dest_gid: Option<u32>,
    pub dest_uid: Option<u32>,
    pub raw_owner: String,
    pub filter: IfFrom,
}
type GidUidFilterOwnerParser = fn(&ArgMatches) -> UResult<GidUidOwnerFilter>;

/// Base implementation for `chgrp` and `chown`.
///
/// An argument called `add_arg_if_not_reference` will be added to `command` if
/// `args` does not contain the `--reference` option.
/// `parse_gid_uid_and_filter` will be called to obtain the target gid and uid, and the filter,
/// from `ArgMatches`.
/// `groups_only` determines whether verbose output will only mention the group.
#[allow(clippy::cognitive_complexity)]
pub fn chown_base(
    mut command: Command,
    args: impl crate::Args,
    add_arg_if_not_reference: &'static str,
    parse_gid_uid_and_filter: GidUidFilterOwnerParser,
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
                .required(true),
        );
    }
    command = command.arg(
        Arg::new(options::ARG_FILES)
            .value_name(options::ARG_FILES)
            .value_hint(clap::ValueHint::FilePath)
            .action(clap::ArgAction::Append)
            .required(true)
            .num_args(1..),
    );
    let matches = command.try_get_matches_from(args)?;

    let files: Vec<String> = matches
        .get_many::<String>(options::ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let preserve_root = matches.get_flag(options::preserve_root::PRESERVE);

    let mut dereference = if matches.get_flag(options::dereference::DEREFERENCE) {
        Some(true)
    } else if matches.get_flag(options::dereference::NO_DEREFERENCE) {
        Some(false)
    } else {
        None
    };

    let mut traverse_symlinks = if matches.get_flag(options::traverse::TRAVERSE) {
        TraverseSymlinks::First
    } else if matches.get_flag(options::traverse::EVERY) {
        TraverseSymlinks::All
    } else {
        TraverseSymlinks::None
    };

    let recursive = matches.get_flag(options::RECURSIVE);
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

    let verbosity_level = if matches.get_flag(options::verbosity::CHANGES) {
        VerbosityLevel::Changes
    } else if matches.get_flag(options::verbosity::SILENT)
        || matches.get_flag(options::verbosity::QUIET)
    {
        VerbosityLevel::Silent
    } else if matches.get_flag(options::verbosity::VERBOSE) {
        VerbosityLevel::Verbose
    } else {
        VerbosityLevel::Normal
    };
    let GidUidOwnerFilter {
        dest_gid,
        dest_uid,
        raw_owner,
        filter,
    } = parse_gid_uid_and_filter(&matches)?;

    let executor = ChownExecutor {
        traverse_symlinks,
        dest_gid,
        dest_uid,
        raw_owner,
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

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    #[cfg(unix)]
    use std::os::unix;
    use std::path::{Component, PathBuf};
    #[cfg(unix)]
    use tempfile::tempdir;

    #[test]
    fn test_empty_string() {
        let path = PathBuf::new();
        assert_eq!(path.to_str(), Some(""));
        // The main point to test here is that we don't crash.
        // The result should be 'false', to avoid unnecessary and confusing warnings.
        assert!(!is_root(&path, false));
        assert!(!is_root(&path, true));
    }

    #[allow(clippy::needless_borrow)]
    #[cfg(unix)]
    #[test]
    fn test_literal_root() {
        let component = Component::RootDir;
        let path: &Path = component.as_ref();
        assert_eq!(
            path.to_str(),
            Some("/"),
            "cfg(unix) but using non-unix path delimiters?!"
        );
        // Must return true, this is the main scenario that --preserve-root shall prevent.
        assert!(is_root(&path, false));
        assert!(is_root(&path, true));
    }

    #[cfg(unix)]
    #[test]
    fn test_symlink_slash() {
        let temp_dir = tempdir().unwrap();
        let symlink_path = temp_dir.path().join("symlink");
        unix::fs::symlink(PathBuf::from("/"), symlink_path).unwrap();
        let symlink_path_slash = temp_dir.path().join("symlink/");
        // Must return true, we're about to "accidentally" recurse on "/",
        // since "symlink/" always counts as an already-entered directory
        // Output from GNU:
        //   $ chown --preserve-root -RH --dereference $(id -u) slink-to-root/
        //   chown: it is dangerous to operate recursively on 'slink-to-root/' (same as '/')
        //   chown: use --no-preserve-root to override this failsafe
        //   [$? = 1]
        //   $ chown --preserve-root -RH --no-dereference $(id -u) slink-to-root/
        //   chown: it is dangerous to operate recursively on 'slink-to-root/' (same as '/')
        //   chown: use --no-preserve-root to override this failsafe
        //   [$? = 1]
        assert!(is_root(&symlink_path_slash, false));
        assert!(is_root(&symlink_path_slash, true));
    }

    #[cfg(unix)]
    #[test]
    fn test_symlink_no_slash() {
        // This covers both the commandline-argument case and the recursion case.
        let temp_dir = tempdir().unwrap();
        let symlink_path = temp_dir.path().join("symlink");
        unix::fs::symlink(PathBuf::from("/"), &symlink_path).unwrap();
        // Only return true  we're about to "accidentally" recurse on "/".
        assert!(!is_root(&symlink_path, false));
        assert!(is_root(&symlink_path, true));
    }
}
