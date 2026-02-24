// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore TODO canonicalizes direntry pathbuf symlinked IRWXO IRWXG
//! Recursively copy the contents of a directory.
//!
//! See the [`copy_directory`] function for more information.
#[cfg(windows)]
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::convert::identity;
use std::env;
use std::fs::{self, exists};
use std::io;
use std::path::{Path, PathBuf, StripPrefixError};

use indicatif::ProgressBar;
use uucore::display::Quotable;
use uucore::error::UIoError;
use uucore::fs::{
    FileInformation, MissingHandling, ResolveMode, canonicalize, path_ends_with_terminator,
};
use uucore::show;
use uucore::translate;
use uucore::uio_error;
use walkdir::{DirEntry, WalkDir};

#[cfg(all(feature = "selinux", target_os = "linux"))]
use crate::set_selinux_context;
use crate::{
    CopyMode, CopyResult, CpError, Options, aligned_ancestors, context_for, copy_attributes,
    copy_file,
};

/// Represents a directory that needs permission fixup after copying its contents.
struct DirNeedingPermissions {
    /// Absolute path to the source directory
    source: PathBuf,
    /// Path to the destination directory
    dest: PathBuf,
    /// Whether this directory was freshly created by the copy operation
    was_created: bool,
}

/// Ensure a Windows path starts with a `\\?`.
#[cfg(target_os = "windows")]
fn adjust_canonicalization(p: &Path) -> Cow<'_, Path> {
    // In some cases, \\? can be missing on some Windows paths.  Add it at the
    // beginning unless the path is prefixed with a device namespace.
    const VERBATIM_PREFIX: &str = r"\\?";
    const DEVICE_NS_PREFIX: &str = r"\\.";

    let has_prefix = p
        .components()
        .next()
        .and_then(|comp| comp.as_os_str().to_str())
        .is_some_and(|p_str| {
            p_str.starts_with(VERBATIM_PREFIX) || p_str.starts_with(DEVICE_NS_PREFIX)
        });

    if has_prefix {
        p.into()
    } else {
        Path::new(VERBATIM_PREFIX).join(p).into()
    }
}

/// Get a descendant path relative to the given parent directory.
///
/// If `root_parent` is `None`, then this just returns the `path`
/// itself. Otherwise, this function strips the parent prefix from the
/// given `path`, leaving only the portion of the path relative to the
/// parent.
fn get_local_to_root_parent(
    path: &Path,
    root_parent: Option<&Path>,
) -> Result<PathBuf, StripPrefixError> {
    match root_parent {
        Some(parent) => {
            // On Windows, some paths are starting with \\?
            // but not always, so, make sure that we are consistent for strip_prefix
            // See https://docs.microsoft.com/en-us/windows/win32/fileio/naming-a-file for more info
            #[cfg(windows)]
            let (path, parent) = (
                adjust_canonicalization(path),
                adjust_canonicalization(parent),
            );
            let path = path.strip_prefix(parent)?;
            Ok(path.to_path_buf())
        }
        None => Ok(path.to_path_buf()),
    }
}

/// Given an iterator, return all its items except the last.
fn skip_last<T>(mut iter: impl Iterator<Item = T>) -> impl Iterator<Item = T> {
    let last = iter.next();
    iter.scan(last, Option::replace)
}

/// Paths that are invariant throughout the traversal when copying a directory.
struct Context<'a> {
    /// The current working directory at the time of starting the traversal.
    current_dir: PathBuf,

    /// The path to the parent of the source directory, if any.
    root_parent: Option<PathBuf>,

    /// The target path to which the directory will be copied.
    target: &'a Path,

    /// Whether the target is an existing file. Cached to avoid repeated `stat` calls.
    target_is_file: bool,

    /// The source path from which the directory will be copied.
    root: &'a Path,
}

impl<'a> Context<'a> {
    fn new(root: &'a Path, target: &'a Path) -> io::Result<Self> {
        let current_dir = env::current_dir()?;
        let root_path = current_dir.join(root);
        let target_is_file = target.is_file();
        let root_parent =
            if target.exists() && !root.as_os_str().as_encoded_bytes().ends_with(b"/.") {
                root_path.parent().map(ToOwned::to_owned)
            } else if root == Path::new(".") && target.is_dir() {
                // Special case: when copying current directory (.) to an existing directory,
                // we don't want to use the parent path as root_parent because we want to
                // copy the contents of the current directory directly into the target directory,
                // not create a subdirectory with the current directory's name.
                None
            } else {
                Some(root_path)
            };
        Ok(Self {
            current_dir,
            root_parent,
            target,
            target_is_file,
            root,
        })
    }
}

