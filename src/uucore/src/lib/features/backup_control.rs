// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Implement GNU-style backup functionality.
//!
//! This module implements the backup functionality as described in the [GNU
//! manual][1]. It provides
//!
//! - pre-defined [`clap`-Arguments][2] for inclusion in utilities that
//!   implement backups
//! - determination of the [backup mode][3]
//! - determination of the [backup suffix][4]
//! - [backup target path construction][5]
//! - [Error types][6] for backup-related errors
//! - GNU-compliant [help texts][7] for backup-related errors
//!
//! Backup-functionality is implemented by the following utilities:
//!
//! - `cp`
//! - `install`
//! - `ln`
//! - `mv`
//!
//!
//! [1]: https://www.gnu.org/software/coreutils/manual/html_node/Backup-options.html
//! [2]: arguments
//! [3]: `determine_backup_mode()`
//! [4]: `determine_backup_suffix()`
//! [5]: `get_backup_path()`
//! [6]: `BackupError`
//! [7]: `BACKUP_CONTROL_LONG_HELP`
//!
//!
//! # Usage example
//!
//! ```
//! #[macro_use]
//! extern crate uucore;
//!
//! use clap::{Command, Arg, ArgMatches};
//! use std::path::{Path, PathBuf};
//! use uucore::backup_control::{self, BackupMode};
//! use uucore::error::{UError, UResult};
//!
//! fn main() {
//!     let usage = String::from("command [OPTION]... ARG");
//!     let long_usage = String::from("And here's a detailed explanation");
//!
//!     let matches = Command::new("command")
//!         .arg(backup_control::arguments::backup())
//!         .arg(backup_control::arguments::backup_no_args())
//!         .arg(backup_control::arguments::suffix())
//!         .override_usage(usage)
//!         .after_help(format!(
//!             "{long_usage}\n{}",
//!             backup_control::BACKUP_CONTROL_LONG_HELP
//!         ))
//!         .get_matches_from(vec![
//!             "command", "--backup=t", "--suffix=bak~"
//!         ]);
//!
//!     let backup_mode = match backup_control::determine_backup_mode(std::env::var("VERSION_CONTROL").ok(), &matches) {
//!         Err(e) => {
//!             show!(e);
//!             return;
//!         },
//!         Ok(mode) => mode,
//!     };
//!     let backup_suffix = backup_control::determine_backup_suffix(&matches);
//!     let target_path = Path::new("/tmp/example");
//!
//!     let backup_path = backup_control::get_backup_path(
//!         backup_mode, target_path, &backup_suffix
//!     );
//!
//!     // Perform your backups here.
//!
//! }
//! ```

// spell-checker:ignore backupopt

use crate::{
    display::Quotable,
    error::{UError, UResult},
    fs::FileInformation,
};
use clap::ArgMatches;
use std::{
    env,
    error::Error,
    ffi::{OsStr, OsString},
    fmt::{Debug, Display},
    fs,
    path::{Path, PathBuf},
};

pub static BACKUP_CONTROL_VALUES: &[&str] = &[
    "simple", "never", "numbered", "t", "existing", "nil", "none", "off",
];

pub const BACKUP_CONTROL_LONG_HELP: &str =
    "The backup suffix is '~', unless set with --suffix or SIMPLE_BACKUP_SUFFIX.
The version control method may be selected via the --backup option or through
the VERSION_CONTROL environment variable.  Here are the values:

  none, off       never make backups (even if --backup is given)
  numbered, t     make numbered backups
  existing, nil   numbered if numbered backups exist, simple otherwise
  simple, never   always make simple backups";

static VALID_ARGS_HELP: &str = "Valid arguments are:
  - 'none', 'off'
  - 'simple', 'never'
  - 'existing', 'nil'
  - 'numbered', 't'";

pub const DEFAULT_BACKUP_SUFFIX: &str = "~";

/// Available backup modes.
///
/// The mapping of the backup modes to the CLI arguments is annotated on the
/// enum variants.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum BackupMode {
    /// Argument 'none', 'off'
    #[default]
    None,
    /// Argument 'simple', 'never'
    Simple,
    /// Argument 'numbered', 't'
    Numbered,
    /// Argument 'existing', 'nil'
    Existing,
}

