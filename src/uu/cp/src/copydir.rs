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
use std::path::Path;

use uucore::display::Quotable;
use uucore::error::UIoError;
use uucore::fs::{canonicalize, FileInformation, MissingHandling, ResolveMode};
use walkdir::WalkDir;

use crate::{
    copy_attributes, copy_file, copy_link, preserve_hardlinks, CopyResult, Error, Options,
    TargetSlice,
};

/// Continue next iteration of loop if result of expression is error
macro_rules! or_continue(
    ($expr:expr) => (match $expr {
        Ok(temp) => temp,
        Err(error) => {
            show_error!("{}", error);
            continue
        },
    })
);

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

    let current_dir =
        env::current_dir().unwrap_or_else(|e| crash!(1, "failed to get current directory {}", e));

    let root_path = current_dir.join(root);

    let root_parent = if target.exists() {
        root_path.parent()
    } else {
        Some(root_path.as_path())
    };

    #[cfg(unix)]
    let mut hard_links: Vec<(String, u64)> = vec![];
    let preserve_hard_links = options.preserve_hard_links();

    // This should be changed once Redox supports hardlinks
    #[cfg(any(windows, target_os = "redox"))]
    let mut hard_links: Vec<(String, u64)> = vec![];

    for path in WalkDir::new(root)
        .same_file_system(options.one_file_system)
        .follow_links(options.dereference)
    {
        let p = or_continue!(path);
        let path = current_dir.join(p.path());

        let local_to_root_parent = match root_parent {
            Some(parent) => {
                #[cfg(windows)]
                {
                    // On Windows, some paths are starting with \\?
                    // but not always, so, make sure that we are consistent for strip_prefix
                    // See https://docs.microsoft.com/en-us/windows/win32/fileio/naming-a-file for more info
                    let parent_can = adjust_canonicalization(parent);
                    let path_can = adjust_canonicalization(&path);

                    or_continue!(&path_can.strip_prefix(&parent_can)).to_path_buf()
                }
                #[cfg(not(windows))]
                {
                    or_continue!(path.strip_prefix(parent)).to_path_buf()
                }
            }
            None => path.clone(),
        };

        let local_to_target = target.join(&local_to_root_parent);
        if p.path().is_symlink() && !options.dereference {
            copy_link(&path, &local_to_target, symlinked_files)?;
        } else if path.is_dir() && !local_to_target.exists() {
            if target.is_file() {
                return Err("cannot overwrite non-directory with directory".into());
            }
            fs::create_dir_all(local_to_target)?;
        } else if !path.is_dir() {
            if preserve_hard_links {
                let mut found_hard_link = false;
                let source = path.to_path_buf();
                let dest = local_to_target.as_path().to_path_buf();
                preserve_hardlinks(&mut hard_links, &source, &dest, &mut found_hard_link)?;
                if !found_hard_link {
                    match copy_file(
                        path.as_path(),
                        local_to_target.as_path(),
                        options,
                        symlinked_files,
                        false,
                    ) {
                        Ok(_) => Ok(()),
                        Err(err) => {
                            if source.is_symlink() {
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
                    path.as_path(),
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
                            p.path().quote()
                        ));
                    }
                    Err(e) => return Err(e),
                }
            }
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