/// Data needed to perform a single copy operation while traversing a directory.
///
/// For convenience while traversing a directory, the [`Entry::new`]
/// function allows creating an entry from a [`Context`] and a
/// [`DirEntry`].
///
/// # Examples
///
/// For example, if the source directory structure is `a/b/c`, the
/// target is `d/`, a directory that already exists, and the copy
/// command is `cp -r a/b/c d`, then the overall set of copy
/// operations could be represented as three entries,
///
/// ```rust,ignore
/// let operations = [
///     Entry {
///         source_absolute: "/tmp/a".into(),
///         source_relative: "a".into(),
///         local_to_target: "d/a".into(),
///         target_is_file: false,
///     }
///     Entry {
///         source_absolute: "/tmp/a/b".into(),
///         source_relative: "a/b".into(),
///         local_to_target: "d/a/b".into(),
///         target_is_file: false,
///     }
///     Entry {
///         source_absolute: "/tmp/a/b/c".into(),
///         source_relative: "a/b/c".into(),
///         local_to_target: "d/a/b/c".into(),
///         target_is_file: false,
///     }
/// ];
/// ```
struct Entry {
    /// The absolute path to file or directory to copy.
    source_absolute: PathBuf,

    /// The relative path to file or directory to copy.
    source_relative: PathBuf,

    /// The path to the destination, relative to the target.
    local_to_target: PathBuf,

    /// Whether the destination is a file.
    target_is_file: bool,
}

impl Entry {
    fn new<A: AsRef<Path>>(
        context: &Context,
        source: A,
        no_target_dir: bool,
    ) -> Result<Self, StripPrefixError> {
        let source = source.as_ref();
        let source_relative = source.to_path_buf();
        let source_absolute = context.current_dir.join(&source_relative);
        let mut descendant =
            get_local_to_root_parent(&source_absolute, context.root_parent.as_deref())?;
        if no_target_dir {
            let source_is_dir = source.is_dir();
            if path_ends_with_terminator(context.target)
                && source_is_dir
                && !exists(context.target).is_ok_and(identity)
            {
                if let Err(e) = fs::create_dir_all(context.target) {
                    eprintln!(
                        "{}",
                        translate!("cp-error-failed-to-create-directory", "error" => e)
                    );
                }
            } else if let Some(stripped) = context
                .root
                .components()
                .next_back()
                .and_then(|stripped| descendant.strip_prefix(stripped).ok())
            {
                descendant = stripped.to_path_buf();
            }
        } else if context.root == Path::new(".") && context.target.is_dir() {
            // Special case: when copying current directory (.) to an existing directory,
            // strip the current directory name from the descendant path to avoid creating
            // an extra level of nesting. For example, if we're in /home/user/source_dir
            // and copying . to /home/user/dest_dir, we want to copy source_dir/file.txt
            // to dest_dir/file.txt, not dest_dir/source_dir/file.txt.
            if let Some(current_dir_name) = context.current_dir.file_name() {
                if let Ok(stripped) = descendant.strip_prefix(current_dir_name) {
                    descendant = stripped.to_path_buf();
                }
            }
        }

        let local_to_target = context.target.join(descendant);
        let target_is_file = context.target_is_file;
        Ok(Self {
            source_absolute,
            source_relative,
            local_to_target,
            target_is_file,
        })
    }
}

