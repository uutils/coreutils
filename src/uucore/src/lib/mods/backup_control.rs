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
//!         .override_usage(&usage[..])
//!         .after_help(&*format!(
//!             "{}\n{}",
//!             long_usage,
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
    fmt::{Debug, Display},
    path::{Path, PathBuf},
};

pub static BACKUP_CONTROL_VALUES: &[&str] = &[
    "simple", "never", "numbered", "t", "existing", "nil", "none", "off",
];

pub static BACKUP_CONTROL_LONG_HELP: &str =
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

/// Available backup modes.
///
/// The mapping of the backup modes to the CLI arguments is annotated on the
/// enum variants.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BackupMode {
    /// Argument 'none', 'off'
    NoBackup,
    /// Argument 'simple', 'never'
    SimpleBackup,
    /// Argument 'numbered', 't'
    NumberedBackup,
    /// Argument 'existing', 'nil'
    ExistingBackup,
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
            BackupError::BackupImpossible() => 2,
            _ => 1,
        }
    }

    fn usage(&self) -> bool {
        // Suggested by clippy.
        matches!(
            self,
            BackupError::InvalidArgument(_, _) | BackupError::AmbiguousArgument(_, _)
        )
    }
}

impl Error for BackupError {}

impl Display for BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use BackupError as BE;
        match self {
            BE::InvalidArgument(arg, origin) => write!(
                f,
                "invalid argument {} for '{}'\n{}",
                arg.quote(),
                origin,
                VALID_ARGS_HELP
            ),
            BE::AmbiguousArgument(arg, origin) => write!(
                f,
                "ambiguous argument {} for '{}'\n{}",
                arg.quote(),
                origin,
                VALID_ARGS_HELP
            ),
            BE::BackupImpossible() => write!(f, "cannot create backup"),
            // Placeholder for later
            // BE::BackupFailed(from, to, e) => Display::fmt(
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
    extern crate clap;

    pub static OPT_BACKUP: &str = "backupopt_backup";
    pub static OPT_BACKUP_NO_ARG: &str = "backupopt_b";
    pub static OPT_SUFFIX: &str = "backupopt_suffix";

    /// '--backup' argument
    pub fn backup<'a>() -> clap::Arg<'a> {
        clap::Arg::new(OPT_BACKUP)
            .long("backup")
            .help("make a backup of each existing destination file")
            .takes_value(true)
            .require_equals(true)
            .min_values(0)
            .value_name("CONTROL")
    }

    /// '-b' argument
    pub fn backup_no_args<'a>() -> clap::Arg<'a> {
        clap::Arg::new(OPT_BACKUP_NO_ARG)
            .short('b')
            .help("like --backup but does not accept an argument")
    }

    /// '-S, --suffix' argument
    pub fn suffix<'a>() -> clap::Arg<'a> {
        clap::Arg::new(OPT_SUFFIX)
            .short('S')
            .long("suffix")
            .help("override the usual backup suffix")
            .takes_value(true)
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
/// 3. By using the default '~' if none of the others apply
///
/// This function directly takes [`clap::ArgMatches`] as argument and looks for
/// the '-S' and '--suffix' arguments itself.
pub fn determine_backup_suffix(matches: &ArgMatches) -> String {
    let supplied_suffix = matches.value_of(arguments::OPT_SUFFIX);
    if let Some(suffix) = supplied_suffix {
        String::from(suffix)
    } else {
        env::var("SIMPLE_BACKUP_SUFFIX").unwrap_or_else(|_| "~".to_owned())
    }
}

/// Determine the "mode" for the backup operation to perform, if any.
///
/// Parses the backup options according to the [GNU manual][1], and converts
/// them to an instance of `BackupMode` for further processing.
///
/// Takes [`clap::ArgMatches`] as argument which **must** contain the options
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
///     assert_eq!(backup_mode, BackupMode::NumberedBackup)
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
    if matches.is_present(arguments::OPT_BACKUP) {
        // Use method to determine the type of backups to make. When this option
        // is used but method is not specified, then the value of the
        // VERSION_CONTROL environment variable is used. And if VERSION_CONTROL
        // is not set, the default backup type is 'existing'.
        if let Some(method) = matches.value_of(arguments::OPT_BACKUP) {
            // Second argument is for the error string that is returned.
            match_method(method, "backup type")
        } else if let Ok(method) = env::var("VERSION_CONTROL") {
            // Second argument is for the error string that is returned.
            match_method(&method, "$VERSION_CONTROL")
        } else {
            // Default if no argument is provided to '--backup'
            Ok(BackupMode::ExistingBackup)
        }
    } else if matches.is_present(arguments::OPT_BACKUP_NO_ARG) {
        // the short form of this option, -b does not accept any argument.
        // Using -b is equivalent to using --backup=existing.
        Ok(BackupMode::ExistingBackup)
    } else {
        // No option was present at all
        Ok(BackupMode::NoBackup)
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
            "simple" | "never" => Ok(BackupMode::SimpleBackup),
            "numbered" | "t" => Ok(BackupMode::NumberedBackup),
            "existing" | "nil" => Ok(BackupMode::ExistingBackup),
            "none" | "off" => Ok(BackupMode::NoBackup),
            _ => unreachable!(), // cannot happen as we must have exactly one match
                                 // from the list above.
        }
    } else if matches.is_empty() {
        Err(BackupError::InvalidArgument(method.to_string(), origin.to_string()).into())
    } else {
        Err(BackupError::AmbiguousArgument(method.to_string(), origin.to_string()).into())
    }
}

