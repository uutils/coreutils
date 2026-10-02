// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (jargon) TOCTOU fchownat fchown egid

//! Common functions to manage permissions

use crate::display::Quotable;
use crate::error::{UResult, USimpleError, strip_errno};
pub use crate::features::entries;
use crate::{show_error, translate};

use clap::{Arg, ArgMatches, Command};

use libc::{gid_t, uid_t};
use options::traverse;
#[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
use std::collections::HashSet;
#[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
use std::ffi::OsStr;
use std::ffi::OsString;

#[cfg(any(target_os = "aix", target_os = "hurd", target_os = "redox"))]
use walkdir::WalkDir;

#[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
use crate::features::fs::FileInformation;
use crate::features::fs::path_is_root_dir;
#[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
use crate::features::safe_traversal::{
    CAN_PIN_SPECIAL_FILES, CAN_PIN_SYMLINKS, DirFd, FileInfo, Metadata as TraversalMetadata,
    PinnedFile, SymlinkBehavior,
};

use std::ffi::CString;
use std::fs::Metadata;
use std::io::Error as IOError;
use std::io::Result as IOResult;
use std::os::unix::fs::MetadataExt;

use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
enum PermsError {
    #[error("{}: {}", translate!("common-write-error"), strip_errno(.0))]
    Write(IOError),
}

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

impl Default for Verbosity {
    fn default() -> Self {
        Self {
            groups_only: false,
            level: VerbosityLevel::Normal,
        }
    }
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
    let path = path.as_ref();
    report_chown(path, meta, dest_uid, dest_gid, verbosity, |uid, gid| {
        chown(path, uid, gid, follow)
    })
}

/// Change the owner of the file `meta` describes through `change`, and describe
/// the outcome the way [`wrap_chown`] does.
fn report_chown<M: MetadataExt>(
    path: &Path,
    meta: &M,
    dest_uid: Option<u32>,
    dest_gid: Option<u32>,
    verbosity: Verbosity,
    change: impl FnOnce(uid_t, gid_t) -> IOResult<()>,
) -> Result<String, String> {
    let dest_uid = dest_uid.unwrap_or_else(|| meta.uid());
    let dest_gid = dest_gid.unwrap_or_else(|| meta.gid());
    let mut out: String = String::new();

    if let Err(e) = change(dest_uid, dest_gid) {
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
                    strip_errno(&e),
                );
                if level == VerbosityLevel::Verbose {
                    let failed =
                        failed_change_line(path, meta, dest_uid, dest_gid, verbosity.groups_only);
                    out = format!("{out}\n{failed}");
                }
            }
        }
        return Err(out);
    }

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
        }
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

    Ok(out)
}

/// The verbose line for a change of the file `meta` describes that did not happen.
fn failed_change_line<M: MetadataExt>(
    path: &Path,
    meta: &M,
    dest_uid: uid_t,
    dest_gid: gid_t,
    groups_only: bool,
) -> String {
    let gid = meta.gid();
    if groups_only {
        format!(
            "failed to change group of {} from {} to {}",
            path.quote(),
            entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string()),
            entries::gid2grp(dest_gid).unwrap_or_else(|_| dest_gid.to_string())
        )
    } else {
        let uid = meta.uid();
        format!(
            "failed to change ownership of {} from {}:{} to {}:{}",
            path.quote(),
            entries::uid2usr(uid).unwrap_or_else(|_| uid.to_string()),
            entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string()),
            entries::uid2usr(dest_uid).unwrap_or_else(|_| dest_uid.to_string()),
            entries::gid2grp(dest_gid).unwrap_or_else(|_| dest_gid.to_string())
        )
    }
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
    pub files: Vec<OsString>,
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
/// Prohibit further traversal only if:
///     (--preserve-root and -R present) &&
///     (path *is* "/" by (st_dev, st_ino), so a bind mount of "/" counts too) &&
///     (
///         (path is a symlink && would traverse/recurse this symlink) ||
///         (path is not a symlink)
///     )
/// The first clause is checked by the caller, the second and third here.
/// The caller has to evaluate -P/-H/-L into 'would_recurse_symlink'.
fn is_root(path: &Path, would_traverse_symlink: bool) -> bool {
    // Compare by (st_dev, st_ino), not name: a bind mount of "/" is an ordinary
    // directory whose path never resolves to "/", so the old syntactic "looks
    // like a directory?" pre-filter waved it through. `would_traverse_symlink`
    // says whether a symlink to "/" here would be followed (only then is it root).
    //
    // FIXME: TOCTOU bug! This stat runs at a different time than the recursion
    // decision it guards; GNU avoids the window by reusing fts's `struct stat`.
    if !path_is_root_dir(path, would_traverse_symlink) {
        return false;
    }

    if path.as_os_str() == "/" {
        show_error!("it is dangerous to operate recursively on '/'");
    } else {
        show_error!(
            "it is dangerous to operate recursively on {} (same as '/')",
            path.quote()
        );
    }
    show_error!("use --no-preserve-root to override this failsafe");
    true
}

