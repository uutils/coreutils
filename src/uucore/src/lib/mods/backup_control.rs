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
/// let OPT_BACKUP: &str = "backup";
/// let OPT_BACKUP_NO_ARG: &str = "b";
/// let matches = App::new("myprog")
///     .arg(Arg::with_name(OPT_BACKUP_NO_ARG)
///         .short(OPT_BACKUP_NO_ARG)
///     .arg(Arg::with_name(OPT_BACKUP)
///         .long(OPT_BACKUP)
///         .takes_value(true)
///         .require_equals(true)
///         .min_values(0))
///     .get_matches_from(vec![
///         "myprog", "-b", "--backup=t"
///     ]);
///
/// let backup_mode = backup_control::determine_backup_mode(
///     matches.is_present(OPT_BACKUP_NO_ARG), matches.is_present(OPT_BACKUP),
///     matches.value_of(OPT_BACKUP)
/// );
/// let backup_mode = match backup_mode {
///     Err(err) => {
///         show_usage_error!("{}", err);
///         return 1;
///     },
///     Ok(mode) => mode,
/// };
/// ```
///
/// This example shows an ambiguous imput, as 'n' may resolve to 4 different
/// backup modes.
///
///
/// ```
/// let OPT_BACKUP: &str = "backup";
/// let OPT_BACKUP_NO_ARG: &str = "b";
/// let matches = App::new("myprog")
///     .arg(Arg::with_name(OPT_BACKUP_NO_ARG)
///         .short(OPT_BACKUP_NO_ARG)
///     .arg(Arg::with_name(OPT_BACKUP)
///         .long(OPT_BACKUP)
///         .takes_value(true)
///         .require_equals(true)
///         .min_values(0))
///     .get_matches_from(vec![
///         "myprog", "-b", "--backup=n"
///     ]);
///
/// let backup_mode = backup_control::determine_backup_mode(
///     matches.is_present(OPT_BACKUP_NO_ARG), matches.is_present(OPT_BACKUP),
///     matches.value_of(OPT_BACKUP)
/// );
/// let backup_mode = match backup_mode {
///     Err(err) => {
///         show_usage_error!("{}", err);
///         return 1;
///     },
///     Ok(mode) => mode,
/// };
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
