// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ugoa cmode

use clap::builder::ValueParser;
use clap::parser::ValuesRef;
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::ffi::OsStr;
use std::ffi::OsString;
use std::io::{self, Write, stdout};
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use uucore::error::ExitCode;
use uucore::error::{FromIo, UResult, USimpleError};
#[cfg(not(windows))]
use uucore::mode;
use uucore::quoting_style::locale_aware_c_single_escape;
#[cfg(all(unix, not(target_os = "redox")))]
use uucore::safe_traversal::{DirFd, SymlinkBehavior};
use uucore::{display::Quotable, fs::dir_strip_dot_for_creation};
use uucore::{format_usage, show_if_err};
use uucore::{translate, translate_text};

static DEFAULT_PERM: u32 = 0o777;

mod options {
    pub const MODE: &str = "mode";
    pub const PARENTS: &str = "parents";
    pub const VERBOSE: &str = "verbose";
    pub const DIRS: &str = "dirs";
    pub const SECURITY_CONTEXT: &str = "z";
    pub const CONTEXT: &str = "context";
}

/// Configuration for directory creation.
pub struct Config<'a> {
    /// Create parent directories as needed.
    pub recursive: bool,

    /// File permissions (octal) if provided via -m
    pub mode: Option<u32>,

    /// Print message for each created directory.
    pub verbose: bool,

    /// Set security context (SELinux/SMACK).
    pub set_security_context: bool,

    /// Specific `SELinux` context.
    pub context: Option<&'a String>,
}

#[cfg(windows)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "fn sig must match on all platforms"
)]
fn get_mode(_matches: &ArgMatches, _diag_args: Option<&[OsString]>) -> UResult<Option<u32>> {
    Ok(None)
}

/// `diag_args` is the argument list a caret can point into, or `None` when the
/// plain one-line message is all that is wanted.
#[cfg(not(windows))]
fn get_mode(matches: &ArgMatches, diag_args: Option<&[OsString]>) -> UResult<Option<u32>> {
    // Not tested on Windows
    let Some(m) = matches.get_one::<String>(options::MODE) else {
        // If no mode argument, let the kernel apply umask and ACLs naturally.
        return Ok(None);
    };
    mode::parse_chmod(DEFAULT_PERM, m, true, mode::get_umask())
        .map(Some)
        .map_err(|err| {
            if diag_args.is_some_and(|args| err.render_mode_value(args, m, 0, &err.to_string())) {
                // The diagnostic is already on stderr; exit quietly.
                ExitCode::new(1)
            } else {
                let quoted_mode = locale_aware_c_single_escape(m);
                USimpleError::new(
                    1,
                    translate!("mkdir-error-invalid-mode", "mode" => quoted_mode),
                )
            }
        })
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    // Linux-specific options, not implemented
    // opts.optflag("Z", "context", "set SELinux security context" +
    // " of each created directory to CTX"),
    let args: Vec<OsString> = args.collect();
    // Kept for the caret in mode diagnostics, which needs the mode as typed.
    let diag_args = uucore::diagnostics::operands(&args);
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let mode = get_mode(&matches, diag_args.as_deref())?;
    let dirs = matches
        .get_many::<OsString>(options::DIRS)
        .unwrap_or_default();
    let verbose = matches.get_flag(options::VERBOSE);
    let recursive = matches.get_flag(options::PARENTS);

    // Extract the SELinux related flags and options
    let set_security_context = matches.get_flag(options::SECURITY_CONTEXT);
    let context = matches.get_one::<String>(options::CONTEXT);

    let config = Config {
        recursive,
        mode,
        verbose,
        set_security_context: set_security_context || context.is_some(),
        context,
    };

    exec(dirs, &config);

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new("mkdir")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("mkdir"))
        .about(translate!("mkdir-about"))
        .override_usage(format_usage(&translate!("mkdir-usage")))
        .infer_long_args(true)
        .after_help(translate!("mkdir-after-help"))
        .arg(
            Arg::new(options::MODE)
                .short('m')
                .long(options::MODE)
                .help(translate!("mkdir-help-mode"))
                .allow_hyphen_values(true)
                .num_args(1),
        )
        .arg(
            Arg::new(options::PARENTS)
                .short('p')
                .long(options::PARENTS)
                .help(translate!("mkdir-help-parents"))
                .overrides_with(options::PARENTS)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long(options::VERBOSE)
                .help(translate!("mkdir-help-verbose"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SECURITY_CONTEXT)
                .short('Z')
                .help(translate!("mkdir-help-selinux"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CONTEXT)
                .long(options::CONTEXT)
                .value_name("CTX")
                .help(translate!("mkdir-help-context")),
        )
        .arg(
            Arg::new(options::DIRS)
                .action(ArgAction::Append)
                .num_args(1..)
                .required(true)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::DirPath),
        )
}

/**
 * Create the list of new directories
 */
fn exec(dirs: ValuesRef<OsString>, config: &Config) {
    for dir in dirs {
        let path_buf = PathBuf::from(dir);
        let path = path_buf.as_path();

        show_if_err!(mkdir(path, config));
    }
}