#[allow(clippy::too_many_arguments)]
/// Copy a single entry during a directory traversal.
///
/// # Returns
///
/// Returns `Ok(true)` if this function created a new directory, `Ok(false)` otherwise.
/// This information is used to determine whether default directory permissions should
/// be preserved during attribute copying.
fn copy_direntry(
    progress_bar: Option<&ProgressBar>,
    entry: &Entry,
    entry_is_symlink: bool,
    entry_is_dir_no_follow: bool,
    options: &Options,
    symlinked_files: &mut HashSet<FileInformation>,
    preserve_hard_links: bool,
    copied_destinations: &HashSet<PathBuf>,
    copied_files: &mut HashMap<FileInformation, PathBuf>,
    created_parent_dirs: &mut HashSet<PathBuf>,
) -> CopyResult<bool> {
    let source_is_symlink = entry_is_symlink;
    let source_is_dir = if source_is_symlink && !options.dereference {
        false
    } else if source_is_symlink {
        entry.source_absolute.is_dir()
    } else {
        entry_is_dir_no_follow
    };

    // If the source is a directory and the destination does not
    // exist, ...
    if source_is_dir && !entry.local_to_target.exists() {
        return if entry.target_is_file {
            Err(translate!("cp-error-cannot-overwrite-non-directory-with-directory").into())
        } else {
            build_dir(
                &entry.local_to_target,
                false,
                options,
                Some(&entry.source_absolute),
            )?;
            if options.verbose {
                println!(
                    "{}",
                    context_for(&entry.source_relative, &entry.local_to_target)
                );
            }
            Ok(true)
        };
    }

    // If the source is not a directory, then we need to copy the file.
    if !source_is_dir {
        if let Err(err) = copy_file(
            progress_bar,
            &entry.source_relative,
            entry.local_to_target.as_path(),
            options,
            symlinked_files,
            copied_destinations,
            copied_files,
            created_parent_dirs,
            false,
        ) {
            if preserve_hard_links {
                if !source_is_symlink {
                    return Err(err);
                }
                // silent the error with a symlink
                // In case we do --archive, we might copy the symlink
                // before the file itself
            } else {
                // At this point, `path` is just a plain old file.
                // Terminate this function immediately if there is any
                // kind of error *except* a "permission denied" error.
                //
                // TODO What other kinds of errors, if any, should
                // cause us to continue walking the directory?
                match err {
                    CpError::IoErrContext(e, _) if e.kind() == io::ErrorKind::PermissionDenied => {
                        show!(uio_error!(
                            e,
                            "{}",
                            translate!(
                                "cp-error-cannot-open-for-reading",
                                "source" => entry.source_relative.quote()
                            ),
                        ));
                    }
                    e => return Err(e),
                }
            }
        }
    }

    // In any other case, there is nothing to do, so we just return to
    // continue the traversal.
    Ok(false)
}