/// Whether `dir_fd` refers to the very object `meta` describes.
///
/// A pathname can be re-pointed between the stat and the open, so comparing
/// (device, inode) is what detects the swap. The descriptor cannot be re-pointed after.
#[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
fn fd_is(dir_fd: &DirFd, meta: &Metadata) -> IOResult<bool> {
    let opened = FileInfo::from_stat(&dir_fd.fstat()?);
    Ok(opened == FileInfo::new(meta.dev(), meta.ino()))
}

/// Whether the file `meta` describes can be held open by [`PinnedFile`].
#[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
// mode_t is u16 on macOS and u32 on Linux
#[allow(clippy::unnecessary_cast)]
fn can_hold<M: MetadataExt>(meta: &M, follow: bool) -> bool {
    match meta.mode() as libc::mode_t & libc::S_IFMT {
        libc::S_IFSOCK | libc::S_IFCHR | libc::S_IFBLK => CAN_PIN_SPECIAL_FILES,
        libc::S_IFLNK => follow || CAN_PIN_SYMLINKS,
        _ => true,
    }
}

pub fn get_metadata(file: &Path, follow: bool) -> std::io::Result<Metadata> {
    if follow {
        file.metadata()
    } else {
        file.symlink_metadata()
    }
}

impl ChownExecutor {
    pub fn exec(&self) -> UResult<()> {
        use std::io::Write;
        let mut ret = 0;
        for f in &self.files {
            ret |= self.traverse(f);
        }
        if let Err(e) = std::io::stdout().flush() {
            show_error!("{}", PermsError::Write(e));
            ret |= 1;
        }
        if ret != 0 {
            return Err(ret.into());
        }
        Ok(())
    }

    #[allow(clippy::cognitive_complexity)]
    fn traverse<P: AsRef<Path>>(&self, root: P) -> i32 {
        let path = root.as_ref();
        let Some(meta) = self.obtain_meta(path, self.dereference) else {
            if self.verbosity.level == VerbosityLevel::Verbose {
                self.write_verbose_line(&format!(
                    "failed to change ownership of {} to {}",
                    path.quote(),
                    self.raw_owner
                ));
            }
            return 1;
        };

        if self.recursive
            && self.preserve_root
            && is_root(path, self.traverse_symlinks != TraverseSymlinks::None)
        {
            // Fail-fast, do not attempt to recurse.
            return 1;
        }

        // Resolve the operand once. `--preserve-root` and the directory-vs-file
        // classification were decided on `meta`; re-opening the pathname would
        // let a swap apply those decisions to a different object.
        #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
        // We cannot check path.is_dir() here, as this would resolve symlinks
        let operand_fd = if meta.is_dir() {
            match DirFd::open(path, SymlinkBehavior::Follow) {
                Ok(dir_fd) => match fd_is(&dir_fd, &meta) {
                    Ok(true) => Some(dir_fd),
                    Ok(false) => {
                        if self.verbosity.level != VerbosityLevel::Silent {
                            show_error!(
                                "{}",
                                translate!("perms-cannot-access-replaced", "file" => path.quote())
                            );
                        }
                        return 1;
                    }
                    Err(e) => {
                        if self.verbosity.level != VerbosityLevel::Silent {
                            show_error!(
                                "{}",
                                translate!("perms-cannot-access", "file" => path.quote(), "error" => strip_errno(&e))
                            );
                        }
                        return 1;
                    }
                },
                Err(_e) => {
                    // Don't show error here - let safe_dive_into handle directory traversal
                    // errors. This prevents duplicate error messages.
                    None
                }
            }
        } else {
            None
        };

        #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
        let ret = match operand_fd.as_ref() {
            // The descriptor is the very directory `meta` describes.
            Some(dir_fd) => {
                self.chown_if_matched(path, &meta, |uid, gid| dir_fd.fchown(Some(uid), Some(gid)))
            }
            None => self.chown_operand(path, &meta),
        };
        #[cfg(any(target_os = "aix", target_os = "hurd", target_os = "redox"))]
        let ret = self.chown_operand(path, &meta);

        if self.recursive {
            #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
            {
                ret | self.safe_dive_into(&root, &meta, operand_fd)
            }
            #[cfg(any(target_os = "aix", target_os = "hurd", target_os = "redox"))]
            {
                ret | self.dive_into(&root)
            }
        } else {
            ret
        }
    }

