use std::{
    env,
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BackupMode {
    NoBackup,
    SimpleBackup,
    NumberedBackup,
    ExistingBackup,
}

pub mod arguments {
    extern crate clap;

    pub static OPT_BACKUP: &str = "backupopt_backup";
    pub static OPT_BACKUP_NO_ARG: &str = "backupopt_b";
    pub static OPT_SUFFIX: &str = "backupopt_suffix";

    pub fn backup() -> clap::Arg<'static, 'static> {
        clap::Arg::with_name(OPT_BACKUP)
            .long("backup")
            .help("make a backup of each existing destination file")
            .takes_value(true)
            .require_equals(true)
            .min_values(0)
            .value_name("CONTROL")
    }

    pub fn backup_no_args() -> clap::Arg<'static, 'static> {
        clap::Arg::with_name(OPT_BACKUP_NO_ARG)
            .short("b")
            .help("like --backup but does not accept an argument")
    }

    pub fn suffix() -> clap::Arg<'static, 'static> {
        clap::Arg::with_name(OPT_SUFFIX)
            .short("S")
            .long("suffix")
            .help("override the usual backup suffix")
            .takes_value(true)
            .value_name("SUFFIX")
    }
}

pub fn determine_backup_suffix(supplied_suffix: Option<&str>) -> String {
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
/// For an explanation of what the arguments mean, refer to the examples below.
///
/// [1]: https://www.gnu.org/software/coreutils/manual/html_node/Backup-options.html
///
///
/// # Errors
///
/// If an argument supplied directly to the long `backup` option, or read in
/// through the `VERSION CONTROL` env var is ambiguous (i.e. may resolve to
/// multiple backup modes) or invalid, an error is returned. The error contains
/// the formatted error string which may then be passed to the
/// [`show_usage_error`] macro.
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
/// use clap::{App, Arg};
///
/// fn main() {
///     let OPT_BACKUP: &str = "backup";
///     let OPT_BACKUP_NO_ARG: &str = "b";
///     let matches = App::new("app")
///         .arg(Arg::with_name(OPT_BACKUP_NO_ARG)
///             .short(OPT_BACKUP_NO_ARG))
///         .arg(Arg::with_name(OPT_BACKUP)
///             .long(OPT_BACKUP)
///             .takes_value(true)
///             .require_equals(true)
///             .min_values(0))
///         .get_matches_from(vec![
///             "app", "-b", "--backup=t"
///         ]);
///    
///     let backup_mode = backup_control::determine_backup_mode(
///         matches.is_present(OPT_BACKUP_NO_ARG), matches.is_present(OPT_BACKUP),
///         matches.value_of(OPT_BACKUP)
///     );
///     let backup_mode = match backup_mode {
///         Err(err) => {
///             show_usage_error!("{}", err);
///             return;
///         },
///         Ok(mode) => mode,
///     };
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
/// use uucore::backup_control::{self, BackupMode};
/// use clap::{crate_version, App, Arg, ArgMatches};
///
/// fn main() {
///     let OPT_BACKUP: &str = "backup";
///     let OPT_BACKUP_NO_ARG: &str = "b";
///     let matches = App::new("app")
///         .arg(Arg::with_name(OPT_BACKUP_NO_ARG)
///             .short(OPT_BACKUP_NO_ARG))
///         .arg(Arg::with_name(OPT_BACKUP)
///             .long(OPT_BACKUP)
///             .takes_value(true)
///             .require_equals(true)
///             .min_values(0))
///         .get_matches_from(vec![
///             "app", "-b", "--backup=n"
///         ]);
///    
///     let backup_mode = backup_control::determine_backup_mode(
///         matches.is_present(OPT_BACKUP_NO_ARG), matches.is_present(OPT_BACKUP),
///         matches.value_of(OPT_BACKUP)
///     );
///     let backup_mode = match backup_mode {
///         Err(err) => {
///             show_usage_error!("{}", err);
///             return;
///         },
///         Ok(mode) => mode,
///     };
/// }
/// ```
pub fn determine_backup_mode(
    short_opt_present: bool,
    long_opt_present: bool,
    long_opt_value: Option<&str>,
) -> Result<BackupMode, String> {
    if long_opt_present {
        // Use method to determine the type of backups to make. When this option
        // is used but method is not specified, then the value of the
        // VERSION_CONTROL environment variable is used. And if VERSION_CONTROL
        // is not set, the default backup type is ‘existing’.
        if let Some(method) = long_opt_value {
            // Second argument is for the error string that is returned.
            match_method(method, "backup type")
        } else if let Ok(method) = env::var("VERSION_CONTROL") {
            // Second argument is for the error string that is returned.
            match_method(&method, "$VERSION_CONTROL")
        } else {
            Ok(BackupMode::ExistingBackup)
        }
    } else if short_opt_present {
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
/// If `method` is ambiguous (i.e. may resolve to multiple backup modes) or
/// invalid, an error is returned. The error contains the formatted error string
/// which may then be passed to the [`show_usage_error`] macro.
fn match_method(method: &str, origin: &str) -> Result<BackupMode, String> {
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
    } else {
        let error_type = if matches.is_empty() {
            "invalid"
        } else {
            "ambiguous"
        };
        Err(format!(
            "{0} argument ‘{1}’ for ‘{2}’
Valid arguments are:
  - ‘none’, ‘off’
  - ‘simple’, ‘never’
  - ‘existing’, ‘nil’
  - ‘numbered’, ‘t’",
            error_type, method, origin
        ))
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

    // Defaults to --backup=existing
    #[test]
    fn test_backup_mode_short_only() {
        let short_opt_present = true;
        let long_opt_present = false;
        let long_opt_value = None;
        let _dummy = TEST_MUTEX.lock().unwrap();

        let result =
            determine_backup_mode(short_opt_present, long_opt_present, long_opt_value).unwrap();

        assert_eq!(result, BackupMode::ExistingBackup);
    }

    // --backup takes precedence over -b
    #[test]
    fn test_backup_mode_long_preferred_over_short() {
        let short_opt_present = true;
        let long_opt_present = true;
        let long_opt_value = Some("none");
        let _dummy = TEST_MUTEX.lock().unwrap();

        let result =
            determine_backup_mode(short_opt_present, long_opt_present, long_opt_value).unwrap();

        assert_eq!(result, BackupMode::NoBackup);
    }

    // --backup can be passed without an argument
    #[test]
    fn test_backup_mode_long_without_args_no_env() {
        let short_opt_present = false;
        let long_opt_present = true;
        let long_opt_value = None;
        let _dummy = TEST_MUTEX.lock().unwrap();

        let result =
            determine_backup_mode(short_opt_present, long_opt_present, long_opt_value).unwrap();

        assert_eq!(result, BackupMode::ExistingBackup);
    }

    // --backup can be passed with an argument only
    #[test]
    fn test_backup_mode_long_with_args() {
        let short_opt_present = false;
        let long_opt_present = true;
        let long_opt_value = Some("simple");
        let _dummy = TEST_MUTEX.lock().unwrap();

        let result =
            determine_backup_mode(short_opt_present, long_opt_present, long_opt_value).unwrap();

        assert_eq!(result, BackupMode::SimpleBackup);
    }

    // --backup errors on invalid argument
    #[test]
    fn test_backup_mode_long_with_args_invalid() {
        let short_opt_present = false;
        let long_opt_present = true;
        let long_opt_value = Some("foobar");
        let _dummy = TEST_MUTEX.lock().unwrap();

        let result = determine_backup_mode(short_opt_present, long_opt_present, long_opt_value);

        assert!(result.is_err());
        let text = result.unwrap_err();
        assert!(text.contains("invalid argument ‘foobar’ for ‘backup type’"));
    }

    // --backup errors on ambiguous argument
    #[test]
    fn test_backup_mode_long_with_args_ambiguous() {
        let short_opt_present = false;
        let long_opt_present = true;
        let long_opt_value = Some("n");
        let _dummy = TEST_MUTEX.lock().unwrap();

        let result = determine_backup_mode(short_opt_present, long_opt_present, long_opt_value);

        assert!(result.is_err());
        let text = result.unwrap_err();
        assert!(text.contains("ambiguous argument ‘n’ for ‘backup type’"));
    }

    // --backup accepts shortened arguments (si for simple)
    #[test]
    fn test_backup_mode_long_with_arg_shortened() {
        let short_opt_present = false;
        let long_opt_present = true;
        let long_opt_value = Some("si");
        let _dummy = TEST_MUTEX.lock().unwrap();

        let result =
            determine_backup_mode(short_opt_present, long_opt_present, long_opt_value).unwrap();

        assert_eq!(result, BackupMode::SimpleBackup);
    }

    // -b ignores the "VERSION_CONTROL" environment variable
    #[test]
    fn test_backup_mode_short_only_ignore_env() {
        let short_opt_present = true;
        let long_opt_present = false;
        let long_opt_value = None;
        let _dummy = TEST_MUTEX.lock().unwrap();
        env::set_var(ENV_VERSION_CONTROL, "none");

        let result =
            determine_backup_mode(short_opt_present, long_opt_present, long_opt_value).unwrap();

        assert_eq!(result, BackupMode::ExistingBackup);
        env::remove_var(ENV_VERSION_CONTROL);
    }

    // --backup can be passed without an argument, but reads env var if existent
    #[test]
    fn test_backup_mode_long_without_args_with_env() {
        let short_opt_present = false;
        let long_opt_present = true;
        let long_opt_value = None;
        let _dummy = TEST_MUTEX.lock().unwrap();
        env::set_var(ENV_VERSION_CONTROL, "none");

        let result =
            determine_backup_mode(short_opt_present, long_opt_present, long_opt_value).unwrap();

        assert_eq!(result, BackupMode::NoBackup);
        env::remove_var(ENV_VERSION_CONTROL);
    }

    // --backup errors on invalid VERSION_CONTROL env var
    #[test]
    fn test_backup_mode_long_with_env_var_invalid() {
        let short_opt_present = false;
        let long_opt_present = true;
        let long_opt_value = None;
        let _dummy = TEST_MUTEX.lock().unwrap();
        env::set_var(ENV_VERSION_CONTROL, "foobar");

        let result = determine_backup_mode(short_opt_present, long_opt_present, long_opt_value);

        assert!(result.is_err());
        let text = result.unwrap_err();
        assert!(text.contains("invalid argument ‘foobar’ for ‘$VERSION_CONTROL’"));
        env::remove_var(ENV_VERSION_CONTROL);
    }

    // --backup errors on ambiguous VERSION_CONTROL env var
    #[test]
    fn test_backup_mode_long_with_env_var_ambiguous() {
        let short_opt_present = false;
        let long_opt_present = true;
        let long_opt_value = None;
        let _dummy = TEST_MUTEX.lock().unwrap();
        env::set_var(ENV_VERSION_CONTROL, "n");

        let result = determine_backup_mode(short_opt_present, long_opt_present, long_opt_value);

        assert!(result.is_err());
        let text = result.unwrap_err();
        assert!(text.contains("ambiguous argument ‘n’ for ‘$VERSION_CONTROL’"));
        env::remove_var(ENV_VERSION_CONTROL);
    }

    // --backup accepts shortened env vars (si for simple)
    #[test]
    fn test_backup_mode_long_with_env_var_shortened() {
        let short_opt_present = false;
        let long_opt_present = true;
        let long_opt_value = None;
        let _dummy = TEST_MUTEX.lock().unwrap();
        env::set_var(ENV_VERSION_CONTROL, "si");

        let result =
            determine_backup_mode(short_opt_present, long_opt_present, long_opt_value).unwrap();

        assert_eq!(result, BackupMode::SimpleBackup);
        env::remove_var(ENV_VERSION_CONTROL);
    }
}