/// Read the contents of the directory `root` and recursively copy the
/// contents to `target`.
///
/// Any errors encountered copying files in the tree will be logged but
/// will not cause a short-circuit.
#[allow(clippy::too_many_arguments)]
pub(crate) fn copy_directory(
    progress_bar: Option<&ProgressBar>,
    root: &Path,
    target: &Path,
    options: &Options,
    symlinked_files: &mut HashSet<FileInformation>,
    copied_destinations: &HashSet<PathBuf>,
    copied_files: &mut HashMap<FileInformation, PathBuf>,
    created_parent_dirs: &mut HashSet<PathBuf>,
    source_in_command_line: bool,
) -> CopyResult<()> {
    // if no-dereference is enabled and this is a symlink, copy it as a file
    if !options.dereference(source_in_command_line) && root.is_symlink() {
        return copy_file(
            progress_bar,
            root,
            target,
            options,
            symlinked_files,
            copied_destinations,
            copied_files,
            created_parent_dirs,
            source_in_command_line,
        );
    }

    if !options.recursive {
        return Err(translate!("cp-error-omitting-directory", "dir" => root.quote()).into());
    }

    // check if root is a prefix of target
    if path_has_prefix(target, root)? {
        let dest_name = root.file_name().unwrap_or(root.as_os_str());
        return Err(translate!("cp-error-cannot-copy-directory-into-itself", "source" => root.quote(), "dest" => target.join(dest_name).quote())
        .into());
    }

    // If in `--parents` mode, create all the necessary ancestor directories.
    //
    // For example, if the command is `cp --parents a/b/c d`, that
    // means we need to copy the two ancestor directories first:
    //
    // a -> d/a
    // a/b -> d/a/b
    //
    let tmp = if options.parents {
        if let Some(parent) = root.parent() {
            let new_target = target.join(parent);
            build_dir(&new_target, true, options, None)?;
            if options.verbose {
                // For example, if copying file `a/b/c` and its parents
                // to directory `d/`, then print
                //
                //     a -> d/a
                //     a/b -> d/a/b
                //
                for (x, y) in aligned_ancestors(root, &target.join(root)) {
                    println!("{} -> {}", x.display(), y.display());
                }
            }

            new_target
        } else {
            target.to_path_buf()
        }
    } else {
        target.to_path_buf()
    };
    let target = tmp.as_path();

    let preserve_hard_links = options.preserve_hard_links();

    // Collect some paths here that are invariant during the traversal
    // of the given directory, like the current working directory and
    // the target directory.
    let context = match Context::new(root, target) {
        Ok(c) => c,
        Err(e) => {
            return Err(translate!("cp-error-failed-get-current-dir", "error" => e).into());
        }
    };

    // The directory we were in during the previous iteration
    let mut last_iter: Option<DirEntry> = None;

    // Keep track of all directories we've created that need permission fixes
    let mut dirs_needing_permissions: Vec<DirNeedingPermissions> = Vec::new();

    // Traverse the contents of the directory, copying each one.
    for direntry_result in WalkDir::new(root)
        .same_file_system(options.one_file_system)
        .follow_links(options.dereference)
    {
        match direntry_result {
            Ok(direntry) => {
                let direntry_type = direntry.file_type();
                let direntry_path = direntry.path();
                let (entry_is_symlink, entry_is_dir_no_follow) =
                    match direntry_path.symlink_metadata() {
                        Ok(metadata) => {
                            let file_type = metadata.file_type();
                            (file_type.is_symlink(), file_type.is_dir())
                        }
                        Err(_) => (direntry_type.is_symlink(), direntry_type.is_dir()),
                    };
                let entry = Entry::new(&context, direntry_path, options.no_target_dir)?;

                let created = copy_direntry(
                    progress_bar,
                    &entry,
                    entry_is_symlink,
                    entry_is_dir_no_follow,
                    options,
                    symlinked_files,
                    preserve_hard_links,
                    copied_destinations,
                    copied_files,
                    created_parent_dirs,
                )?;

                // We omit certain permissions when creating directories
                // to prevent other users from accessing them before they're done.
                // We thus need to fix the permissions of each directory we copy
                // once it's contents are ready.
                // This "fixup" is implemented here in a memory-efficient manner.
                //
                // We detect iterations where we "walk up" the directory tree,
                // and fix permissions on all the directories we exited.
                // (Note that there can be more than one! We might step out of
                // `./a/b/c` into `./a/`, in which case we'll need to fix the
                // permissions of both `./a/b/c` and `./a/b`, in that order.)
                let is_dir_for_permissions =
                    entry_is_dir_no_follow || (options.dereference && direntry_path.is_dir());
                if is_dir_for_permissions {
                    // For --link mode, copy attributes immediately to avoid O(n) memory
                    if options.copy_mode == CopyMode::Link {
                        copy_attributes(
                            &entry.source_absolute,
                            &entry.local_to_target,
                            &options.attributes,
                            false,
                            options.set_selinux_context,
                        )?;
                        continue;
                    }
                    // Add this directory to our list for permission fixing later
                    dirs_needing_permissions.push(DirNeedingPermissions {
                        source: entry.source_absolute.clone(),
                        dest: entry.local_to_target.clone(),
                        was_created: created,
                    });

                    // If true, last_iter is not a parent of this iter.
                    // The means we just exited a directory.
                    let went_up = if let Some(last_iter) = &last_iter {
                        last_iter.path().strip_prefix(direntry_path).is_ok()
                    } else {
                        false
                    };

                    if went_up {
                        // Compute the "difference" between `last_iter` and `direntry`.
                        // For example, if...
                        // - last_iter = `a/b/c/d`
                        // - direntry = `a/b`
                        // then diff = `c/d`
                        //
                        // All the unwraps() here are unreachable.
                        let last_iter = last_iter.as_ref().unwrap();
                        let diff = last_iter.path().strip_prefix(direntry_path).unwrap();

                        // Fix permissions for every entry in `diff`, inside-out.
                        // We skip the last directory (which will be `.`) because
                        // its permissions will be fixed when we walk _out_ of it.
                        // (at this point, we might not be done copying `.`!)
                        for p in skip_last(diff.ancestors()) {
                            let src = direntry_path.join(p);
                            let entry = Entry::new(&context, &src, options.no_target_dir)?;

                            copy_attributes(
                                &entry.source_absolute,
                                &entry.local_to_target,
                                &options.attributes,
                                false,
                                options.set_selinux_context,
                            )?;
                        }
                    }

                    last_iter = Some(direntry);
                }
            }

            // Print an error message, but continue traversing the directory.
            Err(e) => show!(CpError::WalkDirErr(e)),
        }
    }

    // Fix permissions for all directories we created
    // This ensures that even sibling directories get their permissions fixed
    for dir in dirs_needing_permissions {
        copy_attributes(
            &dir.source,
            &dir.dest,
            &options.attributes,
            dir.was_created,
            options.set_selinux_context,
        )?;

        #[cfg(all(feature = "selinux", target_os = "linux"))]
        if options.set_selinux_context {
            set_selinux_context(&dir.dest, options.context.as_ref())?;
        }
    }

    // Also fix permissions for parent directories,
    // if we were asked to create them.
    if options.parents {
        let dest = target.join(root.file_name().unwrap());
        for (x, y) in aligned_ancestors(root, dest.as_path()) {
            if let Ok(src) = canonicalize(x, MissingHandling::Normal, ResolveMode::Physical) {
                copy_attributes(
                    &src,
                    y,
                    &options.attributes,
                    false,
                    options.set_selinux_context,
                )?;

                #[cfg(all(feature = "selinux", target_os = "linux"))]
                if options.set_selinux_context {
                    set_selinux_context(y, options.context.as_ref())?;
                }
            }
        }
    }

    Ok(())
}

