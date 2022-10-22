//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
// spell-checker:ignore TODO canonicalizes direntry pathbuf symlinked
//! Recursively copy the contents of a directory.
//!
//! See the [`copy_directory`] function for more information.
#[cfg(windows)]
use std::borrow::Cow;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf, StripPrefixError};

use uucore::display::Quotable;
use uucore::error::UIoError;
use uucore::fs::{canonicalize, FileInformation, MissingHandling, ResolveMode};
use walkdir::{DirEntry, WalkDir};

use crate::{
    copy_attributes, copy_file, copy_link, preserve_hardlinks, CopyResult, Error, Options,
    TargetSlice,
};

/// Ensure a Windows path starts with a `\\?`.
#[cfg(target_os = "windows")]
fn adjust_canonicalization(p: &Path) -> Cow<Path> {
    // In some cases, \\? can be missing on some Windows paths.  Add it at the
    // beginning unless the path is prefixed with a device namespace.
    const VERBATIM_PREFIX: &str = r#"\\?"#;
    const DEVICE_NS_PREFIX: &str = r#"\\."#;

    let has_prefix = p
        .components()
        .next()
        .and_then(|comp| comp.as_os_str().to_str())
        .map(|p_str| p_str.starts_with(VERBATIM_PREFIX) || p_str.starts_with(DEVICE_NS_PREFIX))
        .unwrap_or_default();

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
            let path = path.strip_prefix(&parent)?;
            Ok(path.to_path_buf())
        }
        None => Ok(path.to_path_buf()),
    }
}

/// Paths that are invariant throughout the traversal when copying a directory.
struct Context<'a> {
    /// The current working directory at the time of starting the traversal.
    current_dir: PathBuf,

    /// The path to the parent of the source directory, if any.
    root_parent: Option<PathBuf>,

    /// The target path to which the directory will be copied.
    target: &'a Path,
}

impl<'a> Context<'a> {
    fn new(root: &'a Path, target: &'a Path) -> std::io::Result<Self> {
        let current_dir = env::current_dir()?;
        let root_path = current_dir.join(root);
        let root_parent = if target.exists() {
            root_path.parent().map(|p| p.to_path_buf())
        } else {
            Some(root_path)
        };
        Ok(Self {
            current_dir,
            root_parent,
            target,
        })
    }
}

/// Data needed to perform a single copy operation while traversing a directory.
///
/// For convenience while traversing a directory, the [`Entry::new`]
/// function allows creating an entry from a [`Context`] and a
/// [`walkdir::DirEntry`].
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
    fn new(context: &Context, direntry: &DirEntry) -> Result<Self, StripPrefixError> {
        let source_relative = direntry.path().to_path_buf();
        let source_absolute = context.current_dir.join(&source_relative);
        let descendant =
            get_local_to_root_parent(&source_absolute, context.root_parent.as_deref())?;
        let local_to_target = context.target.join(descendant);
        let target_is_file = context.target.is_file();
        Ok(Self {
            source_absolute,
            source_relative,
            local_to_target,
            target_is_file,
        })
    }
}

