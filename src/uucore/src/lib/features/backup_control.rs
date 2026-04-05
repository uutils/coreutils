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
//!     let backup_mode = match backup_control::determine_backup_mode(&matches) {
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
};
use clap::ArgMatches;
use std::{
    env,
    error::Error,
    ffi::{OsStr, OsString},
    fmt::{Debug, Display},
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
///     let backup_mode = backup_control::determine_backup_mode(&matches).unwrap();
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
///     let backup_mode = backup_control::determine_backup_mode(&matches);
///
///     assert!(backup_mode.is_err());
///     let err = backup_mode.unwrap_err();
///     // assert_eq!(err, BackupError::AmbiguousArgument);
///     // Use uucore functionality to show the error to the user
///     show!(err);
/// }
/// ```
pub fn determine_backup_mode(matches: &ArgMatches) -> UResult<BackupMode> {
    if matches.contains_id(arguments::OPT_BACKUP) {
        // Use method to determine the type of backups to make. When this option
        // is used but method is not specified, then the value of the
        // VERSION_CONTROL environment variable is used. And if VERSION_CONTROL
        // is not set, the default backup type is 'existing'.
        if let Some(method) = matches.get_one::<String>(arguments::OPT_BACKUP) {
            // Second argument is for the error string that is returned.
            match_method(method, "backup type")
        } else if let Ok(method) = env::var("VERSION_CONTROL") {
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
        if let Ok(method) = env::var("VERSION_CONTROL") {
            match_method(&method, "$VERSION_CONTROL")
        } else {
            Ok(BackupMode::Existing)
        }
    } else if matches.contains_id(arguments::OPT_SUFFIX) {
        // Suffix option is enough to determine mode even if --backup is not set.
        // If VERSION_CONTROL is not set, the default backup type is 'existing'.
        if let Ok(method) = env::var("VERSION_CONTROL") {
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
        if !new_path.exists() {
            return new_path;
        }
        i += 1;
    }
}

fn existing_backup_path<S: AsRef<OsStr>>(path: &Path, suffix: S) -> PathBuf {
    let test_path = simple_backup_path(path, OsString::from(".~1~"));
    if test_path.exists() {
        return numbered_backup_path(path);
    }
    simple_backup_path(path, suffix.as_ref())
}

/// Returns true if the source file is likely to be the simple backup file for the target file.
///
/// # Arguments
///
/// * `source` - A Path reference that holds the source (backup) file path.
/// * `target` - A Path reference that holds the target file path.
/// * `suffix` - Str that holds the backup suffix.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use uucore::backup_control::source_is_target_backup;
/// let source = Path::new("data.txt~");
/// let target = Path::new("data.txt");
/// let suffix = String::from("~");
///
/// assert_eq!(source_is_target_backup(&source, &target, &suffix), true);
/// ```
///
pub fn source_is_target_backup(source: &Path, target: &Path, suffix: &str) -> bool {
    let source_filename = source.as_os_str();
    let mut target_backup_filename = target.as_os_str().to_owned();
    target_backup_filename.push(suffix);
    source_filename == target_backup_filename
}

//
// Tests for this module
//
#[cfg(test)]
mod tests {
    use super::*;
    // Required to instantiate mutex in shared context
    use clap::Command;
    use std::sync::Mutex;

    // The mutex is required here as by default all tests are run as separate
    // threads under the same parent process. As environment variables are
    // specific to processes (and thus shared among threads), data races *will*
    // occur if no precautions are taken. Thus we have all tests that rely on
    // environment variables lock this empty mutex to ensure they don't access
    // it concurrently.
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    // Environment variable for "VERSION_CONTROL"
    static ENV_VERSION_CONTROL: &str = "VERSION_CONTROL";

    fn make_app() -> Command {
        Command::new("command")
            .arg(arguments::backup())
            .arg(arguments::backup_no_args())
            .arg(arguments::suffix())
    }

    // Defaults to --backup=existing
    #[test]
    fn test_backup_mode_short_only() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        let matches = make_app().get_matches_from(vec!["command", "-b"]);

        let result = determine_backup_mode(&matches).unwrap();

        assert_eq!(result, BackupMode::Existing);
    }

    // --backup takes precedence over -b
    #[test]
    fn test_backup_mode_long_preferred_over_short() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        let matches = make_app().get_matches_from(vec!["command", "-b", "--backup=none"]);

        let result = determine_backup_mode(&matches).unwrap();

        assert_eq!(result, BackupMode::None);
    }

    // --backup can be passed without an argument
    #[test]
    fn test_backup_mode_long_without_args_no_env() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        let matches = make_app().get_matches_from(vec!["command", "--backup"]);

        let result = determine_backup_mode(&matches).unwrap();

        assert_eq!(result, BackupMode::Existing);
    }

    // --backup can be passed with an argument only
    #[test]
    fn test_backup_mode_long_with_args() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        let matches = make_app().get_matches_from(vec!["command", "--backup=simple"]);

        let result = determine_backup_mode(&matches).unwrap();

        assert_eq!(result, BackupMode::Simple);
    }

    // --backup errors on invalid argument
    #[test]
    fn test_backup_mode_long_with_args_invalid() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        let matches = make_app().get_matches_from(vec!["command", "--backup=foobar"]);

        let result = determine_backup_mode(&matches);

        assert!(result.is_err());
        let text = format!("{}", result.unwrap_err());
        assert!(text.contains("invalid argument 'foobar' for 'backup type'"));
    }

    // --backup errors on ambiguous argument
    #[test]
    fn test_backup_mode_long_with_args_ambiguous() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        let matches = make_app().get_matches_from(vec!["command", "--backup=n"]);

        let result = determine_backup_mode(&matches);

        assert!(result.is_err());
        let text = format!("{}", result.unwrap_err());
        assert!(text.contains("ambiguous argument 'n' for 'backup type'"));
    }

    // --backup accepts shortened arguments (si for simple)
    #[test]
    fn test_backup_mode_long_with_arg_shortened() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        let matches = make_app().get_matches_from(vec!["command", "--backup=si"]);

        let result = determine_backup_mode(&matches).unwrap();

        assert_eq!(result, BackupMode::Simple);
    }

    // -b doesn't ignores the "VERSION_CONTROL" environment variable
    #[test]
    fn test_backup_mode_short_does_not_ignore_env() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        unsafe { env::set_var(ENV_VERSION_CONTROL, "numbered") };
        let matches = make_app().get_matches_from(vec!["command", "-b"]);

        let result = determine_backup_mode(&matches).unwrap();

        assert_eq!(result, BackupMode::Numbered);
        unsafe { env::remove_var(ENV_VERSION_CONTROL) };
    }

    // --backup can be passed without an argument, but reads env var if existent
    #[test]
    fn test_backup_mode_long_without_args_with_env() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        unsafe { env::set_var(ENV_VERSION_CONTROL, "none") };
        let matches = make_app().get_matches_from(vec!["command", "--backup"]);

        let result = determine_backup_mode(&matches).unwrap();

        assert_eq!(result, BackupMode::None);
        unsafe { env::remove_var(ENV_VERSION_CONTROL) };
    }

    // --backup errors on invalid VERSION_CONTROL env var
    #[test]
    fn test_backup_mode_long_with_env_var_invalid() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        unsafe { env::set_var(ENV_VERSION_CONTROL, "foobar") };
        let matches = make_app().get_matches_from(vec!["command", "--backup"]);

        let result = determine_backup_mode(&matches);

        assert!(result.is_err());
        let text = format!("{}", result.unwrap_err());
        assert!(text.contains("invalid argument 'foobar' for '$VERSION_CONTROL'"));
        unsafe { env::remove_var(ENV_VERSION_CONTROL) };
    }

    // --backup errors on ambiguous VERSION_CONTROL env var
    #[test]
    fn test_backup_mode_long_with_env_var_ambiguous() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        unsafe { env::set_var(ENV_VERSION_CONTROL, "n") };
        let matches = make_app().get_matches_from(vec!["command", "--backup"]);

        let result = determine_backup_mode(&matches);

        assert!(result.is_err());
        let text = format!("{}", result.unwrap_err());
        assert!(text.contains("ambiguous argument 'n' for '$VERSION_CONTROL'"));
        unsafe { env::remove_var(ENV_VERSION_CONTROL) };
    }

    // --backup accepts shortened env vars (si for simple)
    #[test]
    fn test_backup_mode_long_with_env_var_shortened() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        unsafe { env::set_var(ENV_VERSION_CONTROL, "si") };
        let matches = make_app().get_matches_from(vec!["command", "--backup"]);

        let result = determine_backup_mode(&matches).unwrap();

        assert_eq!(result, BackupMode::Simple);
        unsafe { env::remove_var(ENV_VERSION_CONTROL) };
    }

    // Using --suffix without --backup defaults to --backup=existing
    #[test]
    fn test_backup_mode_suffix_without_backup_option() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        let matches = make_app().get_matches_from(vec!["command", "--suffix", ".bak"]);

        let result = determine_backup_mode(&matches).unwrap();

        assert_eq!(result, BackupMode::Existing);
    }

    // Using --suffix without --backup uses env var if existing
    #[test]
    fn test_backup_mode_suffix_without_backup_option_with_env_var() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        unsafe { env::set_var(ENV_VERSION_CONTROL, "numbered") };
        let matches = make_app().get_matches_from(vec!["command", "--suffix", ".bak"]);

        let result = determine_backup_mode(&matches).unwrap();

        assert_eq!(result, BackupMode::Numbered);
        unsafe { env::remove_var(ENV_VERSION_CONTROL) };
    }

    #[test]
    fn test_suffix_takes_hyphen_value() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        let matches = make_app().get_matches_from(vec!["command", "-b", "--suffix", "-v"]);

        let result = determine_backup_suffix(&matches);
        assert_eq!(result, "-v");
    }

    #[test]
    fn test_suffix_rejects_path_traversal() {
        let _dummy = TEST_MUTEX.lock().unwrap();
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
    fn test_source_is_target_backup() {
        let source = Path::new("data.txt.bak");
        let target = Path::new("data.txt");
        let suffix = String::from(".bak");

        assert!(source_is_target_backup(source, target, &suffix));
    }

    #[test]
    fn test_source_is_not_target_backup() {
        let source = Path::new("data.txt");
        let target = Path::new("backup.txt");
        let suffix = String::from(".bak");

        assert!(!source_is_target_backup(source, target, &suffix));
    }

    #[test]
    fn test_source_is_target_backup_with_tilde_suffix() {
        let source = Path::new("example~");
        let target = Path::new("example");
        let suffix = String::from("~");

        assert!(source_is_target_backup(source, target, &suffix));
    }
}