/// Create directory at a given `path`.
///
/// ## Options
///
/// * `recursive` --- create parent directories for the `path`, if they do not
///   exist.
/// * `mode` --- file mode for the directories (not implemented on windows).
/// * `verbose` --- print a message for each created directory.
///
/// ## Trailing dot
///
/// To match the GNU behavior, a path with the last directory being a single dot
/// (like `some/path/to/.`) is created (with the dot stripped).
pub fn mkdir(path: &Path, config: &Config) -> UResult<()> {
    if path.as_os_str().is_empty() {
        return Err(USimpleError::new(
            1,
            translate!("mkdir-error-empty-directory-name"),
        ));
    }
    // Special case to match GNU's behavior:
    // mkdir -p foo/. should work and just create foo/
    // std::fs::create_dir("foo/."); fails in pure Rust
    let path_buf = dir_strip_dot_for_creation(path);
    let path = path_buf.as_path();
    create_dir(path, false, config)
}

#[cfg(all(unix, not(target_os = "redox")))]
fn create_dir(path: &Path, is_parent: bool, config: &Config) -> UResult<()> {
    if path.as_os_str().is_empty() {
        return Ok(());
    }
    if !config.recursive {
        return create_single_dir(path, is_parent, config);
    }

    // Everything to handle, root to leaf: the parents, then the target itself.
    let mut todo: Vec<(&Path, bool)> = path
        .ancestors()
        .skip(1)
        .take_while(|p| !p.as_os_str().is_empty())
        .map(|p| (p, true))
        .collect();
    todo.reverse();
    todo.push((path, is_parent));

    let last = todo.len() - 1;
    let mut anchor: Option<DirFd> = None;
    for (i, (p, parent_flag)) in todo.into_iter().enumerate() {
        let is_last = i == last;
        let next = match anchor.as_ref() {
            // Chain not started: components that already exist are resolved
            // by path and followed, as before.
            None => {
                if is_last {
                    // Nothing comes after the target, so no anchor is needed;
                    // the existing path-based logic handles "already exists".
                    return create_single_dir(p, parent_flag, config);
                }
                if p.exists() {
                    continue;
                }
                create_single_dir(p, parent_flag, config)?;
                DirFd::open_anchor_with(p, SymlinkBehavior::NoFollow).map_err_context(|| {
                    translate_text!("mkdir-error-cannot-create-directory",
                        "path" => locale_aware_c_single_escape(p))
                })?
            }
            // Chain started: every later component goes through the parent's fd.
            Some(dir) => {
                let Some(comp) = p.components().next_back() else {
                    continue;
                };
                create_component(dir, comp.as_os_str(), p, parent_flag, config)?
            }
        };
        anchor = Some(next);
    }
    Ok(())
}

// Create a directory at the given path.
// Uses iterative approach instead of recursion to avoid stack overflow with deep nesting.
#[cfg(not(all(unix, not(target_os = "redox"))))]
fn create_dir(path: &Path, is_parent: bool, config: &Config) -> UResult<()> {
    if path.as_os_str().is_empty() {
        return Ok(());
    }

    // Iterative implementation: collect all directories to create, then create them
    // This avoids stack overflow with deeply nested directories
    if config.recursive {
        // Pre-allocate approximate capacity to avoid reallocations
        let mut dirs_to_create = Vec::with_capacity(16);

        // First pass: collect all parent directories
        dirs_to_create.extend(
            path.ancestors()
                .skip(1)
                .take_while(|path| !path.as_os_str().is_empty()),
        );

        // Second pass: create directories from root to leaf
        // Only create those that don't exist
        dirs_to_create
            .iter()
            .rev()
            .filter(|dir| !dir.exists())
            .try_for_each(|dir| create_single_dir(dir, true, config))?;
    }

    // Create the target directory
    create_single_dir(path, is_parent, config)
}

/// Create a directory with the exact mode specified, bypassing umask.
///
/// GNU mkdir temporarily sets umask to a shaped umask before calling mkdir(2),
/// ensuring the directory is created atomically with the correct permissions.
/// This avoids a race condition where the directory briefly exists with
/// umask-based permissions.
#[cfg(unix)]
fn create_dir_with_mode(path: &Path, mode: u32, shaped_umask: u32) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    mode::with_umask(shaped_umask, || {
        std::fs::DirBuilder::new().mode(mode).create(path)
    })
}

#[cfg(not(unix))]
fn create_dir_with_mode(path: &Path, _mode: u32, _shaped_umask: u32) -> std::io::Result<()> {
    std::fs::create_dir(path)
}

#[cfg(unix)]
fn component_mode(is_parent: bool, config: &Config) -> (u32, u32) {
    let umask = mode::get_umask();
    if is_parent {
        (DEFAULT_PERM, umask & !0o300)
    } else {
        match config.mode {
            Some(m) => (m, umask & !m),
            None => (DEFAULT_PERM, umask),
        }
    }
}