/// Backup error types.
///
/// Errors are currently raised by [`determine_backup_mode`] only. All errors
/// are implemented as [`UError`] for uniform handling across utilities.
#[derive(Debug, Eq, PartialEq)]
pub enum BackupError {
    /// An invalid argument (e.g. 'foo') was given as backup type. First
    /// parameter is the argument, second is the arguments origin (CLI or
    /// ENV-var)
    InvalidArgument(String, String),
    /// An ambiguous argument (e.g. 'n') was given as backup type. First
    /// parameter is the argument, second is the arguments origin (CLI or
    /// ENV-var)
    AmbiguousArgument(String, String),
    /// Currently unused
    BackupImpossible(),
    // BackupFailed(PathBuf, PathBuf, std::io::Error),
}

impl UError for BackupError {
    fn code(&self) -> i32 {
        match self {
            Self::BackupImpossible() => 2,
            _ => 1,
        }
    }

    fn usage(&self) -> bool {
        // Suggested by clippy.
        matches!(
            self,
            Self::InvalidArgument(_, _) | Self::AmbiguousArgument(_, _)
        )
    }
}

impl Error for BackupError {}

impl Display for BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidArgument(arg, origin) => write!(
                f,
                "invalid argument {} for '{origin}'\n{VALID_ARGS_HELP}",
                arg.quote(),
            ),
            Self::AmbiguousArgument(arg, origin) => write!(
                f,
                "ambiguous argument {} for '{origin}'\n{VALID_ARGS_HELP}",
                arg.quote(),
            ),
            Self::BackupImpossible() => write!(f, "cannot create backup"),
            // Placeholder for later
            // Self::BackupFailed(from, to, e) => Display::fmt(
            //     &uio_error!(e, "failed to backup {} to {}", from.quote(), to.quote()),
            //     f
            // ),
        }
    }
}

/// Arguments for backup-related functionality.
///
/// Rather than implementing the `clap`-Arguments for every utility, it is
/// recommended to include the `clap` arguments via the functions provided here.
/// This way the backup-specific arguments are handled uniformly across
/// utilities and can be maintained in one central place.
pub mod arguments {
    use clap::ArgAction;

    pub static OPT_BACKUP: &str = "backupopt_backup";
    pub static OPT_BACKUP_NO_ARG: &str = "backupopt_b";
    pub static OPT_SUFFIX: &str = "backupopt_suffix";

    /// '--backup' argument
    pub fn backup() -> clap::Arg {
        clap::Arg::new(OPT_BACKUP)
            .long("backup")
            .help("make a backup of each existing destination file")
            .action(ArgAction::Set)
            .require_equals(true)
            .num_args(0..=1)
            .value_name("CONTROL")
    }

    /// '-b' argument
    pub fn backup_no_args() -> clap::Arg {
        clap::Arg::new(OPT_BACKUP_NO_ARG)
            .short('b')
            .help("like --backup but does not accept an argument")
            .action(ArgAction::SetTrue)
    }

    /// '-S, --suffix' argument
    pub fn suffix() -> clap::Arg {
        clap::Arg::new(OPT_SUFFIX)
            .short('S')
            .long("suffix")
            .help("override the usual backup suffix")
            .action(ArgAction::Set)
            .value_name("SUFFIX")
            .allow_hyphen_values(true)
    }
}

/// Obtain the suffix to use for a backup.
///
/// In order of precedence, this function obtains the backup suffix
///
/// 1. From the '-S' or '--suffix' CLI argument, if present
/// 2. From the "SIMPLE_BACKUP_SUFFIX" environment variable, if present
/// 3. By using the default '~' if none of the others apply, or if they contained slashes
///
/// This function directly takes [`ArgMatches`] as argument and looks for
/// the '-S' and '--suffix' arguments itself.
pub fn determine_backup_suffix(matches: &ArgMatches) -> String {
    let supplied_suffix = matches.get_one::<String>(arguments::OPT_SUFFIX);
    let suffix = if let Some(suffix) = supplied_suffix {
        String::from(suffix)
    } else {
        env::var("SIMPLE_BACKUP_SUFFIX").unwrap_or_else(|_| DEFAULT_BACKUP_SUFFIX.to_owned())
    };
    if suffix.contains('/') {
        DEFAULT_BACKUP_SUFFIX.to_owned()
    } else {
        suffix
    }
}

