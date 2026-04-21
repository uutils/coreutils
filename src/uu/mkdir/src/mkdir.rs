// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ugoa cmode RAII

use clap::builder::ValueParser;
use clap::parser::ValuesRef;
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::ffi::OsString;
use std::io::{Write, stdout};
use std::path::{Path, PathBuf};
use uucore::error::{UResult, USimpleError};
use uucore::translate;

#[cfg(not(windows))]
use uucore::mode;
use uucore::{display::Quotable, fs::dir_strip_dot_for_creation};
use uucore::{format_usage, show_if_err};

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
fn get_mode(_matches: &ArgMatches) -> Result<Option<u32>, String> {
    Ok(None)
}

#[cfg(not(windows))]
fn get_mode(matches: &ArgMatches) -> Result<Option<u32>, String> {
    // Not tested on Windows
    if let Some(m) = matches.get_one::<String>(options::MODE) {
        mode::parse_chmod(DEFAULT_PERM, m, true, mode::get_umask()).map(Some)
    } else {
        // If no mode argument, let the kernel apply umask and ACLs naturally.
        Ok(None)
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    // Linux-specific options, not implemented
    // opts.optflag("Z", "context", "set SELinux security context" +
    // " of each created directory to CTX"),
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let dirs = matches
        .get_many::<OsString>(options::DIRS)
        .unwrap_or_default();
    let verbose = matches.get_flag(options::VERBOSE);
    let recursive = matches.get_flag(options::PARENTS);

    // Extract the SELinux related flags and options
    let set_security_context = matches.get_flag(options::SECURITY_CONTEXT);
    let context = matches.get_one::<String>(options::CONTEXT);

    match get_mode(&matches) {
        Ok(mode) => {
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
        Err(f) => Err(USimpleError::new(1, f)),
    }
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
/// * `verbose` --- print a message for each printed directory.
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

// Create a directory at the given path.
// Uses iterative approach instead of recursion to avoid stack overflow with deep nesting.
fn create_dir(path: &Path, is_parent: bool, config: &Config) -> UResult<()> {
    let path_exists = path.exists();
    if path_exists && !config.recursive {
        return Err(USimpleError::new(
            1,
            translate!("mkdir-error-file-exists", "path" => path.maybe_quote()),
        ));
    }
    if path == Path::new("") {
        return Ok(());
    }

    // Iterative implementation: collect all directories to create, then create them
    // This avoids stack overflow with deeply nested directories
    if config.recursive {
        // Pre-allocate approximate capacity to avoid reallocations
        let mut dirs_to_create = Vec::with_capacity(16);
        let mut current = path;

        // First pass: collect all parent directories
        while let Some(parent) = current.parent() {
            if parent == Path::new("") {
                break;
            }
            dirs_to_create.push(parent);
            current = parent;
        }

        // Second pass: create directories from root to leaf
        // Only create those that don't exist
        for dir in dirs_to_create.iter().rev() {
            if !dir.exists() {
                create_single_dir(dir, true, config)?;
            }
        }
    }

    // Create the target directory
    create_single_dir(path, is_parent, config)
}

/// RAII guard to restore umask on drop, ensuring cleanup even on panic.
#[cfg(unix)]
struct UmaskGuard(rustix::fs::Mode);

#[cfg(unix)]
impl UmaskGuard {
    /// Set umask to the given value and return a guard that restores the original on drop.
    fn set(new_mask: rustix::fs::Mode) -> Self {
        let old_mask = rustix::process::umask(new_mask);
        Self(old_mask)
    }
}

#[cfg(unix)]
impl Drop for UmaskGuard {
    fn drop(&mut self) {
        rustix::process::umask(self.0);
    }
}

/// Create a directory with the exact mode specified, bypassing umask.
///
/// GNU mkdir temporarily sets umask to shaped mask before calling mkdir(2), ensuring the
/// directory is created atomically with the correct permissions. This avoids a
/// race condition where the directory briefly exists with umask-based permissions.
#[cfg(unix)]
fn create_dir_with_mode(
    path: &Path,
    mode: u32,
    shaped_umask: rustix::fs::Mode,
) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    let _guard = UmaskGuard::set(shaped_umask);

    std::fs::DirBuilder::new().mode(mode).create(path)
}

#[cfg(not(unix))]
fn create_dir_with_mode(path: &Path, _mode: u32, _shaped_umask: u32) -> std::io::Result<()> {
    std::fs::create_dir(path)
}

// Helper function to create a single directory with appropriate permissions
// `is_parent` argument is not used on windows
#[allow(unused_variables)]
fn create_single_dir(path: &Path, is_parent: bool, config: &Config) -> UResult<()> {
    #[cfg(unix)]
    let (mkdir_mode, shaped_umask) = {
        let umask = mode::get_umask();
        let umask_bits = rustix::fs::Mode::from_bits_truncate(umask);
        if is_parent {
            // Parent directories are never affected by -m (matches GNU behavior).
            // We pass 0o777 as the mode and shape the umask so it cannot block
            // owner write or execute (u+wx), ensuring the owner can traverse and
            // write into the parent to create children. All other umask bits are
            // preserved so the kernel applies them — and any default ACL on the
            // grandparent — through the normal mkdir(2) path.
            (
                DEFAULT_PERM,
                umask_bits & !rustix::fs::Mode::from_bits_truncate(0o300),
            )
        } else {
            match config.mode {
                // Explicit -m: shape umask so it cannot block explicitly requested bits.
                Some(m) => (m, umask_bits & !rustix::fs::Mode::from_bits_truncate(m)),
                // No -m: leave umask fully intact; kernel applies umask + ACL naturally.
                None => (DEFAULT_PERM, umask_bits),
            }
        }
    };
    #[cfg(not(unix))]
    let (mkdir_mode, shaped_umask) = (config.mode.unwrap_or(DEFAULT_PERM), 0u32);

    match create_dir_with_mode(path, mkdir_mode, shaped_umask) {
        Ok(()) => {
            if config.verbose {
                writeln!(
                    stdout(),
                    "{}",
                    translate!("mkdir-verbose-created-directory", "util_name" => "mkdir", "path" => path.quote())
                )?;
            }

            // Apply SELinux context if requested
            #[cfg(feature = "selinux")]
            if config.set_security_context && uucore::selinux::is_selinux_enabled() {
                if let Err(e) = uucore::selinux::set_selinux_security_context(path, config.context)
                {
                    let _ = std::fs::remove_dir(path);
                    return Err(USimpleError::new(1, e.to_string()));
                }
            }

            // Apply SMACK context if requested
            #[cfg(feature = "smack")]
            if config.set_security_context {
                uucore::smack::set_smack_label_and_cleanup(path, config.context, |p| {
                    std::fs::remove_dir(p)
                })?;
            }
            Ok(())
        }

        Err(_) if path.is_dir() => {
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
        Err(e) => Err(e.into()),
    }
}