// Helper function to create a single directory with appropriate permissions
// `is_parent` argument is not used on windows
#[cfg_attr(not(unix), allow(unused_variables))]
fn create_single_dir(path: &Path, is_parent: bool, config: &Config) -> UResult<()> {
    #[cfg(unix)]
    let (mkdir_mode, shaped_umask) = component_mode(is_parent, config);
    #[cfg(not(unix))]
    let (mkdir_mode, shaped_umask) = (config.mode.unwrap_or(DEFAULT_PERM), 0u32);

    // Label the directory at creation, as GNU does; relabelling after leaves a window.
    #[cfg(all(feature = "selinux", any(target_os = "android", target_os = "linux")))]
    let _selinux_guard = if config.set_security_context && uucore::selinux::is_selinux_enabled() {
        let mode = uucore::libc::S_IFDIR | mkdir_mode as uucore::libc::mode_t;
        match uucore::selinux::FsCreateContext::new(path, Some(mode), config.context) {
            Ok(guard) => Some(guard),
            Err(e) => return Err(USimpleError::new(1, e.to_string())),
        }
    } else {
        None
    };

    match create_dir_with_mode(path, mkdir_mode, shaped_umask) {
        Ok(()) => {
            if config.verbose {
                writeln!(
                    stdout(),
                    "{}",
                    translate!("mkdir-verbose-created-directory", "util_name" => "mkdir", "path" => path.quote())
                )?;
            }

            // Apply SMACK context if requested
            #[cfg(all(feature = "smack", target_os = "linux"))]
            if config.set_security_context {
                uucore::smack::set_smack_label_and_cleanup(path, config.context, |p| {
                    std::fs::remove_dir(p)
                })?;
            }
            Ok(())
        }

        Err(_) if config.recursive && path.is_dir() => {
            // Directory already exists - check if this is a logical directory creation
            // (i.e., not just a parent reference like "test_dir/..")
            let ends_with_parent_dir = matches!(
                path.components().next_back(),
                Some(std::path::Component::ParentDir)
            );

            // Print verbose message for logical directories, even if they exist
            // This matches GNU behavior for paths like "test_dir/../test_dir_a"
            if config.verbose && is_parent && config.recursive && !ends_with_parent_dir {
                writeln!(
                    stdout(),
                    "{}",
                    translate!("mkdir-verbose-created-directory", "util_name" => "mkdir", "path" => path.quote())
                )?;
            }
            Ok(())
        }
        Err(e) => Err(e.map_err_context(|| {
            translate_text!("mkdir-error-cannot-create-directory", "path" => locale_aware_c_single_escape(path))
        })),
    }
}

/// mkdirat on the parent's fd, under the same umask shaping as `create_dir_with_mode`.
#[cfg(all(unix, not(target_os = "redox")))]
fn mkdir_at_with_mode(
    parent: &DirFd,
    name: &OsStr,
    perm: u32,
    shaped_umask: u32,
) -> io::Result<()> {
    mode::with_umask(shaped_umask, || parent.mkdir_at(name, perm))
}

/// Create one component relative to `parent` and return an fd for it.
/// A directory we created is opened with `O_NOFOLLOW`; one that already
/// existed is followed, and no -v line is printed for it.
#[cfg(all(unix, not(target_os = "redox")))]
fn create_component(
    parent: &DirFd,
    name: &OsStr,
    path: &Path,
    is_parent: bool,
    config: &Config,
) -> UResult<DirFd> {
    let (mkdir_mode, shaped_umask) = component_mode(is_parent, config);
    let ctx = || {
        translate_text!("mkdir-error-cannot-create-directory",
            "path" => locale_aware_c_single_escape(path))
    };

    // Label the directory at creation, as GNU does; relabelling after leaves a window.
    #[cfg(all(feature = "selinux", any(target_os = "android", target_os = "linux")))]
    let _selinux_guard = if config.set_security_context && uucore::selinux::is_selinux_enabled() {
        let mode = uucore::libc::S_IFDIR | mkdir_mode as uucore::libc::mode_t;
        match uucore::selinux::FsCreateContext::new(path, Some(mode), config.context) {
            Ok(guard) => Some(guard),
            Err(e) => return Err(USimpleError::new(1, e.to_string())),
        }
    } else {
        None
    };

    match mkdir_at_with_mode(parent, name, mkdir_mode, shaped_umask) {
        Ok(()) => {
            // Created by us: refuse symlinks from here on.
            let dir = parent
                .open_subdir_anchor(name, SymlinkBehavior::NoFollow)
                .map_err_context(ctx)?;

            if config.verbose {
                writeln!(
                    stdout(),
                    "{}",
                    translate!("mkdir-verbose-created-directory", "util_name" => "mkdir", "path" => path.quote())
                )?;
            }

            // Apply SMACK context if requested
            #[cfg(all(feature = "smack", target_os = "linux"))]
            if config.set_security_context {
                uucore::smack::set_smack_label_and_cleanup(path, config.context, |p| {
                    std::fs::remove_dir(p)
                })?;
            }
            Ok(dir)
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            // Not created by us: follow symlinks, print no -v line.
            // If it exists but isn't a directory, report the original error.
            parent
                .open_subdir_anchor(name, SymlinkBehavior::Follow)
                .map_err(|_| e)
                .map_err_context(ctx)
        }
        Err(e) => Err(e.map_err_context(ctx)),
    }
}