pub fn get_backup_path(
    backup_mode: BackupMode,
    backup_path: &Path,
    suffix: &str,
) -> Option<PathBuf> {
    match backup_mode {
        BackupMode::NoBackup => None,
        BackupMode::SimpleBackup => Some(simple_backup_path(backup_path, suffix)),
        BackupMode::NumberedBackup => Some(numbered_backup_path(backup_path)),
        BackupMode::ExistingBackup => Some(existing_backup_path(backup_path, suffix)),
    }
}

fn simple_backup_path(path: &Path, suffix: &str) -> PathBuf {
    let mut p = path.to_string_lossy().into_owned();
    p.push_str(suffix);
    PathBuf::from(p)
}

fn numbered_backup_path(path: &Path) -> PathBuf {
    for i in 1_u64.. {
        let path_str = &format!("{}.~{}~", path.to_string_lossy(), i);
        let path = Path::new(path_str);
        if !path.exists() {
            return path.to_path_buf();
        }
    }
    panic!("cannot create backup")
}

fn existing_backup_path(path: &Path, suffix: &str) -> PathBuf {
    let test_path_str = &format!("{}.~1~", path.to_string_lossy());
    let test_path = Path::new(test_path_str);
    if test_path.exists() {
        numbered_backup_path(path)
    } else {
        simple_backup_path(path, suffix)
    }
}

//
// Tests for this module
//
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    // Required to instantiate mutex in shared context
    use clap::Command;
    use lazy_static::lazy_static;
    use std::sync::Mutex;

    // The mutex is required here as by default all tests are run as separate
    // threads under the same parent process. As environment variables are
    // specific to processes (and thus shared among threads), data races *will*
    // occur if no precautions are taken. Thus we have all tests that rely on
    // environment variables lock this empty mutex to ensure they don't access
    // it concurrently.
    lazy_static! {
        static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
    }

    // Environment variable for "VERSION_CONTROL"
    static ENV_VERSION_CONTROL: &str = "VERSION_CONTROL";

    fn make_app() -> clap::Command<'static> {
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

        assert_eq!(result, BackupMode::ExistingBackup);
    }

    // --backup takes precedence over -b
    #[test]
    fn test_backup_mode_long_preferred_over_short() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        let matches = make_app().get_matches_from(vec!["command", "-b", "--backup=none"]);

        let result = determine_backup_mode(&matches).unwrap();

        assert_eq!(result, BackupMode::NoBackup);
    }

    // --backup can be passed without an argument
    #[test]
    fn test_backup_mode_long_without_args_no_env() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        let matches = make_app().get_matches_from(vec!["command", "--backup"]);

        let result = determine_backup_mode(&matches).unwrap();

        assert_eq!(result, BackupMode::ExistingBackup);
    }

    // --backup can be passed with an argument only
    #[test]
    fn test_backup_mode_long_with_args() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        let matches = make_app().get_matches_from(vec!["command", "--backup=simple"]);

        let result = determine_backup_mode(&matches).unwrap();

        assert_eq!(result, BackupMode::SimpleBackup);
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

        assert_eq!(result, BackupMode::SimpleBackup);
    }

    // -b ignores the "VERSION_CONTROL" environment variable
    #[test]
    fn test_backup_mode_short_only_ignore_env() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        env::set_var(ENV_VERSION_CONTROL, "none");
        let matches = make_app().get_matches_from(vec!["command", "-b"]);

        let result = determine_backup_mode(&matches).unwrap();

        assert_eq!(result, BackupMode::ExistingBackup);
        env::remove_var(ENV_VERSION_CONTROL);
    }

    // --backup can be passed without an argument, but reads env var if existent
    #[test]
    fn test_backup_mode_long_without_args_with_env() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        env::set_var(ENV_VERSION_CONTROL, "none");
        let matches = make_app().get_matches_from(vec!["command", "--backup"]);

        let result = determine_backup_mode(&matches).unwrap();

        assert_eq!(result, BackupMode::NoBackup);
        env::remove_var(ENV_VERSION_CONTROL);
    }

    // --backup errors on invalid VERSION_CONTROL env var
    #[test]
    fn test_backup_mode_long_with_env_var_invalid() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        env::set_var(ENV_VERSION_CONTROL, "foobar");
        let matches = make_app().get_matches_from(vec!["command", "--backup"]);

        let result = determine_backup_mode(&matches);

        assert!(result.is_err());
        let text = format!("{}", result.unwrap_err());
        assert!(text.contains("invalid argument 'foobar' for '$VERSION_CONTROL'"));
        env::remove_var(ENV_VERSION_CONTROL);
    }

    // --backup errors on ambiguous VERSION_CONTROL env var
    #[test]
    fn test_backup_mode_long_with_env_var_ambiguous() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        env::set_var(ENV_VERSION_CONTROL, "n");
        let matches = make_app().get_matches_from(vec!["command", "--backup"]);

        let result = determine_backup_mode(&matches);

        assert!(result.is_err());
        let text = format!("{}", result.unwrap_err());
        assert!(text.contains("ambiguous argument 'n' for '$VERSION_CONTROL'"));
        env::remove_var(ENV_VERSION_CONTROL);
    }

    // --backup accepts shortened env vars (si for simple)
    #[test]
    fn test_backup_mode_long_with_env_var_shortened() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        env::set_var(ENV_VERSION_CONTROL, "si");
        let matches = make_app().get_matches_from(vec!["command", "--backup"]);

        let result = determine_backup_mode(&matches).unwrap();

        assert_eq!(result, BackupMode::SimpleBackup);
        env::remove_var(ENV_VERSION_CONTROL);
    }

    #[test]
    fn test_suffix_takes_hyphen_value() {
        let _dummy = TEST_MUTEX.lock().unwrap();
        let matches = make_app().get_matches_from(vec!["command", "-b", "--suffix", "-v"]);

        let result = determine_backup_suffix(&matches);
        assert_eq!(result, "-v");
    }
}