/// Determine the "mode" for the backup operation to perform, if any.
///
/// Parses the backup options according to the [GNU manual][1], and converts
/// them to an instance of `BackupMode` for further processing.
///
/// Takes [`ArgMatches`] as argument which **must** contain the options
/// from [`arguments::backup()`] and [`arguments::backup_no_args()`]. Otherwise
/// the `NoBackup` mode is returned unconditionally.
///
/// It is recommended for anyone who would like to implement the
/// backup-functionality to use the arguments prepared in the `arguments`
/// submodule (see examples)
///
/// [1]: https://www.gnu.org/software/coreutils/manual/html_node/Backup-options.html
///
///
/// # Errors
///
/// If an argument supplied directly to the long `backup` option, or read in
/// through the `VERSION CONTROL` env var is ambiguous (i.e. may resolve to
/// multiple backup modes) or invalid, an [`InvalidArgument`][10] or
/// [`AmbiguousArgument`][11] error is returned, respectively.
///
/// [10]: BackupError::InvalidArgument
/// [11]: BackupError::AmbiguousArgument
///
///
/// # Examples
///
/// Here's how one would integrate the backup mode determination into an
/// application.
///
/// ```
/// #[macro_use]
/// extern crate uucore;
/// use uucore::backup_control::{self, BackupMode};
/// use clap::{Command, Arg, ArgMatches};
///
/// fn main() {
///     let matches = Command::new("command")
///         .arg(backup_control::arguments::backup())
///         .arg(backup_control::arguments::backup_no_args())
///         .get_matches_from(vec![
///             "command", "-b", "--backup=t"
///         ]);
///
///     let backup_mode = backup_control::determine_backup_mode(std::env::var("VERSION_CONTROL").ok(), &matches).unwrap();
///     assert_eq!(backup_mode, BackupMode::Numbered)
/// }
/// ```
///
/// This example shows an ambiguous input, as 'n' may resolve to 4 different
/// backup modes.
///
///
/// ```
/// #[macro_use]
/// extern crate uucore;
/// use uucore::backup_control::{self, BackupMode, BackupError};
/// use clap::{Command, Arg, ArgMatches};
///
/// fn main() {
///     let matches = Command::new("command")
///         .arg(backup_control::arguments::backup())
///         .arg(backup_control::arguments::backup_no_args())
///         .get_matches_from(vec![
///             "command", "-b", "--backup=n"
///         ]);
///
///     let backup_mode = backup_control::determine_backup_mode(std::env::var("VERSION_CONTROL").ok(), &matches);
///
///     assert!(backup_mode.is_err());
///     let err = backup_mode.unwrap_err();
///     // assert_eq!(err, BackupError::AmbiguousArgument);
///     // Use uucore functionality to show the error to the user
///     show!(err);
/// }
/// ```
pub fn determine_backup_mode(env_ctl: Option<String>, matches: &ArgMatches) -> UResult<BackupMode> {
    if matches.contains_id(arguments::OPT_BACKUP) {
        // Use method to determine the type of backups to make. When this option
        // is used but method is not specified, then the value of the
        // VERSION_CONTROL environment variable is used. And if VERSION_CONTROL
        // is not set, the default backup type is 'existing'.
        if let Some(method) = matches.get_one::<String>(arguments::OPT_BACKUP) {
            // Second argument is for the error string that is returned.
            match_method(method, "backup type")
        } else if let Some(method) = env_ctl {
            // Second argument is for the error string that is returned.
            match_method(&method, "$VERSION_CONTROL")
        } else {
            // Default if no argument is provided to '--backup'
            Ok(BackupMode::Existing)
        }
    } else if matches.get_flag(arguments::OPT_BACKUP_NO_ARG) {
        // the short form of this option, -b does not accept any argument.
        // if VERSION_CONTROL is not set then using -b is equivalent to
        // using --backup=existing.
        if let Some(method) = env_ctl {
            match_method(&method, "$VERSION_CONTROL")
        } else {
            Ok(BackupMode::Existing)
        }
    } else if matches.contains_id(arguments::OPT_SUFFIX) {
        // Suffix option is enough to determine mode even if --backup is not set.
        // If VERSION_CONTROL is not set, the default backup type is 'existing'.
        if let Some(method) = env_ctl {
            match_method(&method, "$VERSION_CONTROL")
        } else {
            Ok(BackupMode::Existing)
        }
    } else {
        // No option was present at all
        Ok(BackupMode::None)
    }
}