/// Copy a single entry during a directory traversal.
fn copy_direntry(
    entry: Entry,
    options: &Options,
    symlinked_files: &mut HashSet<FileInformation>,
    preserve_hard_links: bool,
    hard_links: &mut Vec<(String, u64)>,
) -> CopyResult<()> {
    let Entry {
        source_absolute,
        source_relative,
        local_to_target,
        target_is_file,
    } = entry;

    // If the source is a symbolic link and the options tell us not to
    // dereference the link, then copy the link object itself.
    if source_absolute.is_symlink() && !options.dereference {
        return copy_link(&source_absolute, &local_to_target, symlinked_files);
    }

    // If the source is a directory and the destination does not
    // exist, ...
    if source_absolute.is_dir() && !local_to_target.exists() {
        if target_is_file {
            return Err("cannot overwrite non-directory with directory".into());
        } else {
            // TODO Since the calling code is traversing from the root
            // of the directory structure, I don't think
            // `create_dir_all()` will have any benefit over
            // `create_dir()`, since all the ancestor directories
            // should have already been created.
            fs::create_dir_all(local_to_target)?;
            return Ok(());
        }
    }

    // If the source is not a directory, then we need to copy the file.
    if !source_absolute.is_dir() {
        if preserve_hard_links {
            let mut found_hard_link = false;
            let dest = local_to_target.as_path().to_path_buf();
            preserve_hardlinks(hard_links, &source_absolute, &dest, &mut found_hard_link)?;
            if !found_hard_link {
                match copy_file(
                    &source_absolute,
                    local_to_target.as_path(),
                    options,
                    symlinked_files,
                    false,
                ) {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        if source_absolute.is_symlink() {
                            // silent the error with a symlink
                            // In case we do --archive, we might copy the symlink
                            // before the file itself
                            Ok(())
                        } else {
                            Err(err)
                        }
                    }
                }?;
            }
        } else {
            // At this point, `path` is just a plain old file.
            // Terminate this function immediately if there is any
            // kind of error *except* a "permission denied" error.
            //
            // TODO What other kinds of errors, if any, should
            // cause us to continue walking the directory?
            match copy_file(
                &source_absolute,
                local_to_target.as_path(),
                options,
                symlinked_files,
                false,
            ) {
                Ok(_) => {}
                Err(Error::IoErrContext(e, _))
                    if e.kind() == std::io::ErrorKind::PermissionDenied =>
                {
                    show!(uio_error!(
                        e,
                        "cannot open {} for reading",
                        source_relative.quote(),
                    ));
                }
                Err(e) => return Err(e),
            }
        }
    }

    // In any other case, there is nothing to do, so we just return to
    // continue the traversal.
    Ok(())
}

/// Read the contents of the directory `root` and recursively copy the
/// contents to `target`.
///
/// Any errors encountered copying files in the tree will be logged but
/// will not cause a short-circuit.
pub(crate) fn copy_directory(
    root: &Path,
    target: &TargetSlice,
    options: &Options,
    symlinked_files: &mut HashSet<FileInformation>,
    source_in_command_line: bool,
) -> CopyResult<()> {
    if !options.recursive {
        return Err(format!("omitting directory {}", root.quote()).into());
    }

    // if no-dereference is enabled and this is a symlink, copy it as a file
    if !options.dereference(source_in_command_line) && root.is_symlink() {
        return copy_file(
            root,
            target,
            options,
            symlinked_files,
            source_in_command_line,
        );
    }

    // check if root is a prefix of target
    if path_has_prefix(target, root)? {
        return Err(format!(
            "cannot copy a directory, {}, into itself, {}",
            root.quote(),
            target.join(root.file_name().unwrap()).quote()
        )
        .into());
    }

    let mut hard_links: Vec<(String, u64)> = vec![];
    let preserve_hard_links = options.preserve_hard_links();

    // Collect some paths here that are invariant during the traversal
    // of the given directory, like the current working directory and
    // the target directory.
    let context = match Context::new(root, target) {
        Ok(c) => c,
        Err(e) => return Err(format!("failed to get current directory {}", e).into()),
    };

    // Traverse the contents of the directory, copying each one.
    for direntry_result in WalkDir::new(root)
        .same_file_system(options.one_file_system)
        .follow_links(options.dereference)
    {
        match direntry_result {
            Ok(direntry) => {
                let entry = Entry::new(&context, &direntry)?;
                copy_direntry(
                    entry,
                    options,
                    symlinked_files,
                    preserve_hard_links,
                    &mut hard_links,
                )?;
            }
            // Print an error message, but continue traversing the directory.
            Err(e) => show_error!("{}", e),
        }
    }
    // Copy the attributes from the root directory to the target directory.
    copy_attributes(root, target, &options.preserve_attributes)?;
    Ok(())
}

/// Decide whether the second path is a prefix of the first.
///
/// This function canonicalizes the paths via
/// [`uucore::fs::canonicalize`] before comparing.
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
