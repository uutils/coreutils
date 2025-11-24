// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ugoa cmode

use clap::builder::ValueParser;
use clap::parser::ValuesRef;
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use uucore::error::FromIo;
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
    pub const SELINUX: &str = "z";
    pub const CONTEXT: &str = "context";
}

/// Configuration for directory creation.
pub struct Config<'a> {
    /// Create parent directories as needed.
    pub recursive: bool,

    /// File permissions (octal).
    pub mode: u32,

    /// Print message for each created directory.
    pub verbose: bool,

    /// Set `SELinux` security context.
    pub set_selinux_context: bool,

    /// Specific `SELinux` context.
    pub context: Option<&'a String>,
}

#[cfg(windows)]
fn get_mode(_matches: &ArgMatches) -> Result<u32, String> {
    Ok(DEFAULT_PERM)
}

#[cfg(not(windows))]
fn get_mode(matches: &ArgMatches) -> Result<u32, String> {
    // Not tested on Windows
    let mut new_mode = DEFAULT_PERM;

    if let Some(m) = matches.get_one::<String>(options::MODE) {
        for mode in m.split(',') {
            if mode.chars().any(|c| c.is_ascii_digit()) {
                new_mode = mode::parse_numeric(new_mode, m, true)?;
            } else {
                new_mode = mode::parse_symbolic(new_mode, mode, mode::get_umask(), true)?;
            }
        }
        Ok(new_mode)
    } else {
        // If no mode argument is specified return the mode derived from umask
        Ok(!mode::get_umask() & 0o0777)
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
    let set_selinux_context = matches.get_flag(options::SELINUX);
    let context = matches.get_one::<String>(options::CONTEXT);

    match get_mode(&matches) {
        Ok(mode) => {
            let config = Config {
                recursive,
                mode,
                verbose,
                set_selinux_context: set_selinux_context || context.is_some(),
                context,
            };
            exec(dirs, &config)
        }
        Err(f) => Err(USimpleError::new(1, f)),
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
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
            Arg::new(options::SELINUX)
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
fn exec(dirs: ValuesRef<OsString>, config: &Config) -> UResult<()> {
    for dir in dirs {
        let path_buf = PathBuf::from(dir);
        let path = path_buf.as_path();

        show_if_err!(mkdir(path, config));
    }
    Ok(())
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

#[cfg(any(unix, target_os = "redox"))]
fn chmod(path: &Path, mode: u32) -> UResult<()> {
    use std::fs::{Permissions, set_permissions};
    use std::os::unix::fs::PermissionsExt;
    let mode = Permissions::from_mode(mode);
    set_permissions(path, mode).map_err_context(
        || translate!("mkdir-error-cannot-set-permissions", "path" => path.quote()),
    )
}

#[cfg(windows)]
fn chmod(_path: &Path, _mode: u32) -> UResult<()> {
    // chmod on Windows only sets the readonly flag, which isn't even honored on directories
    Ok(())
}

// Create a directory at the given path.
// Uses iterative approach instead of recursion to avoid stack overflow with deep nesting.
fn create_dir(path: &Path, is_parent: bool, config: &Config) -> UResult<()> {
    let path_exists = path.exists();
    if path_exists && !config.recursive {
        return Err(USimpleError::new(
            1,
            translate!("mkdir-error-file-exists", "path" => path.to_string_lossy()),
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

// Helper function to create a single directory with appropriate permissions
// `is_parent` argument is not used on windows
#[allow(unused_variables)]
fn create_single_dir(path: &Path, is_parent: bool, config: &Config) -> UResult<()> {
    let path_exists = path.exists();

    match std::fs::create_dir(path) {
        Ok(()) => {
            if config.verbose {
                println!(
                    "{}",
                    translate!("mkdir-verbose-created-directory", "util_name" => uucore::util_name(), "path" => path.quote())
                );
            }

            #[cfg(all(unix, target_os = "linux"))]
            let new_mode = if path_exists {
                config.mode
            } else {
                // TODO: Make this macos and freebsd compatible by creating a function to get permission bits from
                // acl in extended attributes
                let acl_perm_bits = uucore::fsxattr::get_acl_perm_bits_from_xattr(path);

                if is_parent {
                    (!mode::get_umask() & 0o777) | 0o300 | acl_perm_bits
                } else {
                    config.mode | acl_perm_bits
                }
            };
            #[cfg(all(unix, not(target_os = "linux")))]
            let new_mode = if is_parent {
                (!mode::get_umask() & 0o777) | 0o300
            } else {
                config.mode
            };
            #[cfg(windows)]
            let new_mode = config.mode;

            chmod(path, new_mode)?;

            // Apply SELinux context if requested
            #[cfg(feature = "selinux")]
            if config.set_selinux_context && uucore::selinux::is_selinux_enabled() {
                if let Err(e) = uucore::selinux::set_selinux_security_context(path, config.context)
                {
                    let _ = std::fs::remove_dir(path);
                    return Err(USimpleError::new(1, e.to_string()));
                }
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
                println!(
                    "{}",
                    translate!("mkdir-verbose-created-directory", "util_name" => uucore::util_name(), "path" => path.quote())
                );
            }
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