/// Match a backup option string to a `BackupMode`.
///
/// The GNU manual specifies that abbreviations to options are valid as long as
/// they aren't ambiguous. This function matches the given `method` argument
/// against all valid backup options (via `starts_with`), and returns a valid
/// [`BackupMode`] if exactly one backup option matches the `method` given.
///
/// `origin` is required in order to format the generated error message
/// properly, when an error occurs.
///
///
/// # Errors
///
/// If `method` is invalid or ambiguous (i.e. may resolve to multiple backup
/// modes), an [`InvalidArgument`][10] or [`AmbiguousArgument`][11] error is
/// returned, respectively.
///
/// [10]: BackupError::InvalidArgument
/// [11]: BackupError::AmbiguousArgument
fn match_method(method: &str, origin: &str) -> UResult<BackupMode> {
    let matches: Vec<&&str> = BACKUP_CONTROL_VALUES
        .iter()
        .filter(|val| val.starts_with(method))
        .collect();
    if matches.len() == 1 {
        match *matches[0] {
            "simple" | "never" => Ok(BackupMode::Simple),
            "numbered" | "t" => Ok(BackupMode::Numbered),
            "existing" | "nil" => Ok(BackupMode::Existing),
            "none" | "off" => Ok(BackupMode::None),
            _ => unreachable!(), // cannot happen as we must have exactly one match
                                 // from the list above.
        }
    } else if matches.is_empty() {
        Err(BackupError::InvalidArgument(method.to_string(), origin.to_string()).into())
    } else {
        Err(BackupError::AmbiguousArgument(method.to_string(), origin.to_string()).into())
    }
}

pub fn get_backup_path<S: AsRef<OsStr>>(
    backup_mode: BackupMode,
    backup_path: &Path,
    suffix: S,
) -> Option<PathBuf> {
    match backup_mode {
        BackupMode::None => None,
        BackupMode::Simple => Some(simple_backup_path(backup_path, suffix.as_ref())),
        BackupMode::Numbered => Some(numbered_backup_path(backup_path)),
        BackupMode::Existing => Some(existing_backup_path(backup_path, suffix.as_ref())),
    }
}

fn simple_backup_path<S: AsRef<OsStr>>(path: &Path, suffix: S) -> PathBuf {
    let mut file_name = path.file_name().unwrap_or_default().to_os_string();
    file_name.push(suffix.as_ref());
    path.with_file_name(file_name)
}

fn numbered_backup_path(path: &Path) -> PathBuf {
    let mut i: u64 = 1;
    loop {
        let new_path = simple_backup_path(path, OsString::from(format!(".~{i}~")));
        // Use `symlink_metadata` rather than `exists()` so that a dangling
        // symlink still counts as an existing backup (avoiding a silent
        // overwrite), and so we do not report a live symlink as missing when
        // the target cannot be stat'd.
        if fs::symlink_metadata(&new_path).is_err() {
            return new_path;
        }
        i += 1;
    }
}

fn existing_backup_path<S: AsRef<OsStr>>(path: &Path, suffix: S) -> PathBuf {
    let test_path = simple_backup_path(path, OsString::from(".~1~"));
    if fs::symlink_metadata(&test_path).is_ok() {
        return numbered_backup_path(path);
    }
    simple_backup_path(path, suffix.as_ref())
}