    /// Change the operand `path`, which `meta` describes, unless `--from` rules it out.
    ///
    /// `--from` decides on the file's current owner, so that decision and the
    /// change have to concern the same file: see [`Self::hold`]. Without
    /// `--from` the change goes to whatever the name refers to, which is all
    /// that was asked.
    fn chown_operand(&self, path: &Path, meta: &Metadata) -> i32 {
        #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
        if !matches!(self.filter, IfFrom::All) && self.matched(meta.uid(), meta.gid()) {
            match self.hold(meta, self.dereference, || {
                PinnedFile::open(path, self.dereference.into())
            }) {
                Ok(Some((_, held_meta))) if self.replaced(meta, &held_meta) => {
                    return self.report_replaced(path, meta);
                }
                Ok(Some((file, held_meta))) => {
                    return self.chown_if_matched(path, &held_meta, |uid, gid| {
                        file.chown(Some(uid), Some(gid))
                    });
                }
                Err(e) => return self.show_cannot_access(path, &e),
                Ok(None) => {}
            }
        }
        self.chown_if_matched(path, meta, |uid, gid| {
            chown(path, uid, gid, self.dereference)
        })
    }

    /// Hold open the file `meta` describes, which passed `--from`, through
    /// `open`, so that the change goes to that very file: by name, a rename in
    /// between could send it to another one. Returns the file held and its
    /// metadata read through the descriptor, or `None` if it is to be changed
    /// by name, or the error to report instead.
    ///
    /// Changing by name instead is left to an unprivileged caller, who can
    /// only change files it owns, for a file that cannot be held or that it
    /// cannot open. Root is refused such a file.
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    fn hold<M: MetadataExt>(
        &self,
        meta: &M,
        follow: bool,
        open: impl FnOnce() -> IOResult<PinnedFile>,
    ) -> IOResult<Option<(PinnedFile, TraversalMetadata)>> {
        Self::hold_as(!nix::unistd::geteuid().is_root(), meta, follow, open)
    }

    /// Whether the file held is not the one `meta` described when it passed
    /// `--from`, or no longer passes it.
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    fn replaced<M: MetadataExt>(&self, meta: &M, held: &TraversalMetadata) -> bool {
        (held.dev(), held.ino()) != (meta.dev(), meta.ino())
            || !self.matched(held.uid(), held.gid())
    }

    /// Leave alone a file found replaced after passing `--from`. As GNU does,
    /// this fails without an error message, and only `-v` says so. Returns 1.
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    fn report_replaced<M: MetadataExt>(&self, path: &Path, meta: &M) -> i32 {
        if self.verbosity.level == VerbosityLevel::Verbose {
            self.write_verbose_line(&failed_change_line(
                path,
                meta,
                self.dest_uid.unwrap_or_else(|| meta.uid()),
                self.dest_gid.unwrap_or_else(|| meta.gid()),
                self.verbosity.groups_only,
            ));
        }
        1
    }