/// Decide whether the second path is a prefix of the first.
///
/// This function canonicalizes the paths via
/// [`fs::canonicalize`] before comparing.
///
/// # Errors
///
/// If there is an error determining the canonical, absolute form of
/// either path.
///
/// # Examples
///
/// ```rust,ignore
/// assert!(path_has_prefix(Path::new("/usr/bin"), Path::new("/usr")))
/// assert!(!path_has_prefix(Path::new("/usr"), Path::new("/usr/bin")))
/// assert!(!path_has_prefix(Path::new("/usr/bin"), Path::new("/var/log")))
/// ```
pub fn path_has_prefix(p1: &Path, p2: &Path) -> io::Result<bool> {
    let pathbuf1 = canonicalize(p1, MissingHandling::Normal, ResolveMode::Logical)?;
    let pathbuf2 = canonicalize(p2, MissingHandling::Normal, ResolveMode::Logical)?;

    Ok(pathbuf1.starts_with(pathbuf2))
}

/// Builds a directory at the specified path with the given options.
///
/// # Notes
/// - If `copy_attributes_from` is `Some`, the new directory's attributes will be
///   copied from the provided file. Otherwise, the new directory will have the default
///   attributes for the current user.
/// - This method excludes certain permissions if ownership or special mode bits could
///   potentially change. (See `test_dir_perm_race_with_preserve_mode_and_ownership`)
/// - The `recursive` flag determines whether parent directories should be created
///   if they do not already exist.
// we need to allow unused_variable since `options` might be unused in non unix systems
#[allow(unused_variables)]
fn build_dir(
    path: &PathBuf,
    recursive: bool,
    options: &Options,
    copy_attributes_from: Option<&Path>,
) -> CopyResult<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(recursive);

    // To prevent unauthorized access before the folder is ready,
    // exclude certain permissions if ownership or special mode bits
    // could potentially change.
    #[cfg(unix)]
    {
        use crate::Preserve;
        use std::os::unix::fs::PermissionsExt;

        // we need to allow trivial casts here because some systems like linux have u32 constants in
        // in libc while others don't.
        #[allow(clippy::unnecessary_cast)]
        let mut excluded_perms = if matches!(options.attributes.ownership, Preserve::Yes { .. }) {
            libc::S_IRWXG | libc::S_IRWXO // exclude rwx for group and other
        } else if matches!(options.attributes.mode, Preserve::Yes { .. }) {
            libc::S_IWGRP | libc::S_IWOTH //exclude w for group and other
        } else {
            0
        } as u32;

        let umask = if let (Some(from), Preserve::Yes { .. }) =
            (copy_attributes_from, options.attributes.mode)
        {
            !fs::symlink_metadata(from)?.permissions().mode()
        } else {
            uucore::mode::get_umask()
        };

        excluded_perms |= umask;
        let mode = !excluded_perms & 0o777; //use only the last three octet bits
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, mode);
    }

    builder.create(path)?;
    Ok(())
}