/// Returns true if backing up `target` would destroy `source`.
///
/// Backing up renames `target` onto its backup name. When `source` *is* that
/// backup, the rename clobbers it and the operation has nothing left to read,
/// leaving both files empty. GNU refuses instead (`source_is_dst_backup` in
/// `copy.c`), and `cp`, `mv` and `install` all need the same answer.
///
/// The mode is part of the question, not the caller's business: numbered
/// backups never reuse an existing name, and [`BackupMode::None`] performs no
/// rename at all, so neither can destroy anything. Both return false here so no
/// caller has to remember the rule - getting that gate wrong per-utility is
/// exactly how `mv` came to miss `--backup=existing`.
///
/// # Arguments
///
/// * `source` - A Path reference that holds the source (backup) file path.
/// * `target` - A Path reference that holds the target file path.
/// * `suffix` - Str that holds the backup suffix.
/// * `mode` - The backup mode in effect.
/// * `dereference` - Whether the source is resolved through symlinks, matching
///   how the calling utility opens it (`false` for `mv`, `true` for `cp`/`install`).
///
/// # Examples
///
/// ```
/// use std::fs;
/// use std::path::Path;
/// use uucore::backup_control::{BackupMode, backup_would_destroy_source};
///
/// let dir = tempfile::tempdir().unwrap();
/// let target = dir.path().join("data.txt");
/// let source = dir.path().join("data.txt~");
/// fs::write(&target, "").unwrap();
/// fs::write(&source, "").unwrap();
///
/// // `./data.txt~` and `data.txt~` name the same file, so both are caught.
/// assert!(backup_would_destroy_source(
///     &source, &target, "~", BackupMode::Simple, false
/// ));
///
/// // A numbered backup picks a fresh name, so the source is safe.
/// assert!(!backup_would_destroy_source(
///     &source, &target, "~", BackupMode::Numbered, false
/// ));
/// ```
///
pub fn backup_would_destroy_source(
    source: &Path,
    target: &Path,
    suffix: &str,
    mode: BackupMode,
    dereference: bool,
) -> bool {
    if matches!(mode, BackupMode::None | BackupMode::Numbered) {
        return false;
    }
    // GNU gates on the file names first: only a source whose final component is
    // the target's final component plus the suffix can be clobbered by the
    // backup rename. This keeps unrelated files that merely happen to share an
    // inode (a hard link, say) from being refused.
    let (Some(source_base), Some(target_base)) = (source.file_name(), target.file_name()) else {
        return false;
    };
    let mut expected_base = target_base.to_owned();
    expected_base.push(suffix);
    if source_base != expected_base {
        return false;
    }

    // Then compare the files themselves rather than how they were spelled, so
    // `a~`, `./a~` and an absolute path are all recognised. If the backup does
    // not exist yet there is nothing to destroy.
    let mut target_backup_filename = target.as_os_str().to_owned();
    target_backup_filename.push(suffix);
    let target_backup = PathBuf::from(target_backup_filename);

    match (
        FileInformation::from_path(source, dereference),
        FileInformation::from_path(&target_backup, true),
    ) {
        (Ok(source_info), Ok(backup_info)) => source_info == backup_info,
        _ => false,
    }
}