    /// [`Self::hold`], for a caller that is `unprivileged` or root.
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    fn hold_as<M: MetadataExt>(
        unprivileged: bool,
        meta: &M,
        follow: bool,
        open: impl FnOnce() -> IOResult<PinnedFile>,
    ) -> IOResult<Option<(PinnedFile, TraversalMetadata)>> {
        if !can_hold(meta, follow) {
            return if unprivileged {
                Ok(None)
            } else {
                Err(IOError::from_raw_os_error(libc::EOPNOTSUPP))
            };
        }
        let held = open().and_then(|file| {
            let held_meta = file.metadata()?;
            Ok((file, held_meta))
        });
        match held {
            Ok(held) => Ok(Some(held)),
            // Where holding needs read access, an owner may have taken it away.
            Err(e) if unprivileged && e.raw_os_error() == Some(libc::EACCES) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Report that `path` cannot be accessed. Returns 1.
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    fn show_cannot_access(&self, path: &Path, e: &IOError) -> i32 {
        if self.verbosity.level != VerbosityLevel::Silent {
            show_error!(
                "{}",
                translate!("perms-cannot-access", "file" => path.quote(), "error" => strip_errno(e))
            );
        }
        1
    }

    /// Change, through `change`, the owner of the file `meta` describes if it
    /// passes `--from`, reporting the outcome as [`wrap_chown`] does.
    fn chown_if_matched<M: MetadataExt>(
        &self,
        path: &Path,
        meta: &M,
        change: impl FnOnce(uid_t, gid_t) -> IOResult<()>,
    ) -> i32 {
        if !self.matched(meta.uid(), meta.gid()) {
            return self.print_verbose_ownership_retained_as(
                path,
                meta.uid(),
                self.dest_gid.map(|_| meta.gid()),
            );
        }
        match report_chown(
            path,
            meta,
            self.dest_uid,
            self.dest_gid,
            self.verbosity.clone(),
            change,
        ) {
            // GNU: informational verbose/changes lines go to stdout.
            Ok(n) if n.is_empty() => 0,
            Ok(n) => self.write_verbose_line(&n),
            Err(e) => {
                if self.verbosity.level != VerbosityLevel::Silent {
                    show_error!("{e}");
                }
                1
            }
        }
    }

    /// `operand_fd` is the descriptor `traverse` opened and verified against `meta`.
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    fn safe_dive_into<P: AsRef<Path>>(
        &self,
        root: P,
        meta: &Metadata,
        operand_fd: Option<DirFd>,
    ) -> i32 {
        let root = root.as_ref();

        // Classify from `meta`, not a fresh lookup: `meta` already honours the
        // dereference policy, and asking again is what would let the operand be swapped.
        if !meta.is_dir() {
            // No children to visit, matching WalkDir's min_depth(1).
            return 0;
        }

        let dir_fd = if let Some(dir_fd) = operand_fd {
            dir_fd
        } else {
            // The open in `traverse` failed; report it here, once.
            let Some(dir_fd) = self.try_open_dir(root) else {
                return 1;
            };
            if !fd_is(&dir_fd, meta).unwrap_or(false) {
                if self.verbosity.level != VerbosityLevel::Silent {
                    show_error!(
                        "{}",
                        translate!("perms-cannot-access-replaced", "file" => root.quote())
                    );
                }
                return 1;
            }
            dir_fd
        };

        let mut ancestors = HashSet::new();
        let mut ret = 0;
        self.safe_traverse_dir(&dir_fd, root, &mut ret, &mut ancestors);
        ret
    }

    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    fn safe_traverse_dir(
        &self,
        dir_fd: &DirFd,
        dir_path: &Path,
        ret: &mut i32,
        ancestors: &mut HashSet<FileInformation>,
    ) {
        // Cycle detection: identify this directory by (dev, ino) via the already-open
        // fd. Using the fd is TOCTOU-safe (no path re-resolution through symlinks) and
        // avoids a redundant path walk. If it's already on the current path, it's a cycle.
        let dir_info = FileInformation::from_file(dir_fd).ok();
        if dir_info
            .as_ref()
            .is_some_and(|info| !ancestors.insert(info.clone()))
        {
            return; // cycle detected, stop silently
        }

        // Read directory entries
        let entries = match dir_fd.read_dir() {
            Ok(entries) => entries,
            Err(e) => {
                *ret = 1;
                if self.verbosity.level != VerbosityLevel::Silent {
                    show_error!(
                        "cannot read directory {}: {}",
                        dir_path.quote(),
                        strip_errno(&e)
                    );
                }
                return;
            }
        };

        for entry_name in entries {
            let entry_path = dir_path.join(&entry_name);

            // Get metadata for the entry
            let follow = self.traverse_symlinks == TraverseSymlinks::All;

            let meta = match dir_fd.metadata_at(&entry_name, follow.into()) {
                Ok(m) => m,
                Err(e) => {
                    *ret = 1;
                    if self.verbosity.level != VerbosityLevel::Silent {
                        show_error!(
                            "{}",
                            translate!("perms-cannot-access", "file" => entry_path.quote(), "error" => strip_errno(&e))
                        );
                    }
                    continue;
                }
            };

            if self.preserve_root
                && is_root(&entry_path, self.traverse_symlinks == TraverseSymlinks::All)
            {
                *ret = 1;
                return;
            }

            if self.chown_entry(dir_fd, &entry_name, &entry_path, &meta) != 0 {
                *ret = 1;
            }

            // Recurse into subdirectories. Open with the same symlink behavior
            // used for the stat above: with NoFollow (the default, `-P`/`-H`) an
            // attacker that swaps the just-stat'd directory for a symlink between
            // the stat and this open cannot redirect the descent off-tree
            // (O_NOFOLLOW makes openat fail). Only follow when `-L` was requested.
            if meta.is_dir() && (follow || !meta.file_type().is_symlink()) {
                match dir_fd.open_subdir(&entry_name, follow.into()) {
                    Ok(subdir_fd) => {
                        self.safe_traverse_dir(&subdir_fd, &entry_path, ret, ancestors);
                    }
                    Err(e) => {
                        *ret = 1;
                        if self.verbosity.level != VerbosityLevel::Silent {
                            show_error!(
                                "{}",
                                translate!("perms-cannot-access", "file" => entry_path.quote(), "error" => strip_errno(&e))
                            );
                        }
                    }
                }
            }
        }

        // Backtrack so sibling subtrees that legitimately reach the same directory
        // (e.g. two symlinks to one dir) are not mistaken for cycles.
        if let Some(info) = dir_info {
            ancestors.remove(&info);
        }
    }

    /// Change the entry `name` of `dir_fd`, which `meta` describes as stat'd for
    /// the descent, unless `--from` rules it out. Returns 1 on failure.
    ///
    /// As for the operand (see [`Self::chown_operand`]), the change goes to the
    /// file held open. What GNU does about an entry renamed over after its
    /// stat depends on whether symlinks are followed: with -L or -H that is a
    /// failure, as for the operand; with -P the file now under that name is
    /// judged against `--from` in its place.
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    fn chown_entry(
        &self,
        dir_fd: &DirFd,
        name: &OsStr,
        path: &Path,
        meta: &TraversalMetadata,
    ) -> i32 {
        let follow = self.dereference || self.traverse_symlinks == TraverseSymlinks::All;

        // Under -H a symlink is stat'd itself for the descent, but the change
        // goes to the file it points to, which is what `--from` has to judge.
        let target_meta;
        let meta = if follow && meta.file_type().is_symlink() {
            match dir_fd.metadata_at(name, SymlinkBehavior::Follow) {
                Ok(m) => {
                    target_meta = m;
                    &target_meta
                }
                Err(e) => {
                    self.show_entry_chown_error(path, &e);
                    return 1;
                }
            }
        } else {
            meta
        };

        if !matches!(self.filter, IfFrom::All) && self.matched(meta.uid(), meta.gid()) {
            match self.hold(meta, follow, || dir_fd.pin_at(name, follow.into())) {
                Ok(Some((_, held_meta))) if follow && self.replaced(meta, &held_meta) => {
                    return self.report_replaced(path, meta);
                }
                Ok(Some((file, held_meta))) => {
                    return self.chown_entry_if_matched(path, &held_meta, || {
                        file.chown(self.dest_uid, self.dest_gid)
                    });
                }
                Err(e) => return self.show_cannot_access(path, &e),
                Ok(None) => {}
            }
        }
        self.chown_entry_if_matched(path, meta, || {
            dir_fd.chown_at(name, self.dest_uid, self.dest_gid, follow.into())
        })
    }

    /// Change, through `change`, the entry `meta` describes if it passes `--from`.
    /// Returns 1 on failure.
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    fn chown_entry_if_matched(
        &self,
        path: &Path,
        meta: &TraversalMetadata,
        change: impl FnOnce() -> IOResult<()>,
    ) -> i32 {
        if !self.matched(meta.uid(), meta.gid()) {
            return self.print_verbose_ownership_retained_as(
                path,
                meta.uid(),
                self.dest_gid.map(|_| meta.gid()),
            );
        }
        if let Err(e) = change() {
            self.show_entry_chown_error(path, &e);
            return 1;
        }
        self.report_ownership_change_success(path, meta.uid(), meta.gid())
    }

    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    fn show_entry_chown_error(&self, path: &Path, e: &IOError) {
        if self.verbosity.level != VerbosityLevel::Silent {
            show_error!(
                "changing {} of {}: {}",
                if self.verbosity.groups_only {
                    "group"
                } else {
                    "ownership"
                },
                path.quote(),
                strip_errno(e)
            );
        }
    }

    #[cfg(any(target_os = "aix", target_os = "hurd", target_os = "redox"))]
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
                            "{}",
                            translate!(
                                "perms-cannot-access",
                                "file" => path.quote(),
                                "error" => if let Some(error) = e.io_error() {
                                    strip_errno(error)
                                } else {
                                    translate!("error-too-many-symlink-levels")
                                }
                            )
                        );
                    } else {
                        show_error!("{e}");
                    }
                    continue;
                }
                Ok(entry) => entry,
            };
            let path = entry.path();

            let Some(meta) = self.obtain_meta(path, self.dereference) else {
                ret = 1;
                if entry.file_type().is_dir() {
                    // Instruct walkdir to skip this directory to avoid getting another error
                    // when walkdir tries to query the children of this directory.
                    iterator.skip_current_dir();
                }
                continue;
            };

            if self.preserve_root && is_root(path, self.traverse_symlinks == TraverseSymlinks::All)
            {
                // Fail-fast, do not recurse further.
                return 1;
            }

            if !self.matched(meta.uid(), meta.gid()) {
                if self.print_verbose_ownership_retained_as(
                    path,
                    meta.uid(),
                    self.dest_gid.map(|_| meta.gid()),
                ) != 0
                {
                    ret = 1;
                }
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
                        // GNU: informational verbose/changes lines go to stdout.
                        ret = ret.max(self.write_verbose_line(&n));
                    }
                    // retain previous errors
                    ret.max(0)
                }
                Err(e) => {
                    if self.verbosity.level != VerbosityLevel::Silent {
                        show_error!("{e}");
                    }
                    1
                }
            }
        }
        ret
    }

    fn obtain_meta<P: AsRef<Path>>(&self, path: P, follow: bool) -> Option<Metadata> {
        let path = path.as_ref();
        get_metadata(path, follow)
            .inspect_err(|e| {
                if self.verbosity.level != VerbosityLevel::Silent {
                    show_error!(
                        "cannot {} {}: {}",
                        if follow { "dereference" } else { "access" },
                        path.quote(),
                        strip_errno(e)
                    );
                }
            })
            .ok()
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

    /// Write a verbose line to stdout without panicking. Returns 1 on write
    /// failure; the error is reported once at the final flush in `exec`.
    fn write_verbose_line(&self, line: &str) -> i32 {
        use std::io::Write;
        i32::from(writeln!(std::io::stdout(), "{line}").is_err())
    }

    fn print_verbose_ownership_retained_as(&self, path: &Path, uid: u32, gid: Option<u32>) -> i32 {
        if self.verbosity.level == VerbosityLevel::Verbose {
            let ownership = match (self.dest_uid, self.dest_gid, gid) {
                (Some(_), Some(_), Some(gid)) => format!(
                    "{}:{}",
                    entries::uid2usr(uid).unwrap_or_else(|_| uid.to_string()),
                    entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string())
                ),
                (None, Some(_), Some(gid)) => {
                    entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string())
                }
                _ => entries::uid2usr(uid).unwrap_or_else(|_| uid.to_string()),
            };
            let line = if self.verbosity.groups_only {
                format!("group of {} retained as {ownership}", path.quote())
            } else {
                format!("ownership of {} retained as {ownership}", path.quote())
            };
            return self.write_verbose_line(&line);
        }
        0
    }

    /// Try to open directory with error reporting
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    fn try_open_dir(&self, path: &Path) -> Option<DirFd> {
        DirFd::open(path, SymlinkBehavior::Follow)
            .map_err(|e| {
                if self.verbosity.level != VerbosityLevel::Silent {
                    show_error!(
                        "{}",
                        translate!("perms-cannot-access", "file" => path.quote(), "error" => strip_errno(&e))
                    );
                }
            })
            .ok()
    }

    /// Report ownership change with proper verbose output
    /// Returns 0 on success
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    fn report_ownership_change_success(
        &self,
        path: &Path,
        original_uid: u32,
        original_gid: u32,
    ) -> i32 {
        let dest_uid = self.dest_uid.unwrap_or(original_uid);
        let dest_gid = self.dest_gid.unwrap_or(original_gid);
        let changed = dest_uid != original_uid || dest_gid != original_gid;

        if changed {
            match self.verbosity.level {
                VerbosityLevel::Changes | VerbosityLevel::Verbose => {
                    let output = if self.verbosity.groups_only {
                        format!(
                            "changed group of {} from {} to {}",
                            path.quote(),
                            entries::gid2grp(original_gid)
                                .unwrap_or_else(|_| original_gid.to_string()),
                            entries::gid2grp(dest_gid).unwrap_or_else(|_| dest_gid.to_string())
                        )
                    } else {
                        format!(
                            "changed ownership of {} from {}:{} to {}:{}",
                            path.quote(),
                            entries::uid2usr(original_uid)
                                .unwrap_or_else(|_| original_uid.to_string()),
                            entries::gid2grp(original_gid)
                                .unwrap_or_else(|_| original_gid.to_string()),
                            entries::uid2usr(dest_uid).unwrap_or_else(|_| dest_uid.to_string()),
                            entries::gid2grp(dest_gid).unwrap_or_else(|_| dest_gid.to_string())
                        )
                    };
                    // GNU: informational verbose/changes output goes to stdout.
                    return self.write_verbose_line(&output);
                }
                _ => (),
            }
        } else if self.verbosity.level == VerbosityLevel::Verbose {
            let output = if self.verbosity.groups_only {
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
            // GNU: informational verbose output goes to stdout.
            return self.write_verbose_line(&output);
        }
        0
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

/// Determines symbolic link traversal and recursion settings based on flags.
/// Returns the updated `dereference` and `traverse_symlinks` values.
pub fn configure_symlink_and_recursion(
    matches: &ArgMatches,
    default_traverse_symlinks: TraverseSymlinks,
) -> Result<(bool, bool, TraverseSymlinks), Box<dyn crate::error::UError>> {
    let mut dereference = if matches.get_flag(options::dereference::DEREFERENCE) {
        Some(true) // Follow symlinks
    } else if matches.get_flag(options::dereference::NO_DEREFERENCE) {
        Some(false) // Do not follow symlinks
    } else {
        None // Default behavior
    };

    let mut traverse_symlinks = if matches.get_flag("L") {
        TraverseSymlinks::All
    } else if matches.get_flag("H") {
        TraverseSymlinks::First
    } else if matches.get_flag("P") {
        TraverseSymlinks::None
    } else {
        default_traverse_symlinks
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

    Ok((recursive, dereference.unwrap_or(true), traverse_symlinks))
}

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
        if arg.as_encoded_bytes().starts_with(b"--reference=") || arg == "--reference" {
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
            .num_args(1..)
            .value_parser(clap::value_parser!(OsString)),
    );
    let matches = crate::clap_localization::handle_clap_result(command, args)?;

    let files: Vec<OsString> = matches
        .get_many::<OsString>(options::ARG_FILES)
        .map(|v| v.cloned().collect())
        .unwrap_or_default();

    let preserve_root = matches.get_flag(options::preserve_root::PRESERVE);
    let (recursive, dereference, traverse_symlinks) =
        configure_symlink_and_recursion(&matches, TraverseSymlinks::None)?;

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
        dereference,
        preserve_root,
        files,
        filter,
    };
    executor.exec()
}

pub fn common_args() -> Vec<Arg> {
    vec![
        Arg::new(traverse::TRAVERSE)
            .short(traverse::TRAVERSE.chars().next().unwrap())
            .help("if a command line argument is a symbolic link to a directory, traverse it")
            .overrides_with_all([traverse::EVERY, traverse::NO_TRAVERSE])
            .action(clap::ArgAction::SetTrue),
        Arg::new(traverse::EVERY)
            .short(traverse::EVERY.chars().next().unwrap())
            .help("traverse every symbolic link to a directory encountered")
            .overrides_with_all([traverse::TRAVERSE, traverse::NO_TRAVERSE])
            .action(clap::ArgAction::SetTrue),
        Arg::new(traverse::NO_TRAVERSE)
            .short(traverse::NO_TRAVERSE.chars().next().unwrap())
            .help("do not traverse any symbolic links (default)")
            .overrides_with_all([traverse::TRAVERSE, traverse::EVERY])
            .action(clap::ArgAction::SetTrue),
        Arg::new(options::dereference::DEREFERENCE)
            .long(options::dereference::DEREFERENCE)
            .help(
                "affect the referent of each symbolic link (this is the default), \
    rather than the symbolic link itself",
            )
            .action(clap::ArgAction::SetTrue),
        Arg::new(options::dereference::NO_DEREFERENCE)
            .short('h')
            .long(options::dereference::NO_DEREFERENCE)
            .help(
                "affect symbolic links instead of any referenced file \
        (useful only on systems that can change the ownership of a symlink)",
            )
            .action(clap::ArgAction::SetTrue),
    ]
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

    /// `fd_is` must accept the directory that was stat'd and reject anything else.
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    #[test]
    fn test_fd_is_identifies_the_stated_directory() {
        let temp_dir = tempdir().unwrap();
        let dir = temp_dir.path().join("dir");
        let other = temp_dir.path().join("other");
        std::fs::create_dir(&dir).unwrap();
        std::fs::create_dir(&other).unwrap();

        let meta = std::fs::metadata(&dir).unwrap();
        let dir_fd = DirFd::open(&dir, SymlinkBehavior::Follow).unwrap();
        assert!(fd_is(&dir_fd, &meta).unwrap());

        // Same pathname, different object underneath: the descriptor no longer matches
        // the metadata every decision was made on.
        let other_fd = DirFd::open(&other, SymlinkBehavior::Follow).unwrap();
        assert!(!fd_is(&other_fd, &meta).unwrap());

        // A symlink to the directory resolves to the same object, so it must match:
        // following the operand is legitimate when it is what was classified.
        unix::fs::symlink(&dir, temp_dir.path().join("link")).unwrap();
        let link_fd = DirFd::open(&temp_dir.path().join("link"), SymlinkBehavior::Follow).unwrap();
        assert!(fd_is(&link_fd, &meta).unwrap());
    }

    /// Two groups the current process may give its own files, or `None`.
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    fn two_groups() -> Option<(u32, u32)> {
        let egid = nix::unistd::getegid().as_raw();
        if nix::unistd::geteuid().is_root() {
            return Some((egid, egid.wrapping_add(1)));
        }
        let mut groups = [0 as gid_t; 64];
        // SAFETY: the buffer holds as many entries as it is said to.
        let n = unsafe { libc::getgroups(groups.len() as libc::c_int, groups.as_mut_ptr()) };
        let n = usize::try_from(n).ok()?;
        let other = groups[..n].iter().copied().find(|&g| g != egid)?;
        Some((egid, other))
    }

    /// An executor for `--from=:from :to` that reports nothing.
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    fn chgrp_from(from: u32, to: u32, dereference: bool) -> ChownExecutor {
        ChownExecutor {
            dest_uid: None,
            dest_gid: Some(to),
            raw_owner: String::new(),
            traverse_symlinks: TraverseSymlinks::None,
            verbosity: Verbosity {
                groups_only: true,
                level: VerbosityLevel::Silent,
            },
            filter: IfFrom::Group(from),
            files: Vec::new(),
            recursive: false,
            preserve_root: false,
            dereference,
        }
    }

    /// `item` has group `from` and `other` the group `other_group`. Stat `item`
    /// with `stat`, then rename `other` over it, as a concurrent rename could.
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    fn replaced_after_stat<M>(
        from: u32,
        other_group: u32,
        stat: impl FnOnce(&Path) -> M,
    ) -> (tempfile::TempDir, M) {
        let temp_dir = tempdir().unwrap();
        let item = temp_dir.path().join("item");
        let other = temp_dir.path().join("other");
        std::fs::write(&item, "").unwrap();
        std::fs::write(&other, "").unwrap();
        unix::fs::chown(&item, None, Some(from)).unwrap();
        unix::fs::chown(&other, None, Some(other_group)).unwrap();
        let stale = stat(temp_dir.path());
        std::fs::rename(&other, &item).unwrap();
        (temp_dir, stale)
    }

    /// A name re-pointed after the operand passed `--from` must not have the
    /// change land on the new file; as with GNU, that counts as a failure.
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    #[test]
    fn test_from_operand_is_judged_on_the_file_changed() {
        let Some((from, other)) = two_groups() else {
            return;
        };
        let (temp_dir, stale) = replaced_after_stat(from, other, |dir| {
            std::fs::metadata(dir.join("item")).unwrap()
        });
        let item = temp_dir.path().join("item");

        let ret = chgrp_from(from, from, true).chown_operand(&item, &stale);
        assert_eq!(std::fs::metadata(&item).unwrap().gid(), other);
        assert_eq!(ret, 1);
    }

    /// For an entry met during `-R`, the change must not land on the file now
    /// under the name either. As with GNU, that is a failure when symlinks
    /// are followed; with -P the new file is judged in its place, does not
    /// pass `--from`, and is left alone without one.
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    #[test]
    fn test_from_entry_is_judged_on_the_file_changed() {
        let Some((from, other)) = two_groups() else {
            return;
        };
        let name = OsStr::new("item");
        for (follow, expected_ret) in [(false, 0), (true, 1)] {
            let (temp_dir, (dir_fd, stale)) = replaced_after_stat(from, other, |dir| {
                let dir_fd = DirFd::open(dir, SymlinkBehavior::Follow).unwrap();
                let stale = dir_fd.metadata_at(name, follow.into()).unwrap();
                (dir_fd, stale)
            });
            let item = temp_dir.path().join("item");

            let ret = chgrp_from(from, from, follow).chown_entry(&dir_fd, name, &item, &stale);
            assert_eq!(std::fs::metadata(&item).unwrap().gid(), other);
            assert_eq!(ret, expected_ret, "follow: {follow}");
        }
    }

    /// A socket cannot be held everywhere. Where it cannot, root must not
    /// change one that passed `--from` by name, as a rename could send that
    /// change to any file; an unprivileged caller can only reach its own.
    #[cfg(not(any(target_os = "aix", target_os = "hurd", target_os = "redox")))]
    #[test]
    fn test_root_does_not_change_a_socket_by_name() {
        let temp_dir = tempdir().unwrap();
        let socket = temp_dir.path().join("socket");
        let _listener = unix::net::UnixListener::bind(&socket).unwrap();
        let meta = std::fs::metadata(&socket).unwrap();
        let open = || PinnedFile::open(&socket, SymlinkBehavior::Follow);

        let as_root = ChownExecutor::hold_as(false, &meta, true, open);
        if CAN_PIN_SPECIAL_FILES {
            assert!(matches!(as_root, Ok(Some(_))));
            return;
        }
        assert!(matches!(
            as_root,
            Err(e) if e.raw_os_error() == Some(libc::EOPNOTSUPP)
        ));
        assert!(matches!(
            ChownExecutor::hold_as(true, &meta, true, open),
            Ok(None)
        ));
    }

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