//
// Tests for this module
//
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Command;

    fn make_app() -> Command {
        Command::new("command")
            .arg(arguments::backup())
            .arg(arguments::backup_no_args())
            .arg(arguments::suffix())
    }

    // Defaults to --backup=existing
    #[test]
    fn test_backup_mode_short_only() {
        let matches = make_app().get_matches_from(vec!["command", "-b"]);
        let result = determine_backup_mode(None, &matches).unwrap();
        assert_eq!(result, BackupMode::Existing);
    }

    // --backup takes precedence over -b
    #[test]
    fn test_backup_mode_long_preferred_over_short() {
        let matches = make_app().get_matches_from(vec!["command", "-b", "--backup=none"]);
        let result = determine_backup_mode(None, &matches).unwrap();
        assert_eq!(result, BackupMode::None);
    }

    // --backup can be passed without an argument
    #[test]
    fn test_backup_mode_long_without_args_no_env() {
        let matches = make_app().get_matches_from(vec!["command", "--backup"]);
        let result = determine_backup_mode(None, &matches).unwrap();
        assert_eq!(result, BackupMode::Existing);
    }

    // --backup can be passed with an argument only
    #[test]
    fn test_backup_mode_long_with_args() {
        let matches = make_app().get_matches_from(vec!["command", "--backup=simple"]);
        let result = determine_backup_mode(None, &matches).unwrap();
        assert_eq!(result, BackupMode::Simple);
    }

    // --backup errors on invalid argument
    #[test]
    fn test_backup_mode_long_with_args_invalid() {
        let matches = make_app().get_matches_from(vec!["command", "--backup=foobar"]);
        let result = determine_backup_mode(None, &matches);
        assert!(result.is_err());
        let text = format!("{}", result.unwrap_err());
        assert!(text.contains("invalid argument 'foobar' for 'backup type'"));
    }

    // --backup errors on ambiguous argument
    #[test]
    fn test_backup_mode_long_with_args_ambiguous() {
        let matches = make_app().get_matches_from(vec!["command", "--backup=n"]);
        let result = determine_backup_mode(None, &matches);
        assert!(result.is_err());
        let text = format!("{}", result.unwrap_err());
        assert!(text.contains("ambiguous argument 'n' for 'backup type'"));
    }

    // --backup accepts shortened arguments (si for simple)
    #[test]
    fn test_backup_mode_long_with_arg_shortened() {
        let matches = make_app().get_matches_from(vec!["command", "--backup=si"]);
        let result = determine_backup_mode(None, &matches).unwrap();
        assert_eq!(result, BackupMode::Simple);
    }

    // -b doesn't ignores the "VERSION_CONTROL" environment variable
    #[test]
    fn test_backup_mode_short_does_not_ignore_env() {
        let matches = make_app().get_matches_from(vec!["command", "-b"]);
        let result = determine_backup_mode(Some("numbered".into()), &matches).unwrap();
        assert_eq!(result, BackupMode::Numbered);
    }

    // --backup can be passed without an argument, but reads env var if existent
    #[test]
    fn test_backup_mode_long_without_args_with_env() {
        let matches = make_app().get_matches_from(vec!["command", "--backup"]);
        let result = determine_backup_mode(Some("none".into()), &matches).unwrap();
        assert_eq!(result, BackupMode::None);
    }

    // --backup errors on invalid VERSION_CONTROL env var
    #[test]
    fn test_backup_mode_long_with_env_var_invalid() {
        let matches = make_app().get_matches_from(vec!["command", "--backup"]);
        let result = determine_backup_mode(Some("foobar".into()), &matches);
        assert!(result.is_err());
        let text = format!("{}", result.unwrap_err());
        assert!(text.contains("invalid argument 'foobar' for '$VERSION_CONTROL'"));
    }

    // --backup errors on ambiguous VERSION_CONTROL env var
    #[test]
    fn test_backup_mode_long_with_env_var_ambiguous() {
        let matches = make_app().get_matches_from(vec!["command", "--backup"]);
        let result = determine_backup_mode(Some("n".into()), &matches);
        assert!(result.is_err());
        let text = format!("{}", result.unwrap_err());
        assert!(text.contains("ambiguous argument 'n' for '$VERSION_CONTROL'"));
    }

    // --backup accepts shortened env vars (si for simple)
    #[test]
    fn test_backup_mode_long_with_env_var_shortened() {
        let matches = make_app().get_matches_from(vec!["command", "--backup"]);
        let result = determine_backup_mode(Some("si".into()), &matches).unwrap();
        assert_eq!(result, BackupMode::Simple);
    }

    // Using --suffix without --backup defaults to --backup=existing
    #[test]
    fn test_backup_mode_suffix_without_backup_option() {
        let matches = make_app().get_matches_from(vec!["command", "--suffix", ".bak"]);
        let result = determine_backup_mode(None, &matches).unwrap();
        assert_eq!(result, BackupMode::Existing);
    }

    // Using --suffix without --backup uses env var if existing
    #[test]
    fn test_backup_mode_suffix_without_backup_option_with_env_var() {
        let matches = make_app().get_matches_from(vec!["command", "--suffix", ".bak"]);
        let result = determine_backup_mode(Some("numbered".into()), &matches).unwrap();
        assert_eq!(result, BackupMode::Numbered);
    }

    #[test]
    fn test_suffix_takes_hyphen_value() {
        let matches = make_app().get_matches_from(vec!["command", "-b", "--suffix", "-v"]);
        let result = determine_backup_suffix(&matches);
        assert_eq!(result, "-v");
    }

    #[test]
    fn test_suffix_rejects_path_traversal() {
        let matches =
            make_app().get_matches_from(vec!["command", "-b", "--suffix", "_/../../dest"]);
        let result = determine_backup_suffix(&matches);
        assert_eq!(result, DEFAULT_BACKUP_SUFFIX);
    }

    #[test]
    fn test_numbered_backup_path() {
        assert_eq!(numbered_backup_path(Path::new("")), PathBuf::from(".~1~"));
        assert_eq!(numbered_backup_path(Path::new("/")), PathBuf::from("/.~1~"));
        assert_eq!(
            numbered_backup_path(Path::new("/hello/world")),
            PathBuf::from("/hello/world.~1~")
        );
        assert_eq!(
            numbered_backup_path(Path::new("/hello/world/")),
            PathBuf::from("/hello/world.~1~")
        );
    }

    #[test]
    fn test_simple_backup_path() {
        assert_eq!(
            simple_backup_path(Path::new(""), ".bak"),
            PathBuf::from(".bak")
        );
        assert_eq!(
            simple_backup_path(Path::new("/"), ".bak"),
            PathBuf::from("/.bak")
        );
        assert_eq!(
            simple_backup_path(Path::new("/hello/world"), ".bak"),
            PathBuf::from("/hello/world.bak")
        );
        assert_eq!(
            simple_backup_path(Path::new("/hello/world/"), ".bak"),
            PathBuf::from("/hello/world.bak")
        );
    }

    #[test]
    fn test_backup_would_destroy_source() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("data.txt");
        let source = dir.path().join("data.txt.bak");
        fs::write(&target, "").unwrap();
        fs::write(&source, "").unwrap();

        assert!(backup_would_destroy_source(
            &source,
            &target,
            ".bak",
            BackupMode::Simple,
            false
        ));
    }

    #[test]
    fn test_source_is_not_target_backup() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("backup.txt");
        let source = dir.path().join("data.txt");
        fs::write(&target, "").unwrap();
        fs::write(&source, "").unwrap();

        assert!(!backup_would_destroy_source(
            &source,
            &target,
            ".bak",
            BackupMode::Simple,
            false
        ));
    }

    #[test]
    fn test_backup_would_destroy_source_with_tilde_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("example");
        let source = dir.path().join("example~");
        fs::write(&target, "").unwrap();
        fs::write(&source, "").unwrap();

        assert!(backup_would_destroy_source(
            &source,
            &target,
            "~",
            BackupMode::Simple,
            false
        ));
    }

    /// The guard must see through how the operands were spelled.
    #[test]
    fn test_backup_would_destroy_source_ignores_spelling() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a"), "").unwrap();
        fs::write(dir.path().join("a~"), "").unwrap();

        for target in ["a", "./a"] {
            let source = dir.path().join("a~");
            let target = dir.path().join(target);
            assert!(
                backup_would_destroy_source(&source, &target, "~", BackupMode::Simple, false),
                "guard failed open for target spelled {}",
                target.display()
            );
        }
    }

    /// A backup that does not exist yet cannot destroy anything.
    #[test]
    fn test_backup_would_destroy_source_absent_backup() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a");
        let source = dir.path().join("a~");
        fs::write(&target, "").unwrap();

        assert!(!backup_would_destroy_source(
            &source,
            &target,
            "~",
            BackupMode::Simple,
            false
        ));
    }

    /// Sharing the backup's inode under a different name is safe: the rename
    /// only drops one link, so the data survives.
    #[cfg(unix)]
    #[test]
    fn test_backup_would_destroy_source_mode_gate() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("data.txt");
        let source = dir.path().join("data.txt~");
        fs::write(&target, "").unwrap();
        fs::write(&source, "").unwrap();

        // Modes that rename onto an existing name would destroy the source.
        for mode in [BackupMode::Simple, BackupMode::Existing] {
            assert!(
                backup_would_destroy_source(&source, &target, "~", mode, false),
                "{mode:?} should be guarded"
            );
        }

        // Numbered picks a fresh name; None renames nothing at all.
        for mode in [BackupMode::Numbered, BackupMode::None] {
            assert!(
                !backup_would_destroy_source(&source, &target, "~", mode, false),
                "{mode:?} cannot destroy the source"
            );
        }
    }

    #[test]
    fn test_backup_would_destroy_source_hard_link_under_other_name() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a");
        let backup = dir.path().join("a~");
        let source = dir.path().join("b");
        fs::write(&target, "").unwrap();
        fs::write(&backup, "").unwrap();
        fs::hard_link(&backup, &source).unwrap();

        assert!(!backup_would_destroy_source(
            &source,
            &target,
            "~",
            BackupMode::Simple,
            false
        ));
    }
}
